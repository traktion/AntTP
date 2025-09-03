use std::collections::HashMap;
use autonomi::data::DataAddress;
use autonomi::files::PublicArchive;
use bytes::Bytes;
use log::{debug, error};
use serde::{Deserialize, Serialize};
use crate::model::path_detail::PathDetail;

#[derive(Clone,Serialize,Deserialize)]
pub struct Archive {
    data_address_offsets_map: HashMap<String, DataAddressOffset>,
    data_address_offsets_vec: Vec<DataAddressOffset>,
}

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct DataAddressOffset {
    pub data_address: DataAddress,
    pub path: String,
    pub offset: u64,
    pub size: u64,
    pub modified: u64,
}

impl Archive {
    pub fn new(data_address_offsets_map: HashMap<String, DataAddressOffset>, data_address_offsets_vec: Vec<DataAddressOffset>) -> Self {
        Archive { data_address_offsets_map, data_address_offsets_vec }
    }

    pub fn build_from_tar(tar_data_addr: &DataAddress, data: Bytes) -> Self{
        let mut data_address_offsets_map = HashMap::new();
        let mut data_address_offsets_vec = Vec::new();
        match String::from_utf8(data.to_vec()) {
            Ok(tar_index) => {
                let mut entry_counter = 1;
                for entry in tar_index.split('\n') {
                    if entry.is_empty() {
                        continue;
                    }
                    let entry_str = entry.to_string();
                    let parts = entry_str.split(' ').collect::<Vec<&str>>();
                    debug!("parts: [{:?}]", parts);
                    if parts.len() < 3 {
                        continue;
                    }

                    // todo: confirm this handles file names with spaces (maybe %20 though)?
                    let path_string = Self::sanitise_path(parts.get(parts.len() - 3).expect("path missing from tar"));
                    let offset = parts.get(parts.len() - 2).expect("offset missing from tar").parse::<u64>().unwrap_or_else(|_| 0);
                    let tmp_size = parts.get(parts.len() - 1).expect("limit missing from tar").parse::<u64>().unwrap_or_else(|_| u64::MAX);
                    let size = if tmp_size > 0 { tmp_size - 1 } else { tmp_size };

                    let data_address_offset = DataAddressOffset {
                        data_address: *tar_data_addr,
                        // file names can have spaces, so index from right and join on left
                        path: path_string.clone(),
                        offset,
                        size,
                        modified: entry_counter, // note: use a counter to derive date sequence by archive file order, as times are only embedded in the tar file itself
                    };
                    debug!("insert into archive: path_string [{}], data address offset: [{:?}]", path_string, data_address_offset);
                    data_address_offsets_map.insert(
                        path_string.clone(),
                        data_address_offset.clone()
                    );
                    data_address_offsets_vec.push(data_address_offset);
                    entry_counter += 1;
                }
            },
            Err(err) => {
                error!("Failed to parse public data for tar index [{}]", err);
            }
        }
        debug!("data_address_offsets size [{}]", data_address_offsets_map.len());
        Archive::new(data_address_offsets_map, data_address_offsets_vec)
    }

    pub fn sanitise_path(path: &str) -> String {
        path.replace("\\", "/")
            .trim_start_matches("./")
            .trim_start_matches("/")
            .to_string()
    }

    pub fn build_from_public_archive(public_archive: PublicArchive) -> Self {
        public_archive.iter().for_each(|(path_buf, data_address, _)| debug!("archive entry: [{}] at [{:x}]", path_buf.to_str().unwrap().to_string().replace("\\", "/"), data_address.xorname()));

        // todo: Replace with contains() once keys are a more useful shape
        let mut data_address_offsets_map = HashMap::new();
        let mut data_address_offsets_vec = Vec::new();
        for key in public_archive.map().keys() {
            let path_string = Self::sanitise_path(key.to_str().unwrap());

            let (data_addr, metadata) = public_archive.map().get(key).unwrap();
            let data_address_offset = DataAddressOffset {
                data_address: data_addr.clone(),
                path: path_string.clone(),
                offset: 0,
                size: metadata.size,
                modified: metadata.modified
            };
            data_address_offsets_map.insert(
                path_string.clone(),
                data_address_offset.clone()
            );
            data_address_offsets_vec.push(data_address_offset);
        }
        Archive::new(data_address_offsets_map, data_address_offsets_vec)
    }

    pub fn find_file(&self, search_key: String) -> Option<&DataAddressOffset> {
        self.data_address_offsets_map.get(&search_key)
    }

    pub fn list_dir(&self, search_key: String) -> Vec<PathDetail> {
        let search_key_sanitised = if search_key.len() > 0 && search_key[search_key.len()-1..].to_string() != "/" {
            &format!("{}/", &search_key)
        } else {
            &search_key
        };

        let search_key_parts = search_key_sanitised.split("/").collect::<Vec<&str>>();
        debug!("list_dir - search_key: {}", &search_key_sanitised);
        let mut vec = vec![];
        let mut map = HashMap::new();

        for data_address_offset in &self.data_address_offsets_vec {
            let path_parts = &data_address_offset.path.split("/").collect::<Vec<&str>>();

            debug!("search_key_parts.len(): {}, path_parts.len(): {}", search_key_parts.len(), path_parts.len());
            let mut i = 0;
            while i < search_key_parts.len() {
                if i > path_parts.len() - 1 {
                    break;
                }
                debug!("search_key_parts[i]: {}, path_parts[i]: {}", search_key_parts[i], path_parts[i]);
                if search_key_parts[i] != "" && search_key_parts[i] != path_parts[i] {
                    break;
                }
                if i == path_parts.len() - 1 {
                    debug!("adding file: {}", path_parts[i]);
                    let path_detail =  PathDetail {
                        path: path_parts[i].to_string(),
                        display: path_parts[i].to_string(),
                        modified: data_address_offset.modified,
                        size: data_address_offset.size,
                    };
                    vec.push(path_detail);
                } else if i == search_key_parts.len() - 1 && !map.contains_key(&path_parts[i].to_string()) {
                    let dir = format!("{}/", path_parts[i].to_string());
                    debug!("adding dir: {}", path_parts[i]);
                    let path_detail =  PathDetail {
                        path: dir.clone(),
                        display: dir.clone(),
                        modified: data_address_offset.modified,
                        size: 0
                    };
                    vec.push(path_detail.clone());
                    map.insert(path_parts[i].to_string(), path_detail);
                } else if search_key_parts.len() > 1 && !map.contains_key(&"../".to_string()) {
                    let dir = "../".to_string();
                    let path_detail =  PathDetail {
                        path: dir.clone(),
                        display: dir.clone(),
                        modified: data_address_offset.modified,
                        size: 0
                    };
                    vec.push(path_detail.clone());
                    map.insert("../".to_string(), path_detail);
                }
                i += 1;
            }
        }
        vec
    }

    pub fn map(&self) -> &HashMap<String, DataAddressOffset> {
        &self.data_address_offsets_map
    }

    pub fn vec(&self) -> &Vec<DataAddressOffset> {
        &self.data_address_offsets_vec
    }
}