use std::path::Path;
use std::time::{Duration, SystemTime};
use actix_files::file_extension_to_mime;
use actix_http::header;
use actix_web::{Error, HttpRequest, HttpResponse};
use actix_web::dev::ConnectionInfo;
use actix_web::error::{ErrorGatewayTimeout, ErrorNotFound};
use actix_web::http::header::{CacheControl, CacheDirective, ContentRange, ContentRangeSpec, ContentType, ETag, EntityTag, Expires};
use async_stream::{stream};
use autonomi::{ChunkAddress, Client};
use log::{info};
use xor_name::XorName;
use crate::archive_helper::DataState;
use crate::chunk_service::ChunkService;
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
            let cache_control_header = self.build_cache_control_header(&xor_name, false);
            let expires_header = self.build_expires_header(&xor_name, false);
            let etag_header = ETag(EntityTag::new_strong(format!("{:x}", xor_name).to_owned()));
            let cors_allow_all = (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*");
            Ok(HttpResponse::NotModified()
                .insert_header(cache_control_header)
                .insert_header(expires_header)
                .insert_header(etag_header)
                .insert_header(cors_allow_all)
                .finish())
        } else if !is_found {
            Err(ErrorNotFound(format!("File not found {:?}", self.conn.host())))
        } else {
            self.download_data_stream(archive_addr, xor_name, false, &request).await
        }
    }

    pub async fn download_data_stream(
        &self,
        path_str: String,
        xor_name: XorName,
        is_resolved_file_name: bool,
        request: &HttpRequest,
    ) -> Result<HttpResponse, Error> {
        let (range_from, range_to, is_range_request) = self.get_range(&request);

        info!("Streaming item [{}] at addr [{}], range_from [{}], range_to [{}]", path_str, format!("{:x}", xor_name), range_from, range_to);

        info!("getting data map from chunk [{:x}]", xor_name);
        let data_map_chunk = self.autonomi_client
            .chunk_get(&ChunkAddress::new(xor_name))
            .await
            .expect("chunk_get failed")
            .clone();

        let streaming_client = self.autonomi_client.clone();

        let chunk_service = ChunkService::new(streaming_client);
        let data_map = chunk_service.get_data_map_from_bytes(&data_map_chunk.value);
        let stream_chunk_size = chunk_service.get_chunk_size_from_data_map(&data_map);
        let total_size = data_map.file_size();

        let mut next_range_from = range_from;
        let derived_range_to = if range_to == u64::MAX { total_size as u64 } else { range_to };
        let first_chunk_limit = self.get_first_chunk_limit(stream_chunk_size, next_range_from);

        let data_stream = stream!{
            info!("sending data stream");

            let mut chunk_count = 1;
            let mut tasks = Vec::new();

            // todo: limit additions to pool with FIFO queue to cap async tasks
            while next_range_from < derived_range_to {
                info!("Async chunk download from file position [{}] for XOR address [{}]", next_range_from, xor_name);
                let chunk_service_clone = chunk_service.clone();
                let data_map_clone = data_map.clone();
                tasks.push(
                    tokio::spawn(async move {chunk_service_clone.fetch_from_data_map_chunk(data_map_clone, next_range_from, range_to, stream_chunk_size).await})
                );
                next_range_from += if chunk_count == 1 {
                    first_chunk_limit as u64
                } else {
                    stream_chunk_size as u64
                };
                chunk_count += 1;
            };
            chunk_count = 1;
            for task in tasks {
                match task.await {
                    Ok(result) => {
                        match result {
                            Ok(data) => {
                                let bytes_read = data.len();
                                info!("Read [{}] bytes from chunk [{}] at file position [{}] for XOR address [{}]", bytes_read, chunk_count, next_range_from, xor_name);
                                if bytes_read > 0 {
                                    yield Ok(data); // Sending data to the client here
                                }
                                next_range_from += stream_chunk_size as u64;
                                chunk_count += 1;
                            }
                            Err(e) => {
                                yield Err(ErrorGatewayTimeout(e));
                            }
                        }
                    },
                    Err(e) => {
                        yield Err(ErrorGatewayTimeout(e));
                    }
                }
            }
        };

        let cache_control_header = self.build_cache_control_header(&xor_name, is_resolved_file_name);
        let expires_header = self.build_expires_header(&xor_name, is_resolved_file_name);
        let etag_header = ETag(EntityTag::new_strong(format!("{:x}", xor_name).to_owned()));
        let cors_allow_all = (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*");

        let extension = Path::new(&path_str).extension().unwrap_or_default().to_str().unwrap_or_default();
        if is_range_request {
            info!("return partial content for range {} to {}", range_from, derived_range_to);
            Ok(HttpResponse::PartialContent()
                .insert_header(ContentRange(ContentRangeSpec::Bytes { range: Some((range_from, derived_range_to)), instance_length: Some(total_size as u64) }))
                .insert_header(cache_control_header)
                .insert_header(expires_header)
                .insert_header(etag_header)
                .insert_header(cors_allow_all)
                .insert_header(self.get_content_type_from_filename(extension))
                .streaming(data_stream))
        } else {
            info!("return full content as stream");
            Ok(HttpResponse::Ok()
                .insert_header(cache_control_header)
                .insert_header(expires_header)
                .insert_header(etag_header)
                .insert_header(cors_allow_all)
                .insert_header(self.get_content_type_from_filename(extension))
                .streaming(data_stream))
        }
    }

    fn get_first_chunk_limit(&self, stream_chunk_size: usize, next_range_from: u64) -> usize {
        let first_chunk_remainder = next_range_from % stream_chunk_size as u64;
        if first_chunk_remainder > 0 {
            (stream_chunk_size as u64 - first_chunk_remainder) as usize
        } else {
            stream_chunk_size
        }
    }

    pub fn get_range(&self, request: &HttpRequest) -> (u64, u64, bool) {
        if let Some(range) = request.headers().get(header::RANGE) {
            let range_str = range.to_str().unwrap();
            info!("get range: [{}]", range_str);
            let range_value = range_str.split_once("=").unwrap().1;
            // todo: cover comma separated too: https://docs.rs/actix-web/latest/actix_web/http/header/enum.Range.html
            if let Some((range_from_str, range_to_str)) = range_value.split_once("-") {
                let range_from = range_from_str.parse::<u64>().unwrap_or_else(|_| 0);
                let range_to = range_to_str.parse::<u64>().unwrap_or_else(|_| u64::MAX);
                (range_from, range_to, true)
            } else {
                (0, u64::MAX, true)
            }
        } else {
            (0, u64::MAX, false)
        }
    }

    fn build_cache_control_header(&self, xor_name: &XorName, is_resolved_file_name: bool) -> CacheControl {
        if !is_resolved_file_name && self.xor_helper.is_xor(&format!("{:x}", xor_name)) {
            CacheControl(vec![CacheDirective::MaxAge(u32::MAX), CacheDirective::Public]) // immutable
        } else {
            CacheControl(vec![CacheDirective::MaxAge(10u32), CacheDirective::Public]) // mutable
        }
    }

    fn build_expires_header(&self, xor_name: &XorName, is_resolved_file_name: bool) -> Expires {
        if !is_resolved_file_name && self.xor_helper.is_xor(&format!("{:x}", xor_name)) {
            Expires((SystemTime::now() + Duration::from_secs(u32::MAX as u64)).into()) // immutable
        } else {
            Expires((SystemTime::now() + Duration::from_secs(10u64)).into()) // mutable
        }
    }

    fn get_content_type_from_filename(&self, extension: &str) -> ContentType {
        if extension != "" && extension != "md" { // todo: remove markdown exclusion when IMIM fixed
            ContentType(file_extension_to_mime(extension))
        } else {
            ContentType(mime::TEXT_HTML) // default to text/html
        }
    }
}