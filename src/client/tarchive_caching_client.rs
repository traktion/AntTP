use std::cmp::min;
use ant_core::data::XorName;
use bytes::Bytes;
use log::{debug, info};
use mockall::mock;
use mockall_double::double;
use crate::client::caching_client::ARCHIVE_TAR_IDX_BYTES;
#[double]
use crate::client::CachingClient;
#[double]
use crate::client::StreamingClient;
use crate::client::TARCHIVE_CACHE_KEY;
use crate::error::GetError;

#[derive(Clone)]
pub struct TArchiveCachingClient {
    caching_client: CachingClient,
    streaming_client: StreamingClient
}

mock! {
    pub TArchiveCachingClient {
        pub fn new(caching_client: CachingClient, streaming_client: StreamingClient) -> Self;
        pub async fn get_archive_from_tar(&self, addr: &XorName) -> Result<Bytes, GetError>;
    }
    impl Clone for TArchiveCachingClient {
        fn clone(&self) -> Self;
    }
}

impl TArchiveCachingClient {
    pub fn new(caching_client: CachingClient, streaming_client: StreamingClient) -> Self {
        Self { caching_client, streaming_client }
    }

    pub async fn get_archive_from_tar(&self, addr: &XorName) -> Result<Bytes, GetError> {
        let local_streaming_client = self.streaming_client.clone();
        let local_address = addr.clone();
        let cache_entry = self.caching_client.get_hybrid_cache().get_ref().get_or_fetch(&format!("{}{}", TARCHIVE_CACHE_KEY, hex::encode(local_address)), || async move {
            let trailer_bytes = local_streaming_client.download_stream(&local_address, -20480, 0).await;
            match trailer_bytes {
                Ok(trailer_bytes) => {
                    match TArchiveCachingClient::find_subsequence(trailer_bytes.iter().as_slice(), ARCHIVE_TAR_IDX_BYTES) {
                        Some(idx) => {
                            debug!("archive.tar.idx was found in archive.tar");
                            let archive_idx_range_start = idx + 512 + 1;
                            let archive_idx_range_to = min(20480, trailer_bytes.len());
                            info!("retrieved tarchive for [{}] with range_from [{}] and range_to [{}] from network - storing in hybrid cache", hex::encode(local_address), archive_idx_range_start, archive_idx_range_to);
                            Ok(Vec::from(&trailer_bytes[archive_idx_range_start..archive_idx_range_to]))
                        },
                        None => {
                            debug!("no archive.tar.idx found in tar trailer");
                            Err(anyhow::anyhow!(format!("Failed to retrieve archive.tar.idx in tar trailer for [{}] from network", hex::encode(local_address))))
                        }
                    }
                },
                Err(e) => Err(anyhow::anyhow!(format!("Failed to download stream for [{}] from network {:?}", hex::encode(local_address), e)))
            }
        }).await?;
        info!("retrieved tarchive for [{}] from hybrid cache", hex::encode(addr));
        Ok(Bytes::from(cache_entry.value().to_vec()))
    }

    fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack.windows(needle.len()).position(|window| window == needle)
    }
}