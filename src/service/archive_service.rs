use autonomi::Wallet;
use crate::controller::StoreType;
use crate::service::public_archive_service::PublicArchiveService;
use crate::service::tarchive_service::TarchiveService;
use crate::service::resolver_service::ResolverService;
use actix_multipart::form::MultipartForm;
use actix_multipart::form::tempfile::TempFile;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::model::path_detail::PathDetail;
use crate::error::archive_error::ArchiveError;

#[derive(Serialize, Deserialize, Clone, ToSchema, Debug)]
pub struct Upload {
    #[schema(read_only)]
    pub address: Option<String>,
}

impl Upload {
    pub fn new(address: Option<String>) -> Self {
        Upload { address }
    }
}

#[derive(Debug, MultipartForm, ToSchema)]
pub struct ArchiveForm {
    #[multipart(limit = "1GB")]
    #[schema(value_type = Vec<String>, format = Binary, content_media_type = "application/octet-stream")]
    pub files: Vec<TempFile>,
}

#[derive(Serialize, Deserialize, ToSchema, Debug, Clone, PartialEq)]
pub struct ArchiveResponse {
    pub items: Vec<PathDetail>,
    pub content: String,
    pub address: String,
}

impl ArchiveResponse {
    pub fn new(items: Vec<PathDetail>, content: String, address: String) -> Self {
        ArchiveResponse { items, content, address }
    }
}

#[derive(Serialize, Deserialize, ToSchema, Debug, Clone, PartialEq)]
pub struct ArchiveRaw {
    pub items: Vec<PathDetail>,
    #[schema(value_type = String, format = Binary)]
    pub content: Bytes,
    pub address: String,
}

impl ArchiveRaw {
    pub fn new(items: Vec<PathDetail>, content: Bytes, address: String) -> Self {
        ArchiveRaw { items, content, address }
    }
}

pub use crate::model::archive::ArchiveType;
use crate::client::ArchiveCachingClient;
use autonomi::files::archive_public::ArchiveAddress;

#[derive(Debug, Clone)]
pub struct ArchiveService {
    public_archive_service: PublicArchiveService,
    tarchive_service: TarchiveService,
    resolver_service: ResolverService,
    archive_caching_client: ArchiveCachingClient,
}

impl ArchiveService {
    pub fn new(public_archive_service: PublicArchiveService, tarchive_service: TarchiveService, resolver_service: ResolverService, archive_caching_client: ArchiveCachingClient) -> Self {
        Self {
            public_archive_service,
            tarchive_service,
            resolver_service,
            archive_caching_client,
        }
    }

    pub async fn get_archive(&self, address: String, path: Option<String>) -> Result<ArchiveResponse, ArchiveError> {
        let address = self.resolver_service.resolve_name(&address).await.unwrap_or(address);
        let archive_address = ArchiveAddress::from_hex(address.as_str())?;
        let archive = self.archive_caching_client.archive_get(archive_address).await?;
        match archive.archive_type {
            ArchiveType::Public => self.public_archive_service.get_public_archive(address, path).await
                .map(|res| ArchiveResponse::new(res.items, res.content, res.address))
                .map_err(ArchiveError::from),
            ArchiveType::Tarchive => self.tarchive_service.get_tarchive(address, path).await
                .map(|res| ArchiveResponse::new(res.items, res.content, res.address))
                .map_err(ArchiveError::from),
        }
    }

    pub async fn get_archive_binary(&self, address: String, path: Option<String>) -> Result<ArchiveRaw, ArchiveError> {
        let address = self.resolver_service.resolve_name(&address).await.unwrap_or(address);
        let archive_address = ArchiveAddress::from_hex(address.as_str())?;
        let archive = self.archive_caching_client.archive_get(archive_address).await?;
        match archive.archive_type {
            ArchiveType::Public => self.public_archive_service.get_public_archive_binary(address, path).await
                .map(|res| ArchiveRaw::new(res.items, res.content, res.address))
                .map_err(ArchiveError::from),
            ArchiveType::Tarchive => self.tarchive_service.get_tarchive_binary(address, path).await
                .map(|res| ArchiveRaw::new(res.items, res.content, res.address))
                .map_err(ArchiveError::from),
        }
    }

    pub async fn update_archive(
        &self,
        address: String,
        target_path: Option<String>,
        form: MultipartForm<ArchiveForm>,
        wallet: Wallet,
        store_type: StoreType,
    ) -> Result<ArchiveResponse, ArchiveError> {
        let address = self.resolver_service.resolve_name(&address).await.unwrap_or(address);
        let archive_address = ArchiveAddress::from_hex(address.as_str())?;
        let archive = self.archive_caching_client.archive_get(archive_address).await?;
        let files = form.into_inner().files;
        
        match archive.archive_type {
            ArchiveType::Public => {
                use crate::service::public_archive_service::PublicArchiveForm;
                let public_form = MultipartForm(PublicArchiveForm { files });
                self.public_archive_service.update_public_archive(address, target_path, public_form, wallet, store_type).await
                    .map(|res| ArchiveResponse::new(res.items, res.content, res.address))
                    .map_err(ArchiveError::from)
            },
            ArchiveType::Tarchive => {
                use crate::service::public_archive_service::PublicArchiveForm;
                let tarchive_form = MultipartForm(PublicArchiveForm { files });
                self.tarchive_service.update_tarchive(address, target_path, tarchive_form, wallet, store_type).await
                    .map(|res| ArchiveResponse::new(vec![], "".to_string(), res.address.unwrap_or_default()))
                    .map_err(ArchiveError::from)
            },
        }
    }

    pub async fn truncate_archive(&self, address: String, path: String, wallet: Wallet, store_type: StoreType) -> Result<Upload, ArchiveError> {
        let address = self.resolver_service.resolve_name(&address).await.unwrap_or(address);
        let archive_address = ArchiveAddress::from_hex(address.as_str())?;
        let archive = self.archive_caching_client.archive_get(archive_address).await?;
        match archive.archive_type {
            ArchiveType::Public => self.public_archive_service.truncate_public_archive(address, path, wallet, store_type).await.map(|u| Upload { address: u.address }).map_err(ArchiveError::from),
            ArchiveType::Tarchive => self.tarchive_service.truncate_tarchive(address, path, wallet, store_type).await.map(|u| Upload { address: u.address }).map_err(ArchiveError::from),
        }
    }

    pub async fn push_archive(&self, address: String, wallet: Wallet, store_type: StoreType) -> Result<Upload, ArchiveError> {
        let address = self.resolver_service.resolve_name(&address).await.unwrap_or(address);
        let archive_address = ArchiveAddress::from_hex(address.as_str())?;
        let archive = self.archive_caching_client.archive_get(archive_address).await?;
        match archive.archive_type {
            ArchiveType::Public => self.public_archive_service.push_public_archive(address, wallet, store_type).await.map(|u| Upload { address: u.address }).map_err(ArchiveError::from),
            ArchiveType::Tarchive => self.tarchive_service.push_tarchive(address, wallet, store_type).await.map(|u| Upload { address: u.address }).map_err(ArchiveError::from),
        }
    }

    pub async fn create_public_archive(&self, target_path: Option<String>, form: MultipartForm<ArchiveForm>, wallet: Wallet, store_type: StoreType) -> Result<ArchiveResponse, ArchiveError> {
        let files = form.into_inner().files;
        use crate::service::public_archive_service::PublicArchiveForm;
        let public_form = MultipartForm(PublicArchiveForm { files });
        self.public_archive_service.create_public_archive(target_path, public_form, wallet, store_type).await
            .map(|res| ArchiveResponse::new(res.items, res.content, res.address))
            .map_err(ArchiveError::from)
    }

    pub async fn create_tarchive(&self, target_path: Option<String>, form: MultipartForm<ArchiveForm>, wallet: Wallet, store_type: StoreType) -> Result<Upload, ArchiveError> {
        let files = form.into_inner().files;
        use crate::service::public_archive_service::PublicArchiveForm;
        let tarchive_form = MultipartForm(PublicArchiveForm { files });
        self.tarchive_service.create_tarchive(target_path, tarchive_form, wallet, store_type).await.map(|u| Upload { address: u.address }).map_err(ArchiveError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_archive_type_serde() {
        let public = ArchiveType::Public;
        let serialized = serde_json::to_string(&public).unwrap();
        assert_eq!(serialized, "\"public\"");
        
        let deserialized: ArchiveType = serde_json::from_str("\"public\"").unwrap();
        assert!(matches!(deserialized, ArchiveType::Public));

        let tarchive = ArchiveType::Tarchive;
        let serialized = serde_json::to_string(&tarchive).unwrap();
        assert_eq!(serialized, "\"tarchive\"");
        
        let deserialized: ArchiveType = serde_json::from_str("\"tarchive\"").unwrap();
        assert!(matches!(deserialized, ArchiveType::Tarchive));
    }
}
