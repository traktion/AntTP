use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use actix_web::{Error, HttpResponse};
use actix_web::web::{Data, Payload};
use async_stream::__private::AsyncStream;
use bytes::Bytes;
use log::{debug, error};
use tokio::io::Interest;
use tokio::net::TcpStream;

use async_stream::stream;
use futures_core::Stream;
use futures_util::task::noop_waker_ref;
use crate::config::anttp_config::AntTpConfig;

pub async fn forward(
    ant_tp_config_data: Data<AntTpConfig>,
    mut client_stream: Payload,
) -> Result<HttpResponse, Error> {
    let client_writer: AsyncStream<Result<Bytes, Error>, _> = stream! {
        // Connect to a peer
        let server_stream = TcpStream::connect(ant_tp_config_data.https_listen_address).await.unwrap();

        loop {
            let ready = server_stream.ready(Interest::READABLE | Interest::WRITABLE).await.unwrap();

            if ready.is_readable() {
                let mut data = vec![0; 1024 * 8]; // todo: tune
                // Try to read data, this may still fail with `WouldBlock`
                // if the readiness event is a false positive.
                match server_stream.try_read(&mut data) {
                    Ok(n) => {
                        if n > 0 {
                            debug!("read {} bytes from server", n);
                            let bytes = Bytes::copy_from_slice(&data[..n]);
                            yield Ok(bytes);
                            continue;
                        }
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        debug!("read: WouldBlock");
                        continue;
                    }
                    Err(e) => {
                        error!("error reading bytes: {}", e);
                        break;
                    }
                }
            }

            if ready.is_writable() {
                // Try to write data, this may still fail with `WouldBlock`
                // if the readiness event is a false positive.
                let pin_client_stream = Pin::new(&mut client_stream);
                match pin_client_stream.poll_next(&mut Context::from_waker(noop_waker_ref())) {
                    Poll::Ready(Some(chunk_result)) => {
                        match chunk_result {
                            Ok(bytes) => {
                                match server_stream.try_write(bytes.iter().as_slice()) {
                                    Ok(n) => {
                                        debug!("write {} bytes to server", n);
                                        continue;
                                    }
                                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                                        debug!("write: WouldBlock");
                                        continue;
                                    }
                                    Err(e) => {
                                        error!("error writing bytes: {}", e);
                                        break;
                                    }
                                }
                            }
                            Err(e) => {
                                error!("error polling payload: {}", e);
                                break;
                            }
                        }
                    }
                    Poll::Ready(None) => tokio::task::yield_now().await, // Stream exhausted
                    Poll::Pending => tokio::task::yield_now().await, // todo: register a Waker?
                }
            }
            // no reads or writes, so yield and sleep for a bit
            tokio::task::yield_now().await;
            tokio::time::sleep(Duration::from_millis(5)).await; // todo: tune
        }
    };
    Ok(HttpResponse::Ok().streaming(client_writer))
}
