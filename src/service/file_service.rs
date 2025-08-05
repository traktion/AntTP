use std::path::Path;
use std::time::{Duration, SystemTime};
use actix_files::file_extension_to_mime;
use actix_http::header;
use actix_web::{Error, HttpRequest, HttpResponse};
use actix_web::error::{ErrorInternalServerError, ErrorNotFound};
use actix_web::http::header::{CacheControl, CacheDirective, ContentLength, ContentRange, ContentRangeSpec, ContentType, ETag, EntityTag, Expires};
use autonomi::{ChunkAddress};
use autonomi::data::DataAddress;
use bytes::{BufMut, Bytes, BytesMut};
use chunk_streamer::chunk_receiver::ChunkReceiver;
use chunk_streamer::chunk_streamer::{ChunkGetter, ChunkStreamer};
use futures_util::StreamExt;
use log::{debug, error, info};
use self_encryption::{DataMap};
use xor_name::XorName;
use crate::client::caching_client::CachingClient;
use crate::config::anttp_config::AntTpConfig;
use crate::config::app_config::AppConfig;
use crate::service::archive::Archive;
use crate::service::archive_helper::{DataState};
use crate::service::resolver_service::{ResolvedAddress, ResolverService};

pub struct Range {
    pub start: u64,
    pub end: u64,
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
        let data_map_chunk = match self.chunk_getter.chunk_get(&ChunkAddress::new(xor_name)).await {
            Ok(chunk) => chunk,
            Err(e) => return Err(ErrorNotFound(format!("chunk not found [{}]", e)))
        };

        let data_map = match CachingClient::get_data_map_from_bytes(&data_map_chunk.value) {
            Ok(data_map) => data_map,
            Err(e) => return Err(ErrorInternalServerError(format!("invalid data map [{}]", e)))
        };
        let total_size = data_map.file_size();

        /*// check if tarchive
        let (range_from, range_to, is_range_request) = if self.is_tarchive(xor_name, total_size, &data_map).await {
            match self.get_range_from_tar_archive(&path_str, xor_name, &data_map, total_size).await {
                Some((range_from, range_to, is_range_request)) => (range_from, range_to, is_range_request),
                None => return Err(ErrorNotFound(format!("file [{}] not found in tarchive [{}]", path_str, xor_name)))
            }
        } else {
            debug!("small file - not tar");
            self.get_range(&request, offset_modifier, limit_modifier)
        };*/

        let (range_from, range_to, is_range_request) = self.get_range(&request, offset_modifier, limit_modifier);

        info!("Streaming item [{}] at addr [{}], range_from [{}], range_to [{}]",
            path_str, format!("{:x}", xor_name), range_from, range_to);

        let derived_range_to = if range_to == u64::MAX { total_size as u64 - 1 } else { range_to };

        let chunk_streamer = ChunkStreamer::new(xor_name.to_string(), data_map, self.chunk_getter.clone(), self.ant_tp_config.download_threads);
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

    async fn get_range_from_tar_archive(&self, path_str: &String, xor_name: XorName, data_map: &DataMap, total_size: usize) -> Option<(u64, u64, bool)> {
        match self.get_archive_from_tar(xor_name, data_map, total_size).await {
            Some(archive) => {
                let resolved_path_str = match archive.find("app-conf.json".to_string()) {
                    Some(idx) => {
                        let buf = self.download_stream(xor_name, data_map.clone(), idx.offset, idx.offset + idx.limit).await;
                        let json = String::from_utf8(buf.to_vec()).unwrap_or(String::new());
                        debug!("json [{}], raw [{:?}]", json, buf.to_vec());
                        let app_config: AppConfig = serde_json::from_str(&json.as_str().trim()).expect(format!("failed to deserialize json [{}]", json).as_str());
                        let (resolved_path, has_found) = app_config.resolve_route(path_str.clone(), path_str.clone());
                        if has_found {
                            resolved_path
                        } else {
                            path_str.clone()
                        }
                    },
                    None => path_str.clone(),
                };
                match archive.find(resolved_path_str.clone()) {
                    Some(data_address_offset) => {
                        debug!("path_str [{}] was found in archive.tar.idx", xor_name);
                        Some((data_address_offset.offset, data_address_offset.offset + data_address_offset.limit, false))
                    },
                    None => {
                        debug!("path_str [{}] was not found in archive.tar.idx", resolved_path_str);
                        None
                    },
                }
            },
            None => None
        }
    }

    async fn get_archive_from_tar(&self, xor_name: XorName, data_map: &DataMap, total_size: usize) -> Option<Archive> {
        let trailer_bytes = self.download_stream(xor_name, data_map.clone(), total_size as u64 - 10240, total_size as u64).await;
        match String::from_utf8(trailer_bytes.to_vec()) {
            Ok(trailer) => {
                match trailer.find("archive.tar.idx") {
                    Some(idx) => {
                        debug!("archive.tar.idx was found in archive.tar");
                        let app_config_range_start = idx + 512;
                        let app_config_range_to = 10240;
                        debug!("creating archive with range_from [{}] and range_to [{}]", app_config_range_start, app_config_range_to);
                        Some(
                            Archive::build_from_tar(&DataAddress::new(xor_name), Bytes::copy_from_slice(&trailer_bytes[app_config_range_start..app_config_range_to]))
                        )
                    },
                    None => {
                        debug!("no archive.tar.idx found in tar trailer");
                        None
                    }
                }
            },
            Err(_) => {
                debug!("no tar trailer found");
                None
            }
        }
    }

    pub async fn is_tarchive(&self, xor_name: XorName, total_size: usize, data_map: &DataMap) -> bool {
        // https://www.gnu.org/software/tar/manual/html_node/Standard.html
        if total_size > 512 {
            let tar_magic = self.download_stream(xor_name, data_map.clone(), 257, 261).await.to_vec();
            String::from_utf8(tar_magic.clone()).unwrap_or(String::new()) == "ustar"
        } else {
            false
        }
    }

    pub async fn download_data(&self, xor_name: XorName, offset: u64, limit: u64) -> Result<ChunkReceiver, Error> {
        debug!("download data xor_name: [{}], offset: [{}], limit: [{}]", xor_name.clone(), offset, limit);
        let data_map_chunk = match self.chunk_getter.chunk_get(&ChunkAddress::new(xor_name)).await {
            Ok(chunk) => chunk,
            Err(e) => return Err(ErrorNotFound(format!("chunk not found [{}]", e)))
        };

        let data_map = match CachingClient::get_data_map_from_bytes(&data_map_chunk.value) {
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

    pub async fn download_stream(
        &self,
        xor_name: XorName,
        data_map: DataMap,
        range_from: u64,
        range_to: u64,
    ) -> Bytes {
        let chunk_streamer = ChunkStreamer::new(xor_name.to_string(), data_map, self.chunk_getter.clone(), self.ant_tp_config.download_threads);
        let mut chunk_receiver = chunk_streamer.open(range_from, range_to);

        let mut buf = BytesMut::new();
        let mut has_data = true;
        while has_data {
            match chunk_receiver.next().await {
                Some(item) => match item {
                    Ok(bytes) => buf.put(bytes),
                    Err(e) => {
                        error!("Error streaming app-config from archive: {}", e);
                        has_data = false
                    },
                },
                None => has_data = false
            };
        }
        buf.freeze()
    }
}