use std::pin::Pin;
use std::task::{Context, Poll};
use bytes::Bytes;
use futures::Stream;
use futures_util::{FutureExt};
use log::{debug, info};
use self_encryption::Error;
use tokio::sync::mpsc::{Receiver};
use tokio::task::{JoinHandle};

pub struct ChunkReceiver {
    receiver: Receiver<JoinHandle<Result<Bytes, Error>>>,
    id: String,
    file_position: usize,
    chunk_index: i32,
    current_task: Option<JoinHandle<Result<Bytes, Error>>>,
}

impl ChunkReceiver {
    pub fn new(receiver: Receiver<JoinHandle<Result<Bytes, Error>>>, id: String) -> ChunkReceiver {
        ChunkReceiver { receiver, id, file_position: 0, chunk_index: 1, current_task: None }
    }

    fn poll_current_task(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Result<Bytes, Error>>> {
        match self.current_task.as_mut() {
            Some(join_handle) => {
                match join_handle.poll_unpin(cx) {
                    Poll::Pending => {
                        debug!("Join handle pending");
                        Poll::Pending
                    }
                    Poll::Ready(result) => {
                        let data = result.unwrap().unwrap();
                        let bytes_read = data.len();
                        if bytes_read > 0 {
                            info!("Read [{}] bytes from chunk [{}] at file position [{}] for ID [{}]", bytes_read, self.chunk_index, self.file_position, self.id);
                            self.file_position += bytes_read;
                            self.chunk_index += 1;
                            self.current_task = None;
                            Poll::Ready(Some(Ok(data))) // Sending data to the client here
                        } else {
                            info!("No more data at file position [{}] for ID [{}]", self.file_position, self.id);
                            debug!("End of stream A - closing channel");
                            self.receiver.close();
                            Poll::Ready(None) // end of stream - break
                        }
                    },
                }
            },
            None => {
                debug!("End of stream B - closing channel");
                self.receiver.close();
                Poll::Ready(None) // end of stream - break
            }
        }
    }
}

impl Stream for ChunkReceiver {
    type Item = Result<Bytes, Error>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.current_task.is_none() {
            match self.receiver.poll_recv(cx) {
                Poll::Pending => {
                    debug!("Pending data in receiver");
                    Poll::Pending
                },
                Poll::Ready(maybe_join_handle) => {
                    self.current_task = maybe_join_handle;
                    self.poll_current_task(cx)
                }
            }
        } else {
            match self.poll_current_task(cx) {
                Poll::Pending => {
                    debug!("Pending join handle finishing");
                    Poll::Pending
                },
                Poll::Ready(data) => {
                    debug!("Returning join handle result");
                    Poll::Ready(data)
                }
            }
        }
    }
}