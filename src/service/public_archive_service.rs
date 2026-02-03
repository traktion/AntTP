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
use mockall_double::double;
#[double]
use crate::client::public_archive_caching_client::PublicArchiveCachingClient;
#[double]
use crate::client::public_data_caching_client::PublicDataCachingClient;
use crate::service::file_service::{FileService, RangeProps};
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use futures_util::StreamExt;
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
    #[schema(value_type = Vec<String>, example = "[\"path/to/dir1\", \"path/to/dir2\"]")]
    pub target_path: Vec<actix_multipart::form::text::Text<String>>,
}

impl Upload {
    pub fn new(address: Option<String>) -> Self {
        Upload { address }
    }
}

#[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
pub struct ArchiveContent {
    pub files: Vec<String>,
    pub content: String,
    pub address: String,
}

#[derive(Debug)]
pub struct PublicArchiveService {
    file_client: Option<FileService>,
    public_archive_caching_client: PublicArchiveCachingClient,
    public_data_caching_client: PublicDataCachingClient,
}

impl PublicArchiveService {
    
    pub fn new(file_client: FileService, public_archive_caching_client: PublicArchiveCachingClient, public_data_caching_client: PublicDataCachingClient) -> Self {
        PublicArchiveService { file_client: Some(file_client), public_archive_caching_client, public_data_caching_client }
    }

    #[cfg(test)]
    pub fn create_test_service(public_archive_caching_client: PublicArchiveCachingClient, public_data_caching_client: PublicDataCachingClient) -> Self {
        PublicArchiveService { 
            file_client: None,
            public_archive_caching_client, 
            public_data_caching_client 
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
        if let Some(ref file_client) = self.file_client {
            file_client.download_data_request(request, archive_info.path_string, archive_info.resolved_xor_addr, archive_info.offset, archive_info.size).await
        } else {
            Err(ChunkError::GetError(crate::error::GetError::RecordNotFound("File client not initialized".to_string())))
        }
    }

    pub async fn get_app_config(&self, archive: &Archive, archive_address_xorname: &XorName) -> AppConfig {
        let path_str = "app-conf.json";
        let mut path_parts = Vec::<String>::new();
        path_parts.push("ignore".to_string());
        path_parts.push(path_str.to_string());
        match archive.find_file(&path_str.to_string()) {
            Some(data_address_offset) => {
                info!("Downloading app-config [{}] with addr [{}] from archive [{}]", path_str, format!("{:x}", data_address_offset.data_address.xorname()), format!("{:x}", archive_address_xorname));
                if let Some(ref file_client) = self.file_client {
                    match file_client.download_data_bytes(*data_address_offset.data_address.xorname(), data_address_offset.offset, data_address_offset.size).await {
                        Ok(buf) => {
                            let json = String::from_utf8(buf.to_vec()).unwrap_or(String::new());
                            debug!("json [{}]", json);
                            serde_json::from_str(&json.as_str().trim()).unwrap_or(AppConfig::default())
                        }
                        Err(_) => AppConfig::default()
                    }
                } else {
                    AppConfig::default()
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
        let mut public_archive = self.public_archive_caching_client.archive_get_public(ArchiveAddress::from_hex(address.as_str())?).await?;
        info!("Uploading updated public archive to the network");
        Ok(self.update_public_archive_common(public_archive_form, evm_wallet, &mut public_archive, store_type).await?)
    }

    pub async fn get_public_archive(&self, address: String, path: Option<String>) -> Result<ArchiveContent, PublicArchiveError> {
        let path_str = path.unwrap_or_default();
        debug!("get_public_archive: address: {}, path: {}", address, path_str);
        let archive_address = ArchiveAddress::from_hex(address.as_str())?;
        let public_archive = self.public_archive_caching_client.archive_get_public(archive_address).await?;
        let sanitised_path = if path_str.is_empty() || path_str == "/" {
            "".to_string()
        } else {
            path_str.trim_start_matches('/').to_string()
        };

        // Check if the path is a file
        if let Some((data_address, _)) = public_archive.map().get(&PathBuf::from(&sanitised_path)) {
            let bytes = self.public_data_caching_client.data_get_public(data_address).await?;
            return Ok(ArchiveContent {
                files: Vec::new(),
                content: BASE64_STANDARD.encode(bytes),
                address,
            });
        }

        // Check if the path is a directory (or root)
        let mut dir_files = Vec::new();
        let prefix = if sanitised_path.is_empty() {
            "".to_string()
        } else if sanitised_path.ends_with('/') {
            sanitised_path.clone()
        } else {
            format!("{}/", sanitised_path)
        };

        for key in public_archive.map().keys() {
            let key_str = key.to_string_lossy();
            if key_str == sanitised_path {
                continue; // Already handled as file
            }

            if key_str.starts_with(&prefix) {
                let relative = &key_str[prefix.len()..];
                if let Some(first_part) = relative.split('/').next() {
                    if !first_part.is_empty() && !dir_files.contains(&first_part.to_string()) {
                        dir_files.push(first_part.to_string());
                    }
                }
            }
        }

        if dir_files.is_empty() && !sanitised_path.is_empty() {
             return Err(PublicArchiveError::GetError(crate::error::GetError::RecordNotFound(format!("Path [{}] not found in archive", path_str))));
        }

        dir_files.sort();

        Ok(ArchiveContent {
            files: dir_files,
            content: "".to_string(),
            address,
        })
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
    use super::*;
    use crate::client::public_archive_caching_client::MockPublicArchiveCachingClient;
    use crate::client::public_data_caching_client::MockPublicDataCachingClient;
    use crate::client::chunk_caching_client::MockChunkCachingClient;
    use crate::client::CachingClient;
    use autonomi::data::DataAddress;
    use bytes::Bytes;

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
    async fn test_get_public_archive_file() {
        use autonomi::data::DataAddress;
        use bytes::Bytes;
        use crate::client::public_archive_caching_client::MockPublicArchiveCachingClient;
        use crate::client::public_data_caching_client::MockPublicDataCachingClient;
        use crate::client::chunk_caching_client::MockChunkCachingClient;
        use std::mem::MaybeUninit;
        
        let mut mock_archive_client = MockPublicArchiveCachingClient::default();
        let mut mock_data_client = MockPublicDataCachingClient::default();
        
        let address_hex = "0000000000000000000000000000000000000000000000000000000000000000";
        let path = "file.txt";
        let content = "hello world";
        
        let mut archive = PublicArchive::new();
        let data_addr = DataAddress::new(xor_name::XorName([0; 32]));
        archive.add_file(PathBuf::from(path), data_addr.clone(), Metadata { size: content.len() as u64, modified: 0, created: 0, extra: None });

        mock_archive_client.expect_archive_get_public()
            .returning(move |_| Ok(archive.clone()));
            
        mock_data_client.expect_data_get_public()
            .returning(move |_| Ok(Bytes::from(content)));

        let service = PublicArchiveService::create_test_service(
            mock_archive_client,
            mock_data_client
        );

        let result = service.get_public_archive(address_hex.to_string(), Some(path.to_string())).await.unwrap();
        
        assert_eq!(result.content, BASE64_STANDARD.encode(content));
        assert!(result.files.is_empty());
        assert_eq!(result.address, address_hex);
    }

    #[tokio::test]
    async fn test_get_public_archive_directory() {
        use autonomi::data::DataAddress;
        use crate::client::public_archive_caching_client::MockPublicArchiveCachingClient;
        use crate::client::public_data_caching_client::MockPublicDataCachingClient;
        use crate::client::chunk_caching_client::MockChunkCachingClient;
        use std::mem::MaybeUninit;

        let mut mock_archive_client = MockPublicArchiveCachingClient::default();
        let mock_data_client = MockPublicDataCachingClient::default();
        
        let address_hex = "0000000000000000000000000000000000000000000000000000000000000000";
        
        let mut archive = PublicArchive::new();
        archive.add_file(PathBuf::from("dir1/file1.txt"), DataAddress::new(xor_name::XorName([0; 32])), Metadata { size: 0, modified: 0, created: 0, extra: None });
        archive.add_file(PathBuf::from("dir1/subdir/file2.txt"), DataAddress::new(xor_name::XorName([0; 32])), Metadata { size: 0, modified: 0, created: 0, extra: None });
        archive.add_file(PathBuf::from("file3.txt"), DataAddress::new(xor_name::XorName([0; 32])), Metadata { size: 0, modified: 0, created: 0, extra: None });

        mock_archive_client.expect_archive_get_public()
            .returning(move |_| Ok(archive.clone()));

        let service = PublicArchiveService::create_test_service(
            mock_archive_client,
            mock_data_client
        );

        // Test root
        let result = service.get_public_archive(address_hex.to_string(), None).await.unwrap();
        assert_eq!(result.files, vec!["dir1", "file3.txt"]);
        assert!(result.content.is_empty());

        let result = service.get_public_archive(address_hex.to_string(), Some("".to_string())).await.unwrap();
        assert_eq!(result.files, vec!["dir1", "file3.txt"]);

        // Test subdirectory
        let result = service.get_public_archive(address_hex.to_string(), Some("dir1".to_string())).await.unwrap();
        assert_eq!(result.files, vec!["file1.txt", "subdir"]);
    }
}