/*use std::collections::BTreeMap;
use std::time::Instant;
use actix_web::Error;
use autonomi::{ChunkAddress, Client};
use bytes::Bytes;
use futures_util::StreamExt;
use log::{error, info};
use self_encryption::{DataMap, EncryptedChunk};

pub fn chunked_stream(
    client: Client,
    dl_threads: usize,
    data_map: DataMap,
    range_from: u64,
    range_to: u64,
) -> impl futures_util::stream::Stream<Item = Result<Bytes, Error>> {
    let infos = data_map.infos().clone();
    let chunk_size = infos[0].src_size as u64;
    let start_idx = (range_from / chunk_size) as usize;
    let end_idx = (range_to / chunk_size).min(infos.len().saturating_sub(1) as u64) as usize;


    // Prepare the unordered stream of (idx, decrypted_bytes) pairs
    let unordered = futures_util::stream::iter(start_idx..=end_idx)
        .map(move |idx| {
            let client = client.clone();
            let dm = data_map.clone();
            let info = infos[idx].clone();
            let is_first = idx == start_idx;
            let is_last = idx == end_idx;
            let rf = range_from;
            let de = range_to;

            async move {

                let offset = if is_first { (rf % chunk_size) as usize } else { 0 };
                let length = if is_last {
                    ((de % chunk_size) as usize) + 1
                } else {
                    info.src_size
                };


                // Async download

                info!("Start chunk download {} off={} len={}", idx, offset, length);

                let dl_start = Instant::now();
                let chunk = client
                    .chunk_get(&ChunkAddress::new(info.dst_hash))
                    .await
                    .map_err(|e| {
                        error!(
                            "chunk_get error for idx {}: {:?}",
                            idx,
                            e
                        );
                        actix_web::error::ErrorInternalServerError(
                            format!("chunk_get failed: {:?}", e),
                        )
                    })?;
                let data = chunk.value;
                let dl_dur = dl_start.elapsed();
                info!("Finish chunk download {} in {:?}", idx, dl_dur);

                // Offload decryption and potential waiting in consumption of data by client to blocking pool
                let decrypt_start = Instant::now();
                let decrypted = tokio::task::spawn_blocking(move || {

                    info!("Start chunk decryption {}", idx);
                    let out = self_encryption::decrypt_range(&dm,&[EncryptedChunk {index: idx,content: data}],offset,length).expect("decrypt_range failed");
                    let d_dur = decrypt_start.elapsed();
                    info!("Finish chunk decryption {} in {:?}", idx, d_dur);
                    out
                })
                    .await
                    .map_err(|join_err| {
                        error!("Decryption panicked for idx {}: {:?}", idx, join_err);
                        actix_web::error::ErrorInternalServerError("decryption panicked")
                    })?;

                // (idx, decrypted_bytes)
                Ok::<(usize, Bytes), actix_web::Error>((idx, decrypted))
            }
        })
        .buffer_unordered(dl_threads);


    // Reorder into a new stream of Bytes
    let reordered = {futures_util::stream::unfold(
        (unordered, BTreeMap::new(), start_idx), move |(mut inner, mut buf, mut next)| async move {
            loop {
                //  If there is next_idx in buffer, serve it.
                if let Some(chunk) = buf.remove(&next) {
                    next += 1;
                    return Some((Ok(chunk), (inner, buf, next)));
                }
                //  Otherwise pull the next completed download
                match inner.next().await {
                    Some(Ok((idx, bytes))) => {
                        buf.insert(idx, bytes);
                        // loop to see if it matches next
                        continue;
                    }
                    Some(Err(_e)) => {
                        // What to do In case there is a decrypt error?  Abort stream?
                        return Some((Err(actix_web::error::ErrorInternalServerError("decrypt")), (inner, buf, next)));
                    }
                    None => {
                        // done
                        return None;
                    }
                }
            }
        })
    };

    reordered
}*/