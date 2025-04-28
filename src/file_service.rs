use std::path::Path;
use std::time::{Duration, SystemTime};
use actix_files::file_extension_to_mime;
use actix_http::header;
use actix_web::{Error, HttpRequest, HttpResponse};
use actix_web::dev::ConnectionInfo;
use actix_web::error::{ErrorNotFound};
use actix_web::http::header::{CacheControl, CacheDirective, ContentLength, ContentRange, ContentRangeSpec, ContentType, ETag, EntityTag, Expires};
use autonomi::{ChunkAddress, Client};
use autonomi::client::GetError;
use bytes::Bytes;
use log::{error, info};
use self_encryption::DataMap;
use serde::{Deserialize, Serialize};
use xor_name::XorName;
use crate::anttp_config::AntTpConfig;
use crate::archive_helper::DataState;
use crate::chunk::ChunkChannel;
use crate::xor_helper::XorHelper;

#[derive(Serialize, Deserialize)]
enum DataMapLevel {
    // Holds the data map to the source data.
    First(DataMap),
    // Holds the data map of an _additional_ level of chunks
    // resulting from chunking up a previous level data map.
    // This happens when that previous level data map was too big to fit in a chunk itself.
    Additional(DataMap),
}

pub struct FileService {
    autonomi_client: Client,
    xor_helper: XorHelper,
    conn: ConnectionInfo,
    ant_tp_config: AntTpConfig,
}

impl FileService {
    pub fn new(autonomi_client: Client, xor_helper: XorHelper, conn: ConnectionInfo, ant_tp_config: AntTpConfig) -> Self {
        FileService { autonomi_client, xor_helper, conn, ant_tp_config }
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

        info!("streaming item [{}] at addr [{}], range_from [{}], range_to [{}]", path_str, format!("{:x}", xor_name), range_from, range_to);

        info!("getting data map from chunk [{:x}]", xor_name);
        let data_map_chunk = self.autonomi_client
            .chunk_get(&ChunkAddress::new(xor_name))
            .await
            .expect("chunk_get failed")
            .clone();
        
        let data_map = self.get_data_map_from_bytes(&data_map_chunk.value);
        let total_size = data_map.file_size();
        
        let derived_range_to = if range_to == u64::MAX { total_size as u64 - 1 } else { range_to };
        
        let chunk_download_threads = self.ant_tp_config.chunk_download_threads.clone();

        let chunk_channel = ChunkChannel::new(xor_name.to_string(), data_map, self.autonomi_client.clone(), chunk_download_threads);
        let chunk_receiver = chunk_channel.open(range_from, derived_range_to);
        
        let etag_header = ETag(EntityTag::new_strong(format!("{:x}", xor_name).to_owned()));
        let cors_allow_all = (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*");
        
        let cache_control_header = self.build_cache_control_header(&xor_name, is_resolved_file_name);
        let expires_header = self.build_expires_header(&xor_name, is_resolved_file_name);
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
                .streaming(chunk_receiver))
        } else {
            info!("return full content as stream");
            Ok(HttpResponse::Ok()
                .insert_header(ContentLength(total_size))
                .insert_header(cache_control_header)
                .insert_header(expires_header)
                .insert_header(etag_header)
                .insert_header(cors_allow_all)
                .insert_header(self.get_content_type_from_filename(extension))
                .streaming(chunk_receiver))
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
        } else {
            ContentType(mime::TEXT_HTML) // default to text/html
        }
    }

    pub fn get_data_map_from_bytes(&self, data_map_bytes: &Bytes) -> DataMap {
        let data_map_level: DataMapLevel = rmp_serde::from_slice(data_map_bytes)
            .map_err(GetError::InvalidDataMap)
            .inspect_err(|err| error!("Error deserializing data map: {err:?}"))
            .expect("failed to parse data map level");

        match data_map_level {
            DataMapLevel::First(map) => map,
            DataMapLevel::Additional(map) => map,
        }
    }
}