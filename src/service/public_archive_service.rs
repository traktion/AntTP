use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use bytes::Bytes;
use std::{env, fs};
use std::fs::create_dir;
use std::io::Error;
use std::path::{PathBuf};
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
use mockall_double::double;
use crate::service::archive_helper::{ArchiveHelper, ArchiveInfo};
#[double]
use crate::client::PublicArchiveCachingClient;
#[double]
use crate::client::PublicDataCachingClient;
#[double]
use crate::service::file_service::FileService;
use crate::service::file_service::RangeProps;
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
use crate::error::{CreateError, UpdateError};
use crate::model::archive::Archive;
use crate::model::path_detail::PathDetail;

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

#[derive(Serialize, Deserialize, ToSchema, Debug, Clone, PartialEq)]
pub struct PublicArchiveResponse {
    pub items: Vec<PathDetail>,
    pub content: String,
    pub address: String,
}

impl PublicArchiveResponse {
    pub fn new(items: Vec<PathDetail>, content: String, address: String) -> Self {
        PublicArchiveResponse { items, content, address }
    }
}

#[derive(Serialize, Deserialize, ToSchema, Debug, Clone, PartialEq)]
pub struct PublicArchiveRaw {
    pub items: Vec<PathDetail>,
    #[schema(value_type = String, format = Binary)]
    pub content: Bytes,
    pub address: String,
}

impl PublicArchiveRaw {
    pub fn new(items: Vec<PathDetail>, content: Bytes, address: String) -> Self {
        PublicArchiveRaw { items, content, address }
    }
}

#[derive(Debug)]
pub struct PublicArchiveService {
    file_service: FileService,
    public_archive_caching_client: PublicArchiveCachingClient,
    public_data_caching_client: PublicDataCachingClient,
}

impl PublicArchiveService {
    
    pub fn new(file_client: FileService, public_archive_caching_client: PublicArchiveCachingClient, public_data_caching_client: PublicDataCachingClient) -> Self {
        PublicArchiveService { file_service: file_client, public_archive_caching_client, public_data_caching_client }
    }

    pub async fn push_public_archive(&self, address: String, evm_wallet: Wallet, store_type: StoreType) -> Result<Upload, PublicArchiveError> {
        // Retrieve the staged archive (from cache or network)
        let archive_address = ArchiveAddress::from_hex(address.as_str())?;
        let public_archive = self.public_archive_caching_client.archive_get_public(archive_address).await?;
        // Persist to target store_type (default expected to be network at call-site)
        match self.public_archive_caching_client.archive_put_public(&public_archive, PaymentOption::Wallet(evm_wallet), store_type).await {
            Ok(new_address) => Ok(Upload::new(Some(new_address.to_hex()))),
            Err(e) => Err(e)
        }
    }

    pub async fn get_public_archive(&self, address: String, path: Option<String>) -> Result<PublicArchiveResponse, PublicArchiveError> {
        let res = self.get_public_archive_binary(address, path).await?;
        Ok(PublicArchiveResponse::new(res.items, BASE64_STANDARD.encode(res.content), res.address))
    }

    pub async fn get_public_archive_binary(&self, address: String, path: Option<String>) -> Result<PublicArchiveRaw, PublicArchiveError> {
        let archive_address = ArchiveAddress::from_hex(address.as_str())?;
        let public_archive = self.public_archive_caching_client.archive_get_public(archive_address).await?;
        let archive = Archive::build_from_public_archive(public_archive);
        let path = path.unwrap_or_default();

        match archive.find_file(&path) {
            Some(data_address_offset) => {
                debug!("download file from public archive at [{}]", path);
                let bytes = self.public_data_caching_client.data_get_public(&data_address_offset.data_address).await?;
                Ok(PublicArchiveRaw::new(vec![], bytes, address))
            }
            None => {
                debug!("download directory from public archive at [{}]", path);
                let path_details = archive.list_dir(path);
                Ok(PublicArchiveRaw::new(path_details, Bytes::new(), address))
            }
        }
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
        self.file_service.download_data_request(request, archive_info.path_string, archive_info.resolved_xor_addr, archive_info.offset, archive_info.size).await
    }

    pub async fn get_app_config(&self, archive: &Archive, archive_address_xorname: &XorName) -> AppConfig {
        let path_str = "app-conf.json";
        let mut path_parts = Vec::<String>::new();
        path_parts.push("ignore".to_string());
        path_parts.push(path_str.to_string());
        match archive.find_file(&path_str.to_string()) {
            Some(data_address_offset) => {
                info!("Downloading app-config [{}] with addr [{}] from archive [{}]", path_str, format!("{:x}", data_address_offset.data_address.xorname()), format!("{:x}", archive_address_xorname));
                match self.file_service.download_data_bytes(*data_address_offset.data_address.xorname(), data_address_offset.offset, data_address_offset.size).await {
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

    pub async fn create_public_archive(&self, target_path: Option<String>, public_archive_form: MultipartForm<PublicArchiveForm>, evm_wallet: Wallet, store_type: StoreType) -> Result<PublicArchiveResponse, PublicArchiveError> {
        info!("Uploading new public archive to the network");
        let upload = self.update_public_archive_common(target_path.clone(), public_archive_form, evm_wallet, &mut PublicArchive::new(), store_type).await?;
        self.get_public_archive(upload.address.unwrap_or_default(), target_path).await
    }

    pub async fn update_public_archive(&self, address: String, target_path: Option<String>, public_archive_form: MultipartForm<PublicArchiveForm>, evm_wallet: Wallet, store_type: StoreType) -> Result<PublicArchiveResponse, PublicArchiveError> {
        let public_archive_data = self.public_archive_caching_client.archive_get_public(ArchiveAddress::from_hex(address.as_str())?).await?;
        let archive = Archive::build_from_public_archive(public_archive_data.clone());

        if let Some(target_path_str) = &target_path {
            if archive.find_file(target_path_str).is_some() {
                return Err(UpdateError::InvalidData(format!("Target path [{}] is a file, not a directory", target_path_str)).into());
            }
        }

        let mut public_archive = public_archive_data;
        info!("Uploading updated public archive to the network [{:?}]", public_archive);
        let upload = self.update_public_archive_common(target_path.clone(), public_archive_form, evm_wallet, &mut public_archive, store_type).await?;
        self.get_public_archive(upload.address.unwrap_or_default(), target_path).await
    }

    pub async fn truncate_public_archive(&self, address: String, path: String, evm_wallet: Wallet, store_type: StoreType) -> Result<Upload, PublicArchiveError> {
        let archive_address = ArchiveAddress::from_hex(address.as_str())?;
        let public_archive = self.public_archive_caching_client.archive_get_public(archive_address).await?;
        let archive = Archive::build_from_public_archive(public_archive);

        let tmp_dir = Self::create_tmp_dir()?;
        let path = Archive::sanitise_path(&path);

        for (file_path_str, data_address_offset) in archive.map() {
            if file_path_str == &path || file_path_str.starts_with(&format!("{}/", path)) {
                info!("Skipping file [{}] from truncated archive", file_path_str);
                continue;
            }

            let bytes = self.public_data_caching_client.data_get_public(&data_address_offset.data_address).await?;
            let mut file_path = tmp_dir.clone();
            file_path.push(file_path_str);

            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(file_path, bytes)?;
        }

        let mut new_public_archive = PublicArchive::new();
        if let Some(e) = self.update_archive(&mut new_public_archive, tmp_dir.clone(), evm_wallet.clone(), store_type.clone()).await.err() {
            Self::purge_tmp_dir(&tmp_dir);
            return Err(e);
        }

        match self.public_archive_caching_client.archive_put_public(&new_public_archive, PaymentOption::Wallet(evm_wallet), store_type).await {
            Ok(new_address) => {
                Self::purge_tmp_dir(&tmp_dir);
                Ok(Upload::new(Some(new_address.to_hex())))
            }
            Err(e) => {
                Self::purge_tmp_dir(&tmp_dir);
                Err(e)
            }
        }
    }

    pub async fn update_public_archive_common(&self, target_path: Option<String>, public_archive_form: MultipartForm<PublicArchiveForm>, evm_wallet: Wallet, public_archive: &mut PublicArchive, store_type: StoreType) -> Result<Upload, PublicArchiveError> {
        let tmp_dir = Self::create_tmp_dir()?;
        if let Some(e) = Self::move_files_to_tmp_dir(target_path, public_archive_form, tmp_dir.clone()).err() {
            Self::purge_tmp_dir(&tmp_dir);
            return Err(e);
        }
        if let Some(e) = self.update_archive(public_archive, tmp_dir.clone(), evm_wallet.clone(), store_type.clone()).await.err() {
            Self::purge_tmp_dir(&tmp_dir);
            return Err(e);
        }

        info!("Uploading public archive [{:?}]", public_archive);
        match self.public_archive_caching_client.archive_put_public(&public_archive, PaymentOption::Wallet(evm_wallet), store_type).await {
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
        self.update_archive_recursive(public_archive, &tmp_dir, &tmp_dir, evm_wallet, store_type).await
    }

    #[async_recursion::async_recursion]
    async fn update_archive_recursive(&self, public_archive: &mut PublicArchive, base_dir: &PathBuf, current_dir: &PathBuf, evm_wallet: Wallet, store_type: StoreType) -> Result<(), PublicArchiveError> {
        for entry in fs::read_dir(current_dir)? {
            let path = entry?.path();
            if path.is_dir() {
                self.update_archive_recursive(public_archive, base_dir, &path, evm_wallet.clone(), store_type.clone()).await?;
            } else {
                info!("Reading file path: {:?}", path);

                let data_address = match self.public_data_caching_client
                    .file_content_upload_public(path.clone(), PaymentOption::Wallet(evm_wallet.clone()), store_type.clone())
                    .await {
                    Ok(data_address) => data_address,
                    Err(e) => {
                        return Err(PublicArchiveError::CreateError(CreateError::Encryption(e.to_string())))
                    }
                };
                let created_at = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
                let custom_metadata = Metadata {
                    created: created_at,
                    modified: created_at,
                    size: path.metadata()?.len(),
                    extra: None,
                };

                let relative_path = path.strip_prefix(base_dir).map_err(|e| UpdateError::TemporaryStorage(format!("Failed to strip prefix: {}", e)))?;
                info!("Adding file [{:?}] at address [{}] to public archive", relative_path, data_address.to_hex());
                public_archive.add_file(PathBuf::from(relative_path), data_address, custom_metadata);
            }
        }
        Ok(())
    }

    fn create_tmp_dir() -> Result<PathBuf, Error> {
        let random_name = Uuid::new_v4();
        let tmp_dir = env::temp_dir().as_path().join(random_name.to_string());
        create_dir(&tmp_dir)?;
        info!("Created temporary directory for archive with prefix: {:?}", &tmp_dir);
        Ok(tmp_dir)
    }

    fn move_files_to_tmp_dir(target_path: Option<String>, public_archive_form: MultipartForm<PublicArchiveForm>, tmp_dir: PathBuf) -> Result<(), PublicArchiveError> {
        info!("Moving files in {:?} to tmp directory: {:?}", public_archive_form.files, &tmp_dir);

        for temp_file in public_archive_form.files.iter() {
            match temp_file.file_name.clone() {
                Some(raw_file_name) => {
                    let file_name = sanitize(raw_file_name);
                    let mut file_path = tmp_dir.clone();

                    // Check if target_path is provided
                    if let Some(target_path_str) = &target_path {
                        if !target_path_str.is_empty() {
                            // Sanitise and split target path to avoid traversals
                            for part in target_path_str.split('/') {
                                let sanitised_part = sanitize(part);
                                if !sanitised_part.is_empty() && sanitised_part != ".." && sanitised_part != "." {
                                    file_path.push(sanitised_part);
                                }
                            }
                            // Ensure the target directory exists
                            fs::create_dir_all(&file_path)?;
                        }
                    }

                    file_path.push(&file_name);

                    info!("Creating temporary file for archive: {:?}", file_path);
                    fs::copy(temp_file.file.path(), file_path)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use actix_multipart::form::tempfile::TempFile;
    use std::fs::File;
    use std::io::{Read, Write};
    use bytes::Bytes;
    use tempfile::tempdir;

    #[test]
    fn test_move_files_to_tmp_dir_with_target_path() {
        let tmp_parent = tempdir().unwrap();
        let tmp_dir = tmp_parent.path().to_path_buf();

        // Create a fake temp file
        let mut f1 = tempfile::NamedTempFile::new().unwrap();
        writeln!(f1, "file1 content").unwrap();

        let mut f2 = tempfile::NamedTempFile::new().unwrap();
        writeln!(f2, "file2 content").unwrap();

        let form = PublicArchiveForm {
            files: vec![
                TempFile {
                    file: f1,
                    file_name: Some("f1.txt".to_string()),
                    content_type: None,
                    size: 13,
                },
                TempFile {
                    file: f2,
                    file_name: Some("f2.txt".to_string()),
                    content_type: None,
                    size: 13,
                },
            ],
        };

        PublicArchiveService::move_files_to_tmp_dir(Some("dir1/subdir".to_string()), MultipartForm(form), tmp_dir.clone()).unwrap();

        // Check if files are in the right place
        let expected_f1 = tmp_dir.join("dir1").join("subdir").join("f1.txt");
        let expected_f2 = tmp_dir.join("dir1").join("subdir").join("f2.txt");

        assert!(expected_f1.exists());
        assert!(expected_f2.exists());

        let mut content1 = String::new();
        File::open(expected_f1).unwrap().read_to_string(&mut content1).unwrap();
        assert_eq!(content1.trim(), "file1 content");

        let mut content2 = String::new();
        File::open(expected_f2).unwrap().read_to_string(&mut content2).unwrap();
        assert_eq!(content2.trim(), "file2 content");
    }

    #[test]
    fn test_move_files_to_tmp_dir_mismatched_lengths() {
        let tmp_parent = tempdir().unwrap();
        let tmp_dir = tmp_parent.path().to_path_buf();

        let mut f1 = tempfile::NamedTempFile::new().unwrap();
        writeln!(f1, "file1 content").unwrap();

        let form = PublicArchiveForm {
            files: vec![
                TempFile {
                    file: f1,
                    file_name: Some("f1.txt".to_string()),
                    content_type: None,
                    size: 13,
                },
            ],
        };

        PublicArchiveService::move_files_to_tmp_dir(None, MultipartForm(form), tmp_dir.clone()).unwrap();

        let expected_f1 = tmp_dir.join("f1.txt");
        assert!(expected_f1.exists());
    }

    #[test]
    fn test_move_files_to_tmp_dir_with_comma_separated_target_path() {
        let tmp_parent = tempdir().unwrap();
        let tmp_dir = tmp_parent.path().to_path_buf();

        let mut f1 = tempfile::NamedTempFile::new().unwrap();
        writeln!(f1, "file1 content").unwrap();

        let mut f2 = tempfile::NamedTempFile::new().unwrap();
        writeln!(f2, "file2 content").unwrap();

        let form = PublicArchiveForm {
            files: vec![
                TempFile {
                    file: f1,
                    file_name: Some("f1.txt".to_string()),
                    content_type: None,
                    size: 13,
                },
                TempFile {
                    file: f2,
                    file_name: Some("f2.txt".to_string()),
                    content_type: None,
                    size: 13,
                },
            ],
        };

        PublicArchiveService::move_files_to_tmp_dir(Some("dir1".to_string()), MultipartForm(form), tmp_dir.clone()).unwrap();

        let expected_f1 = tmp_dir.join("dir1").join("f1.txt");
        let expected_f2 = tmp_dir.join("dir1").join("f2.txt");

        assert!(expected_f1.exists(), "File 1 should be in dir1");
        assert!(expected_f2.exists(), "File 2 should be in dir1");
    }

    #[tokio::test]
    async fn test_get_public_archive_directory_listing() {
        use crate::client::MockPublicArchiveCachingClient;
        use crate::client::MockPublicDataCachingClient;
        use crate::service::file_service::MockFileService;
        use mockall::predicate::eq;

        let mut mock_archive_client = MockPublicArchiveCachingClient::default();
        let mock_data_client = MockPublicDataCachingClient::default();
        let mock_file_service = MockFileService::default();

        let addr_hex = "0000000000000000000000000000000000000000000000000000000000000000";
        let archive_address = ArchiveAddress::from_hex(addr_hex).unwrap();
        let mut public_archive = PublicArchive::new();
        
        let file_addr = autonomi::data::DataAddress::new(XorName([1; 32]));
        public_archive.add_file(PathBuf::from("index.html"), file_addr, Metadata { created: 0, modified: 0, size: 11, extra: None });

        mock_archive_client.expect_archive_get_public()
            .with(eq(archive_address))
            .times(1)
            .returning(move |_| Ok(public_archive.clone()));

        let service = PublicArchiveService::new(
            mock_file_service,
            mock_archive_client,
            mock_data_client,
        );

        let result = service.get_public_archive(addr_hex.to_string(), None).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.address, addr_hex);
        assert_eq!(response.items.len(), 1);
        let item = &response.items[0];
        assert_eq!(item.display, "index.html");
        assert_eq!(item.path_type, crate::model::path_detail::PathDetailType::FILE);
        assert_eq!(response.content, "".to_string());
    }

    #[tokio::test]
    async fn test_get_public_archive_with_path() {
        use crate::client::MockPublicArchiveCachingClient;
        use crate::client::MockPublicDataCachingClient;
        use crate::service::file_service::MockFileService;
        use mockall::predicate::eq;

        let mut mock_archive_client = MockPublicArchiveCachingClient::default();
        let mut mock_data_client = MockPublicDataCachingClient::default();
        let mock_file_service = MockFileService::default();

        let addr_hex = "0000000000000000000000000000000000000000000000000000000000000000";
        let archive_address = ArchiveAddress::from_hex(addr_hex).unwrap();
        let mut public_archive = PublicArchive::new();
        
        let file_data = Bytes::from("some content");
        let file_addr = autonomi::data::DataAddress::new(XorName([2; 32]));
        public_archive.add_file(PathBuf::from("test.txt"), file_addr, Metadata { created: 0, modified: 0, size: 12, extra: None });

        mock_archive_client.expect_archive_get_public()
            .with(eq(archive_address))
            .times(1)
            .returning(move |_| Ok(public_archive.clone()));

        mock_data_client.expect_data_get_public()
            .with(eq(file_addr))
            .times(1)
            .returning(move |_| Ok(file_data.clone()));

        let service = PublicArchiveService::new(
            mock_file_service,
            mock_archive_client,
            mock_data_client,
        );

        let result = service.get_public_archive(addr_hex.to_string(), Some("test.txt".to_string())).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.address, addr_hex);
        assert_eq!(response.items, Vec::<PathDetail>::new());
        assert_eq!(response.content, BASE64_STANDARD.encode("some content"));
    }

    #[tokio::test]
    async fn test_get_public_archive_file_not_found() {
        use crate::client::MockPublicArchiveCachingClient;
        use crate::client::MockPublicDataCachingClient;
        use crate::service::file_service::MockFileService;
        use mockall::predicate::eq;

        let mut mock_archive_client = MockPublicArchiveCachingClient::default();
        let mock_data_client = MockPublicDataCachingClient::default();
        let mock_file_service = MockFileService::default();

        let addr_hex = "0000000000000000000000000000000000000000000000000000000000000000";
        let archive_address = ArchiveAddress::from_hex(addr_hex).unwrap();
        let public_archive = PublicArchive::new();

        mock_archive_client.expect_archive_get_public()
            .with(eq(archive_address))
            .times(1)
            .returning(move |_| Ok(public_archive.clone()));

        let service = PublicArchiveService::new(
            mock_file_service,
            mock_archive_client,
            mock_data_client,
        );

        let result = service.get_public_archive(addr_hex.to_string(), Some("missing.txt".to_string())).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.items.len(), 0);
        assert_eq!(response.content, "".to_string());
    }

    #[tokio::test]
    async fn test_get_archive_info() {
        use crate::client::MockPublicArchiveCachingClient;
        use crate::client::MockPublicDataCachingClient;
        use crate::service::file_service::MockFileService;

        let mock_archive_client = MockPublicArchiveCachingClient::default();
        let mock_data_client = MockPublicDataCachingClient::default();
        let mock_file_service = MockFileService::default();

        let xor_name = XorName([0; 32]);
        let archive = Archive::new(std::collections::HashMap::new(), Vec::new());
        
        let resolved_address = ResolvedAddress::new(
            true,
            Some(archive.clone()),
            xor_name,
            "test.txt".to_string(),
            false,
            false,
            true,
            5
        );
        let request = actix_web::test::TestRequest::with_uri("/test.txt").to_http_request();

        // mock get_app_config dependency (it calls file_client.download_data_bytes)
        // actually get_app_config is called, and it tries to find "app-conf.json" in archive
        // which is empty, so it returns AppConfig::default()

        let service = PublicArchiveService::new(
            mock_file_service,
            mock_archive_client,
            mock_data_client,
        );

        let info = service.get_archive_info(&resolved_address, &request).await;
        assert_eq!(info.path_string, "test.txt");
    }

    #[tokio::test]
    async fn test_get_data() {
        use crate::client::MockPublicArchiveCachingClient;
        use crate::client::MockPublicDataCachingClient;
        use crate::service::file_service::MockFileService;

        let mock_archive_client = MockPublicArchiveCachingClient::default();
        let mock_data_client = MockPublicDataCachingClient::default();
        let mut mock_file_service = MockFileService::default();

        let xor_name = XorName([1; 32]);
        let archive_info = ArchiveInfo::new(
            "test.txt".to_string(),
            xor_name,
            crate::service::archive_helper::ArchiveAction::Data,
            false,
            0,
            100,
            0
        );
        let request = actix_web::test::TestRequest::with_uri("/test.txt").to_http_request();

        mock_file_service.expect_download_data_request()
            .times(1)
            .returning(|_, _, _, _, _| Err(ChunkError::GetError(crate::error::GetError::RecordNotFound("test".to_string()))));

        let service = PublicArchiveService::new(
            mock_file_service,
            mock_archive_client,
            mock_data_client,
        );

        let result = service.get_data(&request, archive_info).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_truncate_public_archive() {
        use crate::client::MockPublicArchiveCachingClient;
        use crate::client::MockPublicDataCachingClient;
        use crate::service::file_service::MockFileService;
        use mockall::predicate::eq;

        let mut mock_archive_client = MockPublicArchiveCachingClient::default();
        let mut mock_data_client = MockPublicDataCachingClient::default();
        let mock_file_service = MockFileService::default();

        let addr_hex = "0000000000000000000000000000000000000000000000000000000000000000";
        let archive_address = ArchiveAddress::from_hex(addr_hex).unwrap();
        let mut public_archive = PublicArchive::new();
        
        let file1_data = Bytes::from("file1 content");
        let file1_addr = autonomi::data::DataAddress::new(XorName([1; 32]));
        public_archive.add_file(PathBuf::from("file1.txt"), file1_addr, Metadata { created: 0, modified: 0, size: 13, extra: None });

        let file2_addr = autonomi::data::DataAddress::new(XorName([2; 32]));
        public_archive.add_file(PathBuf::from("dir/file2.txt"), file2_addr, Metadata { created: 0, modified: 0, size: 13, extra: None });

        mock_archive_client.expect_archive_get_public()
            .with(eq(archive_address))
            .times(1)
            .returning(move |_| Ok(public_archive.clone()));

        mock_data_client.expect_data_get_public()
            .with(eq(file1_addr))
            .times(1)
            .returning(move |_| Ok(file1_data.clone()));

        // We expect dir/file2.txt to be skipped if we truncate "dir"
        
        mock_data_client.expect_file_content_upload_public()
            .times(1)
            .returning(move |_, _, _| Ok(file1_addr));

        let new_archive_address = ArchiveAddress::from_hex("1111111111111111111111111111111111111111111111111111111111111111").unwrap();
        mock_archive_client.expect_archive_put_public()
            .times(1)
            .returning(move |_, _, _| Ok(new_archive_address));

        let service = PublicArchiveService::new(
            mock_file_service,
            mock_archive_client,
            mock_data_client,
        );

        let wallet = Wallet::new_with_random_wallet(autonomi::Network::ArbitrumOne);
        let result = service.truncate_public_archive(addr_hex.to_string(), "dir".to_string(), wallet, StoreType::Memory).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap().address, Some(new_archive_address.to_hex()));
    }
}