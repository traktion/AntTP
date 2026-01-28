use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema, Clone, Debug, PartialEq)]
pub struct KeyValue {
    pub bucket: String,
    pub object: String,
    pub content: String,
}

impl KeyValue {
    pub fn new(bucket: String, object: String, content: String) -> Self {
        Self {
            bucket,
            object,
            content,
        }
    }
}
