use std::path::Path;
use std::time::{Duration, SystemTime};
use actix_files::file_extension_to_mime;
use actix_http::header;
use actix_web::{Error, HttpRequest, HttpResponse};
use actix_web::error::{ErrorInternalServerError, ErrorNotFound};
use actix_web::http::header::{CacheControl, CacheDirective, ContentLength, ContentRange, ContentRangeSpec, ContentType, ETag, EntityTag, Expires};
use autonomi::{ChunkAddress};
use chunk_streamer::chunk_receiver::ChunkReceiver;
use chunk_streamer::chunk_streamer::{ChunkGetter, ChunkStreamer};
use log::{debug, info};
use xor_name::XorName;
use crate::client::caching_client::CachingClient;
use crate::config::anttp_config::AntTpConfig;
use crate::service::archive_helper::{DataState};
use crate::service::resolver_service::{ResolvedAddress, ResolverService};

pub struct Range {
    pub start: u64,
    pub end: u64,
}

pub struct FileService {
    caching_client: CachingClient,
    xor_helper: ResolverService,
    ant_tp_config: AntTpConfig,
}

impl FileService {
    pub fn new(caching_client: CachingClient, xor_helper: ResolverService, ant_tp_config: AntTpConfig) -> Self {
        FileService { caching_client, xor_helper, ant_tp_config }
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
            let server_header = (header::SERVER, format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")));
            Ok(HttpResponse::NotModified()
                .insert_header(cache_control_header)
                .insert_header(expires_header)
                .insert_header(etag_header)
                .insert_header(cors_allow_all)
                .insert_header(server_header)
                .finish())
        } else {
            self.download_data_stream(archive_relative_path, resolved_address.xor_name, resolved_address, &request,  0, 0).await
        }
    }

    pub async fn download_data_stream(
        &self,
        path_str: String,
        xor_name: XorName,
        resolved_address: ResolvedAddress,
        request: &HttpRequest,
        offset_modifier: u64,
        size_modifier: u64,
    ) -> Result<HttpResponse, Error> {
        let data_map_chunk = match self.caching_client.chunk_get(&ChunkAddress::new(xor_name)).await {
            Ok(chunk) => chunk,
            Err(e) => return Err(ErrorNotFound(format!("chunk not found [{}]", e)))
        };

        let chunk_streamer = ChunkStreamer::new(xor_name.to_string(), data_map_chunk.value, self.caching_client.clone(), self.ant_tp_config.download_threads);
        let content_length = if size_modifier > 0 {
            // file is in an archive (so, we already have the size)
            size_modifier
        } else {
            // file is standalone (so, need to calculate the size)
            let total_size = chunk_streamer.get_stream_size().await;
            u64::try_from(total_size).unwrap()
        };

        let (range_from, range_to, range_length, is_range_request) = self.get_range(&request, offset_modifier, content_length);
        if is_range_request && range_length == 0 {
            return Ok(HttpResponse::RangeNotSatisfiable()
                .insert_header(ContentRange(ContentRangeSpec::Bytes { range: None, instance_length: Some(content_length) }))
                .finish());
        }

        let chunk_receiver = match chunk_streamer.open(range_from, range_to).await {
            Ok(chunk_receiver) => chunk_receiver,
            Err(e) => return Err(ErrorInternalServerError(format!("failed to open chunk stream: {}", e))),
        };
        
        let etag_header = ETag(EntityTag::new_strong(format!("{:x}", xor_name).to_owned()));
        let cors_allow_all = (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*");
        let server_header = (header::SERVER, format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")));
        
        let cache_control_header = self.build_cache_control_header(&xor_name, resolved_address.is_mutable);
        let expires_header = self.build_expires_header(&xor_name, resolved_address.is_mutable);
        let extension = Path::new(&path_str).extension().unwrap_or_default().to_str().unwrap_or_default();
        if is_range_request {
            let response_range_from = range_from - offset_modifier;
            let response_range_to = range_to - offset_modifier;
            info!("streaming item [{}] at addr [{}], range_from: [{}], range_to: [{}], offset_modifier: [{}], size_modifier: [{}], content_length: [{}], range_length: [{}], response_range_from: [{}], response_range_to: [{}]",
            path_str, format!("{:x}", xor_name), range_from, range_to, offset_modifier, size_modifier, content_length, range_length, response_range_from, response_range_to);
            Ok(HttpResponse::PartialContent()
                .insert_header(ContentRange(ContentRangeSpec::Bytes { range: Some((response_range_from, response_range_to)), instance_length: Some(content_length) }))
                .insert_header((header::ACCEPT_RANGES, "bytes"))
                .insert_header(cache_control_header)
                .insert_header(expires_header)
                .insert_header(etag_header)
                .insert_header(cors_allow_all)
                .insert_header(self.get_content_type_from_filename(extension))
                .insert_header(server_header)
                .streaming(chunk_receiver))
        } else {
            info!("streaming item [{}] at addr [{}], offset_modifier: [{}], size_modifier: [{}], file_size: [{}]",
            path_str, format!("{:x}", xor_name), offset_modifier, size_modifier, content_length);
            Ok(HttpResponse::Ok()
                .insert_header(ContentLength(usize::try_from(content_length).unwrap()))
                .insert_header(cache_control_header)
                .insert_header(expires_header)
                .insert_header(etag_header)
                .insert_header(cors_allow_all)
                .insert_header(self.get_content_type_from_filename(extension))
                .insert_header(server_header)
                .streaming(chunk_receiver))
        }
    }

    pub async fn download_data(&self, xor_name: XorName, range_from: u64, size: u64) -> Result<ChunkReceiver, Error> {
        debug!("download data xor_name: [{}], offset: [{}], size: [{}]", xor_name.clone(), range_from, size);
        let data_map_chunk = match self.caching_client.chunk_get(&ChunkAddress::new(xor_name)).await {
            Ok(chunk) => chunk,
            Err(e) => return Err(ErrorNotFound(format!("chunk not found [{}]", e)))
        };

        let chunk_streamer = ChunkStreamer::new(xor_name.to_string(), data_map_chunk.value, self.caching_client.clone(), self.ant_tp_config.download_threads);

        let range_to = if size == u64::MAX {
            u64::try_from(chunk_streamer.get_stream_size().await - 1).unwrap()
        } else {
            range_from + size - 1
        };

        match chunk_streamer.open(range_from, range_to).await {
            Ok(chunk_receiver) => Ok(chunk_receiver),
            Err(e) => Err(ErrorInternalServerError(format!("download_data failed [{}]", e)))
        }
    }

    pub fn get_range(&self, request: &HttpRequest, offset_modifier: u64, size_modifier: u64) -> (u64, u64, u64, bool) {
        debug!("get_range - offset_modifier [{}], size_modifier [{}]", offset_modifier, size_modifier);
        let length = if size_modifier > 0 { size_modifier - 1 } else { 0 }; // ranges are zero indexed
        let range_to= offset_modifier + length;
        if let Some(range) = request.headers().get(header::RANGE) {
            // e.g. bytes=100-200/201
            let range_value = range.to_str()
                .unwrap_or("")
                .split_once("=")
                .unwrap_or(("", "")).1;
            // todo: cover comma separated too: https://docs.rs/actix-web/latest/actix_web/http/header/enum.Range.html
            if let Some((range_from_str, range_to_str)) = range_value.split_once("-") {
                let range_from_override = offset_modifier + range_from_str.parse::<u64>().unwrap_or_else(|_| 0);
                let range_to_override = offset_modifier + range_to_str.parse::<u64>().unwrap_or_else(|_| length);
                (range_from_override, range_to_override, range_to_override - range_from_override, true)
            } else {
                (offset_modifier, range_to, length, true)
            }
        } else {
            (offset_modifier, range_to, length, false)
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
}