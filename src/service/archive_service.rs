
use crate::controller::StoreType;
use crate::service::tarchive_service::TarchiveService;
#[double]
use crate::service::resolver_service::ResolverService;
use actix_multipart::form::MultipartForm;
use actix_multipart::form::tempfile::TempFile;
use actix_web::HttpRequest;
use ant_core::data::{Wallet, XorName};
use bytes::Bytes;
use hex::FromHex;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::model::path_detail::PathDetail;
use crate::error::archive_error::ArchiveError;
pub use crate::model::archive::ArchiveType;
#[double]
use crate::client::ArchiveCachingClient;
use mockall_double::double;
use crate::config::app_config::AppConfig;
use crate::error::CreateError;
use crate::model::archive::Archive;
use crate::service::archive_helper::{ArchiveHelper, ArchiveInfo};
#[double]
use crate::service::file_service::FileService;
use crate::service::resolver_service::ResolvedAddress;

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

#[derive(Debug, MultipartForm, ToSchema)]
pub struct PublicArchiveForm {
    #[multipart(limit = "1GB")]
    #[schema(value_type = Vec<String>, format = Binary, content_media_type = "application/octet-stream")]
    pub files: Vec<TempFile>,
}

#[derive(Clone)]
pub struct ArchiveService {
    /*public_archive_service: PublicArchiveService,*/
    tarchive_service: TarchiveService,
    resolver_service: ResolverService,
    archive_caching_client: ArchiveCachingClient,
    file_service: FileService
}

impl ArchiveService {
    pub fn new(/*public_archive_service: PublicArchiveService,*/ tarchive_service: TarchiveService, resolver_service: ResolverService, archive_caching_client: ArchiveCachingClient, file_service: FileService) -> Self {
        Self {
            /*public_archive_service,*/
            tarchive_service,
            resolver_service,
            archive_caching_client,
            file_service
        }
    }

    pub async fn get_archive(&self, address: String, path: Option<String>) -> Result<ArchiveResponse, ArchiveError> {
        let address = self.resolver_service.resolve_name(&address).await.unwrap_or(address);
        let archive_address = XorName::from_hex(address.as_str())?;
        let archive = self.archive_caching_client.archive_get(archive_address).await?;
        match archive.archive_type {
            ArchiveType::Public => /*self.public_archive_service.get_public_archive(address, path).await
                .map(|res| ArchiveResponse::new(res.items, res.content, res.address))
                .map_err(ArchiveError::from)*/
                Err(ArchiveError::CreateError(CreateError::InvalidData("Not implemented".to_string()))),
            ArchiveType::Tarchive => self.tarchive_service.get_tarchive(address, path).await
                .map(|res| ArchiveResponse::new(res.items, res.content, res.address))
                .map_err(ArchiveError::from),
        }
    }

    pub async fn get_archive_binary(&self, address: String, path: Option<String>) -> Result<ArchiveRaw, ArchiveError> {
        let address = self.resolver_service.resolve_name(&address).await.unwrap_or(address);
        let archive_address = XorName::from_hex(address.as_str())?;
        let archive = self.archive_caching_client.archive_get(archive_address).await?;
        match archive.archive_type {
            ArchiveType::Public => /*self.public_archive_service.get_public_archive_binary(address, path).await
                .map(|res| ArchiveRaw::new(res.items, res.content, res.address))
                .map_err(ArchiveError::from)*/
                Err(ArchiveError::CreateError(CreateError::InvalidData("Not implemented".to_string()))),
            ArchiveType::Tarchive => self.tarchive_service.get_tarchive_binary(address, path).await
                .map(|res| ArchiveRaw::new(res.items, res.content, res.address))
                .map_err(ArchiveError::from),
        }    }

    pub async fn update_archive(
        &self,
        address: String,
        target_path: Option<String>,
        form: MultipartForm<ArchiveForm>,
        wallet: Wallet,
        store_type: StoreType,
    ) -> Result<ArchiveResponse, ArchiveError> {
        let address = self.resolver_service.resolve_name(&address).await.unwrap_or(address);
        let archive_address = XorName::from_hex(address.as_str())?;
        let archive = self.archive_caching_client.archive_get(archive_address).await?;
        let files = form.into_inner().files;
        
        match archive.archive_type {
            ArchiveType::Public => {
                let public_form = MultipartForm(PublicArchiveForm { files });
                /*self.public_archive_service.update_public_archive(address, target_path, public_form, wallet, store_type).await
                    .map(|res| ArchiveResponse::new(res.items, res.content, res.address))
                    .map_err(ArchiveError::from)*/
                Err(ArchiveError::CreateError(CreateError::InvalidData("Not implemented".to_string())))
            },
            ArchiveType::Tarchive => {
                let tarchive_form = MultipartForm(PublicArchiveForm { files });
                self.tarchive_service.update_tarchive(address, target_path, tarchive_form, wallet, store_type).await
                    .map(|res| ArchiveResponse::new(vec![], "".to_string(), res.address.unwrap_or_default()))
                    .map_err(ArchiveError::from)
            },
        }
    }

    pub async fn truncate_archive(&self, address: String, path: String, wallet: Wallet, store_type: StoreType) -> Result<Upload, ArchiveError> {
        let address = self.resolver_service.resolve_name(&address).await.unwrap_or(address);
        let archive_address = XorName::from_hex(address.as_str())?;
        let archive = self.archive_caching_client.archive_get(archive_address).await?;
        match archive.archive_type {
            ArchiveType::Public => /*self.public_archive_service.truncate_public_archive(address, path, wallet, store_type).await.map(|u| Upload { address: u.address }).map_err(ArchiveError::from)*/
                Err(ArchiveError::CreateError(CreateError::InvalidData("Not implemented".to_string()))),
            ArchiveType::Tarchive =>
                self.tarchive_service.truncate_tarchive(address, path, wallet, store_type).await.map(|u| Upload { address: u.address }).map_err(ArchiveError::from),
        }
    }

    pub async fn push_archive(&self, address: String, wallet: Wallet, store_type: StoreType) -> Result<Upload, ArchiveError> {
        let address = self.resolver_service.resolve_name(&address).await.unwrap_or(address);
        let archive_address = XorName::from_hex(address.as_str())?;
        let archive = self.archive_caching_client.archive_get(archive_address).await?;
        match archive.archive_type {
            ArchiveType::Public => /*self.public_archive_service.push_public_archive(address, wallet, store_type).await.map(|u| Upload { address: u.address }).map_err(ArchiveError::from)*/
                Err(ArchiveError::CreateError(CreateError::InvalidData("Not implemented".to_string()))),
            ArchiveType::Tarchive =>
                self.tarchive_service.push_tarchive(address, wallet, store_type).await.map(|u| Upload { address: u.address }).map_err(ArchiveError::from),
        }
    }

    pub async fn create_public_archive(&self, target_path: Option<String>, form: MultipartForm<ArchiveForm>, wallet: Wallet, store_type: StoreType) -> Result<ArchiveResponse, ArchiveError> {
        let files = form.into_inner().files;
        let public_form = MultipartForm(PublicArchiveForm { files });
        /*self.public_archive_service.create_public_archive(target_path, public_form, wallet, store_type).await
            .map(|res| ArchiveResponse::new(res.items, res.content, res.address))
            .map_err(ArchiveError::from)*/
        Err(ArchiveError::CreateError(CreateError::InvalidData("Not implemented".to_string())))
    }

    pub async fn create_tarchive(&self, target_path: Option<String>, form: MultipartForm<ArchiveForm>, wallet: Wallet, store_type: StoreType) -> Result<Upload, ArchiveError> {
        let files = form.into_inner().files;
        let tarchive_form = MultipartForm(PublicArchiveForm { files });
        self.tarchive_service.create_tarchive(target_path, tarchive_form, wallet, store_type).await.map(|u| Upload { address: u.address }).map_err(ArchiveError::from)
    }

    pub async fn get_archive_info(&self, resolved_address: &ResolvedAddress, request: &HttpRequest) -> ArchiveInfo {
        let archive = resolved_address.archive.clone().expect("Archive not found");
        // load app_config from archive and resolve route
        let app_config = self.get_app_config(&archive, &resolved_address.xor_name).await;
        // resolve route
        let (resolved_route_path, has_route_map) = app_config.resolve_route(&resolved_address.file_path);

        debug!("Get data for archive_addr [{}], archive_file_name [{}]", hex::encode(resolved_address.xor_name), resolved_route_path);

        // resolve file name to chunk address
        let archive_helper = ArchiveHelper::new(archive.clone());
        archive_helper.resolve_archive_info(&resolved_address, &request, &resolved_route_path, has_route_map).await
    }

    pub async fn get_app_config(&self, archive: &Archive, archive_address_xorname: &XorName) -> AppConfig {
        let path_str = "app-conf.json";
        let mut path_parts = Vec::<String>::new();
        path_parts.push("ignore".to_string());
        path_parts.push(path_str.to_string());
        match archive.find_file(&path_str.to_string()) {
            Some(data_address_offset) => {
                info!("Downloading app-config [{}] with addr [{}] from archive [{}]", path_str, hex::encode(data_address_offset.data_address), hex::encode(archive_address_xorname));
                match self.file_service.download_data_bytes(data_address_offset.data_address, data_address_offset.offset, data_address_offset.size).await {
                    Ok(buf) => {
                        let json = String::from_utf8(buf.to_vec()).unwrap_or(String::new());
                        debug!("json [{}]", json);
                        serde_json::from_str(&json.as_str().trim()).unwrap_or(AppConfig::default())
                    }
                    Err(_) => AppConfig::default()
                }
            },
            None => AppConfig::default()
        }
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
