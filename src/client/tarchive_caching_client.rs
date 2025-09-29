use autonomi::client::GetError;
use autonomi::data::DataAddress;
use bytes::Bytes;
use log::{debug, info};
use crate::client::caching_client::ARCHIVE_TAR_IDX_BYTES;
use crate::client::CachingClient;

impl CachingClient {

    pub async fn get_archive_from_tar(&self, addr: &DataAddress) -> Result<Bytes, GetError> {
        let local_caching_client = self.clone();
        let local_address = addr.clone();
        let local_hybrid_cache = self.hybrid_cache.clone();
        match self.hybrid_cache.get_ref().fetch(format!("tar{}", local_address.to_hex()), || async move {
            // todo: confirm whether checking header for tar signature improves performance/reliability
            // 20480
            let trailer_bytes = local_caching_client.download_stream(local_address, -20480, 0).await;
            match trailer_bytes {
                Ok(trailer_bytes) => {
                    match CachingClient::find_subsequence(trailer_bytes.iter().as_slice(), ARCHIVE_TAR_IDX_BYTES) {
                        Some(idx) => {
                            debug!("archive.tar.idx was found in archive.tar");
                            let archive_idx_range_start = idx + 512 + 1;
                            let archive_idx_range_to = 20480;
                            info!("retrieved tarchive for [{}] with range_from [{}] and range_to [{}] from network - storing in hybrid cache", local_address.to_hex(), archive_idx_range_start, archive_idx_range_to);
                            info!("hybrid cache stats [{:?}], memory cache usage [{:?}]", local_hybrid_cache.statistics(), local_hybrid_cache.memory().usage());
                            Ok(Vec::from(&trailer_bytes[archive_idx_range_start..archive_idx_range_to]))
                        },
                        None => {
                            debug!("no archive.tar.idx found in tar trailer");
                            Err(foyer::Error::other(format!("Failed to retrieve archive.tar.idx in tar trailer for [{}] from network", local_address.to_hex())))
                        }
                    }
                },
                Err(err) => Err(foyer::Error::other(format!("Failed to download stream for [{}] from network {:?}", local_address.to_hex(), err)))
            }
        }).await {
            Ok(cache_entry) => {
                info!("retrieved tarchive for [{}] from hybrid cache", addr.to_hex());
                Ok(Bytes::from(cache_entry.value().to_vec()))
            },
            Err(e) => Err(GetError::UnrecognizedDataMap(e.to_string())),
        }
    }

    fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack.windows(needle.len()).position(|window| window == needle)
    }

    // todo: is this needed? see above
    /*pub async fn is_tarchive(&self, xor_name: XorName, total_size: usize, data_map: &DataMap) -> bool {
        // https://www.gnu.org/software/tar/manual/html_node/Standard.html
        if total_size > 512 {
            let tar_magic = self.download_stream(xor_name, data_map.clone(), 257, 261).await.to_vec();
            String::from_utf8(tar_magic.clone()).unwrap_or(String::new()) == "ustar"
        } else {
            false
        }
    }*/
}