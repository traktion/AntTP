use std::path::Path;
use std::time::{Duration, SystemTime};
use actix_files::file_extension_to_mime;
use actix_http::header;
use actix_web::{Error, HttpRequest, HttpResponse};
use actix_web::error::{ErrorInternalServerError, ErrorNotFound};
use actix_web::http::header::{CacheControl, CacheDirective, ContentLength, ContentRange, ContentRangeSpec, ContentType, ETag, EntityTag, Expires};
use autonomi::{ChunkAddress};
use bytes::Bytes;
use chunk_streamer::chunk_receiver::ChunkReceiver;
use chunk_streamer::chunk_streamer::{ChunkGetter, ChunkStreamer};
use log::{debug, info};
use self_encryption::{DataMap};
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

pub struct FileService<T> {
    chunk_getter: T,
    xor_helper: ResolverService,
    ant_tp_config: AntTpConfig,
}

impl<T: ChunkGetter> FileService<T> {
    pub fn new(autonomi_client: T, xor_helper: ResolverService, ant_tp_config: AntTpConfig) -> Self {
        FileService { chunk_getter: autonomi_client, xor_helper, ant_tp_config }
    }

    pub async fn get_data(&self, resolved_address: ResolvedAddress, request: HttpRequest, path_parts: Vec<String>) -> Result<HttpResponse, Error> {
        let (archive_addr, _) = self.xor_helper.assign_path_parts(path_parts.clone());
        let archive_relative_path = path_parts[1..].join("/").to_string();

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
        } else {
            self.download_data_stream(archive_relative_path, resolved_address.xor_name, resolved_address, &request, 0, 0).await
        }
    }

    pub async fn download_data_stream(
        &self,
        path_str: String,
        xor_name: XorName,
        resolved_address: ResolvedAddress,
        request: &HttpRequest,
        offset_modifier: u64,
        limit_modifier: u64,
    ) -> Result<HttpResponse, Error> {
        let (range_from, range_to, is_range_request) = self.get_range(&request, offset_modifier, limit_modifier);

        info!("Streaming item [{}] at addr [{}], range_from [{}], range_to [{}]",
            path_str, format!("{:x}", xor_name), range_from, range_to);

        let data_map_chunk = match self.chunk_getter.chunk_get(&ChunkAddress::new(xor_name)).await {
            Ok(chunk) => chunk,
            Err(e) => return Err(ErrorNotFound(format!("chunk not found [{}]", e)))
        };
        //debug!("data_map_chunk: [{:?}]", data_map_chunk.value.slice(..4*1024));

        let data_map = match self.get_data_map_from_bytes(&data_map_chunk.value) {
            Ok(data_map) => data_map,
            Err(e) => return Err(ErrorInternalServerError(format!("invalid data map [{}]", e)))
        };
        let total_size = data_map.file_size();

        let derived_range_to = if range_to == u64::MAX { total_size as u64 - 1 } else { range_to };

        let chunk_streamer = ChunkStreamer::new(xor_name.to_string(), data_map, self.chunk_getter.clone(), self.ant_tp_config.download_threads);
        let chunk_receiver = chunk_streamer.open(range_from, derived_range_to);

        // experimental, higher performance, chunk streamer
        /*let chunk_receiver = chunk_stream::chunked_stream(
            self.autonomi_client.clone(),
            self.ant_tp_config.download_threads,
            data_map.clone(),
            range_from,
            derived_range_to,
        );*/
        
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

    pub async fn download_data(&self, xor_name: XorName, offset: u64, limit: u64) -> Result<ChunkReceiver, Error> {
        debug!("download data xor_name: [{}], offset: [{}], limit: [{}]", xor_name.clone(), offset, limit);
        let data_map_chunk = match self.chunk_getter.chunk_get(&ChunkAddress::new(xor_name)).await {
            Ok(chunk) => chunk,
            Err(e) => return Err(ErrorNotFound(format!("chunk not found [{}]", e)))
        };

        let data_map = match self.get_data_map_from_bytes(&data_map_chunk.value) {
            Ok(data_map) => data_map,
            Err(e) => return Err(ErrorInternalServerError(format!("invalid data map [{}]", e)))
        };
        let total_size = data_map.file_size();

        let derived_range_to = if limit == u64::MAX { total_size as u64 - 1 } else { offset + limit - 1 };

        let chunk_streamer = ChunkStreamer::new(xor_name.to_string(), data_map, self.chunk_getter.clone(), self.ant_tp_config.download_threads);
        Ok(chunk_streamer.open(offset, derived_range_to))
    }

    pub fn get_range(&self, request: &HttpRequest, offset_modifier: u64, limit_modifier: u64) -> (u64, u64, bool) {
        let range_from = 0 + offset_modifier;
        let range_to= if limit_modifier != 0 {
            range_from + limit_modifier
        } else {
            u64::MAX
        };
        if let Some(range) = request.headers().get(header::RANGE) {
            let range_str = range.to_str().unwrap();
            let range_value = range_str.split_once("=").unwrap().1;
            // todo: cover comma separated too: https://docs.rs/actix-web/latest/actix_web/http/header/enum.Range.html
            if let Some((range_from_str, range_to_str)) = range_value.split_once("-") {
                let range_from_override = range_from_str.parse::<u64>().unwrap_or_else(|_| 0) + offset_modifier;
                let range_to_override = match range_to_str.parse::<u64>() {
                    Ok(range_to_value) => range_to_value + limit_modifier,
                    Err(_) => {
                        if limit_modifier != 0 {
                            range_from_override + limit_modifier
                        } else {
                            u64::MAX
                        }
                    }
                };
                (range_from_override, range_to_override, true)
            } else {
                (range_from, range_to, true)
            }
        } else {
            (range_from, range_to, false)
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
            Expires((SystemTime::now() + Duration::from_secs(u64::from(u32::MAX))).into()) // immutable
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

    pub fn get_data_map_from_bytes(&self, data_map_bytes: &Bytes) -> Result<DataMap, Error> {
        match rmp_serde::from_slice(data_map_bytes) {
            Ok(data_map_level) => match data_map_level {
                DataMapLevel::First(map) => Ok(map),
                DataMapLevel::Additional(map) => Ok(map),
            },
            Err(e) => Err(ErrorInternalServerError(format!("data map format invalid [{}]", e)))
        }
    }
}