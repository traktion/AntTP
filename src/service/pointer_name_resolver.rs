use ant_protocol::storage::{Pointer, PointerAddress, PointerTarget};
use autonomi::{Client, SecretKey};
use log::{debug, error};
use crate::client::CachingClient;
use crate::error::GetError;
use crate::error::pointer_error::PointerError;

#[derive(Debug)]
pub struct PointerNameResolver {
    caching_client: CachingClient,
    pointer_name_resolver_secret_key: SecretKey,
}

impl PointerNameResolver {
    pub fn new(caching_client: CachingClient, pointer_name_resolver_secret_key: SecretKey) -> PointerNameResolver {
        PointerNameResolver { caching_client, pointer_name_resolver_secret_key }
    }

    pub async fn is_resolved(&self, name: &String) -> bool {
        self.resolve(name).await.is_some()
    }

    pub async fn resolve(&self, name: &String) -> Option<String> {
        if name.is_empty() {
            None
        } else {
            debug!("get key from name: {}", name);
            let pointer_key = Client::register_key_from_name(&self.pointer_name_resolver_secret_key, name.as_str());
            debug!("found: name={}, pointer_key={}, public_key={}", name, pointer_key.to_hex(), &pointer_key.public_key().to_hex());
            match self.resolve_pointer(&pointer_key.public_key().to_hex(), 0).await.ok() {
                Some(pointer) => {
                    Some(pointer.target().to_hex())
                }
                None => None,
            }
        }
    }

    async fn resolve_pointer(&self, address: &String, iteration: usize) -> Result<Pointer, PointerError> {
        debug!("resolve_pointer: address={}, iteration={}", address, iteration);
        if iteration > 10 {
            error!("cyclic reference loop - resolve aborting");
            Err(PointerError::GetError(GetError::RecordNotFound(format!("Too many iterations which resolving: {}", address))))
        } else {
            match PointerAddress::from_hex(address) {
                Ok(pointer_address) => match self.caching_client.pointer_get(&pointer_address).await {
                    Ok(pointer) => match pointer.target() {
                        PointerTarget::ChunkAddress(_) => Ok(pointer),
                        _ => Box::pin(self.resolve_pointer(&pointer.target().to_hex(), iteration + 1)).await,
                    }
                    Err(_) => Err(PointerError::GetError(GetError::RecordNotFound(format!("Not found: {}", address))))
                }
                Err(_) => Err(PointerError::GetError(GetError::RecordNotFound(format!("Not found: {}", address))))
            }
        }
    }
}