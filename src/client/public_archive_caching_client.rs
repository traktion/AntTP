use crate::client::{CachingClient, PUBLIC_ARCHIVE_CACHE_KEY};
use ant_evm::AttoTokens;
use autonomi::client::payment::PaymentOption;
use autonomi::data::DataAddress;
use autonomi::files::archive_public::ArchiveAddress;
use autonomi::files::PublicArchive;
use bytes::Bytes;
use log::{info, warn};
use crate::error::CreateError;
use crate::controller::CacheType;
use crate::error::public_archive_error::PublicArchiveError;

impl CachingClient {

    pub async fn archive_put_public(&self, archive: &PublicArchive, payment_option: PaymentOption, cache_only: Option<CacheType>) -> Result<(AttoTokens, ArchiveAddress), PublicArchiveError> {
        match archive.to_bytes() {
            Ok(bytes) => Ok(self.data_put_public(bytes, payment_option, cache_only).await?),
            Err(e) => Err(CreateError::Serialization(format!("Failed to serialize archive: {}", e.to_string())).into()),
        }
    }

    /// Fetch an archive from the network
    pub async fn archive_get_public(&self, archive_address: ArchiveAddress) -> Result<PublicArchive, PublicArchiveError> {
        match self.archive_get_public_raw(&archive_address).await {
            Ok(bytes) => match PublicArchive::from_bytes(bytes) {
                Ok(public_archive) => Ok(public_archive),
                Err(e) => Err(CreateError::Serialization(format!("Failed to deserialize archive: {}", e.to_string())).into()),
            },
            Err(e) => Err(e),
        }
    }

    pub async fn archive_get_public_raw(&self, addr: &DataAddress) -> Result<Bytes, PublicArchiveError> {
        let local_caching_client = self.clone();
        let local_address = addr.clone();
        match self.hybrid_cache.get_ref().fetch(format!("{}{}", PUBLIC_ARCHIVE_CACHE_KEY, local_address.to_hex()), || async move {
            // todo: optimise range_to to first chunk length (to avoid downloading other chunks when not needed)
            let maybe_bytes = local_caching_client.download_stream(&local_address, 0, 524288).await;
            match maybe_bytes {
                Ok(bytes) => {
                    let maybe_public_archive = PublicArchive::from_bytes(bytes.clone());
                    match maybe_public_archive {
                        // confirm that serialization can be successful, before returning the data
                        Ok(public_archive) => {
                            info!("retrieved public archive for [{}] from network - storing in hybrid cache", local_address.to_hex());
                            match public_archive.to_bytes() {
                                Ok(cache_item) => Ok(Vec::from(cache_item)),
                                Err(e) => Err(foyer::Error::other(format!("Failed to convert PublicArchive to bytes for [{}]: {}", local_address.to_hex(), e.to_string())))
                            }
                        },
                        Err(e) => {
                            warn!("Failed to retrieve public archive for [{}] from network {:?}", local_address.to_hex(), e);
                            Err(foyer::Error::other(format!("Failed to retrieve public archive for [{}] from network: {:?}", local_address.to_hex(), e)))
                        }
                    }
                },
                Err(e) => Err(foyer::Error::other(format!("Failed to download stream for [{}] from network: {:?}", local_address.to_hex(), e)))
            }
        }).await {
            Ok(cache_entry) => {
                info!("retrieved public archive for [{}] from hybrid cache", addr.to_hex());
                Ok(Bytes::from(cache_entry.value().to_vec()))
            },
            Err(e) => Err(PublicArchiveError::GetError(e.into()))
        }
    }
}