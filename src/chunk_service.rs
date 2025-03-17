use autonomi::{ChunkAddress, Client};
use autonomi::client::GetError;
use bytes::{Bytes};
use log::{error, info};
use self_encryption::{DataMap, EncryptedChunk, Error};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
enum DataMapLevel {
    // Holds the data map to the source data.
    First(DataMap),
    // Holds the data map of an _additional_ level of chunks
    // resulting from chunking up a previous level data map.
    // This happens when that previous level data map was too big to fit in a chunk itself.
    Additional(DataMap),
}

pub struct ChunkService {
    autonomi_client: Client,
}

impl ChunkService {
    
    pub fn new(autonomi_client: Client) -> Self {
        ChunkService { autonomi_client }
    }

    pub async fn fetch_from_data_map_chunk(
        &self,
        data_map_bytes: &Bytes,
        position_start: u64,
        position_end: u64,
    ) -> Result<Bytes, Error> {
        info!("fetch from data map chunk");

        let data_map = self.get_data_map_from_bytes(data_map_bytes);
        let stream_chunk_size = self.get_chunk_size_from_data_map(&data_map);

        let chunk_position = (position_start / stream_chunk_size as u64) as usize;
        let chunk_start_offset = (position_start % stream_chunk_size as u64) as usize;
        let derived_chunk_size = self.get_chunk_size(position_start as usize, position_end as usize, stream_chunk_size) - chunk_start_offset;

        info!("decrypt chunk in position=[{}] of [{}], offset=[{}], size=[{}], total_size=[{}]", chunk_position, data_map.infos().len()-1, chunk_start_offset, derived_chunk_size, data_map.file_size());
        match data_map.infos().get(chunk_position) {
            Some(chunk_info) => {
                info!("get chunk from data map with hash {:?} and size {}", chunk_info.dst_hash, chunk_info.src_size);
                let chunk = self.autonomi_client.chunk_get(&ChunkAddress::new(chunk_info.dst_hash)).await.expect("get chunk failed");

                info!("self decrypt chunk: {:?}", chunk_info.dst_hash);
                let encrypted_chunks = &[EncryptedChunk { index: chunk_position, content: chunk.clone().value }];
                match self_encryption::decrypt_range(&data_map, encrypted_chunks, chunk_start_offset, derived_chunk_size) {
                    Ok(chunk_bytes) => Ok(chunk_bytes),
                    Err(e) => Err(e)
                }
            }
            None => {
                Err(Error::Decryption(format!("failed to get chunk at position: [{}]", chunk_position)))
            }
        }
    }

    pub fn get_data_map_from_bytes(&self, data_map_bytes: &Bytes) -> DataMap {
        let data_map_level: DataMapLevel = rmp_serde::from_slice(data_map_bytes)
            .map_err(GetError::InvalidDataMap)
            .inspect_err(|err| error!("Error deserializing data map: {err:?}"))
            .expect("failed to parse data map level");

        match data_map_level {
            DataMapLevel::First(map) => map,
            DataMapLevel::Additional(map) => map,
        }
    }

    fn get_chunk_size_from_data_map(&self, data_map: &DataMap) -> usize {
        if data_map.infos().len() > 0 {
            match data_map.infos().get(0) {
                Some(chunk_info) => {
                    chunk_info.src_size
                },
                None => {
                    1
                }
            }
        } else {
            1
        }
    }

    pub fn get_chunk_size(&self, position_start: usize, position_end: usize, stream_chunk_size: usize) -> usize {
        if position_end - position_start > stream_chunk_size {
            stream_chunk_size
        } else {
            position_end - position_start
        }
    }
}