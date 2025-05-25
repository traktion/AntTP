use std::path::Path;
use std::time::{Duration, SystemTime};
use actix_files::file_extension_to_mime;
use actix_http::header;
use actix_web::{Error, HttpRequest, HttpResponse};
use actix_web::error::{ErrorNotFound};
use actix_web::http::header::{CacheControl, CacheDirective, ContentLength, ContentRange, ContentRangeSpec, ContentType, ETag, EntityTag, Expires};
use autonomi::{ChunkAddress, Client};
use autonomi::client::GetError;
use bytes::Bytes;
use chunk_streamer::chunk_streamer::ChunkStreamer;
use log::{error, info};
use self_encryption::DataMap;
use serde::{Deserialize, Serialize};
use xor_name::XorName;
use crate::config::anttp_config::AntTpConfig;
use crate::service::archive_helper::{DataState};
use crate::service::resolver_service::{ResolvedAddress, ResolverService};

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
    xor_helper: ResolverService,
    ant_tp_config: AntTpConfig,
}

impl FileService {
    pub fn new(autonomi_client: Client, xor_helper: ResolverService, ant_tp_config: AntTpConfig) -> Self {
        FileService { autonomi_client, xor_helper, ant_tp_config }
    }

    pub async fn get_data(&self, path_parts: Vec<String>, request: HttpRequest, resolved_address: ResolvedAddress) -> Result<HttpResponse, Error> {
        let (archive_addr, _) = self.xor_helper.assign_path_parts(path_parts.clone());

        if self.xor_helper.get_data_state(request.headers(), &resolved_address.xor_name) == DataState::NotModified {
            info!("ETag matches for path [{}] at address [{}]. Client can use cached version", archive_addr, format!("{:x}", resolved_address.xor_name).as_str());
            let cache_control_header = self.build_cache_control_header(&resolved_address.xor_name, resolved_address.is_mutable);
            let expires_header = self.build_expires_header(&resolved_address.xor_name, resolved_address.is_mutable);
            let etag_header = ETag(EntityTag::new_strong(format!("{:x}", resolved_address.xor_name).to_owned()));
            let cors_allow_all = (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*");
            Ok(HttpResponse::NotModified()
                .insert_header(cache_control_header)
                .insert_header(expires_header)
                .insert_header(etag_header)
                .insert_header(cors_allow_all)
                .finish())
        //} else if !resolved_address.is_found {
        //    Err(ErrorNotFound(format!("File not found {:?}", self.conn.host())))
        } else {
            self.download_data_stream(archive_addr, resolved_address.xor_name, resolved_address, &request).await
        }
    }

    pub async fn download_data_stream(
        &self,
        path_str: String,
        xor_name: XorName,
        resolved_address: ResolvedAddress,
        request: &HttpRequest,
    ) -> Result<HttpResponse, Error> {
        let (range_from, range_to, is_range_request) = self.get_range(&request);

        info!("Streaming item [{}] at addr [{}], range_from [{}], range_to [{}]", path_str, format!("{:x}", xor_name), range_from, range_to);

        let data_map_chunk = match self.autonomi_client.chunk_get(&ChunkAddress::new(xor_name)).await {
            Ok(chunk) => chunk,
            Err(e) => return Err(ErrorNotFound(format!("{}", e)))
        };
        
        let data_map = self.get_data_map_from_bytes(&data_map_chunk.value);
        let total_size = data_map.file_size();
        
        let derived_range_to = if range_to == u64::MAX { total_size as u64 - 1 } else { range_to };

        let chunk_streamer = ChunkStreamer::new(xor_name.to_string(), data_map, self.autonomi_client.clone(), self.ant_tp_config.download_threads);
        let chunk_receiver = chunk_streamer.open(range_from, derived_range_to);
        
        let etag_header = ETag(EntityTag::new_strong(format!("{:x}", xor_name).to_owned()));
        let cors_allow_all = (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*");
        
        let cache_control_header = self.build_cache_control_header(&xor_name, resolved_address.is_mutable);
        let expires_header = self.build_expires_header(&xor_name, resolved_address.is_mutable);
        let extension = Path::new(&path_str).extension().unwrap_or_default().to_str().unwrap_or_default();
        if is_range_request {
            Ok(HttpResponse::PartialContent()
                .insert_header(ContentRange(ContentRangeSpec::Bytes { range: Some((range_from, derived_range_to)), instance_length: Some(total_size as u64) }))
                .insert_header(cache_control_header)
                .insert_header(expires_header)
                .insert_header(etag_header)
                .insert_header(cors_allow_all)
                .insert_header(self.get_content_type_from_filename(extension))
                .streaming(chunk_receiver))
        } else {
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

    pub fn get_range(&self, request: &HttpRequest) -> (u64, u64, bool) {
        if let Some(range) = request.headers().get(header::RANGE) {
            let range_str = range.to_str().unwrap();
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
        if !is_resolved_file_name && self.xor_helper.is_immutable_address(&format!("{:x}", xor_name)) {
            CacheControl(vec![CacheDirective::MaxAge(u32::MAX), CacheDirective::Public]) // immutable
        } else {
            CacheControl(vec![CacheDirective::MaxAge(self.ant_tp_config.cached_mutable_ttl as u32), CacheDirective::Public]) // mutable
        }
    }

    fn build_expires_header(&self, xor_name: &XorName, is_resolved_file_name: bool) -> Expires {
        if !is_resolved_file_name && self.xor_helper.is_immutable_address(&format!("{:x}", xor_name)) {
            Expires((SystemTime::now() + Duration::from_secs(u32::MAX as u64)).into()) // immutable
        } else {
            Expires((SystemTime::now() + Duration::from_secs(self.ant_tp_config.cached_mutable_ttl)).into()) // mutable
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