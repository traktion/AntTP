use std::{env, fs, io};
use std::fs::{create_dir, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use actix_multipart::form::MultipartForm;
use actix_web::web::Bytes;
use autonomi::Wallet;
use log::{info, warn};
use sanitize_filename::sanitize;
use uuid::Uuid;
use tar::Builder;

use crate::service::public_archive_service::{PublicArchiveForm, Upload};
use crate::service::public_data_service::PublicDataService;
use crate::error::tarchive_error::TarchiveError;
use crate::error::UpdateError;
use crate::controller::StoreType;
use crate::model::tarchive::Tarchive;

#[derive(Debug)]
pub struct TarchiveService {
    public_data_service: PublicDataService,
}

impl TarchiveService {
    pub fn new(public_data_service: PublicDataService) -> Self {
        TarchiveService { public_data_service }
    }

    pub async fn create_tarchive(&self, public_archive_form: MultipartForm<PublicArchiveForm>, evm_wallet: Wallet, store_type: StoreType) -> Result<Upload, TarchiveError> {
        info!("Creating new tarchive");
        let tmp_dir = Self::create_tmp_dir()?;
        let tar_path = tmp_dir.join("archive.tar");

        // Create new tar file
        {
            let tar_file = fs::File::create(&tar_path)?;
            let mut builder = Builder::new(tar_file);
            self.build_tar_from_form(&mut builder, public_archive_form)?;
            builder.finish()?;
        }

        // Generate and append index
        self.append_index(&tar_path)?;

        // Upload as public data
        let result = self.upload_tar(&tar_path, evm_wallet, store_type).await;
        Self::purge_tmp_dir(&tmp_dir);
        result
    }

    pub async fn update_tarchive(&self, address: String, public_archive_form: MultipartForm<PublicArchiveForm>, evm_wallet: Wallet, store_type: StoreType) -> Result<Upload, TarchiveError> {
        info!("Updating tarchive at address [{}]", address);
        let tmp_dir = Self::create_tmp_dir()?;
        let tar_path = tmp_dir.join("archive.tar");

        // Download existing tar
        let existing_data = self.public_data_service.get_public_data_binary(address).await?;
        let mut tar_file = fs::File::create(&tar_path)?;
        tar_file.write_all(&existing_data)?;

        // Append new files
        let tar_file = OpenOptions::new().append(true).read(true).open(&tar_path)?;
        let mut builder = Builder::new(tar_file);
        self.build_tar_from_form(&mut builder, public_archive_form)?;
        builder.finish()?;

        // Generate and append index
        self.append_index(&tar_path)?;

        // Upload as public data
        let result = self.upload_tar(&tar_path, evm_wallet, store_type).await;
        Self::purge_tmp_dir(&tmp_dir);
        result
    }

    fn build_tar_from_form<W: Write>(&self, builder: &mut Builder<W>, public_archive_form: MultipartForm<PublicArchiveForm>) -> Result<(), TarchiveError> {
        let mut target_paths = Vec::new();
        for tp in &public_archive_form.target_path {
            for part in tp.0.split(',') {
                target_paths.push(part.to_string());
            }
        }

        for (i, temp_file) in public_archive_form.files.iter().enumerate() {
            if let Some(raw_file_name) = &temp_file.file_name {
                let mut file_path = PathBuf::new();
                if let Some(target_path_str) = target_paths.get(i) {
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

    fn append_index(&self, tar_path: &PathBuf) -> Result<(), TarchiveError> {
        let index_str = {
            let mut tar_file = fs::File::open(tar_path)?;
            Tarchive::index(&mut tar_file)?
        };

        let tar_file = OpenOptions::new().append(true).open(tar_path)?;
        let mut builder = Builder::new(tar_file);
        
        let mut header = tar::Header::new_gnu();
        header.set_size(index_str.len() as u64);
        header.set_path("archive.tar.idx").unwrap();
        header.set_cksum();
        builder.append(&header, index_str.as_bytes())?;
        builder.finish()?;
        Ok(())
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
