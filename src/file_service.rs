use std::path::Path;
use std::time::{Duration, SystemTime};
use actix_files::file_extension_to_mime;
use actix_http::header;
use actix_web::{Error, HttpRequest, HttpResponse};
use actix_web::dev::ConnectionInfo;
use actix_web::error::{ErrorInternalServerError, ErrorNotFound};
use actix_web::http::header::{CacheControl, CacheDirective, ContentRange, ContentRangeSpec, ContentType, ETag, EntityTag, Expires};
use async_stream::{stream};
use autonomi::{ChunkAddress, Client};
use log::{info};
use tokio::sync::mpsc::channel;
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
        //let total_chunks = data_map.infos().len();

        let mut next_range_from = range_from;
        let derived_range_to = if range_to == u64::MAX { total_size as u64 - 1 } else { range_to };
        let first_chunk_limit = self.get_first_chunk_limit(stream_chunk_size, next_range_from);

        let data_stream = stream!{
            info!("sending data stream");

            let mut chunk_count = 1;

            // todo: tune channel buffer size. Should total be shared between connections, as shared autonomi connection
            let (sender, mut receiver) = channel(64);

            tokio::spawn(async move {
                while next_range_from < derived_range_to {
                    info!("Async chunk download from file position [{}] for XOR address [{}]", next_range_from, xor_name);
                    let chunk_service_clone = chunk_service.clone();
                    let data_map_clone = data_map.clone();
                    let handle = tokio::spawn(async move {chunk_service_clone.fetch_from_data_map_chunk(data_map_clone, next_range_from, range_to, stream_chunk_size).await});
                    info!("Sender channel capacity [{}] of [{}]", sender.capacity(), sender.max_capacity());
                    sender.send(handle).await.unwrap(); // todo: add handling
                    next_range_from += if chunk_count == 1 {
                        first_chunk_limit as u64
                    } else {
                        stream_chunk_size as u64
                    };
                    chunk_count += 1;
                };
            });

            let mut file_position = 0;
            let mut chunk_index = 1;
            //while chunk_index <= total_chunks {
            loop {
                let maybe_join_handle = receiver.recv().await;
                match maybe_join_handle {
                    Some(join_handle) => {
                        match join_handle.await {
                            Ok(result) => {
                                match result {
                                    Ok(data) => {
                                        let bytes_read = data.len();
                                        info!("Read [{}] bytes from chunk [{}] at file position [{}] for XOR address [{}]", bytes_read, chunk_index, file_position, xor_name);
                                        if bytes_read > 0 {
                                            yield Ok(data); // Sending data to the client here
                                        }
                                        file_position += stream_chunk_size as u64;
                                        chunk_index += 1;
                                    },
                                    Err(e) => {
                                        yield Err(ErrorInternalServerError(e));    
                                    }
                                }
                            },
                            Err(e) => {
                                yield Err(ErrorInternalServerError(e));
                            }
                        }
                    }
                    None => {
                        break; // end of stream - break
                    }
                }
            }
            receiver.close();
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

    /*pub async fn download_data_body(
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

                let extension = Path::new(&path_str).extension().unwrap_or_default().to_str().unwrap_or_default();
                Ok(HttpResponse::Ok()
                    .insert_header(cache_control_header)
                    .insert_header(etag_header)
                    .insert_header(cors_allow_all)
                    .insert_header(self.get_content_type_from_filename(extension))
                    .body(data))
            }
            Err(e) => {
                Err(ErrorInternalServerError(format!("Failed to download [{:?}]", e)))
            }
        }
    }*/

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
        // todo: remove markdown exclusion when IMIM fixed
        if extension != "" && extension != "md" {
            ContentType(file_extension_to_mime(extension))
        } else if extension == "md" {
            ContentType(mime::TEXT_HTML)
        } else {
            ContentType(mime::TEXT_PLAIN) // default to text/html
        }
    }
}