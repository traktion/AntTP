use std::cmp::min;
use autonomi::data::DataAddress;
use bytes::Bytes;
use log::{debug, info};
use crate::client::caching_client::ARCHIVE_TAR_IDX_BYTES;
use crate::client::{CachingClient, TARCHIVE_CACHE_KEY};
use crate::error::GetError;

#[derive(Debug, Clone)]
pub struct TArchiveCachingClient {
    caching_client: CachingClient,
}

impl TArchiveCachingClient {
    pub fn new(caching_client: CachingClient) -> Self {
        Self { caching_client }
    }

    pub async fn get_archive_from_tar(&self, addr: &DataAddress) -> Result<Bytes, GetError> {
        let local_caching_client = self.caching_client.clone();
        let local_address = addr.clone();
        let cache_entry = self.caching_client.hybrid_cache.get_ref().fetch(format!("{}{}", TARCHIVE_CACHE_KEY, local_address.to_hex()), || async move {
            let trailer_bytes = local_caching_client.download_stream(&local_address, -20480, 0).await;
            match trailer_bytes {
                Ok(trailer_bytes) => {
                    match TArchiveCachingClient::find_subsequence(trailer_bytes.iter().as_slice(), ARCHIVE_TAR_IDX_BYTES) {
                        Some(idx) => {
                            debug!("archive.tar.idx was found in archive.tar");
                            let archive_idx_range_start = idx + 512 + 1;
                            let archive_idx_range_to = min(20480, trailer_bytes.len());
                            info!("retrieved tarchive for [{}] with range_from [{}] and range_to [{}] from network - storing in hybrid cache", local_address.to_hex(), archive_idx_range_start, archive_idx_range_to);
                            Ok(Vec::from(&trailer_bytes[archive_idx_range_start..archive_idx_range_to]))
                        },
                        None => {
                            debug!("no archive.tar.idx found in tar trailer");
                            Err(foyer::Error::other(format!("Failed to retrieve archive.tar.idx in tar trailer for [{}] from network", local_address.to_hex())))
                        }
                    }
                },
                Err(e) => Err(foyer::Error::other(format!("Failed to download stream for [{}] from network {:?}", local_address.to_hex(), e)))
            }
        }).await?;
        info!("retrieved tarchive for [{}] from hybrid cache", addr.to_hex());
        Ok(Bytes::from(cache_entry.value().to_vec()))
    }

    fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack.windows(needle.len()).position(|window| window == needle)
    }
}