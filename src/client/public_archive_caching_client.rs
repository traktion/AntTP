use crate::client::CachingClient;
use ant_evm::AttoTokens;
use autonomi::client::payment::PaymentOption;
use autonomi::client::PutError;
use autonomi::data::DataAddress;
use autonomi::files::archive_public::ArchiveAddress;
use autonomi::files::{PublicArchive, UploadError};
use bytes::Bytes;
use rmp_serde::decode;
use std::path::PathBuf;

impl CachingClient {

    pub async fn archive_put_public(&self, archive: &PublicArchive, payment_option: PaymentOption, is_cache_only: bool) -> Result<(AttoTokens, ArchiveAddress), PutError> {
        let bytes = archive
            .to_bytes()
            .map_err(|e| PutError::Serialization(format!("Failed to serialize archive: {e:?}")))?;

        self.data_put_public(bytes, payment_option, is_cache_only).await
    }

    /// Fetch an archive from the network
    pub async fn archive_get_public(&self, archive_address: ArchiveAddress) -> Result<PublicArchive, decode::Error> {
        match self.data_get_public(&archive_address).await {
            Ok(bytes) => PublicArchive::from_bytes(bytes),
            Err(err) => Err(decode::Error::Uncategorized(format!("Failed to retrieve public archive at [{}] from hybrid cache: {:?}", archive_address.to_hex(), err))),
        }
    }

    pub async fn file_content_upload_public(&self, path: PathBuf, payment_option: PaymentOption, is_cache_only: bool) -> Result<(AttoTokens, DataAddress), UploadError> {
        let data = tokio::fs::read(path.clone()).await?;
        let data = Bytes::from(data);
        let (cost, addr) = self.data_put_public(data, payment_option.clone(), is_cache_only).await?;
        Ok((cost, addr))
    }
}