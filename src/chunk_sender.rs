use bytes::Bytes;
use log::info;
use self_encryption::{DataMap, Error};
use tokio::sync::mpsc::{Sender};
use tokio::task::JoinHandle;
use xor_name::XorName;
use crate::chunk_service::ChunkService;

pub struct ChunkSender {
    sender: Sender<JoinHandle<Result<Bytes, Error>>>,
    chunk_service: ChunkService,
    data_map: DataMap,
    first_chunk_limit: usize,
    stream_chunk_size: usize,
    xor_name: XorName,
}

impl ChunkSender {
    pub fn new(sender: Sender<JoinHandle<Result<Bytes, Error>>>, chunk_service: ChunkService, data_map: DataMap, first_chunk_limit: usize, stream_chunk_size: usize, xor_name: XorName) -> ChunkSender {
        ChunkSender { sender, chunk_service, data_map, first_chunk_limit, stream_chunk_size, xor_name }
    }
    
    pub async fn send(&self, mut next_range_from: u64, derived_range_to: u64, range_to: u64) {
        let mut chunk_count = 1;
        while next_range_from < derived_range_to {
            info!("Async fetch chunk [{}] at file position [{}] for XOR address [{}], channel capacity [{}] of [{}]", chunk_count, next_range_from, self.xor_name, self.sender.capacity(), self.sender.max_capacity());
            let chunk_service_clone = self.chunk_service.clone();
            let data_map_clone = self.data_map.clone();
            let stream_chunk_size_clone = self.stream_chunk_size.clone();

            // todo: check if range_to can be derived_range_to
            let join_handle = tokio::spawn(async move {
                chunk_service_clone.fetch_from_data_map_chunk(data_map_clone, next_range_from, range_to, stream_chunk_size_clone).await
            });
            self.sender.send(join_handle).await.unwrap();

            next_range_from += if chunk_count == 1 {
                self.first_chunk_limit as u64
            } else {
                self.stream_chunk_size as u64
            };
            chunk_count += 1;
        };
    }
}