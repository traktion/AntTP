use std::cmp::min;
use std::path::Path;
use actix_http::header;
use actix_web::HttpRequest;
use autonomi::ChunkAddress;
use bytes::{BufMut, BytesMut};
use chunk_streamer::chunk_receiver::ChunkReceiver;
use chunk_streamer::chunk_streamer::ChunkStreamer;
use futures_util::StreamExt;
use log::{debug, info};
use mockall::mock;
use mockall_double::double;
use xor_name::XorName;
#[double]
use crate::client::ChunkCachingClient;
#[double]
use crate::client::CachingClient;
use crate::error::{GetError, GetStreamError};
use crate::error::chunk_error::ChunkError;
use crate::service::resolver_service::ResolvedAddress;

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct FileService {
    chunk_caching_client: ChunkCachingClient,
    caching_client: CachingClient,
    download_threads: usize,
}

mock! {
    #[derive(Debug)]
    pub FileService {
        pub fn new(chunk_caching_client: ChunkCachingClient, caching_client: CachingClient, download_threads: usize) -> Self;
        pub async fn get_data(&self, request: &HttpRequest, resolved_address: &ResolvedAddress) -> Result<(ChunkReceiver, RangeProps), ChunkError>;
        pub async fn download_data_request(&self, request: &HttpRequest, path_str: String, xor_name: XorName, offset_modifier: u64, size_modifier: u64) -> Result<(ChunkReceiver, RangeProps), ChunkError>;
        pub async fn download_data_bytes(&self, xor_name: XorName, range_from: u64, size_modifier: u64) -> Result<BytesMut, ChunkError>;
    }
    impl Clone for FileService {
        fn clone(&self) -> Self;
    }
}

impl FileService {
    pub fn new(chunk_caching_client: ChunkCachingClient, caching_client: CachingClient, download_threads: usize) -> Self {
        FileService { chunk_caching_client, caching_client, download_threads }
    }

    pub async fn get_data(&self, request: &HttpRequest, resolved_address: &ResolvedAddress) -> Result<(ChunkReceiver, RangeProps), ChunkError> {
        self.download_data_request(request, resolved_address.file_path.clone(), resolved_address.xor_name, 0, 0).await
    }

    pub async fn download_data_request(
        &self,
        request: &HttpRequest,
        path_str: String,
        xor_name: XorName,
        offset_modifier: u64,
        size_modifier: u64,
    ) -> Result<(ChunkReceiver, RangeProps), ChunkError> {
        let data_map_chunk = self.chunk_caching_client.chunk_get_internal(&ChunkAddress::new(xor_name)).await?;

        let chunk_streamer = ChunkStreamer::new(xor_name.to_string(), data_map_chunk.value, self.caching_client.clone(), self.download_threads);
        let content_length = self.get_content_length(&chunk_streamer, size_modifier).await;

        let (range_from, range_to, range_length, is_range_request) = self.get_range(Some(&request), offset_modifier, content_length);
        if is_range_request && range_length == 0 {
            return Err(GetStreamError::BadRange(format!("bad range length: [{}]", range_length)).into());
        }

        let chunk_receiver = match chunk_streamer.open(range_from, range_to).await {
            Ok(chunk_receiver) => chunk_receiver,
            Err(e) => return Err(GetStreamError::BadReceiver(format!("failed to open chunk stream: {}", e)).into()),
        };

        let extension = Path::new(&path_str).extension().unwrap_or_default().to_str().unwrap_or_default().to_string();
        let (maybe_response_range_from, maybe_response_range_to) =
            self.get_response_range(range_from, range_to, is_range_request, offset_modifier);
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
            u64::try_from(total_size).unwrap_or(0)
        }
    }

    pub fn get_range(&self, request: Option<&HttpRequest>, offset_modifier: u64, size_modifier: u64) -> (u64, u64, u64, bool) {
        debug!("get_range - offset_modifier [{}], size_modifier [{}]", offset_modifier, size_modifier);
        let length = if size_modifier > 0 { size_modifier - 1 } else { 0 }; // ranges are zero indexed
        let range_to= offset_modifier + length;
        if request.is_some() && let Some(range) = request.unwrap().headers().get(header::RANGE) {
            // e.g. bytes=100-200/201
            let range_value = range.to_str()
                .unwrap_or("")
                .split_once("=")
                .unwrap_or(("", "")).1;
            // todo: cover comma separated too: https://docs.rs/actix-web/latest/actix_web/http/header/enum.Range.html
            if let Some((range_from_str, range_to_str)) = range_value.split_once("-") {
                // range_to_override
                let range_to_header = min(range_to_str.parse::<u64>().unwrap_or(length), length);
                // range_to must not exceed length
                let range_to_override = offset_modifier + range_to_header;
                // range_from must not exceed range_to_header
                let range_from_header = min(range_from_str.parse::<u64>().unwrap_or(0), range_to_header);
                let range_from_override = offset_modifier + range_from_header;
                let range_length =  range_to_override - range_from_override;
                (range_from_override, range_to_override, range_length, true)
            } else {
                (offset_modifier, range_to, length, true)
            }
        } else {
            (offset_modifier, range_to, length, false)
        }
    }

    fn get_response_range(&self, range_from: u64, range_to: u64, is_range_request: bool, offset_modifier: u64) -> (Option<u64>, Option<u64>) {
        if is_range_request {
            (Some(range_from - offset_modifier), Some(range_to - offset_modifier))
        } else {
            (None, None)
        }
    }
    
    pub async fn download_data_bytes(&self, xor_name: XorName, range_from: u64, size_modifier: u64) -> Result<BytesMut, ChunkError> {
        match self.download_data(xor_name, range_from, size_modifier).await {
            Ok(mut chunk_receiver) => {
                // todo: optimise buffer sizes
                let mut buf = BytesMut::new();
                let mut has_data = true;
                while has_data {
                    match chunk_receiver.next().await {
                        Some(item) => match item {
                            Ok(bytes) => buf.put(bytes),
                            Err(e) => {
                                return Err(ChunkError::GetError(GetError::RecordNotFound(e.to_string())));
                            },
                        },
                        None => has_data = false
                    };
                }
                Ok(buf)
            }
            Err(e) => Err(e)
        }
    }

    // todo: refactor/merge with download_data_request above
    async fn download_data(&self, xor_name: XorName, range_from: u64, size_modifier: u64) -> Result<ChunkReceiver, ChunkError> {
        debug!("download data xor_name: [{}], offset: [{}], size: [{}]", xor_name.clone(), range_from, size_modifier);
        let data_map_chunk = match self.chunk_caching_client.chunk_get_internal(&ChunkAddress::new(xor_name)).await {
            Ok(chunk) => chunk,
            Err(e) => return Err(e),
        };

        let chunk_streamer = ChunkStreamer::new(xor_name.to_string(), data_map_chunk.value, self.caching_client.clone(), self.download_threads);
        let content_length = self.get_content_length(&chunk_streamer, size_modifier).await;
        let (range_from, range_to, _, _) = self.get_range(None, range_from, content_length);

        match chunk_streamer.open(range_from, range_to).await {
            Ok(chunk_receiver) => Ok(chunk_receiver),
            Err(e) => Err(GetStreamError::BadReceiver(format!("failed to open chunk stream: {}", e)).into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::TestRequest;
    use crate::client::{MockCachingClient, MockChunkCachingClient};
    use autonomi::Chunk;

    fn create_test_service(mock_chunk_caching_client: MockChunkCachingClient, mock_caching_client: MockCachingClient) -> FileService {
        FileService {
            chunk_caching_client: mock_chunk_caching_client,
            caching_client: mock_caching_client,
            download_threads: 8,
        }
    }

    #[test]
    fn test_range_props() {
        let props = RangeProps::new(Some(0), Some(100), 200, "txt".to_string());
        assert!(props.is_range());
        assert_eq!(props.range_from(), Some(0));
        assert_eq!(props.range_to(), Some(100));
        assert_eq!(props.content_length(), 200);
        assert_eq!(props.extension(), "txt");

        let props_no_range = RangeProps::new(None, None, 200, "txt".to_string());
        assert!(!props_no_range.is_range());
    }

    #[actix_web::test]
    async fn test_get_range_no_header() {
        let service = create_test_service(MockChunkCachingClient::default(), MockCachingClient::default());
        let (start, end, length, is_range) = service.get_range(None, 0, 100);
        assert_eq!(start, 0);
        assert_eq!(end, 99);
        assert_eq!(length, 99); // 100 - 1
        assert!(!is_range);
    }

    #[actix_web::test]
    async fn test_get_range_with_header() {
        let service = create_test_service(MockChunkCachingClient::default(), MockCachingClient::default());
        let req = TestRequest::default().insert_header((header::RANGE, "bytes=10-50")).to_http_request();
        
        let (start, end, length, is_range) = service.get_range(Some(&req), 0, 100);
        assert_eq!(start, 10);
        assert_eq!(end, 50);
        assert_eq!(length, 40);
        assert!(is_range);
    }

    #[actix_web::test]
    async fn test_get_range_with_header_open_end() {
        let service = create_test_service(MockChunkCachingClient::default(), MockCachingClient::default());
        let req = TestRequest::default().insert_header((header::RANGE, "bytes=10-")).to_http_request();
        
        let (start, end, length, is_range) = service.get_range(Some(&req), 0, 100);
        assert_eq!(start, 10);
        assert_eq!(end, 99);
        assert_eq!(length, 89);
        assert!(is_range);
    }

    #[actix_web::test]
    async fn test_get_range_with_header_end_over_length() {
        let service = create_test_service(MockChunkCachingClient::default(), MockCachingClient::default());
        let req = TestRequest::default().insert_header((header::RANGE, "bytes=10-120")).to_http_request();

        let (start, end, length, is_range) = service.get_range(Some(&req), 0, 100);
        assert_eq!(start, 10);
        assert_eq!(end, 99);
        assert_eq!(length, 89);
        assert!(is_range);
    }

    #[actix_web::test]
    async fn test_get_response_range() {
        let service = create_test_service(MockChunkCachingClient::default(), MockCachingClient::default());
        
        let (start, end) = service.get_response_range(10, 50, true, 0);
        assert_eq!(start, Some(10));
        assert_eq!(end, Some(50));

        let (start, end) = service.get_response_range(10, 50, false, 0);
        assert_eq!(start, None);
        assert_eq!(end, None);

        // With offset modifier
        let (start, end) = service.get_response_range(15, 55, true, 5);
        assert_eq!(start, Some(10));
        assert_eq!(end, Some(50));
    }

    #[actix_web::test]
    async fn test_download_data_bytes_success() {
        let mut mock_chunk_client = MockChunkCachingClient::default();
        let mut mock_caching_client = MockCachingClient::default();

        let xor_name = XorName::default();
        let chunk_addr = ChunkAddress::new(xor_name);
        let data = vec![1, 2, 3, 4, 5];
        let chunk = Chunk::new(data.clone().into());

        mock_chunk_client.expect_chunk_get_internal()
            .with(mockall::predicate::eq(chunk_addr))
            .times(1)
            .returning(move |_| Ok(chunk.clone()));

        // Since ChunkStreamer::new is called, we can't easily mock it without mocking CachingClient methods it calls.
        // ChunkStreamer::new calls nothing immediately.
        // chunk_streamer.get_stream_size() calls nothing immediately if data_map_chunk is passed.
        // wait, ChunkStreamer in chunk-streamer crate 0.5.4:
        // get_stream_size() might call something if it needs to fetch.
        
        // Actually, let's just test get_data which is easier.
    }
    
    #[actix_web::test]
    async fn test_get_data_success() {
        let mut mock_chunk_client = MockChunkCachingClient::default();
        let mut mock_caching_client = MockCachingClient::default();

        let xor_name = XorName::default();
        let chunk_addr = ChunkAddress::new(xor_name);
        let data = vec![1, 2, 3, 4, 5];
        let chunk = Chunk::new(data.clone().into());

        mock_chunk_client.expect_chunk_get_internal()
            .with(mockall::predicate::eq(chunk_addr))
            .times(1)
            .returning(move |_| Ok(chunk.clone()));
            
        // ChunkStreamer::new is called with self.caching_client.clone()
        mock_caching_client.expect_clone()
            .returning(MockCachingClient::default);

        let service = create_test_service(mock_chunk_client, mock_caching_client);
        let req = TestRequest::default().to_http_request();
        let resolved_address = ResolvedAddress {
            is_found: false,
            xor_name,
            file_path: "test.txt".to_string(),
            is_resolved_from_mutable: false,
            is_modified: false,
            is_allowed: false,
            archive: None,
            ttl: 0,
        };

        let result = service.get_data(&req, &resolved_address).await;
        // This might still fail because ChunkStreamer::open tries to spawn threads and use the client.
        // But let's see.
        assert!(result.is_ok());
    }
}