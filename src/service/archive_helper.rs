use std::path::PathBuf;
use actix_http::header::HeaderMap;
use actix_web::{HttpRequest};
use chrono::DateTime;
use log::{debug, info};
use xor_name::XorName;
use crate::client::caching_client::CachingClient;
use crate::config::anttp_config::AntTpConfig;
use crate::service::archive::{Archive, DataAddressOffset};
use crate::service::resolver_service::ResolverService;

#[derive(Clone)]
pub struct ArchiveHelper {
    archive: Archive,
    ant_tp_config: AntTpConfig
}

#[derive(Clone)]
pub struct ArchiveInfo {
    pub path_string: String,
    pub resolved_xor_addr: XorName,
    pub action: ArchiveAction,
    pub state: DataState,
    pub offset: u64,
    pub size: u64,
}

#[derive(Clone, PartialEq, Eq)]
pub enum ArchiveAction {
    Data, Listing, Redirect, NotFound
}

#[derive(Clone, PartialEq, Eq)]
pub enum DataState {
    Modified, NotModified
}

impl ArchiveInfo {
    pub fn new(path_string: String, resolved_xor_addr: XorName, action: ArchiveAction, state: DataState, offset: u64, size: u64) -> ArchiveInfo {
        ArchiveInfo { path_string, resolved_xor_addr, action, state, offset, size }
    }
}

impl ArchiveHelper {
    pub fn new(archive: Archive, ant_tp_config: AntTpConfig) -> ArchiveHelper {
        ArchiveHelper { archive, ant_tp_config }
    }
    
    pub fn list_files(&self, header_map: &HeaderMap) -> String{
        if header_map.contains_key("Accept")
            && header_map.get("Accept").unwrap().to_str().unwrap().to_string().contains( "json") {
            self.list_files_json()
        } else {
            self.list_files_html()
        }
    }

    fn list_files_html(&self) -> String {
        let mut output = "<html><body><ul>".to_string();

        // todo: Replace with contains() once keys are a more useful shape
        for key in self.archive.map().keys() {
            let filepath = key.trim_start_matches("./").trim_start_matches("/").to_string();
            output.push_str(&format!("<li><a href=\"{}\">{}</a></li>\n", filepath, filepath));
        }
        output.push_str("</ul></body></html>");
        output
    }

    fn list_files_json(&self) -> String {
        let mut output = "[\n".to_string();

        let mut i = 1;
        let count = self.archive.map().keys().len();
        for key in self.archive.map().keys() {
            let value = self.archive.map().get(key).unwrap();
            let mtime_datetime = DateTime::from_timestamp_millis(value.modified as i64 * 1000).unwrap();
            let mtime_iso = mtime_datetime.format("%+");
            let filepath = key.trim_start_matches("./").trim_start_matches("/").to_string();            output.push_str("{");
            output.push_str(&format!("\"name\": \"{}\", \"type\": \"file\", \"mtime\": \"{}\", \"size\": \"{}\"", filepath, mtime_iso, value.size));
            output.push_str("}");
            if i < count {
                output.push_str(",");
            }
            output.push_str("\n");
            i+=1;
        }
        output.push_str("]");
        output
    }

    pub fn resolve_data_addr(&self, path_parts: Vec<String>) -> Option<DataAddressOffset> {
        // todo: Replace with contains() once keys are a more useful shape
        let path_parts_string = path_parts[1..].join("/");
        for key in self.archive.map().keys() {
            if key.replace("\\", "/").trim_start_matches("./").trim_start_matches("/").ends_with(path_parts_string.as_str()) {
                let value = self.archive.map().get(key).unwrap();
                return Some(value.clone());
            }
        }
        None
    }

    pub async fn resolve_archive_info(&self, path_parts: Vec<String>, request: HttpRequest, resolved_relative_path_route: String, has_route_map: bool, caching_client: CachingClient) -> ArchiveInfo {
        let request_path = request.path();
        let xor_helper = ResolverService::new(self.ant_tp_config.clone(), caching_client.clone());
        
        if self.has_moved_permanently(request_path, &resolved_relative_path_route) {
            debug!("has moved permanently");
            ArchiveInfo::new(resolved_relative_path_route, XorName::default(), ArchiveAction::Redirect, DataState::Modified, 0, 0)
        } else if has_route_map {
            debug!("retrieve route map index");
            match self.archive.find(resolved_relative_path_route.clone()) {
                Some(data_address_offset) => {
                    let path_buf = &PathBuf::from(resolved_relative_path_route.clone());
                    info!("Resolved path [{}], path_buf [{}] to xor address [{}]", resolved_relative_path_route, path_buf.display(), format!("{:x}", *data_address_offset.data_address.xorname()));
                    ArchiveInfo::new(
                        format!("{}{}", request_path.to_string(), data_address_offset.path.clone()),
                        *data_address_offset.data_address.xorname(),
                        ArchiveAction::Data,
                        xor_helper.get_data_state(request.headers(), data_address_offset.data_address.xorname()),
                        data_address_offset.offset,
                        data_address_offset.size
                    )
                }
                None => ArchiveInfo::new(resolved_relative_path_route, XorName::default(), ArchiveAction::NotFound, DataState::Modified, 0, 0)
            }
        } else if !resolved_relative_path_route.is_empty() {
            debug!("retrieve path and data address");
            match self.archive.find(path_parts[1..].join("/")) {
                Some(data_address_offset) => {
                    let path_buf = &PathBuf::from(resolved_relative_path_route.clone());
                    info!("Resolved path [{}], path_buf [{}] to xor address [{}]", resolved_relative_path_route, path_buf.display(), format!("{:x}", *data_address_offset.data_address.xorname()));
                    ArchiveInfo::new(
                        resolved_relative_path_route,
                        *data_address_offset.data_address.xorname(),
                        ArchiveAction::Data,
                        xor_helper.get_data_state(request.headers(), data_address_offset.data_address.xorname()),
                        data_address_offset.offset,
                        data_address_offset.size
                    )
                }
                None => ArchiveInfo::new(resolved_relative_path_route, XorName::default(), ArchiveAction::NotFound, DataState::Modified, 0, 0)
            }
        } else {
            info!("retrieve file listing");
            ArchiveInfo::new(resolved_relative_path_route, XorName::default(), ArchiveAction::Listing, DataState::Modified, 0, 0)
        }
    }

    fn has_moved_permanently(&self, request_path: &str, resolved_relative_path_route: &String) -> bool {
        resolved_relative_path_route.is_empty() && request_path.to_string().chars().last() != Some('/')
    }
}