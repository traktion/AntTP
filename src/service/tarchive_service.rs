use std::{env, fs, io};
use std::fs::create_dir;
use std::io::Write;
use std::path::PathBuf;
use actix_multipart::form::MultipartForm;
use actix_web::web::Bytes;
use autonomi::Wallet;
use log::{debug, info, warn};
use sanitize_filename::sanitize;
use uuid::Uuid;
use tar::Builder;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use autonomi::data::DataAddress;

use crate::service::public_archive_service::{PublicArchiveForm, Upload, ArchiveResponse, ArchiveRaw};
use crate::service::public_data_service::PublicDataService;
use crate::service::file_service::FileService;
use mockall_double::double;
#[double]
use crate::client::TArchiveCachingClient;
use crate::error::tarchive_error::TarchiveError;
use crate::error::UpdateError;
use crate::controller::StoreType;
use crate::model::tarchive::Tarchive;
use crate::model::archive::Archive;

#[derive(Debug, Clone)]
pub struct TarchiveService {
    public_data_service: PublicDataService,
    tarchive_caching_client: TArchiveCachingClient,
    file_service: FileService,
}

impl TarchiveService {
    pub fn new(public_data_service: PublicDataService, tarchive_caching_client: TArchiveCachingClient, file_service: FileService) -> Self {
        TarchiveService { public_data_service, tarchive_caching_client, file_service }
    }

    pub async fn get_tarchive(&self, address: String, path: Option<String>) -> Result<ArchiveResponse, TarchiveError> {
        let res = self.get_tarchive_binary(address, path).await?;
        Ok(ArchiveResponse::new(res.items, BASE64_STANDARD.encode(res.content), res.address))
    }

    pub async fn get_tarchive_binary(&self, address: String, path: Option<String>) -> Result<ArchiveRaw, TarchiveError> {
        let data_address = DataAddress::from_hex(address.as_str())
            .map_err(|e| TarchiveError::GetError(crate::error::GetError::BadAddress(e.to_string())))?;

        let bytes = self.tarchive_caching_client.get_archive_from_tar(&data_address).await?;
        let archive = Archive::build_from_tar(&data_address, bytes);
        let path = path.unwrap_or_default();

        match archive.find_file(&path) {
            Some(data_address_offset) => {
                debug!("download file from tarchive at [{}]", path);
                let bytes = self.file_service.download_data_bytes(*data_address_offset.data_address.xorname(), data_address_offset.offset, data_address_offset.size).await?;
                Ok(ArchiveRaw::new(vec![], bytes.into(), address))
            }
            None => {
                debug!("download directory from tarchive at [{}]", path);
                let path_details = archive.list_dir(path);
                Ok(ArchiveRaw::new(path_details, Bytes::new(), address))
            }
        }
    }

    pub async fn create_tarchive(&self, target_path: Option<String>, tarchive_form: MultipartForm<PublicArchiveForm>, evm_wallet: Wallet, store_type: StoreType) -> Result<Upload, TarchiveError> {
        info!("Creating new tarchive");
        let tmp_dir = Self::create_tmp_dir()?;
        let tar_path = tmp_dir.join("archive.tar");

        // Create new tar file
        {
            let tar_file = fs::File::create(&tar_path)?;
            let mut builder = Builder::new(tar_file);
            self.build_tar_from_form(&mut builder, target_path, tarchive_form)?;
            builder.finish()?;
        }

        // Generate index and create final tar
        let final_tar_path = self.rebuild_with_index(&tar_path, &tmp_dir)?;

        // Upload as public data
        let result = self.upload_tar(&final_tar_path, evm_wallet, store_type).await;
        Self::purge_tmp_dir(&tmp_dir);
        result
    }

    pub async fn update_tarchive(&self, address: String, target_path: Option<String>, tarchive_form: MultipartForm<PublicArchiveForm>, evm_wallet: Wallet, store_type: StoreType) -> Result<Upload, TarchiveError> {
        info!("Updating tarchive at address [{}]", address);
        let tmp_dir = Self::create_tmp_dir()?;
        let tar_path = tmp_dir.join("archive.tar");

        // Download existing tar
        let existing_data = self.public_data_service.get_public_data_binary(address).await?;
        let mut tar_file = fs::File::create(&tar_path)?;
        tar_file.write_all(&existing_data)?;

        // Create a new tar by adding files from existing tar AND new files from form
        let updated_tar_path = tmp_dir.join("updated_archive.tar");
        {
            let updated_tar_file = fs::File::create(&updated_tar_path)?;
            let mut builder = Builder::new(updated_tar_file);

            // Add existing entries
            let mut existing_tar_file = fs::File::open(&tar_path)?;
            let mut archive = tar::Archive::new(&mut existing_tar_file);
            for entry_result in archive.entries()? {
                let mut entry = entry_result?;
                let header = entry.header().clone();
                let path = entry.path()?.to_path_buf();
                // Skip existing index if it exists, it will be recreated
                if path.to_str() == Some("archive.tar.idx") {
                    continue;
                }
                builder.append_data(&mut header.clone(), path, &mut entry)?;
            }

            // Add new files from form
            self.build_tar_from_form(&mut builder, target_path, tarchive_form)?;
            builder.finish()?;
        }

        // Generate index and create final tar
        let final_tar_path = self.rebuild_with_index(&updated_tar_path, &tmp_dir)?;

        // Upload as public data
        let result = self.upload_tar(&final_tar_path, evm_wallet, store_type).await;
        Self::purge_tmp_dir(&tmp_dir);
        result
    }

    pub async fn truncate_tarchive(&self, address: String, path: String, evm_wallet: Wallet, store_type: StoreType) -> Result<Upload, TarchiveError> {
        info!("Truncating tarchive at address [{}] with path [{}]", address, path);
        let tmp_dir = Self::create_tmp_dir()?;
        let tar_path = tmp_dir.join("archive.tar");

        // Download existing tar
        let existing_data = self.public_data_service.get_public_data_binary(address).await?;
        let mut tar_file = fs::File::create(&tar_path)?;
        tar_file.write_all(&existing_data)?;

        // Create a new tar by adding files from existing tar, omitting files at path
        let updated_tar_path = tmp_dir.join("updated_archive.tar");
        {
            let updated_tar_file = fs::File::create(&updated_tar_path)?;
            let mut builder = Builder::new(updated_tar_file);

            let sanitised_delete_path = Tarchive::sanitise_path(&path);
            let delete_prefix = format!("{}/", sanitised_delete_path);

            // Add existing entries
            let mut existing_tar_file = fs::File::open(&tar_path)?;
            let mut archive = tar::Archive::new(&mut existing_tar_file);
            for entry_result in archive.entries()? {
                let mut entry = entry_result?;
                let header = entry.header().clone();
                let entry_path = entry.path()?.to_path_buf();
                let entry_path_str = entry_path.to_str().unwrap_or_default();

                // Skip existing index
                if entry_path_str == "archive.tar.idx" {
                    continue;
                }

                // Skip files at delete path
                if entry_path_str == sanitised_delete_path || entry_path_str.starts_with(&delete_prefix) {
                    info!("Skipping file [{}] from truncated tarchive", entry_path_str);
                    continue;
                }

                builder.append_data(&mut header.clone(), entry_path, &mut entry)?;
            }
            builder.finish()?;
        }

        // Generate index and create final tar
        let final_tar_path = self.rebuild_with_index(&updated_tar_path, &tmp_dir)?;

        // Upload as public data
        let result = self.upload_tar(&final_tar_path, evm_wallet, store_type).await;
        Self::purge_tmp_dir(&tmp_dir);
        result
    }

    fn build_tar_from_form<W: Write>(&self, builder: &mut Builder<W>, target_path: Option<String>, tarchive_form: MultipartForm<PublicArchiveForm>) -> Result<(), TarchiveError> {
        for temp_file in tarchive_form.files.iter() {
            if let Some(raw_file_name) = &temp_file.file_name {
                let mut file_path = PathBuf::new();
                if let Some(target_path_str) = &target_path {
                    for part in target_path_str.split('/') {
                        let sanitised_part = sanitize(part);
                        if !sanitised_part.is_empty() && sanitised_part != ".." && sanitised_part != "." {
                            file_path.push(sanitised_part);
                        }
                    }
                }

                let file_name = sanitize(raw_file_name);
                file_path.push(file_name);
                builder.append_path_with_name(temp_file.file.path(), file_path)?;
            } else {
                return Err(UpdateError::TemporaryStorage("Failed to get filename from multipart field".to_string()).into());
            }
        }
        Ok(())
    }

    fn rebuild_with_index(&self, tar_path: &PathBuf, tmp_dir: &PathBuf) -> Result<PathBuf, TarchiveError> {
        let index_str = {
            let mut tar_file = fs::File::open(tar_path)?;
            Tarchive::index(&mut tar_file)?
        };

        let final_tar_path = tmp_dir.join("final_archive.tar");
        let final_tar_file = fs::File::create(&final_tar_path)?;
        let mut builder = Builder::new(final_tar_file);

        // Copy all entries from the source tar to the new tar
        let mut src_tar_file = fs::File::open(tar_path)?;
        let mut archive = tar::Archive::new(&mut src_tar_file);
        for entry_result in archive.entries()? {
            let mut entry = entry_result?;
            let header = entry.header().clone();
            let path = entry.path()?.to_path_buf();
            builder.append_data(&mut header.clone(), path, &mut entry)?;
        }

        // Add index
        let mut header = tar::Header::new_gnu();
        header.set_size(index_str.len() as u64);
        header.set_path("archive.tar.idx").unwrap();
        header.set_cksum();
        builder.append(&header, index_str.as_bytes())?;
        builder.finish()?;

        Ok(final_tar_path)
    }

    async fn upload_tar(&self, tar_path: &PathBuf, evm_wallet: Wallet, store_type: StoreType) -> Result<Upload, TarchiveError> {
        let tar_data = fs::read(tar_path)?;
        let chunk = self.public_data_service.create_public_data(Bytes::from(tar_data), evm_wallet, store_type).await?;
        Ok(Upload::new(chunk.address))
    }

    fn create_tmp_dir() -> Result<PathBuf, io::Error> {
        let random_name = Uuid::new_v4();
        let tmp_dir = env::temp_dir().as_path().join(random_name.to_string());
        create_dir(&tmp_dir)?;
        info!("Created temporary directory for tarchive: {:?}", &tmp_dir);
        Ok(tmp_dir)
    }

    fn purge_tmp_dir(tmp_dir: &PathBuf) {
        fs::remove_dir_all(tmp_dir.clone()).unwrap_or_else(|e| warn!("failed to delete temporary directory at [{:?}]: {}", tmp_dir, e));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::{MockPublicDataCachingClient, MockChunkCachingClient, MockTArchiveCachingClient};
    use autonomi::data::DataAddress;
    use xor_name::XorName;
    
    fn create_mock_service() -> TarchiveService {
        let mut mock_client = MockPublicDataCachingClient::default();
        let mut mock_chunk_client = MockChunkCachingClient::default();
        let mut mock_tarchive_client = MockTArchiveCachingClient::default();

        // Mock get_public_data_binary
        mock_client.expect_data_get_public()
            .returning(|_| Ok(Bytes::from(vec![])));

        // Mock create_public_data
        mock_client.expect_data_put_public()
            .returning(|_, _, _| Ok(DataAddress::new(XorName([0; 32]))));
        
        mock_tarchive_client.expect_get_archive_from_tar()
            .returning(|_| Ok(Bytes::from(vec![])));

        let public_data_service = PublicDataService::new(mock_client);
        let file_service = FileService::new(mock_chunk_client, 1);
        TarchiveService::new(public_data_service, mock_tarchive_client, file_service)
    }

    #[test]
    fn test_truncate_tarchive() {
        let mut mock_client = MockPublicDataCachingClient::default();

        // Prepare initial tar data
        let mut initial_tar = Vec::new();
        {
            let mut builder = Builder::new(&mut initial_tar);
            let data = b"content1";
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_path("keep.txt").unwrap();
            header.set_cksum();
            builder.append(&header, &data[..]).unwrap();

            let data2 = b"content2";
            let mut header2 = tar::Header::new_gnu();
            header2.set_size(data2.len() as u64);
            header2.set_path("delete.txt").unwrap();
            header2.set_cksum();
            builder.append(&header2, &data2[..]).unwrap();

            builder.finish().unwrap();
        }

        let initial_tar_bytes = Bytes::from(initial_tar);
        let get_tar_bytes = initial_tar_bytes.clone();
        mock_client.expect_data_get_public()
            .returning(move |_| Ok(get_tar_bytes.clone()));

        let xor_name = XorName::from_content(b"test");
        let address = DataAddress::new(xor_name).to_hex();

        mock_client.expect_data_put_public()
            .returning(|_, _, _| Ok(DataAddress::new(XorName([0; 32]))));

        let public_data_service = PublicDataService::new(mock_client);
        let mock_chunk_client = MockChunkCachingClient::default();
        let mock_tarchive_client = MockTArchiveCachingClient::default();
        let file_service = FileService::new(mock_chunk_client, 1);
        let service = TarchiveService::new(public_data_service, mock_tarchive_client, file_service);

        let wallet = Wallet::new_with_random_wallet(autonomi::Network::ArbitrumOne);
        let result = tokio::runtime::Runtime::new().unwrap().block_on(
            service.truncate_tarchive(address, "delete.txt".to_string(), wallet, StoreType::Memory)
        ).unwrap();

        assert!(result.address.is_some());
    }

    #[test]
    fn test_get_tarchive_directory_listing() {
        let mut mock_client = MockPublicDataCachingClient::default();
        let mut mock_chunk_client = MockChunkCachingClient::default();
        let mut mock_tarchive_client = MockTArchiveCachingClient::default();

        // Prepare index data
        let index_data = "file1.txt 512 11\n";

        mock_tarchive_client.expect_get_archive_from_tar()
            .returning(move |_| Ok(Bytes::from(index_data)));

        let public_data_service = PublicDataService::new(mock_client);
        let file_service = FileService::new(mock_chunk_client, 1);
        let service = TarchiveService::new(public_data_service, mock_tarchive_client, file_service);

        let xor_name = XorName::from_content(b"test");
        let address = DataAddress::new(xor_name).to_hex();

        let result = tokio::runtime::Runtime::new().unwrap().block_on(
            service.get_tarchive(address, None)
        ).unwrap();

        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].display, "file1.txt");
        assert!(result.content.is_empty());
    }

    #[test]
    fn test_get_tarchive_file() {
        let mut mock_client = MockPublicDataCachingClient::default();
        let mut mock_chunk_client = MockChunkCachingClient::default();
        let mut mock_tarchive_client = MockTArchiveCachingClient::default();

        // Prepare index data
        let index_data = "file1.txt 512 8\n";

        mock_tarchive_client.expect_get_archive_from_tar()
            .returning(move |_| Ok(Bytes::from(index_data)));

        // Mock chunk_get_internal for download_data_bytes
        mock_chunk_client.expect_chunk_get_internal()
            .returning(|_| Ok(autonomi::Chunk::new(Bytes::from(b"content1".to_vec()))));

        mock_chunk_client.expect_clone()
            .returning(|| {
                let mut m = MockChunkCachingClient::default();
                m.expect_chunk_get_internal()
                    .returning(|_| Ok(autonomi::Chunk::new(Bytes::from(b"content1".to_vec()))));
                m.expect_clone()
                    .returning(|| MockChunkCachingClient::default());
                m
            });

        let public_data_service = PublicDataService::new(mock_client);
        let file_service = FileService::new(mock_chunk_client, 1);
        let service = TarchiveService::new(public_data_service, mock_tarchive_client, file_service);

        let xor_name = XorName::from_content(b"test");
        let address = DataAddress::new(xor_name).to_hex();

        let result = tokio::runtime::Runtime::new().unwrap().block_on(
            service.get_tarchive(address, Some("file1.txt".to_string()))
        ).unwrap();

        assert!(result.items.is_empty());
        assert_eq!(BASE64_STANDARD.decode(result.content).unwrap(), b"content1");
    }

    #[test]
    fn test_rebuild_with_index() {
        let service = create_mock_service();
        let tmp_dir = TarchiveService::create_tmp_dir().unwrap();
        let tar_path = tmp_dir.join("test.tar");
        
        // Create initial tar
        {
            let file = fs::File::create(&tar_path).unwrap();
            let mut builder = Builder::new(file);
            let mut header = tar::Header::new_gnu();
            let data = b"content";
            header.set_size(data.len() as u64);
            header.set_path("test.txt").unwrap();
            header.set_cksum();
            builder.append(&header, &data[..]).unwrap();
            builder.finish().unwrap();
        }
        
        let final_tar_path = service.rebuild_with_index(&tar_path, &tmp_dir).unwrap();
        assert!(final_tar_path.exists());
        
        // Verify final tar contains both file and index
        let file = fs::File::open(final_tar_path).unwrap();
        let mut archive = tar::Archive::new(file);
        let entries: Vec<_> = archive.entries().unwrap().map(|e| e.unwrap().path().unwrap().to_str().unwrap().to_string()).collect();
        
        assert!(entries.contains(&"test.txt".to_string()));
        assert!(entries.contains(&"archive.tar.idx".to_string()));
        
        TarchiveService::purge_tmp_dir(&tmp_dir);
    }
}
