use std::io::Write;
use std::fs::File;
use actix_http::header;
use actix_multipart::Multipart;
use actix_web::http::header::{ETag, EntityTag};
use actix_web::{Error, HttpRequest, HttpResponse};
use actix_web::error::{ErrorBadGateway, ErrorNotFound};
use actix_web::web::Data;
use ant_evm::AttoTokens;
use autonomi::{Client, Wallet};
use autonomi::client::payment::PaymentOption;
use autonomi::files::{PublicArchive, UploadError};
use autonomi::files::archive_public::ArchiveAddress;
use log::{info, warn};
use xor_name::XorName;
use crate::archive_helper::{ArchiveAction, ArchiveHelper, DataState};
use crate::caching_client::CachingClient;
use crate::file_service::FileService;
use crate::xor_helper::XorHelper;
use tempdir::TempDir;
use futures_util::{StreamExt as _};
use crate::AppState;

pub struct ArchiveService {
    autonomi_client: Client,
    caching_autonomi_client: CachingClient,
    file_client: FileService,
    xor_helper: XorHelper,
    app_state: Data<AppState>,
}

impl ArchiveService {
    
    pub fn new(autonomi_client: Client, caching_autonomi_client: CachingClient, file_client: FileService, xor_helper: XorHelper, app_state: Data<AppState>) -> Self {
        ArchiveService { autonomi_client, caching_autonomi_client, file_client, xor_helper, app_state }
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
        } else if self.file_client.has_range(&request) {
            let (range_from, range_to, _) = self.file_client.get_range(&request);
            self.file_client.download_data_stream(archive_addr, archive_info.resolved_xor_addr, false, range_from, range_to).await
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

        let local_client = self.autonomi_client.clone();
        let handle = tokio::spawn(async move {
            info!("Uploading chunks to autonomi network");
            let result: Result<(AttoTokens, ArchiveAddress), UploadError> = local_client
                .dir_upload_public(tmp_dir.into_path(), PaymentOption::Wallet(evm_wallet))
                .await;
            match result {
                Ok((_, archive_address)) => {
                    info!("Uploaded directory to network at [{:?}]", archive_address);
                    archive_address
                }
                Err(e) => {
                    warn!("Failed to upload directory to network: [{:?}]", e);
                    ArchiveAddress::new(XorName::default())
                }
            }
        });
        let handle_id = handle.id(); // todo: change to ArchiveAddress or hash for security
        self.app_state.upload_map.lock().unwrap().insert(handle_id.to_string(), handle);

        info!("Upload directory scheduled with handle id [{:?}]", handle_id.to_string());
        Ok(HttpResponse::Ok().body(format!("{:?}", handle_id.to_string())))
    }

    pub async fn get_status(&self, handle_id: String) -> Result<HttpResponse, Error> {
        match self.app_state.upload_map.lock().unwrap().get_mut(&handle_id) {
            Some(handle) => {
                if handle.is_finished() {
                    let xorname = *handle.await.unwrap().xorname();
                    if xorname != XorName::default() {
                        Ok(HttpResponse::Ok().body(format!("Upload task complete: [{:?}], archive address: [{:x}]", handle_id, xorname)))
                    } else {
                        Err(ErrorBadGateway(format!("Upload task failed: [{:?}]", handle_id)))
                    }
                } else {
                    Ok(HttpResponse::Ok().body(format!("Upload task in progress: [{:?}]", handle_id)))
                }
            }
            None => {
                Err(ErrorNotFound(format!("Upload task not found: [{:?}]", handle_id)))
            }
        }
    }
}