use autonomi::files::archive_public::ArchiveAddress;
use autonomi::files::PublicArchive;
use log::{debug, info};
use rmp_serde::decode;
use tokio::join;
use crate::client::CachingClient;
use crate::model::archive::Archive;

impl CachingClient {

    pub async fn archive_get(&self, addr: ArchiveAddress) -> Result<Archive, decode::Error> {
        // todo: could remove caching of sub-calls, unless called directly elsewhere?
        let local_caching_client = self.clone();
        let local_address = addr.clone();
        let local_hybrid_cache = self.hybrid_cache.clone();
        match self.hybrid_cache.get_ref().fetch(format!("ar{}", local_address.to_hex()), || async move {
            let (public_archive, tarchive) = join!(local_caching_client.archive_get_public_raw(&addr), local_caching_client.get_archive_from_tar(&addr));
            match public_archive {
                Ok(bytes) => match PublicArchive::from_bytes(bytes) {
                    Ok(public_archive) => {
                        debug!("hybrid cache stats [{:?}], memory cache usage [{:?}]", local_hybrid_cache.statistics(), local_hybrid_cache.memory().usage());
                        Ok(rmp_serde::to_vec(&Archive::build_from_public_archive(public_archive)).expect("Failed to serialize archive"))
                    },
                    Err(err) => Err(foyer::Error::other(format!("Failed to retrieve public archive at [{}] from hybrid cache: {:?}", addr.to_hex(), err))),
                },
                Err(_) => match tarchive {
                    Ok(bytes) => {
                        debug!("hybrid cache stats [{:?}], memory cache usage [{:?}]", local_hybrid_cache.statistics(), local_hybrid_cache.memory().usage());
                        Ok(rmp_serde::to_vec(&Archive::build_from_tar(&addr, bytes)).expect("Failed to serialize archive"))
                    },
                    Err(err) => Err(foyer::Error::other(format!("Failed to retrieve tarchive at [{}] from hybrid cache: {:?}", addr.to_hex(), err))),
                }
            }
        }).await {
            Ok(cache_entry) => {
                info!("retrieved archive for [{}] from hybrid cache", addr.to_hex());
                match rmp_serde::from_slice(cache_entry.value()) {
                    Ok(archive) => Ok(archive),
                    Err(e) => {
                        debug!("Failed to deserialize archive for [{}] from hybrid cache: {:?}. Removing entry and retrying.", addr.to_hex(), e);
                        self.hybrid_cache.remove(cache_entry.key());
                        Box::pin(self.archive_get(addr)).await
                    }
                }
            },
            Err(e) => Err(decode::Error::Uncategorized(e.to_string())),
        }
    }
}