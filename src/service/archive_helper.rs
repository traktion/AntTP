use std::path::PathBuf;
use actix_http::header::HeaderMap;
use actix_web::{Error, HttpRequest};
use actix_web::error::ErrorInternalServerError;
use autonomi::data::{DataAddress};
use autonomi::files::PublicArchive;
use chrono::DateTime;
use log::{debug, info};
use xor_name::XorName;
use crate::client::caching_client::CachingClient;
use crate::config::anttp_config::AntTpConfig;
use crate::service::resolver_service::ResolverService;

#[derive(Clone)]
pub struct ArchiveHelper {
    public_archive: PublicArchive,
    ant_tp_config: AntTpConfig
}

#[derive(Clone)]
pub struct ArchiveInfo {
    pub path_string: String,
    pub resolved_xor_addr: XorName,
    pub action: ArchiveAction,
    pub state: DataState,
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
    pub fn new(path_string: String, resolved_xor_addr: XorName, action: ArchiveAction, state: DataState) -> ArchiveInfo {
        ArchiveInfo { path_string, resolved_xor_addr, action, state }
    }
}

impl ArchiveHelper {
    pub fn new(public_archive: PublicArchive, ant_tp_config: AntTpConfig) -> ArchiveHelper {
        ArchiveHelper { public_archive, ant_tp_config }
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
        for key in self.public_archive.map().keys() {
            let filepath = key.to_str().unwrap().to_string().trim_start_matches("./").trim_start_matches("/").to_string();
            output.push_str(&format!("<li><a href=\"{}\">{}</a></li>\n", filepath, filepath));
        }
        output.push_str("</ul></body></html>");
        output
    }

    fn list_files_json(&self) -> String {
        let mut output = "[\n".to_string();

        let mut i = 1;
        let count = self.public_archive.map().keys().len();
        for key in self.public_archive.map().keys() {
            let (_, metadata) = self.public_archive.map().get(key).unwrap();
            let mtime_datetime = DateTime::from_timestamp_millis(metadata.modified as i64 * 1000).unwrap();
            let mtime_iso = mtime_datetime.format("%+");
            let filepath = key.to_str().unwrap().to_string().trim_start_matches("./").trim_start_matches("/").to_string();            output.push_str("{");
            output.push_str(&format!("\"name\": \"{}\", \"type\": \"file\", \"mtime\": \"{}\", \"size\": \"{}\"", filepath, mtime_iso, metadata.size));
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

    pub fn resolve_data_addr(&self, path_parts: Vec<String>) -> Result<DataAddress, Error> {
        self.public_archive.iter().for_each(|(path_buf, data_address, _)| debug!("archive entry: [{}] at [{:x}]", path_buf.to_str().unwrap().to_string().replace("\\", "/"), data_address.xorname()));

        // todo: Replace with contains() once keys are a more useful shape
        let path_parts_string = path_parts[1..].join("/");
        for key in self.public_archive.map().keys() {
            if key.to_str().unwrap().to_string().replace("\\", "/").trim_start_matches("./").trim_start_matches("/").ends_with(path_parts_string.as_str()) {
                let (data_addr, _) = self.public_archive.map().get(key).unwrap();
                return Ok(data_addr.clone())
            }
        }
        Err(ErrorInternalServerError(format!("Failed to find item [{}] in archive", path_parts_string)))
    }

    pub fn resolve_file_from_archive(&self, request_path: String, resolved_filename_string: String) -> (String, XorName) {
        // hack to return index.html when present in directory root
        for key in self.public_archive.map().keys() {
            if key.ends_with(resolved_filename_string.to_string()) {
                let path_string = request_path + key.to_str().unwrap();
                let data_address = self.public_archive.map().get(key).unwrap().0;
                return (path_string, *data_address.xorname())
            }
        }
        (String::new(), XorName::default())
    }

    pub fn resolve_archive_info(&self, path_parts: Vec<String>, request: HttpRequest, resolved_relative_path_route: String, has_route_map: bool, caching_client: CachingClient) -> ArchiveInfo {
        let request_path = request.path();
        let xor_helper = ResolverService::new(self.ant_tp_config.clone(), caching_client);
        
        if self.has_moved_permanently(request_path, &resolved_relative_path_route) {
            debug!("has moved permanently");
            ArchiveInfo::new(resolved_relative_path_route, XorName::default(), ArchiveAction::Redirect, DataState::Modified)
        } else if has_route_map {
            // retrieve route map index
            debug!("retrieve route map index");
            let (resolved_relative_path_route, resolved_xor_addr) = self.resolve_file_from_archive(request_path.to_string(), resolved_relative_path_route);
            ArchiveInfo::new(resolved_relative_path_route, resolved_xor_addr, ArchiveAction::Data, xor_helper.get_data_state(request.headers(), &resolved_xor_addr))
        } else if !resolved_relative_path_route.is_empty() {
            // retrieve path and data address
            debug!("retrieve path and data address");
            match self.resolve_data_addr(path_parts.clone()) {
                Ok(resolved_data_address) => {
                    let path_buf = &PathBuf::from(resolved_relative_path_route.clone());
                    info!("Resolved path [{}], path_buf [{}] to xor address [{}]", resolved_relative_path_route, path_buf.display(), format!("{:x}", resolved_data_address.xorname()));
                    ArchiveInfo::new(resolved_relative_path_route, *resolved_data_address.xorname(), ArchiveAction::Data, xor_helper.get_data_state(request.headers(), resolved_data_address.xorname()))
                }
                Err(_err) => {
                    ArchiveInfo::new(resolved_relative_path_route, XorName::default(), ArchiveAction::NotFound, DataState::Modified)
                }
            }
        } else {
            // retrieve file listing
            info!("retrieve file listing");
            ArchiveInfo::new(resolved_relative_path_route, XorName::default(), ArchiveAction::Listing, DataState::Modified)
        }
    }

    fn has_moved_permanently(&self, request_path: &str, resolved_relative_path_route: &String) -> bool {
        resolved_relative_path_route.is_empty() && request_path.to_string().chars().last() != Some('/')
    }
}