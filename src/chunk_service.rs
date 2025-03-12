use autonomi::{ChunkAddress, Client};
use autonomi::client::GetError;
use bytes::{Bytes};
use log::{error, info};
use self_encryption::{DataMap, EncryptedChunk, Error};
use serde::{Deserialize, Serialize};

pub const STREAM_CHUNK_SIZE: usize = 4096 * 1024;

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
        position_start: usize,
        position_end: usize,
    ) -> Result<Bytes, Error> {
        info!("fetch from data map chunk");
            
        let data_map_level: DataMapLevel = rmp_serde::from_slice(data_map_bytes)
            .map_err(GetError::InvalidDataMap)
            .inspect_err(|err| error!("Error deserializing data map: {err:?}"))
            .expect("failed to parse data map level");

        let data_map = match &data_map_level {
            DataMapLevel::First(map) => map,
            DataMapLevel::Additional(map) => map,
        };
        
        let position = position_start / STREAM_CHUNK_SIZE;
        let relative_position = position_start % STREAM_CHUNK_SIZE;
        let relative_size = self.get_chunk_size(position_start, position_end) - relative_position;

        info!("decrypt chunk in position [{}], relative position [{}], relative size [{}]", position, relative_position, relative_size);
        match data_map.infos().get(position) {
            Some(chunk_info) => {
                info!("get chunk from data map: {:?}", chunk_info.dst_hash);
                let chunk = self.autonomi_client.chunk_get(&ChunkAddress::new(chunk_info.dst_hash)).await.expect("get chunk failed");

                info!("self decrypt chunk: {:?}", chunk_info.dst_hash);
                let encrypted_chunks = &[EncryptedChunk { index: position, content: chunk.clone().value }];
                match self_encryption::decrypt_range(&data_map, encrypted_chunks, relative_position, relative_size) {
                    Ok(chunk_bytes) => Ok(chunk_bytes),
                    Err(e) => Err(e)
                }
            }
            None => {
                Err(Error::Decryption(format!("failed to get chunk at position: [{}]", position)))
            }
        }
    }
    
    pub fn get_chunk_size(&self, position_start: usize, position_end: usize) -> usize {
        if position_end - position_start > STREAM_CHUNK_SIZE {
            STREAM_CHUNK_SIZE
        } else {
            position_end - position_start
        }
    }
}