use crate::client::CachingClient;
use ant_evm::AttoTokens;
use autonomi::client::payment::PaymentOption;
use autonomi::client::PutError;
use autonomi::files::archive_public::ArchiveAddress;
use autonomi::files::{PublicArchive};
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
        match self.data_get_public(&archive_address).await {
            Ok(bytes) => PublicArchive::from_bytes(bytes),
            Err(err) => Err(decode::Error::Uncategorized(format!("Failed to retrieve public archive at [{}] from hybrid cache: {:?}", archive_address.to_hex(), err))),
        }
    }
}