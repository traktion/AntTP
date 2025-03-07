use std::io::Write;
use std::fs::File;
use actix_http::header;
use actix_multipart::Multipart;
use actix_web::http::header::{ETag, EntityTag};
use actix_web::{Error, HttpRequest, HttpResponse};
use actix_web::error::ErrorNotFound;
use autonomi::{Client, Wallet};
use autonomi::client::payment::PaymentOption;
use autonomi::files::PublicArchive;
use log::{info, warn};
use xor_name::XorName;
use crate::archive_helper::{ArchiveAction, ArchiveHelper, DataState};
use crate::caching_client::CachingClient;
use crate::file_service::FileService;
use crate::xor_helper::XorHelper;
use tempdir::TempDir;
use futures_util::{StreamExt as _};

pub struct ArchiveService {
    autonomi_client: Client,
    caching_autonomi_client: CachingClient,
    file_client: FileService,
    xor_helper: XorHelper,
}

impl ArchiveService {
    
    pub fn new(autonomi_client: Client, caching_autonomi_client: CachingClient, file_client: FileService, xor_helper: XorHelper) -> Self {
        ArchiveService { autonomi_client, caching_autonomi_client, file_client, xor_helper }
    }
    
    pub async fn get_data(&self, archive: PublicArchive, xor_addr: XorName, request: HttpRequest, path_parts: Vec<String>) -> Result<HttpResponse, Error> {
        let (archive_addr, archive_file_name) = self.xor_helper.assign_path_parts(path_parts.clone());
        info!("archive_addr [{}], archive_file_name [{}]", archive_addr, archive_file_name);
        
        // load app_config from archive and resolve route
        let app_config = self.caching_autonomi_client.config_get_public(archive.clone(), xor_addr).await;
        // resolve route
        let archive_relative_path = path_parts[1..].join("/").to_string();
        let (resolved_relative_path_route, has_route_map) = app_config.resolve_route(archive_relative_path.clone(), archive_file_name.clone());

        // resolve file name to chunk address
        let archive_helper = ArchiveHelper::new(archive.clone());
        let archive_info = archive_helper.resolve_archive_info(path_parts, request.clone(), resolved_relative_path_route.clone(), has_route_map);

        if archive_info.state == DataState::NotModified {
            info!("ETag matches for path [{}] at address [{}]. Client can use cached version", archive_info.path_string, format!("{:x}", archive_info.resolved_xor_addr));
            Ok(HttpResponse::NotModified().into())
        } else if archive_info.action == ArchiveAction::Redirect {
            info!("Redirect to archive directory [{}]", request.path().to_string() + "/");
            Ok(HttpResponse::MovedPermanently()
                .insert_header((header::LOCATION, request.path().to_string() + "/"))
                .finish())
        } else if archive_info.action == ArchiveAction::NotFound {
            warn!("Path not found {:?}", archive_info.path_string);
            Err(ErrorNotFound(format!("File not found {:?}", archive_info.path_string)))
        } else if archive_info.action == ArchiveAction::Listing {
            info!("List files in archive [{}]", archive_addr);
            // todo: set header when js file
            Ok(HttpResponse::Ok()
                .insert_header(ETag(EntityTag::new_strong(format!("{:x}", xor_addr).to_owned())))
                .insert_header((header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"))
                .body(archive_helper.list_files(request.headers())))
        } else {
            self.file_client.download_data_body(archive_relative_path, archive_info.resolved_xor_addr, true).await
        }
    }

    pub async fn post_data(&self, mut payload: Multipart, evm_wallet: Wallet) -> Result<HttpResponse, Error> {
        let tmp_dir = TempDir::new("anttp")?;
        info!("Creating temporary directory for archive with prefix: {:?}", tmp_dir.path().to_str());

        while let Some(item) = payload.next().await {
            let mut field = item?;

            let filename = field.content_disposition().unwrap().get_filename().expect("Failed to get filename from multipart field");
            let file_path = tmp_dir.path().join(filename);
            info!("Creating temporary file for archive: {:?}", file_path.to_str().unwrap());
            let mut tmp_file = File::create(file_path)?;

            while let Some(chunk) = field.next().await {
                tmp_file.write_all(&chunk?)?;
            }
        }

        info!("Uploading chunks");
        let (cost, archive_address) = self.autonomi_client
            .dir_upload_public(tmp_dir.into_path(), PaymentOption::Wallet(evm_wallet))
            .await
            .expect("Failed to upload archive");
        info!("Uploaded directory to network for: {}", cost);

        // Wait for the data to be replicated
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        // todo: confirm if this is needed - files are removed automatically anyway
        //drop(tmp_file);
        //tmp_dir.close().expect("Failed to close temp dir");

        info!("Successfully uploaded data at [{:?}]", archive_address);
        Ok(HttpResponse::Ok().body(format!("{:?}", archive_address)))
    }
}