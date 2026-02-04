use autonomi::files::archive_public::ArchiveAddress;
use autonomi::files::PublicArchive;
use log::{info, debug};
use mockall_double::double;
use tokio::join;
#[double]
use crate::client::CachingClient;
#[double]
use crate::client::PublicArchiveCachingClient;
#[double]
use crate::client::StreamingClient;
use crate::client::{ARCHIVE_CACHE_KEY, TArchiveCachingClient};
use crate::error::archive_error::ArchiveError;
use crate::model::archive::Archive;

#[derive(Debug, Clone)]
pub struct ArchiveCachingClient {
    caching_client: CachingClient,
    streaming_client: StreamingClient
}

impl ArchiveCachingClient {
    pub fn new(caching_client: CachingClient, streaming_client: StreamingClient) -> Self {
        Self { caching_client, streaming_client }
    }

    pub async fn archive_get(&self, addr: ArchiveAddress) -> Result<Archive, ArchiveError> {
        // todo: could remove caching of sub-calls, unless called directly elsewhere?
        let local_caching_client = self.caching_client.clone();
        let local_address = addr.clone();
        let local_streaming_client = self.streaming_client.clone();
        let cache_entry = self.caching_client.get_hybrid_cache().get_ref().fetch(format!("{}{}", ARCHIVE_CACHE_KEY, local_address.to_hex()), || async move {
            // todo: can these be injected?
            let public_archive_caching_client = PublicArchiveCachingClient::new(local_caching_client.clone(), local_streaming_client.clone());
            let tarchive_caching_client = TArchiveCachingClient::new(local_caching_client.clone(), local_streaming_client.clone());
            let (public_archive, tarchive) = join!(
                public_archive_caching_client.archive_get_public_raw(&addr),
                tarchive_caching_client.get_archive_from_tar(&addr)
            );
            debug!("searching for archive or tarchive at address [{}]", local_address.to_hex());
            match public_archive.ok() {
                Some(bytes) => match PublicArchive::from_bytes(bytes) {
                    Ok(public_archive) => {
                        Ok(rmp_serde::to_vec(&Archive::build_from_public_archive(public_archive)).expect("Failed to serialize public archive"))
                    },
                    Err(err) => Err(foyer::Error::other(format!("Failed to retrieve public archive at [{}] from hybrid cache: {:?}", addr.to_hex(), err))),
                },
                None => match tarchive {
                    Ok(bytes) => {
                        Ok(rmp_serde::to_vec(&Archive::build_from_tar(&addr, bytes)).expect("Failed to serialize tarchive"))
                    },
                    Err(err) => Err(foyer::Error::other(format!("Failed to retrieve tarchive at [{}] from hybrid cache: {:?}", addr.to_hex(), err))),
                }
            }
        }).await?;
        info!("retrieved archive for [{}] from hybrid cache", addr.to_hex());
        Ok(rmp_serde::from_slice(cache_entry.value())?)
    }
}