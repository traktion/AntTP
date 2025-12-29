use autonomi::ChunkAddress;
use log::{debug, warn};
use crate::client::CachingClient;
use crate::model::antns_list::AntNsList;

pub struct AntNsResolver {
    caching_client: CachingClient,
}

impl AntNsResolver {
    pub fn new(caching_client: CachingClient) -> AntNsResolver {
        AntNsResolver { caching_client }
    }

    pub async fn resolve(&self, name: &String) -> Option<String> {
        match ChunkAddress::from_hex(&name) {
            Ok(chunk_address) => match self.caching_client.chunk_get_internal(&chunk_address).await {
                Ok(chunk) => {
                    match serde_json::from_slice::<AntNsList>(&chunk.value) {
                        Ok(antns_list) => {
                            debug!("deserialized {} antns records", antns_list.antns().len());
                            for antns in antns_list.antns() {
                                if antns.name().is_empty() {
                                    debug!("found default antns record (empty name)");
                                    return Some(antns.address().to_string());
                                }
                            }
                            debug!("no default antns record found");
                            None
                        },
                        Err(e) => {
                            warn!("failed to deserialize chunk as AntNs records: {}", e);
                            None
                        }
                    }
                },
                Err(e) => {
                    warn!("failed to get chunk content: {:?}", e);
                    None
                }
            },
            Err(e) => {
                warn!("failed to get chunk_address from name: {:?}", e);
                None
            }
        }
    }
}
