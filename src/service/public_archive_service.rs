use std::{env, fs};
use std::fs::create_dir;
use std::io::Error;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use actix_multipart::form::MultipartForm;
use actix_multipart::form::tempfile::TempFile;
use actix_web::HttpRequest;
use autonomi::Wallet;
use autonomi::client::payment::PaymentOption;
use autonomi::files::{Metadata, PublicArchive};
use autonomi::files::archive_public::ArchiveAddress;
use chunk_streamer::chunk_receiver::ChunkReceiver;
use log::{debug, info, warn};
use crate::service::archive_helper::{ArchiveHelper, ArchiveInfo};
use crate::client::CachingClient;
use crate::service::file_service::{FileService, RangeProps};
use crate::service::resolver_service::ResolvedAddress;
use sanitize_filename::sanitize;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use xor_name::XorName;
use crate::error::public_archive_error::PublicArchiveError;
use crate::error::chunk_error::ChunkError;
use crate::config::app_config::AppConfig;
use crate::controller::StoreType;
use crate::error::UpdateError;
use crate::model::archive::Archive;

#[derive(Serialize, Deserialize, Clone, ToSchema)]
pub struct Upload {
    #[schema(read_only)]
    pub address: Option<String>,
}

#[derive(Debug, MultipartForm, ToSchema)]
pub struct PublicArchiveForm {
    #[multipart(limit = "1GB")]
    #[schema(value_type = Vec<String>, format = Binary, content_media_type = "application/octet-stream")]
    pub files: Vec<TempFile>,
}

impl Upload {
    pub fn new(address: Option<String>) -> Self {
        Upload { address }
    }
}

#[derive(Debug)]
pub struct PublicArchiveService {
    file_client: FileService,
    caching_client: CachingClient,
}

impl PublicArchiveService {
    
    pub fn new(file_client: FileService, caching_client: CachingClient) -> Self {
        PublicArchiveService { file_client, caching_client }
    }

    pub async fn get_archive_info(&self, resolved_address: &ResolvedAddress, request: &HttpRequest) -> ArchiveInfo {
        let archive = resolved_address.archive.clone().expect("Archive not found");
        // load app_config from archive and resolve route
        let app_config = self.get_app_config(&archive, &resolved_address.xor_name).await;
        // resolve route
        let (resolved_route_path, has_route_map) = app_config.resolve_route(&resolved_address.file_path);

        debug!("Get data for archive_addr [{:x}], archive_file_name [{}]", resolved_address.xor_name, resolved_route_path);

        // resolve file name to chunk address
        let archive_helper = ArchiveHelper::new(archive.clone());
        archive_helper.resolve_archive_info(&resolved_address, &request, &resolved_route_path, has_route_map).await
    }
    
    pub async fn get_data(&self, request: &HttpRequest, archive_info: ArchiveInfo) -> Result<(ChunkReceiver, RangeProps), ChunkError> {
        self.file_client.download_data_request(request, archive_info.path_string, archive_info.resolved_xor_addr, archive_info.offset, archive_info.size).await
    }

    pub async fn get_app_config(&self, archive: &Archive, archive_address_xorname: &XorName) -> AppConfig {
        let path_str = "app-conf.json";
        let mut path_parts = Vec::<String>::new();
        path_parts.push("ignore".to_string());
        path_parts.push(path_str.to_string());
        match archive.find_file(&path_str.to_string()) {
            Some(data_address_offset) => {
                info!("Downloading app-config [{}] with addr [{}] from archive [{}]", path_str, format!("{:x}", data_address_offset.data_address.xorname()), format!("{:x}", archive_address_xorname));
                match self.file_client.download_data_bytes(*data_address_offset.data_address.xorname(), data_address_offset.offset, data_address_offset.size).await {
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

    pub async fn create_public_archive(&self, public_archive_form: MultipartForm<PublicArchiveForm>, evm_wallet: Wallet, store_type: StoreType) -> Result<Upload, PublicArchiveError> {
        info!("Uploading new public archive to the network");
        Ok(self.update_public_archive_common(public_archive_form, evm_wallet, &mut PublicArchive::new(), store_type).await?)
    }

    pub async fn update_public_archive(&self, address: String, public_archive_form: MultipartForm<PublicArchiveForm>, evm_wallet: Wallet, store_type: StoreType) -> Result<Upload, PublicArchiveError> {
        let public_archive = &mut self.caching_client.archive_get_public(ArchiveAddress::from_hex(address.as_str())?).await?;
        info!("Uploading updated public archive to the network [{:?}]", public_archive);
        Ok(self.update_public_archive_common(public_archive_form, evm_wallet, public_archive, store_type).await?)
    }

    pub async fn update_public_archive_common(&self, public_archive_form: MultipartForm<PublicArchiveForm>, evm_wallet: Wallet, public_archive: &mut PublicArchive, store_type: StoreType) -> Result<Upload, PublicArchiveError> {
        let tmp_dir = Self::create_tmp_dir()?;
        if let Some(e) = Self::move_files_to_tmp_dir(public_archive_form, tmp_dir.clone()).err() {
            Self::purge_tmp_dir(&tmp_dir);
            return Err(e);
        }
        if let Some(e) = self.update_archive(public_archive, tmp_dir.clone(), evm_wallet.clone(), store_type.clone()).await.err() {
            Self::purge_tmp_dir(&tmp_dir);
            return Err(e);
        }

        info!("Uploading public archive [{:?}]", public_archive);
        match self.caching_client.archive_put_public(&public_archive, PaymentOption::Wallet(evm_wallet), store_type).await {
            Ok(archive_address) => {
                info!("Queued command to upload public archive at [{:?}]", archive_address);
                Self::purge_tmp_dir(&tmp_dir);
                Ok(Upload::new(Some(archive_address.to_hex())))
            }
            Err(e) => {
                warn!("Failed to upload public archive: [{:?}]", e);
                Self::purge_tmp_dir(&tmp_dir);
                Err(e)
            }
        }
    }

    async fn update_archive(&self, public_archive: &mut PublicArchive, tmp_dir: PathBuf, evm_wallet: Wallet, store_type: StoreType) -> Result<(), PublicArchiveError> {
        info!("Reading directory: {:?}", &tmp_dir);
        for entry in fs::read_dir(tmp_dir.clone())? {
            let path = entry?.path();
            info!("Reading directory path: {:?}", path);

            let data_address = self.caching_client
                .file_content_upload_public(path.clone(), PaymentOption::Wallet(evm_wallet.clone()), store_type.clone())
                .await?;
            let created_at = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
            let custom_metadata = Metadata {
                created: created_at,
                modified: created_at,
                size: path.metadata()?.len(),
                extra: None,
            };

            match path.file_name() {
                Some(os_file_name) => {
                    let file_name = os_file_name.to_str().unwrap_or("").to_string();
                    let target_path = PathBuf::from(file_name);
                    info!("Adding file [{:?}] at address [{}] to public archive", target_path, data_address.to_hex());
                    public_archive.add_file(target_path, data_address, custom_metadata);
                }
                None => return Err(UpdateError::TemporaryStorage("Failed to get filename from temporary file".to_string()).into())
            }
        }
        info!("public archive [{:?}]", public_archive);
        Ok(())
    }

    fn create_tmp_dir() -> Result<PathBuf, Error> {
        let random_name = Uuid::new_v4();
        let tmp_dir = env::temp_dir().as_path().join(random_name.to_string());
        create_dir(&tmp_dir)?;
        info!("Created temporary directory for archive with prefix: {:?}", &tmp_dir);
        Ok(tmp_dir)
    }

    fn move_files_to_tmp_dir(public_archive_form: MultipartForm<PublicArchiveForm>, tmp_dir: PathBuf) -> Result<(), PublicArchiveError> {
        info!("Moving files in {:?} to tmp directory: {:?}", public_archive_form.files, &tmp_dir);
        for temp_file in public_archive_form.files.iter() {
            match temp_file.file_name.clone() {
                Some(raw_file_name) => {
                    let file_name = sanitize(raw_file_name);
                    let file_path = tmp_dir.clone().join(&file_name);

                    info!("Creating temporary file for archive: {:?}", file_path);
                    fs::rename(temp_file.file.path(), file_path)?;
                }
                None => return Err(UpdateError::TemporaryStorage("Failed to get filename from multipart field".to_string()).into())
            }
        }
        Ok(())
    }

    fn purge_tmp_dir(tmp_dir: &PathBuf) {
        fs::remove_dir_all(tmp_dir.clone()).unwrap_or_else(|e| warn!("failed to delete temporary directory at [{:?}]: {}", tmp_dir, e));
    }
}