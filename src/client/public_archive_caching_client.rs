use crate::client::{CachingClient, PUBLIC_ARCHIVE_CACHE_KEY};
use ant_evm::AttoTokens;
use autonomi::client::payment::PaymentOption;
use autonomi::client::{GetError, PutError};
use autonomi::data::DataAddress;
use autonomi::files::archive_public::ArchiveAddress;
use autonomi::files::{PublicArchive};
use bytes::Bytes;
use log::{info, warn};
use rmp_serde::decode;
use crate::controller::CacheType;

impl CachingClient {

    pub async fn archive_put_public(&self, archive: &PublicArchive, payment_option: PaymentOption, cache_only: Option<CacheType>) -> Result<(AttoTokens, ArchiveAddress), PutError> {
        let bytes = archive
            .to_bytes()
            .map_err(|e| PutError::Serialization(format!("Failed to serialize archive: {e:?}")))?;
        self.data_put_public(bytes, payment_option, cache_only).await
    }

    /// Fetch an archive from the network
    pub async fn archive_get_public(&self, archive_address: ArchiveAddress) -> Result<PublicArchive, decode::Error> {
        match self.archive_get_public_raw(&archive_address).await {
            Ok(bytes) => PublicArchive::from_bytes(bytes),
            Err(err) => Err(decode::Error::Uncategorized(format!("Failed to retrieve public archive at [{}] from hybrid cache: {:?}", archive_address.to_hex(), err))),
        }
    }

    pub async fn archive_get_public_raw(&self, addr: &DataAddress) -> Result<Bytes, GetError> {
        let local_caching_client = self.clone();
        let local_address = addr.clone();
        match self.hybrid_cache.get_ref().fetch(format!("{}{}", PUBLIC_ARCHIVE_CACHE_KEY, local_address.to_hex()), || async move {
            // todo: optimise range_to to first chunk length (to avoid downloading other chunks when not needed)
            let maybe_bytes = local_caching_client.download_stream(&local_address, 0, 524288).await;
            match maybe_bytes {
                Ok(bytes) => {
                    let maybe_public_archive = PublicArchive::from_bytes(bytes.clone());
                    match maybe_public_archive {
                        // confirm that serialisation can be successful, before returning the data
                        Ok(public_archive) => {
                            info!("retrieved public archive for [{}] from network - storing in hybrid cache", local_address.to_hex());
                            Ok(Vec::from(public_archive.to_bytes().expect("failed to convert PublicArchive to bytes")))
                        },
                        Err(err) => {
                            warn!("Failed to retrieve public archive for [{}] from network {:?}", local_address.to_hex(), err);
                            Err(foyer::Error::other(format!("Failed to retrieve public archive for [{}] from network {:?}", local_address.to_hex(), err)))
                        }
                    }
                },
                Err(err) => Err(foyer::Error::other(format!("Failed to download stream for [{}] from network {:?}", local_address.to_hex(), err)))
            }
        }).await {
            Ok(cache_entry) => {
                info!("retrieved public archive for [{}] from hybrid cache", addr.to_hex());
                Ok(Bytes::from(cache_entry.value().to_vec()))
            },
            Err(_) => Err(GetError::RecordNotFound),
        }
    }
}