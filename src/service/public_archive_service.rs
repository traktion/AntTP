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
use crate::error::UpdateError;
use crate::model::archive::Archive;
use bytes::Bytes;
use crate::error::GetError;

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
    #[schema(value_type = Vec<String>, example = "[\"path/to/dir1\", \"path/to/dir2\"]")]
    pub target_path: Vec<actix_multipart::form::text::Text<String>>,
}

impl Upload {
    pub fn new(address: Option<String>) -> Self {
        Upload { address }
    }
}

#[derive(Debug, Clone)]
pub struct PublicArchiveService {
    file_client: FileService,
    public_archive_caching_client: PublicArchiveCachingClient,
    public_data_caching_client: PublicDataCachingClient,
}

impl PublicArchiveService {
    
    pub fn new(file_client: FileService, public_archive_caching_client: PublicArchiveCachingClient, public_data_caching_client: PublicDataCachingClient) -> Self {
        PublicArchiveService { file_client, public_archive_caching_client, public_data_caching_client }
    }

    pub async fn get_public_archive(&self, address: String, path: Option<String>) -> Result<Bytes, PublicArchiveError> {
        let archive_address = ArchiveAddress::from_hex(address.as_str())?;
        let public_archive = self.public_archive_caching_client.archive_get_public(archive_address).await?;

        match path {
            Some(file_path) => {
                let archive = Archive::build_from_public_archive(public_archive);
                match archive.find_file(&file_path) {
                    Some(data_address_offset) => {
                        Ok(self.public_archive_caching_client.archive_get_public_raw(&data_address_offset.data_address).await?)
                    },
                    None => Err(PublicArchiveError::GetError(GetError::RecordNotFound(format!("File not found in archive: {}", file_path))))
                }
            }
            None => {
                Ok(self.public_archive_caching_client.archive_get_public_raw(&archive_address).await?)
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
        let public_archive = &mut self.public_archive_caching_client.archive_get_public(ArchiveAddress::from_hex(address.as_str())?).await?;
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

                let data_address = self.public_data_caching_client
                    .file_content_upload_public(path.clone(), PaymentOption::Wallet(evm_wallet.clone()), store_type.clone())
                    .await?;
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

    fn move_files_to_tmp_dir(public_archive_form: MultipartForm<PublicArchiveForm>, tmp_dir: PathBuf) -> Result<(), PublicArchiveError> {
        info!("Moving files in {:?} to tmp directory: {:?}", public_archive_form.files, &tmp_dir);

        let mut target_paths = Vec::new();
        for tp in &public_archive_form.target_path {
            for part in tp.0.split(',') {
                target_paths.push(part.to_string());
            }
        }

        for (i, temp_file) in public_archive_form.files.iter().enumerate() {
            match temp_file.file_name.clone() {
                Some(raw_file_name) => {
                    let file_name = sanitize(raw_file_name);
                    let mut file_path = tmp_dir.clone();

                    // Check if target_path is provided for this file
                    if let Some(target_path_str) = target_paths.get(i) {
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
            target_path: vec![
                actix_multipart::form::text::Text("dir1/subdir".to_string()),
                actix_multipart::form::text::Text("dir2".to_string()),
            ],
        };

        PublicArchiveService::move_files_to_tmp_dir(MultipartForm(form), tmp_dir.clone()).unwrap();

        // Check if files are in the right place
        let expected_f1 = tmp_dir.join("dir1").join("subdir").join("f1.txt");
        let expected_f2 = tmp_dir.join("dir2").join("f2.txt");

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
            target_path: vec![], // Empty target_path
        };

        PublicArchiveService::move_files_to_tmp_dir(MultipartForm(form), tmp_dir.clone()).unwrap();

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
            target_path: vec![
                actix_multipart::form::text::Text("dir1,dir2".to_string()),
            ],
        };

        PublicArchiveService::move_files_to_tmp_dir(MultipartForm(form), tmp_dir.clone()).unwrap();

        let expected_f1 = tmp_dir.join("dir1").join("f1.txt");
        let expected_f2 = tmp_dir.join("dir2").join("f2.txt");

        assert!(expected_f1.exists(), "File 1 should be in dir1");
        assert!(expected_f2.exists(), "File 2 should be in dir2");
    }

    #[tokio::test]
    async fn test_get_public_archive_optional_path() {
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
        
        let file_data = Bytes::from("hello world");
        let file_addr = autonomi::data::DataAddress::new(XorName([1; 32]));
        public_archive.add_file(PathBuf::from("index.html"), file_addr, Metadata { created: 0, modified: 0, size: 11, extra: None });

        mock_archive_client.expect_archive_get_public()
            .with(eq(archive_address))
            .times(1)
            .returning(move |_| Ok(public_archive.clone()));

        mock_archive_client.expect_archive_get_public_raw()
            .with(eq(file_addr))
            .times(1)
            .returning(move |_| Ok(file_data.clone()));

        let service = PublicArchiveService::new(
            mock_file_service,
            mock_archive_client,
            mock_data_client,
        );

        let result = service.get_public_archive(addr_hex.to_string(), None).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Bytes::from("hello world"));
    }

    #[tokio::test]
    async fn test_get_public_archive_with_path() {
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
        
        let file_data = Bytes::from("some content");
        let file_addr = autonomi::data::DataAddress::new(XorName([2; 32]));
        public_archive.add_file(PathBuf::from("test.txt"), file_addr, Metadata { created: 0, modified: 0, size: 12, extra: None });

        mock_archive_client.expect_archive_get_public()
            .with(eq(archive_address))
            .times(1)
            .returning(move |_| Ok(public_archive.clone()));

        mock_archive_client.expect_archive_get_public_raw()
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
        assert_eq!(result.unwrap(), Bytes::from("some content"));
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
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_archive_info() {
        use crate::client::MockPublicArchiveCachingClient;
        use crate::client::MockPublicDataCachingClient;
        use crate::service::file_service::MockFileService;

        let mock_archive_client = MockPublicArchiveCachingClient::default();
        let mock_data_client = MockPublicDataCachingClient::default();
        let mut mock_file_service = MockFileService::default();

        let addr_hex = "0000000000000000000000000000000000000000000000000000000000000000";
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
}