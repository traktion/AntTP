use std::io::{Cursor, Read};
use actix_multipart::form::MultipartForm;
use actix_multipart::form::tempfile::TempFile;
use autonomi::Wallet;
use bytes::Bytes;
use log::{debug, info};
use sanitize_filename::sanitize;
use serde::{Deserialize, Serialize};
use tar::{Builder, Header};
use utoipa::ToSchema;
use crate::controller::StoreType;
use crate::error::tarchive_error::TarchiveError;
use crate::service::public_data_service::PublicDataService;
use crate::model::tarchive::Tarchive;

#[derive(Serialize, Deserialize, Clone, ToSchema)]
pub struct TarchiveUpload {
    #[schema(read_only)]
    pub address: Option<String>,
}

#[derive(Debug, MultipartForm, ToSchema)]
pub struct TarchiveForm {
    #[multipart(limit = "1GB")]
    #[schema(value_type = Vec<String>, format = Binary, content_media_type = "application/octet-stream")]
    pub files: Vec<TempFile>,
}

impl TarchiveUpload {
    pub fn new(address: Option<String>) -> Self {
        TarchiveUpload { address }
    }
}

#[derive(Debug, Clone)]
pub struct TarchiveService {
    public_data_service: PublicDataService,
}

impl TarchiveService {
    pub fn new(public_data_service: PublicDataService) -> Self {
        TarchiveService { public_data_service }
    }

    pub async fn create_tarchive(&self, tarchive_form: MultipartForm<TarchiveForm>, evm_wallet: Wallet, store_type: StoreType) -> Result<TarchiveUpload, TarchiveError> {
        info!("Creating new tarchive");
        let tar_bytes = self.append_to_tar(Vec::new(), tarchive_form.into_inner())?;
        let chunk = self.public_data_service.create_public_data(tar_bytes, evm_wallet, store_type).await?;
        Ok(TarchiveUpload::new(chunk.address))
    }

    pub async fn update_tarchive(&self, address: String, tarchive_form: MultipartForm<TarchiveForm>, evm_wallet: Wallet, store_type: StoreType) -> Result<TarchiveUpload, TarchiveError> {
        info!("Updating tarchive at [{}]", address);
        let mut existing_tar_bytes = self.public_data_service.get_public_data_binary(address).await?.to_vec();
        
        // Find the index file and truncate if found to "update" (append to) the tar
        // archive.tar.idx header will have "archive.tar.idx" in the first 100 bytes
        // We look for the filename in the tar header to find where the index starts.
        // A tar header is 512 bytes, and the filename is at the beginning.
        if let Some(idx) = self.find_subsequence(&existing_tar_bytes, b"archive.tar.idx") {
             let header_start = idx;
             debug!("Found existing index at [{}], truncating for update", header_start);
             existing_tar_bytes.truncate(header_start);
        }

        let updated_tar_bytes = self.append_to_tar(existing_tar_bytes, tarchive_form.into_inner())?;
        let chunk = self.public_data_service.create_public_data(updated_tar_bytes, evm_wallet, store_type).await?;
        Ok(TarchiveUpload::new(chunk.address))
    }

    fn append_to_tar(&self, existing_data: Vec<u8>, form: TarchiveForm) -> Result<Bytes, TarchiveError> {
        let mut builder = Builder::new(existing_data);

        for temp_file in form.files {
            let file_name = match temp_file.file_name {
                Some(name) => sanitize(name),
                None => "unnamed_file".to_string(),
            };

            let mut file = temp_file.file.reopen()?;
            let mut content = Vec::new();
            file.read_to_end(&mut content)?;

            let mut header = Header::new_gnu();
            header.set_size(content.len() as u64);
            header.set_mode(0o644);

            builder.append_data(&mut header, &file_name, content.as_slice())?;
        }

        let tar_bytes = builder.into_inner()?;

        // Generate index from the final tar content
        let index = Tarchive::index(Cursor::new(&tar_bytes))?;

        // Append index as a file in the tar
        let mut builder = Builder::new(tar_bytes);
        let mut header = Header::new_gnu();
        header.set_size(index.len() as u64);
        header.set_mode(0o644);
        builder.append_data(&mut header, "archive.tar.idx", index.as_bytes())?;
        builder.finish()?;
        let final_tar_bytes = builder.into_inner()?;

        Ok(Bytes::from(final_tar_bytes))
    }

    fn find_subsequence(&self, haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack.windows(needle.len()).position(|window| window == needle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use crate::config::anttp_config::AntTpConfig;
    use clap::Parser;

    #[tokio::test]
    async fn test_append_to_tar_creates_index() {
        use actix_web::web::Data;
        let file1 = NamedTempFile::new().unwrap();
        let mut file_reopen = file1.reopen().unwrap();
        writeln!(file_reopen, "test content 1").unwrap();
        let size = file_reopen.metadata().unwrap().len();

        let form = TarchiveForm {
            files: vec![
                TempFile {
                    file: file1,
                    file_name: Some("file1.txt".to_string()),
                    content_type: None,
                    size: size as usize,
                }
            ],
        };

        let config = AntTpConfig::parse_from(vec!["anttp"]);
        let client_harness = Data::new(tokio::sync::Mutex::new(crate::client::client_harness::ClientHarness::new(ant_evm::EvmNetwork::ArbitrumOne, config.clone())));
        let (command_executor, _) = tokio::sync::mpsc::channel(1);
        let command_executor = Data::new(command_executor);
        let hybrid_cache = Data::new(foyer::HybridCacheBuilder::new()
            .memory(1024)
            .storage()
            .build()
            .await
            .unwrap());
        let caching_client = crate::client::CachingClient::new(client_harness, config, hybrid_cache, command_executor);
        
        let service = TarchiveService {
            public_data_service: PublicDataService::new(caching_client), 
        };

        let result = service.append_to_tar(Vec::new(), form).unwrap();
        let tar_bytes = result.to_vec();

        // Check if archive.tar.idx is present in the tar
        let idx_pos = tar_bytes.windows(b"archive.tar.idx".len())
            .position(|window| window == b"archive.tar.idx")
            .expect("Index file not found in tar");

        // The index content is in the data block after the 512-byte header
        // For a small index, it will be at idx_pos - (offset of name in header) + 512
        // Since name is at offset 0, it's idx_pos + 512
        let index_content_start = idx_pos + 512;
        let index_content = String::from_utf8_lossy(&tar_bytes[index_content_start..]);
        assert!(index_content.contains("file1.txt 512 15\n"));
    }

    #[tokio::test]
    async fn test_find_subsequence() {
        use actix_web::web::Data;
        let config = AntTpConfig::parse_from(vec!["anttp"]);
        let client_harness = Data::new(tokio::sync::Mutex::new(crate::client::client_harness::ClientHarness::new(ant_evm::EvmNetwork::ArbitrumOne, config.clone())));
        let (command_executor, _) = tokio::sync::mpsc::channel(1);
        let command_executor = Data::new(command_executor);
        let hybrid_cache = Data::new(foyer::HybridCacheBuilder::new()
            .memory(1024)
            .storage()
            .build()
            .await
            .unwrap());
        let caching_client = crate::client::CachingClient::new(client_harness, config, hybrid_cache, command_executor);
        
        let service = TarchiveService {
            public_data_service: PublicDataService::new(caching_client), 
        };
        let haystack = b"hello world";
        let needle = b"world";
        assert_eq!(service.find_subsequence(haystack, needle), Some(6));
        assert_eq!(service.find_subsequence(haystack, b"notfound"), None);
    }
}
