use std::collections::HashMap;
use crate::model::bookmark_list::BookmarkList;

pub struct BookmarkResolver {
    map: HashMap<String, String>,
}

impl BookmarkResolver {

    pub fn new() -> BookmarkResolver {
        let map = HashMap::new();
        BookmarkResolver { map }
    }

    pub fn update(&mut self, bookmark_list: &BookmarkList) {
        self.map = bookmark_list.bookmarks().clone();
    }

    pub fn is_bookmark(&self, name: &String) -> bool {
        self.map.contains_key(name)
    }

    pub fn resolve_bookmark(&self, name: &String) -> Option<String> {
        self.map.get(name).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::bookmark_list::BookmarkList;

    fn create_bookmark_list(bookmarks: HashMap<String, String>) -> BookmarkList {
        let json = serde_json::json!({
            "bookmarks": bookmarks
        });
        serde_json::from_value(json).unwrap()
    }

    #[test]
    fn test_new() {
        let resolver = BookmarkResolver::new();
        assert!(!resolver.is_bookmark(&"test".to_string()));
        assert!(resolver.resolve_bookmark(&"test".to_string()).is_none());
    }

    #[test]
    fn test_update_and_resolve() {
        let mut resolver = BookmarkResolver::new();
        let mut bookmarks = HashMap::new();
        bookmarks.insert("google".to_string(), "https://google.com".to_string());
        bookmarks.insert("local".to_string(), "http://localhost:8080".to_string());
        
        let list = create_bookmark_list(bookmarks);
        resolver.update(&list);

        assert_eq!(resolver.resolve_bookmark(&"google".to_string()), Some("https://google.com".to_string()));
        assert_eq!(resolver.resolve_bookmark(&"local".to_string()), Some("http://localhost:8080".to_string()));
    }

    #[test]
    fn test_is_bookmark() {
        let mut resolver = BookmarkResolver::new();
        let mut bookmarks = HashMap::new();
        bookmarks.insert("exists".to_string(), "target".to_string());
        
        let list = create_bookmark_list(bookmarks);
        resolver.update(&list);

        assert!(resolver.is_bookmark(&"exists".to_string()));
        assert!(!resolver.is_bookmark(&"missing".to_string()));
    }

    #[test]
    fn test_resolve_non_existent() {
        let mut resolver = BookmarkResolver::new();
        let mut bookmarks = HashMap::new();
        bookmarks.insert("exists".to_string(), "target".to_string());
        
        let list = create_bookmark_list(bookmarks);
        resolver.update(&list);

        assert!(resolver.resolve_bookmark(&"missing".to_string()).is_none());
    }
}