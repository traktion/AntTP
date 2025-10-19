use std::path::Path;
use actix_http::header;
use actix_web::HttpRequest;
use autonomi::{ChunkAddress};
use chunk_streamer::chunk_receiver::ChunkReceiver;
use chunk_streamer::chunk_streamer::ChunkStreamer;
use log::{debug, info};
use xor_name::XorName;
use crate::client::CachingClient;
use crate::client::error::{ChunkError, GetStreamError};
use crate::config::anttp_config::AntTpConfig;
use crate::service::resolver_service::ResolvedAddress;

pub struct RangeProps {
    range_from: Option<u64>,
    range_to: Option<u64>,
    content_length: u64,
    extension: String,
}

impl RangeProps {
    pub fn new(range_from: Option<u64>, range_to: Option<u64>, content_length: u64, extension: String) -> Self {
        Self { range_from, range_to, content_length, extension }
    }

    pub fn is_range(&self) -> bool {
        self.range_from.is_some() && self.range_to.is_some()
    }


    pub fn range_from(&self) -> Option<u64> {
        self.range_from
    }

    pub fn range_to(&self) -> Option<u64> {
        self.range_to
    }

    pub fn content_length(&self) -> u64 {
        self.content_length
    }

    pub fn extension(&self) -> &str {
        &self.extension
    }
}

pub struct Range {
    pub start: u64,
    pub end: u64,
}

pub struct FileService {
    caching_client: CachingClient,
    ant_tp_config: AntTpConfig,
}

impl FileService {
    pub fn new(caching_client: CachingClient, ant_tp_config: AntTpConfig) -> Self {
        FileService { caching_client, ant_tp_config }
    }

    pub async fn get_data(&self, request: &HttpRequest, resolved_address: &ResolvedAddress) -> Result<(ChunkReceiver, RangeProps), ChunkError> {
        self.download_data_stream(request, resolved_address.file_path.clone(), resolved_address.xor_name, 0, 0).await
    }

    pub async fn download_data_stream(
        &self,
        request: &HttpRequest,
        path_str: String,
        xor_name: XorName,
        offset_modifier: u64,
        size_modifier: u64,
    ) -> Result<(ChunkReceiver, RangeProps), ChunkError> {
        let data_map_chunk = match self.caching_client.chunk_get_internal(&ChunkAddress::new(xor_name)).await {
            Ok(chunk) => chunk,
            Err(e) => return Err(e),
        };

        let chunk_streamer = ChunkStreamer::new(xor_name.to_string(), data_map_chunk.value, self.caching_client.clone(), self.ant_tp_config.download_threads);
        let content_length = self.get_content_length(&chunk_streamer, size_modifier).await;

        let (range_from, range_to, range_length, is_range_request) = self.get_range(&request, offset_modifier, content_length);
        if is_range_request && range_length == 0 {
            return Err(ChunkError::GetStreamError(GetStreamError::BadRange(format!("Bad range length: [{}]", range_length))));
        }

        let chunk_receiver = match chunk_streamer.open(range_from, range_to).await {
            Ok(chunk_receiver) => chunk_receiver,
            Err(e) => return Err(ChunkError::GetStreamError(GetStreamError::BadReceiver(format!("failed to open chunk stream: {}", e)))),
        };

        let extension = Path::new(&path_str).extension().unwrap_or_default().to_str().unwrap_or_default().to_string();
        let (maybe_response_range_from, maybe_response_range_to) =
            self.get_response_range(range_to, range_from, is_range_request, offset_modifier);
        info!("streaming item [{}] at addr [{}], range_from: [{}], range_to: [{}], offset_modifier: [{}], size_modifier: [{}], content_length: [{}], range_length: [{}], response_range_from: [{}], response_range_to: [{}]",
                path_str, format!("{:x}", xor_name), range_from, range_to, offset_modifier, size_modifier, content_length, range_length, maybe_response_range_from.unwrap_or(0), maybe_response_range_to.unwrap_or(0));
        Ok((chunk_receiver, RangeProps::new(maybe_response_range_from, maybe_response_range_to, content_length, extension)))
    }

    async fn get_content_length(&self, chunk_streamer: &ChunkStreamer<CachingClient>, size_modifier: u64) -> u64 {
        if size_modifier > 0 {
            // file is in an archive (so, we already have the size)
            size_modifier
        } else {
            // file is standalone (so, need to calculate the size)
            let total_size = chunk_streamer.get_stream_size().await;
            u64::try_from(total_size).unwrap()
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

    fn get_response_range(&self, range_to: u64, range_from: u64, is_range_request: bool, offset_modifier: u64) -> (Option<u64>, Option<u64>) {
        if is_range_request {
            (Some(range_from - offset_modifier), Some(range_to - offset_modifier))
        } else {
            (None, None)
        }
    }

    pub async fn download_data(&self, xor_name: XorName, range_from: u64, size: u64) -> Result<ChunkReceiver, ChunkError> {
        debug!("download data xor_name: [{}], offset: [{}], size: [{}]", xor_name.clone(), range_from, size);
        let data_map_chunk = match self.caching_client.chunk_get_internal(&ChunkAddress::new(xor_name)).await {
            Ok(chunk) => chunk,
            Err(e) => return Err(e),
        };

        let chunk_streamer = ChunkStreamer::new(xor_name.to_string(), data_map_chunk.value, self.caching_client.clone(), self.ant_tp_config.download_threads);

        let range_to = if size == u64::MAX {
            u64::try_from(chunk_streamer.get_stream_size().await - 1).unwrap()
        } else {
            range_from + size - 1
        };

        match chunk_streamer.open(range_from, range_to).await {
            Ok(chunk_receiver) => Ok(chunk_receiver),
            Err(e) => Err(ChunkError::GetStreamError(GetStreamError::BadReceiver(format!("failed to open chunk stream: {}", e)))),
        }
    }
}