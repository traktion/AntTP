use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema, Clone, Debug, PartialEq)]
pub struct KeyValue {
    pub content: String,
}

impl KeyValue {
    pub fn new(content: String) -> Self {
        Self {
            content,
        }
    }
}
