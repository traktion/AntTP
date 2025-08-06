use std::collections::HashMap;
use autonomi::data::DataAddress;
use autonomi::files::PublicArchive;
use bytes::Bytes;
use log::{debug, error};
use crate::client::caching_client::CachingClient;

#[derive(Clone)]
pub struct Archive {
    pub data_address_offsets: HashMap<String, DataAddressOffset>
}

#[derive(Clone,Debug)]
pub struct DataAddressOffset {
    pub data_address: DataAddress,
    pub path: String,
    pub offset: u64,
    pub size: u64,
    pub modified: u64,
}

impl Archive {
    pub fn new(data_address_offsets: HashMap<String, DataAddressOffset>) -> Self {
        Archive { data_address_offsets }
    }

    pub async fn build(public_archive: PublicArchive, caching_client: CachingClient) -> Self {
        let maybe_archive_tar_idx = public_archive
            .map()
            .keys()
            .find(|key| key.to_str().unwrap()
                .to_string()
                .replace("\\", "/")
                .trim_start_matches("./")
                .trim_start_matches("/")
                .ends_with("archive.tar.idx"));

        let maybe_archive_tar = public_archive
            .map()
            .keys()
            .find(|key| key
                .to_str().unwrap()
                .to_string()
                .replace("\\", "/")
                .trim_start_matches("./")
                .trim_start_matches("/")
                .ends_with("archive.tar"));

        if maybe_archive_tar_idx.is_none() || maybe_archive_tar.is_none() {
            return Archive::build_from_public_archive(public_archive);
        }

        let archive_tar_idx = maybe_archive_tar_idx.unwrap();
        let archive_tar = maybe_archive_tar.unwrap();
        let (tar_data_addr, _) = public_archive.map().get(&archive_tar.clone()).unwrap();
        let (tar_idx_data_addr, _) = public_archive.map().get(&archive_tar_idx.clone()).unwrap();
        match caching_client.data_get_public(tar_idx_data_addr).await {
            Ok(data) => {
                Self::build_from_tar(tar_data_addr, data)
            },
            Err(err) => {
                error!("Failed to get public data for tar index [{}]", err);
                Archive::new(HashMap::new())
            }
        }
    }

    pub fn build_from_tar(tar_data_addr: &DataAddress, data: Bytes) -> Self{
        let mut data_address_offsets = HashMap::new();
        match String::from_utf8(data.to_vec()) {
            Ok(tar_index) => {
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

                    // todo: handle file names with spaces (maybe %20 though)?
                    let path_string = parts.get(parts.len() - 3)
                        .expect("path missing from tar")
                        .replace("\\", "/")
                        .trim_start_matches("./")
                        .trim_start_matches("/")
                        .to_string();
                    let offset = parts.get(parts.len() - 2).expect("offset missing from tar").parse::<u64>().unwrap_or_else(|_| 0);
                    let limit = parts.get(parts.len() - 1).expect("limit missing from tar").parse::<u64>().unwrap_or_else(|_| u64::MAX) - 1;

                    let data_address_offset = DataAddressOffset {
                        data_address: *tar_data_addr,
                        // file names can have spaces, so index from right and join on left
                        path: path_string.clone(),
                        offset: offset,
                        size: limit,
                        modified: 1, // todo: derive modified epoch millis
                    };
                    debug!("insert into archive: path_string [{}], data address offset: [{:?}]", path_string, data_address_offset);
                    data_address_offsets.insert(
                        path_string.clone(),
                        data_address_offset
                    );
                }
            },
            Err(err) => {
                error!("Failed to parse public data for tar index [{}]", err);
            }
        }
        debug!("data_address_offsets size [{}]", data_address_offsets.len());
        Archive::new(data_address_offsets.clone())
    }

    pub fn build_from_public_archive(public_archive: PublicArchive) -> Self {
        public_archive.iter().for_each(|(path_buf, data_address, _)| debug!("archive entry: [{}] at [{:x}]", path_buf.to_str().unwrap().to_string().replace("\\", "/"), data_address.xorname()));

        // todo: Replace with contains() once keys are a more useful shape
        let mut data_address_offsets = HashMap::new();
        for key in public_archive.map().keys() {
            let key_string = key.to_str().unwrap()
                .replace("\\", "/")
                .trim_start_matches("./")
                .trim_start_matches("/")
                .to_string();

            let (data_addr, metadata) = public_archive.map().get(key).unwrap();
            data_address_offsets.insert(
                key_string.clone(),
                DataAddressOffset {
                    data_address: data_addr.clone(),
                    path: key_string.clone(),
                    offset: 0,
                    size: u64::MAX,
                    modified: metadata.modified
                }
            );
        }
        Archive::new(data_address_offsets)
    }

    pub fn find(&self, search_key: String) -> Option<&DataAddressOffset> {
        // hack to return index.html when present in directory root
        for key in self.data_address_offsets.keys() {
            debug!("archive key [{}], search_key [{}]", key, search_key);
            if key.ends_with(&search_key) {
                return self.data_address_offsets.get(key);
            }
        }
        None
    }
    
    pub fn map(&self) -> &HashMap<String, DataAddressOffset> {
        &self.data_address_offsets
    }
}