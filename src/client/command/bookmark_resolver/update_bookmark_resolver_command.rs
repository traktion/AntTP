use actix_http::header::HeaderMap;
use actix_web::web::Data;
use async_trait::async_trait;
use indexmap::IndexMap;
use log::{debug, info};
use sha2::Digest;
use tokio::sync::Mutex;
#[double]
use crate::client::PointerCachingClient;
use crate::client::{ArchiveCachingClient, CachingClient, ChunkCachingClient, RegisterCachingClient};
use mockall_double::double;
use crate::client::command::error::CommandError;
use crate::client::command::Command;
use crate::config::anttp_config::AntTpConfig;
use crate::model::bookmark_list::BookmarkList;
use crate::service::access_checker::AccessChecker;
use crate::service::bookmark_resolver::BookmarkResolver;
use crate::service::pointer_name_resolver::PointerNameResolver;
use crate::service::file_service::FileService;
use crate::service::resolver_service::ResolverService;

pub struct UpdateBookmarkResolverCommand {
    id: u128,
    caching_client: Data<Mutex<CachingClient>>,
    ant_tp_config: AntTpConfig,
    access_checker: Data<Mutex<AccessChecker>>,
    bookmark_resolver: Data<Mutex<BookmarkResolver>>,
    pointer_name_resolver: Data<PointerNameResolver>,
}

impl UpdateBookmarkResolverCommand {
    pub fn new(caching_client: Data<Mutex<CachingClient>>,
               ant_tp_config: AntTpConfig,
               access_checker: Data<Mutex<AccessChecker>>,
               bookmark_resolver: Data<Mutex<BookmarkResolver>>,
               pointer_name_resolver: Data<PointerNameResolver>,
    ) -> Self {
        let id = rand::random::<u128>();
        Self { id, caching_client, ant_tp_config, access_checker, bookmark_resolver, pointer_name_resolver }
    }
}

const STRUCT_NAME: &'static str = "UpdateBookmarkResolverCommand";

#[async_trait]
impl Command for UpdateBookmarkResolverCommand {
    async fn execute(&self) -> Result<(), CommandError> {
        let caching_client = self.caching_client.get_ref().lock().await.clone();
        let resolver_service = ResolverService::new(
            ArchiveCachingClient::new(caching_client.clone()),
            PointerCachingClient::new(caching_client.clone()),
            RegisterCachingClient::new(caching_client.clone()),
            self.access_checker.clone(),
            self.bookmark_resolver.clone(),
            self.pointer_name_resolver.clone(),
            self.ant_tp_config.cached_mutable_ttl,
        );
        let file_service = FileService::new(ChunkCachingClient::new(caching_client.clone()), caching_client, 1);
        
        let bookmark_list = match resolver_service.resolve(&self.ant_tp_config.bookmarks_address, &"", &HeaderMap::new()).await {
            Some(resolved_address) => match file_service.download_data_bytes(resolved_address.xor_name, 0, 0).await {
                Ok(buf) => {
                    let json = String::from_utf8(buf.to_vec()).unwrap_or(String::new());
                    debug!("json [{}]", json);
                    serde_json::from_str(&json.as_str().trim()).unwrap_or(BookmarkList::default())
                }
                Err(_) => BookmarkList::default()
            }
            None => BookmarkList::default()
        };
        self.bookmark_resolver.lock().await.update(&bookmark_list);
        info!("bookmark list at [{}] processed successfully: [{:?}]", self.ant_tp_config.bookmarks_address, bookmark_list);
        Ok(())
    }

    fn action_hash(&self) -> Vec<u8> {
        let mut hasher = sha2::Sha256::new();
        hasher.update(STRUCT_NAME);
        hasher.update(self.ant_tp_config.access_list_address.clone());
        hasher.finalize().to_ascii_lowercase()
    }

    fn id(&self) -> u128 {
        self.id
    }

    fn name(&self) -> String {
        STRUCT_NAME.to_string()
    }

    fn properties(&self) -> IndexMap<String, String> {
        let mut properties = IndexMap::new();
        properties.insert("bookmarks_address".to_string(), self.ant_tp_config.bookmarks_address.clone());
        properties
    }
}