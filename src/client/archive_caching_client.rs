use ant_core::data::XorName;
use log::{info, debug, error, warn};
use mockall::mock;
use mockall_double::double;
#[double]
use crate::client::CachingClient;
/*#[double]
use crate::client::PublicArchiveCachingClient;*/
#[double]
use crate::client::StreamingClient;
use crate::client::ARCHIVE_CACHE_KEY;
#[double]
use crate::client::TArchiveCachingClient;
use crate::error::archive_error::ArchiveError;
use crate::model::archive::Archive;

#[derive(Clone)]
pub struct ArchiveCachingClient {
    caching_client: CachingClient,
    streaming_client: StreamingClient
}

mock! {
    pub ArchiveCachingClient {
        pub fn new(caching_client: CachingClient, streaming_client: StreamingClient) -> Self;
        pub async fn archive_get(&self, addr: XorName) -> Result<Archive, ArchiveError>;
    }
    impl Clone for ArchiveCachingClient {
        fn clone(&self) -> Self;
    }
}

impl ArchiveCachingClient {
    pub fn new(caching_client: CachingClient, streaming_client: StreamingClient) -> Self {
        Self { caching_client, streaming_client }
    }

    pub async fn archive_get(&self, addr: XorName) -> Result<Archive, ArchiveError> {
        // todo: could remove caching of sub-calls, unless called directly elsewhere?
        let local_caching_client = self.caching_client.clone();
        let local_address = addr.clone();
        let local_streaming_client = self.streaming_client.clone();
        let cache_key = format!("{}{}", ARCHIVE_CACHE_KEY, hex::encode(local_address));
        let cache_entry = self.caching_client.get_hybrid_cache().get_ref().get_or_fetch(&cache_key.clone(), || async move {
            // todo: can these be injected?
            /*let public_archive_caching_client = PublicArchiveCachingClient::new(local_caching_client.clone(), local_streaming_client.clone());*/
            let tarchive_caching_client = TArchiveCachingClient::new(local_caching_client.clone(), local_streaming_client.clone());
            /*let (public_archive, tarchive) = join!(
                public_archive_caching_client.archive_get_public_raw(&addr),
                tarchive_caching_client.get_archive_from_tar(&addr)
            );*/
            let tarchive = tarchive_caching_client.get_archive_from_tar(&addr).await;
            debug!("searching for archive or tarchive at address [{}]", hex::encode(local_address));
            /*match public_archive {
                Ok(bytes) => match PublicArchive::from_bytes(bytes).ok() {
                    Some(public_archive) => {
                        debug!("found public archive at [{}]", local_address.to_hex());
                        match rmp_serde::to_vec(&Archive::build_from_public_archive(public_archive)) {
                            Ok(bytes) => Ok(bytes),
                            Err(e) => Err(anyhow::anyhow!(format!("Failed to serialize public archive for [{}]: {}", local_address.to_hex(), e.to_string())))
                        }
                    },
                    None => {*/
                        match tarchive {
                            Ok(bytes) => {
                                debug!("found tarchive at [{}]", hex::encode(local_address));
                                match rmp_serde::to_vec(&Archive::build_from_tar(&addr, bytes)) {
                                    Ok(bytes) => Ok(bytes),
                                    Err(e) => Err(anyhow::anyhow!(format!("Failed to serialize tarchive for [{}]: {}", hex::encode(local_address), e.to_string())))
                                }
                            },
                            Err(err) => {
                                error!("Failed to retrieve tarchive at [{}] from hybrid cache: {:?}", hex::encode(addr), err);
                                Err(anyhow::anyhow!(format!("Failed to retrieve tarchive at [{}] from hybrid cache: {:?}", hex::encode(addr), err)))
                            },
                        }
                    /*},
                },
                Err(err) =>  {
                    error!("Failed to retrieve public archive at [{}] from hybrid cache: {:?}", addr.to_hex(), err);
                    Err(anyhow::anyhow!(format!("Failed to retrieve public archive at [{}] from hybrid cache: {:?}", addr.to_hex(), err)))
                }
            }*/
        }).await?;
        info!("retrieved archive for [{}] from hybrid cache", hex::encode(addr));
        match rmp_serde::from_slice(cache_entry.value()) {
            Ok(archive) => Ok(archive),
            Err(err) => {
                warn!("Failed to deserialize archive for [{}] from hybrid cache: {:?}. Evicting and retrying...", hex::encode(addr), err);
                self.caching_client.get_hybrid_cache().get_ref().remove(&cache_key);
                Box::pin(self.archive_get(addr)).await
            }
        }
    }
}