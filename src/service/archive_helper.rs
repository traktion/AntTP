use actix_http::header::{HeaderMap, ACCEPT};
use actix_web::{HttpRequest};
use chrono::DateTime;
use log::{debug, info};
use xor_name::XorName;
use crate::model::archive::Archive;
use crate::service::html_directory_renderer::HtmlDirectoryRenderer;
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
    pub modified_time: u64,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ArchiveAction {
    Data, Listing, Redirect, NotFound
}

impl ArchiveInfo {
    pub fn new(path_string: String, resolved_xor_addr: XorName, action: ArchiveAction, is_modified: bool, offset: u64, size: u64, modified_time: u64) -> ArchiveInfo {
        // note: offset is 0 indexed, size is 1 indexed
        //       offset is never 0 in a tarchive, due to header
        let limit = if size > 0 { size - 1 } else { 0 };
        ArchiveInfo { path_string, resolved_xor_addr, action, is_modified, offset, size, limit, modified_time }
    }
}

impl ArchiveHelper {
    pub fn new(archive: Archive) -> ArchiveHelper {
        ArchiveHelper { archive }
    }
    
    pub fn list_files(&self, path: String, header_map: &HeaderMap) -> String {
        if header_map.contains_key(ACCEPT)
            && header_map.get(ACCEPT).unwrap().to_str().unwrap_or("").to_string().contains( "json") {
            self.list_files_json(path)
        } else {
            self.list_files_html(path)
        }
    }

    fn list_files_html(&self, path: String) -> String {
        HtmlDirectoryRenderer::render(&self.archive, path)
    }

    fn list_files_json(&self, path: String) -> String {
        let mut output = "[\n".to_string();

        let list_dir = self.archive.list_dir(path);
        let mut i = 1;
        let count = list_dir.len();
        for path_detail in list_dir {
            let mtime_datetime = DateTime::from_timestamp_millis( i64::try_from(path_detail.modified)
                .unwrap_or(0) * 1000)
                .unwrap_or(DateTime::default());
            let mtime_iso = mtime_datetime.format("%+");
            output.push_str("{");
            output.push_str(
                &format!("\"name\": \"{}\", \"type\": \"{:?}\", \"mtime\": \"{}\", \"size\": \"{}\"",
                         path_detail.display, path_detail.path_type, mtime_iso, path_detail.size));
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
            ArchiveInfo::new(resolved_route_path.clone(), XorName::default(), ArchiveAction::Redirect, true, 0, 0, 0)
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
                        data_address_offset.size,
                        data_address_offset.modified
                    )
                }
                None => ArchiveInfo::new(resolved_route_path.clone(), XorName::default(), ArchiveAction::NotFound, true, 0, 0, 0)
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
                        data_address_offset.size,
                        data_address_offset.modified
                    )
                }
                None => if !self.archive.list_dir(resolved_address.file_path.clone()).is_empty() {
                    if resolved_address.file_path.to_string().chars().last() != Some('/') {
                        ArchiveInfo::new(format!("{}/", resolved_address.file_path.clone()), XorName::default(), ArchiveAction::Redirect, true, 0, 0, 0)
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
                                    data_address_offset.size,
                                    data_address_offset.modified
                                )
                            }
                            None => {
                                debug!("default index not found, retrieve file listing");
                                ArchiveInfo::new(resolved_address.file_path.clone(), XorName::default(), ArchiveAction::Listing, true, 0, 0, 0)
                            }
                        }
                    }
                } else {
                    ArchiveInfo::new(resolved_route_path.clone(), XorName::default(), ArchiveAction::NotFound, true, 0, 0, 0)
                }
            }
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
                        data_address_offset.size,
                        data_address_offset.modified
                    )
                }
                None => {
                    debug!("default index not found, retrieve file listing");
                    ArchiveInfo::new(resolved_address.file_path.clone(), XorName::default(), ArchiveAction::Listing, true, 0, 0, 0)
                }
            }
        }
    }

    fn has_moved_permanently(&self, request_path: &str, resolved_relative_path_route: &String) -> bool {
        resolved_relative_path_route.is_empty() && request_path.to_string().chars().last() != Some('/')
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::TestRequest;
    use std::collections::HashMap;
    use autonomi::data::DataAddress;
    use crate::model::archive::DataAddressOffset;
    use actix_http::header::HeaderName;

    fn create_test_archive() -> Archive {
        let mut map = HashMap::new();
        let mut vec = Vec::new();

        let file1 = DataAddressOffset {
            data_address: DataAddress::from_hex("0000000000000000000000000000000000000000000000000000000000000000").unwrap(),
            path: "index.html".to_string(),
            offset: 0,
            size: 100,
            modified: 1,
        };
        map.insert("index.html".to_string(), file1.clone());
        vec.push(file1);

        let file2 = DataAddressOffset {
            data_address: DataAddress::from_hex("0000000000000000000000000000000000000000000000000000000000000000").unwrap(),
            path: "style.css".to_string(),
            offset: 100,
            size: 50,
            modified: 2,
        };
        map.insert("style.css".to_string(), file2.clone());
        vec.push(file2);

        let file3 = DataAddressOffset {
            data_address: DataAddress::from_hex("0000000000000000000000000000000000000000000000000000000000000000").unwrap(),
            path: "sub/test.txt".to_string(),
            offset: 150,
            size: 20,
            modified: 3,
        };
        map.insert("sub/test.txt".to_string(), file3.clone());
        vec.push(file3);

        Archive::new(map, vec)
    }

    fn create_resolved_address(file_path: &str) -> ResolvedAddress {
        ResolvedAddress::new(
            true,
            None,
            XorName::default(),
            file_path.to_string(),
            false,
            false,
            true,
            5
        )
    }

    #[test]
    fn test_list_files_html() {
        let archive = create_test_archive();
        let helper = ArchiveHelper::new(archive);
        let header_map = HeaderMap::new();

        let output = helper.list_files("".to_string(), &header_map);
        assert!(output.contains("Index of /"));
        assert!(output.contains("index.html"));
        assert!(output.contains("style.css"));
        assert!(output.contains("sub/"));
    }

    #[test]
    fn test_list_files_json() {
        let archive = create_test_archive();
        let helper = ArchiveHelper::new(archive);
        let mut header_map = HeaderMap::new();
        header_map.insert(HeaderName::from_static("accept"), "application/json".parse().unwrap());

        let output = helper.list_files("".to_string(), &header_map);
        assert!(output.contains("["));
        assert!(output.contains("\"name\": \"index.html\""));
        assert!(output.contains("\"name\": \"style.css\""));
        assert!(output.contains("]"));
    }

    #[actix_web::test]
    async fn test_resolve_file() {
        let archive = create_test_archive();
        let helper = ArchiveHelper::new(archive);
        let req = TestRequest::with_uri("/index.html").to_http_request();
        let resolved_addr = create_resolved_address("index.html");

        let info = helper.resolve_archive_info(&resolved_addr, &req, &"index.html".to_string(), false).await;
        
        assert_eq!(info.action, ArchiveAction::Data);
        assert_eq!(info.path_string, "index.html");
        assert_eq!(info.size, 100);
    }

    #[actix_web::test]
    async fn test_resolve_directory_redirect() {
        let archive = create_test_archive();
        let helper = ArchiveHelper::new(archive);
        let req = TestRequest::with_uri("/sub").to_http_request();
        let resolved_addr = create_resolved_address("sub");

        let info = helper.resolve_archive_info(&resolved_addr, &req, &"sub".to_string(), false).await;
        
        assert_eq!(info.action, ArchiveAction::Redirect);
        assert_eq!(info.path_string, "sub/");
    }

    #[actix_web::test]
    async fn test_resolve_directory_index() {
        let archive = create_test_archive();
        let helper = ArchiveHelper::new(archive);
        // "root" maps to empty string in file path logic usually, but here we simulate resolving to empty path (root)
        let req = TestRequest::with_uri("/").to_http_request();
        let resolved_addr = create_resolved_address("");

        let info = helper.resolve_archive_info(&resolved_addr, &req, &"".to_string(), false).await;
        
        // Should resolve to index.html
        assert_eq!(info.action, ArchiveAction::Data);
        assert_eq!(info.path_string, ""); // The resolved route path passed in is empty
        // But internally it found index.html data
        assert_eq!(info.size, 100);
    }

    #[actix_web::test]
    async fn test_resolve_directory_listing() {
        let archive = create_test_archive();
        let helper = ArchiveHelper::new(archive);
        let req = TestRequest::with_uri("/sub/").to_http_request();
        let resolved_addr = create_resolved_address("sub/");

        let info = helper.resolve_archive_info(&resolved_addr, &req, &"".to_string(), false).await;
        
        assert_eq!(info.action, ArchiveAction::Listing);
        assert_eq!(info.path_string, "sub/");
    }

    #[actix_web::test]
    async fn test_resolve_not_found() {
        let archive = create_test_archive();
        let helper = ArchiveHelper::new(archive);
        let req = TestRequest::with_uri("/missing.txt").to_http_request();
        let resolved_addr = create_resolved_address("missing.txt");

        let info = helper.resolve_archive_info(&resolved_addr, &req, &"missing.txt".to_string(), false).await;
        
        assert_eq!(info.action, ArchiveAction::NotFound);
    }
}