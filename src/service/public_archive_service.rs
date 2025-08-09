use std::{env, fs};
use std::io::Write;
use std::fs::{create_dir, File};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use actix_http::body::MessageBody;
use actix_http::header;
use actix_multipart::Multipart;
use actix_web::http::header::{ETag, EntityTag};
use actix_web::{Error, HttpRequest, HttpResponse};
use actix_web::error::{ErrorNotFound};
use actix_web::web::Data;
use autonomi::{Wallet};
use autonomi::client::payment::PaymentOption;
use autonomi::files::{Metadata, PublicArchive};
use autonomi::files::archive_public::ArchiveAddress;
use bytes::{BufMut, BytesMut};
use log::{debug, error, info, warn};
use crate::service::archive_helper::{ArchiveAction, ArchiveHelper, DataState};
use crate::client::caching_client::CachingClient;
use crate::service::file_service::FileService;
use crate::service::resolver_service::{ResolvedAddress, ResolverService};
use futures_util::{StreamExt as _};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use xor_name::XorName;
use crate::config::anttp_config::AntTpConfig;
use crate::{UploadState, UploaderState};
use crate::config::app_config::AppConfig;
use crate::service::archive::Archive;

#[derive(Serialize, Deserialize, Clone, ToSchema)]
pub struct Upload {
    id: String,
    status: String,
    message: String,
    address: Option<String>,
}

impl Upload {
    pub fn new(id: String, status: String, message: String, address: Option<String>) -> Self {
        Upload { id, status, message, address }
    }
}

pub struct PublicArchiveService {
    file_client: FileService<CachingClient>,
    resolver_service: ResolverService,
    uploader_state: Data<UploaderState>,
    upload_state: Data<UploadState>,
    ant_tp_config: AntTpConfig,
    caching_client: CachingClient,
}

impl PublicArchiveService {
    
    pub fn new(file_client: FileService<CachingClient>, resolver_service: ResolverService, uploader_state: Data<UploaderState>, upload_state: Data<UploadState>, ant_tp_config: AntTpConfig, caching_client: CachingClient) -> Self {
        PublicArchiveService { file_client, resolver_service, uploader_state, upload_state, ant_tp_config, caching_client }
    }
    
    pub async fn get_data(&self, resolved_address: ResolvedAddress, request: HttpRequest, path_parts: Vec<String>) -> Result<HttpResponse, Error> {
        let archive = resolved_address.archive.clone().expect("Archive not found");
        let (archive_addr, archive_file_name) = self.resolver_service.assign_path_parts(path_parts.clone());
        debug!("Get data for archive_addr [{}], archive_file_name [{}]", archive_addr, archive_file_name);

        // load app_config from archive and resolve route
        let app_config = self.get_app_config(archive.clone(), resolved_address.xor_name).await;
        // resolve route
        let archive_relative_path = path_parts[1..].join("/").to_string();
        let (resolved_relative_path_route, has_route_map) = app_config.resolve_route(archive_relative_path.clone(), archive_file_name.clone());

        // resolve file name to chunk address
        let archive_helper = ArchiveHelper::new(archive.clone(), self.ant_tp_config.clone());
        let archive_info = archive_helper.resolve_archive_info(path_parts, request.clone(), resolved_relative_path_route.clone(), has_route_map, self.caching_client.clone()).await;

        if archive_info.state == DataState::NotModified {
            debug!("ETag matches for path [{}] at address [{}]. Client can use cached version", archive_info.path_string, format!("{:x}", archive_info.resolved_xor_addr));
            Ok(HttpResponse::NotModified().into())
        } else if archive_info.action == ArchiveAction::Redirect {
            debug!("Redirect to archive directory [{}]", request.path().to_string() + "/");
            Ok(HttpResponse::MovedPermanently()
                .insert_header((header::LOCATION, request.path().to_string() + "/"))
                .finish())
        } else if archive_info.action == ArchiveAction::NotFound {
            debug!("Path not found {:?}", archive_info.path_string);
            Err(ErrorNotFound(format!("File not found {:?}", archive_info.path_string)))
        } else if archive_info.action == ArchiveAction::Listing {
            debug!("List files in archive [{}]", archive_addr);
            // todo: set header when js file
            Ok(HttpResponse::Ok()
                .insert_header(ETag(EntityTag::new_strong(format!("{:x}", resolved_address.xor_name).to_owned())))
                .insert_header((header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"))
                .body(archive_helper.list_files(request.headers()))) // todo: return .json / .body depending on accept header
        } else {
            self.file_client.download_data_stream(archive_relative_path, archive_info.resolved_xor_addr, resolved_address, &request, archive_info.offset, archive_info.size).await
        }
    }

    pub async fn get_app_config(&self, archive: Archive, archive_address_xorname: XorName) -> AppConfig {
        let path_str = "app-conf.json";
        let mut path_parts = Vec::<String>::new();
        path_parts.push("ignore".to_string());
        path_parts.push(path_str.to_string());
        match archive.find(path_str.to_string()) {
            Some(data_address_offset) => {
                info!("Downloading app-config [{}] with addr [{}] from archive [{}]", path_str, format!("{:x}", data_address_offset.data_address.xorname()), format!("{:x}", archive_address_xorname));
                match self.file_client.download_data(*data_address_offset.data_address.xorname(), data_address_offset.offset, data_address_offset.size).await {
                    Ok(mut chunk_receiver) => {
                        // todo: optimise buffer sizes
                        let mut buf = BytesMut::new();
                        let mut has_data = true;
                        while has_data {
                            match chunk_receiver.next().await {
                                Some(item) => match item {
                                    Ok(bytes) => buf.put(bytes),
                                    Err(e) => {
                                        error!("Error streaming app-config from archive: {}", e);
                                        has_data = false
                                    },
                                },
                                None => has_data = false
                            };
                        }
                        let json = String::from_utf8(buf.to_vec()).unwrap_or(String::new());
                        debug!("json [{}], raw [{:?}]", json, buf.to_vec());
                        serde_json::from_str(&json.as_str().trim()).unwrap_or(AppConfig::default())
                    }
                    Err(_) => AppConfig::default()
                }
            },
            None => AppConfig::default()
        }
    }

    pub async fn create_public_archive(&self, payload: Multipart, evm_wallet: Wallet) -> Result<HttpResponse, Error> {
        info!("Uploading new public archive to the network");
        self.update_public_archive_common(payload, evm_wallet, PublicArchive::new()).await
    }

    pub async fn update_public_archive(&self, address: String, payload: Multipart, evm_wallet: Wallet) -> Result<HttpResponse, Error> {
        match self.caching_client.archive_get_public(ArchiveAddress::from_hex(address.as_str()).unwrap()).await {
            Ok(public_archive) => {
                info!("Uploading updated public archive to the network [{:?}]", public_archive);
                self.update_public_archive_common(payload, evm_wallet, public_archive).await
            }
            Err(e) => {
                Err(ErrorNotFound(format!("Upload task not found: [{:?}]", e)))
            }
        }
    }

    pub async fn update_public_archive_common(&self, mut payload: Multipart, evm_wallet: Wallet, mut public_archive: PublicArchive) -> Result<HttpResponse, Error> {
        let random_name = Uuid::new_v4();
        let tmp_dir = env::temp_dir().as_path().join(random_name.to_string());
        create_dir(tmp_dir.clone()).unwrap();
        info!("Created temporary directory for archive with prefix: {:?}", tmp_dir.to_str());

        while let Some(item) =  payload.next().await {
            let mut field = item.unwrap();

            let filename = field.content_disposition().unwrap().get_filename().expect("Failed to get filename from multipart field");
            let file_path = tmp_dir.clone().join(filename);

            info!("Creating temporary file for archive: {:?}", file_path.to_str().unwrap());
            let mut tmp_file = File::create(file_path.clone()).unwrap();

            while let Some(chunk) = field.next().await {
                tmp_file.write_all(&chunk.unwrap()).unwrap();
            }
            tmp_file.flush().unwrap().size();
        }

        let local_client = self.caching_client.clone();
        let handle = tokio::spawn(async move {
            info!("Reading directory: {:?}", tmp_dir.clone());
            for entry in fs::read_dir(tmp_dir.clone()).unwrap() {
                info!("Reading directory entry: {:?}", entry);
                let entry = entry.expect("Failed to get directory entry");
                let path = entry.path();

                let (_, data_address) = local_client
                    .file_content_upload_public(path.clone(), PaymentOption::Wallet(evm_wallet.clone()))
                    .await.unwrap();
                let created_at = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
                let custom_metadata = Metadata {
                    created: created_at,
                    modified: created_at,
                    size: path.metadata().unwrap().len(),
                    extra: None,
                };

                let filename = path.file_name().unwrap().to_str().unwrap().to_string();
                // todo: derive path for CLI uploads with subdirs, or just migrate archive to move all files to root (better!)?
                let target_path = PathBuf::from(format!("./{}", filename));
                info!("Adding file [{:?}] at address [{}] to public archive", target_path, data_address.to_hex());
                public_archive.add_file(target_path, data_address, custom_metadata);
            }
            info!("public archive [{:?}]", public_archive);

            info!("Uploading public archive to the network [{:?}]", public_archive);
            match local_client.archive_put_public(&public_archive, PaymentOption::Wallet(evm_wallet)).await {
                Ok((cost, archive_address)) => {
                    info!("Uploaded public archive to network at [{:?}] for cost [{:?}]", archive_address, cost);
                    fs::remove_dir_all(tmp_dir.clone()).unwrap();
                    Some(archive_address)
                }
                Err(e) => {
                    warn!("Failed to upload public archive to network: [{:?}]", e);
                    fs::remove_dir_all(tmp_dir.clone()).unwrap();
                    None
                }
            }
        });
        let task_id = Uuid::new_v4();
        self.uploader_state.uploader_map.lock().unwrap().insert(task_id.to_string(), handle);

        info!("Upload directory scheduled with handle id [{:?}]", task_id.to_string());
        let upload_response = Upload::new(task_id.to_string(), "scheduled".to_string(), "".to_string(), None);
        Ok(HttpResponse::Ok().json(upload_response))
    }

    pub async fn get_status(&self, task_id: String) -> Result<HttpResponse, Error> {
        // todo: update response with message containing a reason for success/failure
        // todo: rewrite - can't poll join handle multiple times after completion (bug!)
        let _ = match self.upload_state.upload_map.lock().unwrap().get_mut(&task_id) {
            Some(uploader_state) => return Ok(HttpResponse::Ok().json(uploader_state)),
            None => false 
        };
            
        let upload_response = match self.uploader_state.uploader_map.lock().unwrap().get_mut(&task_id) {
            Some(handle) => {
                if handle.is_finished() {
                    match handle.await {
                        Ok(archive_address) => {
                            if archive_address.is_some() {
                                let upload = Upload::new(task_id.to_string(), "succeeded".to_string(), "".to_string(), Some(archive_address.unwrap().to_hex()));
                                self.upload_state.upload_map.lock().unwrap().insert(task_id.to_string(), upload.clone());
                                upload
                            } else {
                                let upload = Upload::new(task_id.to_string(), "failed".to_string(), "Missing address".to_string(), None);
                                self.upload_state.upload_map.lock().unwrap().insert(task_id.to_string(), upload.clone());
                                upload
                            }
                        }
                        Err(e) => {
                            let upload = Upload::new(task_id.to_string(), "failed".to_string(), e.to_string(), None);
                            self.upload_state.upload_map.lock().unwrap().insert(task_id.to_string(), upload.clone());
                            upload
                        }
                    }
                } else {
                    Upload::new(task_id.to_string(), "started".to_string(), "".to_string(), None)
                }
            }
            None => {
                Upload::new(task_id.to_string(), "unknown".to_string(), "".to_string(), None)
            }
        };
        if upload_response.status == "failed" || upload_response.status == "succeeded" {
            self.uploader_state.uploader_map.lock().unwrap().remove(&task_id);
        }
        Ok(HttpResponse::Ok().json(upload_response))
    }
}