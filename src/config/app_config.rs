use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use globset::Glob;
use log::{debug, info};

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    route_map: HashMap<String, String>
}

impl AppConfig {
    pub fn resolve_route(&self, search_string: &String) -> (String, bool) {
        debug!("resolving route [{}]", search_string);
        for (key, value) in self.route_map.clone() {
            let glob = Glob::new(key.as_str()).unwrap().compile_matcher();
            debug!("route mapper comparing path [{}] with glob [{}]", search_string, key);
            if glob.is_match(&search_string) {
                info!("route mapper resolved path [{}] to [{}] with glob [{}]", search_string, key, value);
                return (value, true);
            }
        };
        (search_string.clone(), false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_config(routes: Vec<(&str, &str)>) -> AppConfig {
        let mut map = HashMap::new();
        for (key, value) in routes {
            map.insert(key.to_string(), value.to_string());
        }
        AppConfig { route_map: map }
    }

    #[test]
    fn test_resolve_route_match() {
        let config = create_config(vec![("test.html", "resolved.html")]);
        let (resolved, found) = config.resolve_route(&"test.html".to_string());
        assert!(found);
        assert_eq!(resolved, "resolved.html");
    }

    #[test]
    fn test_resolve_route_no_match() {
        let config = create_config(vec![("test.html", "resolved.html")]);
        let (resolved, found) = config.resolve_route(&"other.html".to_string());
        assert!(!found);
        assert_eq!(resolved, "other.html");
    }

    #[test]
    fn test_resolve_route_glob() {
        let config = create_config(vec![("*.html", "index.html")]);
        
        let (resolved, found) = config.resolve_route(&"any.html".to_string());
        assert!(found);
        assert_eq!(resolved, "index.html");

        let (resolved_other, found_other) = config.resolve_route(&"image.png".to_string());
        assert!(!found_other);
        assert_eq!(resolved_other, "image.png");
    }
}