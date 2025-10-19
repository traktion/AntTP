use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use globset::Glob;
use log::{debug, info};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    route_map: HashMap<String, String>
}

impl AppConfig {

    pub fn default() -> Self {
        Self {
            route_map: HashMap::new(),
        }
    }
    
    pub fn resolve_route(&self, search_string: String) -> (String, bool) {
        debug!("resolving route [{}]", search_string);
        for (key, value) in self.route_map.clone() {
            let glob = Glob::new(key.as_str()).unwrap().compile_matcher();
            debug!("route mapper comparing path [{}] with glob [{}]", search_string, key);
            if glob.is_match(&search_string) {
                info!("route mapper resolved path [{}] to [{}] with glob [{}]", search_string, key, value);
                return (value, true);
            }
        };
        (search_string, false)
    }
}