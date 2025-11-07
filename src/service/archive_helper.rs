use actix_http::header::HeaderMap;
use actix_web::{HttpRequest};
use chrono::DateTime;
use log::{debug, info};
use xor_name::XorName;
use crate::model::archive::Archive;
use crate::service::resolver_service::ResolvedAddress;

#[derive(Clone)]
pub struct ArchiveHelper {
    archive: Archive
}

#[derive(Clone)]
pub struct ArchiveInfo {
    pub path_string: String,
    pub resolved_xor_addr: XorName,
    pub action: ArchiveAction,
    pub is_modified: bool,
    pub offset: u64,
    pub size: u64,
    pub limit: u64,
}

#[derive(Clone, PartialEq, Eq)]
pub enum ArchiveAction {
    Data, Listing, Redirect, NotFound
}

impl ArchiveInfo {
    pub fn new(path_string: String, resolved_xor_addr: XorName, action: ArchiveAction, is_modified: bool, offset: u64, size: u64) -> ArchiveInfo {
        // note: offset is 0 indexed, size is 1 indexed
        //       offset is never 0 in a tarchive, due to header
        let limit = if size > 0 { size - 1 } else { 0 };
        ArchiveInfo { path_string, resolved_xor_addr, action, is_modified, offset, size, limit }
    }
}

impl ArchiveHelper {
    pub fn new(archive: Archive) -> ArchiveHelper {
        ArchiveHelper { archive }
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
            let mtime_datetime = DateTime::from_timestamp_millis( i64::try_from(path_detail.modified).unwrap() * 1000).unwrap();
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
            let mtime_datetime = DateTime::from_timestamp_millis(i64::try_from(path_detail.modified).unwrap() * 1000).unwrap();
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

    pub async fn resolve_archive_info(&self, resolved_address: &ResolvedAddress, request: &HttpRequest, resolved_route_path: &String, has_route_map: bool) -> ArchiveInfo {
        let request_path = request.path();

        if self.has_moved_permanently(request_path, &resolved_route_path) {
            debug!("has moved permanently");
            ArchiveInfo::new(resolved_route_path.clone(), XorName::default(), ArchiveAction::Redirect, true, 0, 0)
        } else if has_route_map {
            debug!("retrieve route map index");
            match self.archive.find_file(resolved_route_path) {
                Some(data_address_offset) => {
                    info!("Resolved path [{}] to xor address [{}]", resolved_route_path, format!("{:x}", *data_address_offset.data_address.xorname()));
                    ArchiveInfo::new(
                        data_address_offset.path.clone(),
                        *data_address_offset.data_address.xorname(),
                        ArchiveAction::Data,
                        resolved_address.is_modified,
                        data_address_offset.offset,
                        data_address_offset.size
                    )
                }
                None => ArchiveInfo::new(resolved_route_path.clone(), XorName::default(), ArchiveAction::NotFound, true, 0, 0)
            }
        } else if !resolved_route_path.is_empty() {
            debug!("retrieve path and data address");
            match self.archive.find_file(&resolved_address.file_path) {
                Some(data_address_offset) => {
                    info!("Resolved path [{}] to xor address [{}]", resolved_route_path, format!("{:x}", *data_address_offset.data_address.xorname()));
                    ArchiveInfo::new(
                        resolved_route_path.clone(),
                        *data_address_offset.data_address.xorname(),
                        ArchiveAction::Data,
                        resolved_address.is_modified,
                        data_address_offset.offset,
                        data_address_offset.size
                    )
                }
                None => if !self.archive.list_dir(resolved_address.file_path.clone()).is_empty() {
                    if resolved_address.file_path.to_string().chars().last() != Some('/') {
                        ArchiveInfo::new(format!("{}/", resolved_address.file_path.clone()), XorName::default(), ArchiveAction::Redirect, true, 0, 0)
                    } else {
                        let default_index = format!("{}index.html", resolved_address.file_path.clone());
                        debug!("Lookup default index: {}", default_index);
                        match self.archive.find_file(&default_index) {
                            Some(data_address_offset) => {
                                info!("Resolved path [{}] to xor address [{}] to default [{}]", resolved_route_path, format!("{:x}", *data_address_offset.data_address.xorname()), default_index);
                                ArchiveInfo::new(
                                    resolved_route_path.clone(),
                                    *data_address_offset.data_address.xorname(),
                                    ArchiveAction::Data,
                                    resolved_address.is_modified,
                                    data_address_offset.offset,
                                    data_address_offset.size
                                )
                            }
                            None => {
                                debug!("default index not found, retrieve file listing");
                                ArchiveInfo::new(resolved_address.file_path.clone(), XorName::default(), ArchiveAction::Listing, true, 0, 0)
                            }
                        }
                    }
                } else {
                    ArchiveInfo::new(resolved_route_path.clone(), XorName::default(), ArchiveAction::NotFound, true, 0, 0)
                }
            }
        } else {
            debug!("resolved_route_path not found, retrieve file listing");
            ArchiveInfo::new(resolved_route_path.clone(), XorName::default(), ArchiveAction::Listing, true, 0, 0)
        }
    }

    fn has_moved_permanently(&self, request_path: &str, resolved_relative_path_route: &String) -> bool {
        resolved_relative_path_route.is_empty() && request_path.to_string().chars().last() != Some('/')
    }
}