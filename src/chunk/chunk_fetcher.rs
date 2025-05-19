use autonomi::{ChunkAddress, Client};
use bytes::{Bytes};
use log::{info};
use self_encryption::{DataMap, EncryptedChunk, Error};

#[derive(Clone)]
pub struct ChunkFetcher {
    autonomi_client: Client,
}

impl ChunkFetcher {
    
    pub fn new(autonomi_client: Client) -> Self {
        ChunkFetcher { autonomi_client }
    }

    pub async fn fetch_from_data_map_chunk(
        &self,
        data_map: DataMap,
        position_start: u64,
        position_end: u64
    ) -> Result<Bytes, Error> {
        info!("fetch from data map chunk");
        let stream_chunk_size = data_map.infos().get(0).unwrap().src_size;
        let chunk_position = (position_start / stream_chunk_size as u64) as usize; // chunk_info.src_size needed for exact size, as last chunk size varies 
        let chunk_start_offset = (position_start % stream_chunk_size as u64) as usize;
        
        info!("decrypt chunk in position=[{}] of [{}], offset=[{}], total_size=[{}]", chunk_position+1, data_map.infos().len(), chunk_start_offset, data_map.file_size());
        match data_map.infos().get(chunk_position) {
            Some(chunk_info) => {
                info!("get chunk from data map with hash {:?} and size {}", chunk_info.dst_hash, chunk_info.src_size);
                let derived_chunk_size = self.get_chunk_size(position_start as usize, position_end as usize, chunk_info.src_size) - chunk_start_offset;
                let chunk = self.autonomi_client.chunk_get(&ChunkAddress::new(chunk_info.dst_hash)).await.expect("get chunk failed");

                info!("self decrypt chunk: {:?}", chunk_info.dst_hash);
                let encrypted_chunks = &[EncryptedChunk { index: chunk_position, content: chunk.clone().value }];
                match self_encryption::decrypt_range(&data_map, encrypted_chunks, chunk_start_offset, derived_chunk_size) {
                    Ok(chunk_bytes) => Ok(chunk_bytes),
                    Err(e) => Err(e)
                }
            }
            None => {
                Ok(Bytes::new())
            }
        }
    }

    fn get_chunk_size(&self, position_start: usize, position_end: usize, stream_chunk_size: usize) -> usize {
        let len = (position_end - position_start) + 1;
        if len >= stream_chunk_size {
            stream_chunk_size
        } else {
            len
        }
    }
}