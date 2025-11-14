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