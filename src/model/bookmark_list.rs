use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BookmarkList {
    bookmarks: HashMap<String, String>,
}

impl BookmarkList {
    pub fn bookmarks(&self) -> &HashMap<String, String> {
        &self.bookmarks
    }
}