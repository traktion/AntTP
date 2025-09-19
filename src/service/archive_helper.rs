use std::path::PathBuf;
use actix_http::header::HeaderMap;
use actix_web::{HttpRequest};
use chrono::DateTime;
use log::{debug, info};
use xor_name::XorName;
use crate::client::CachingClient;
use crate::config::anttp_config::AntTpConfig;
use crate::model::archive::Archive;
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
    pub limit: u64,
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
        // note: offset is 0 indexed, size is 1 indexed
        //       offset is never 0 in a tarchive, due to header
        let limit = if size > 0 { size - 1 } else { 0 };
        ArchiveInfo { path_string, resolved_xor_addr, action, state, offset, size, limit }
    }
}

impl ArchiveHelper {
    pub fn new(archive: Archive, ant_tp_config: AntTpConfig) -> ArchiveHelper {
        ArchiveHelper { archive, ant_tp_config }
    }
    
    pub fn list_files(&self, path: String, header_map: &HeaderMap) -> String{
        if header_map.contains_key("Accept")
            && header_map.get("Accept").unwrap().to_str().unwrap().to_string().contains( "json") {
            self.list_files_json(path)
        } else {
            self.list_files_html(path)
        }
    }

    fn list_files_html(&self, path: String) -> String {
        let mut output = "<html><head><style>table { width: 60%; text-align: left; }</style></head><body><center>\n<table>".to_string();

        output.push_str(&format!("<h1>Index of /{}</h1>\n", path));
        output.push_str("<tr><th>Name</th><th>Last Modified</th><th>Size</th></tr>\n");

        for path_detail in self.archive.list_dir(path) {
            let mtime_datetime = DateTime::from_timestamp_millis(path_detail.modified as i64 * 1000).unwrap();
            let mtime_iso = mtime_datetime.format("%+");
            output.push_str("<tr>");
            output.push_str(&format!("<td><a href=\"{}\">{}</a></td>\n", path_detail.path, path_detail.display));
            output.push_str(&format!("<td>{}</td>\n", mtime_iso));
            output.push_str(&format!("<td>{}</td>\n", path_detail.size));
            output.push_str("</tr>");
        }
        output.push_str("</table></center></body></html>");
        debug!("list_files_html: {}", output);
        output
    }

    fn list_files_json(&self, path: String) -> String {
        let mut output = "[\n".to_string();

        let list_dir = self.archive.list_dir(path);
        let mut i = 1;
        let count = list_dir.len();
        for path_detail in list_dir {
            let mtime_datetime = DateTime::from_timestamp_millis(path_detail.modified as i64 * 1000).unwrap();
            let mtime_iso = mtime_datetime.format("%+");
            output.push_str("{");
            output.push_str(
                &format!("\"name\": \"{}\", \"type\": \"{:?}\", \"mtime\": \"{}\", \"size\": \"{}\"",
                         path_detail.path, path_detail.path_type, mtime_iso, path_detail.size));
            output.push_str("}");
            if i < count {
                output.push_str(",");
            }
            output.push_str("\n");
            i+=1;
        }

        output.push_str("]");
        debug!("list_files_json: {}", output);
        output
    }

    pub async fn resolve_archive_info(&self, path_parts: Vec<String>, request: HttpRequest, resolved_relative_path_route: String, has_route_map: bool, caching_client: CachingClient) -> ArchiveInfo {
        let request_path = request.path();
        let xor_helper = ResolverService::new(self.ant_tp_config.clone(), caching_client.clone());
        
        if self.has_moved_permanently(request_path, &resolved_relative_path_route) {
            debug!("has moved permanently");
            ArchiveInfo::new(resolved_relative_path_route, XorName::default(), ArchiveAction::Redirect, DataState::Modified, 0, 0)
        } else if has_route_map {
            debug!("retrieve route map index");
            match self.archive.find_file(resolved_relative_path_route.clone()) {
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
            let sub_path_part = path_parts[1..].join("/");
            match self.archive.find_file(sub_path_part.clone()) {
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
                None => if !self.archive.list_dir(sub_path_part.clone()).is_empty() {
                    if sub_path_part.to_string().chars().last() != Some('/') {
                        ArchiveInfo::new(format!("{}/", sub_path_part.clone()), XorName::default(), ArchiveAction::Redirect, DataState::Modified, 0, 0)
                    } else {
                        ArchiveInfo::new(sub_path_part.clone(), XorName::default(), ArchiveAction::Listing, DataState::Modified, 0, 0)
                    }
                } else {
                    ArchiveInfo::new(resolved_relative_path_route, XorName::default(), ArchiveAction::NotFound, DataState::Modified, 0, 0)
                }
            }
        } else {
            debug!("retrieve file listing");
            ArchiveInfo::new(resolved_relative_path_route, XorName::default(), ArchiveAction::Listing, DataState::Modified, 0, 0)
        }
    }

    fn has_moved_permanently(&self, request_path: &str, resolved_relative_path_route: &String) -> bool {
        resolved_relative_path_route.is_empty() && request_path.to_string().chars().last() != Some('/')
    }
}