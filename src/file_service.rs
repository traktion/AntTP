use actix_http::header;
use actix_web::{Error, HttpRequest, HttpResponse};
use actix_web::dev::ConnectionInfo;
use actix_web::error::{ErrorBadGateway, ErrorInternalServerError, ErrorNotFound};
use actix_web::http::header::{CacheControl, CacheDirective, ContentRange, ContentRangeSpec, ContentType, ETag, EntityTag};
use async_stream::{stream};
use autonomi::{ChunkAddress, Client};
use autonomi::data::DataAddress;
use log::{info};
use xor_name::XorName;
use crate::archive_helper::DataState;
use crate::chunk_service::{ChunkService};
use crate::xor_helper::XorHelper;

pub struct FileService {
    autonomi_client: Client,
    xor_helper: XorHelper,
    conn: ConnectionInfo
}

impl FileService {
    pub fn new(autonomi_client: Client, xor_helper: XorHelper, conn: ConnectionInfo) -> Self {
        FileService { autonomi_client, xor_helper, conn }
    }

    pub async fn get_data(&self, path_parts: Vec<String>, request: HttpRequest, xor_name: XorName, is_found: bool) -> Result<HttpResponse, Error> {
        let (archive_addr, _) = self.xor_helper.assign_path_parts(path_parts.clone());
        info!("archive_addr [{}]", archive_addr);

        if self.xor_helper.get_data_state(request.headers(), &xor_name) == DataState::NotModified {
            info!("ETag matches for path [{}] at address [{}]. Client can use cached version", archive_addr, format!("{:x}", xor_name).as_str());
            Ok(HttpResponse::NotModified().into())
        } else if !is_found {
            Err(ErrorNotFound(format!("File not found {:?}", self.conn.host())))
        } else if request.headers().get(header::IF_RANGE).is_some() {
            let (range_from, range_to) = self.get_range(&request);
            self.download_data_stream(archive_addr, xor_name, false, range_from, range_to).await
        } else {
            self.download_data_body(archive_addr, xor_name, false).await
        }
    }

    pub async fn download_data_body(
        &self,
        path_str: String,
        xor_name: XorName,
        is_resolved_file_name: bool
    ) -> Result<HttpResponse, Error> {
        info!("Downloading item [{}] at addr [{}] ", path_str, format!("{:x}", xor_name));
        let data_address =  DataAddress::new(xor_name);
        match self.autonomi_client.data_get_public(&data_address).await {
            Ok(data) => {
                info!("Read [{}] bytes of item [{}] at addr [{}]", data.len(), path_str, format!("{:x}", xor_name));
                let cache_control_header = self.build_cache_control_header(&xor_name, is_resolved_file_name);
                let etag_header = ETag(EntityTag::new_strong(format!("{:x}", xor_name).to_owned()));
                let cors_allow_all = (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*");

                if path_str.ends_with(".js") {
                    Ok(HttpResponse::Ok()
                        .insert_header(cache_control_header)
                        .insert_header(etag_header)
                        .insert_header(cors_allow_all)
                        .insert_header(self.get_content_type_from_filename(path_str)) // todo: why necessary?
                        .body(data))
                } else {
                    Ok(HttpResponse::Ok()
                        .insert_header(cache_control_header)
                        .insert_header(etag_header)
                        .insert_header(cors_allow_all)
                        .body(data))
                }
            }
            Err(e) => {
                Err(ErrorInternalServerError(format!("Failed to download [{:?}]", e)))
            }
        }
    }

    pub async fn download_data_stream(
        &self,
        path_str: String,
        xor_name: XorName,
        is_resolved_file_name: bool,
        range_from: u64,
        range_to: u64
    ) -> Result<HttpResponse, Error> {
        info!("Streaming item [{}] at addr [{}], range_from [{}], range_to [{}]", path_str, format!("{:x}", xor_name), range_from, range_to);

        info!("getting data map from chunk [{:x}]", xor_name);
        let data_map_chunk = self.autonomi_client
            .chunk_get(&ChunkAddress::new(xor_name))
            .await
            .expect("chunk_get failed")
            .clone();

        let streaming_client = self.autonomi_client.clone();

        let chunk_service = ChunkService::new(streaming_client);
        let total_size = chunk_service.get_data_map_from_bytes(&data_map_chunk.value).file_size();
        
        let mut next_range_from = range_from;
        let derived_range_to = if range_to == u64::MAX { total_size as u64 - 1 } else { range_to };

        let data_stream = stream!{
            info!("sending data stream");

            let mut chunk_count = 1;
            loop {
                match chunk_service.fetch_from_data_map_chunk(&data_map_chunk.value, next_range_from, range_to).await {
                    Ok(data) => {
                        let bytes_read = data.len();
                        info!("Read [{}] bytes from file position [{}] for XOR address [{}]", bytes_read, next_range_from, xor_name);
                        if bytes_read > 0 {
                            yield Ok(data); // Sending data to the client here
                        } else {
                            info!("Last chunk [{}] read for XOR address [{}] - breaking", chunk_count, xor_name);
                            break;
                        }
                        next_range_from += bytes_read as u64;
                        chunk_count += 1;
                    }
                    Err(e) => {
                        yield Err(ErrorBadGateway(e));
                        break;
                    }
                }
            }
        };

        let cache_control_header = self.build_cache_control_header(&xor_name, is_resolved_file_name);
        let etag_header = ETag(EntityTag::new_strong(format!("{:x}", xor_name).to_owned()));
        let cors_allow_all = (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*");

        // todo: When there is only 1 known chunk, we could use body (with 'content-length: x') instead of
        //       streaming (with 'transfer-encoding: chunked') to improve performance.
        info!("return partial content");
        Ok(HttpResponse::PartialContent()
            .insert_header(ContentRange(ContentRangeSpec::Bytes { range: Some((range_from, derived_range_to)), instance_length: Some(total_size as u64) }))
            .insert_header(cache_control_header)
            .insert_header(etag_header)
            .insert_header(cors_allow_all)
            .streaming(data_stream))
    }

    pub fn get_range(&self, request: &HttpRequest) -> (u64, u64) {
        if let Some(range) = request.headers().get(header::RANGE) {
            let range_str = range.to_str().unwrap();
            info!("get range: [{}]", range_str);
            let range_value = range_str.split_once("=").unwrap().1;
            // todo: cover comma separated too: https://docs.rs/actix-web/latest/actix_web/http/header/enum.Range.html
            if let Some((range_from_str, range_to_str)) = range_value.split_once("-") {
                let range_from = range_from_str.parse::<u64>().unwrap_or_else(|_| 0);
                let range_to = range_to_str.parse::<u64>().unwrap_or_else(|_| u64::MAX);
                (range_from, range_to)
            } else {
                (0, u64::MAX)
            }
        } else {
            (0, u64::MAX)
        }
    }

    fn build_cache_control_header(&self, xor_name: &XorName, is_resolved_file_name: bool) -> CacheControl {
        if !is_resolved_file_name && self.xor_helper.is_xor(&format!("{:x}", xor_name)) {
            CacheControl(vec![CacheDirective::MaxAge(31536000u32)]) // immutable
        } else {
            CacheControl(vec![CacheDirective::MaxAge(0u32)]) // mutable
        }
    }

    fn get_content_type_from_filename(&self, filename: String) -> ContentType {
        if filename.ends_with(".js") {
            ContentType(mime::APPLICATION_JAVASCRIPT)
        } else if filename.ends_with(".html") {
            ContentType(mime::TEXT_HTML)
        } else if filename.ends_with(".css") {
            ContentType(mime::TEXT_CSS)
        } else {
            ContentType(mime::TEXT_PLAIN) // todo: use actix function to derive default
        }
    }
}