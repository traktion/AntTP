use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PnrZone {
    pub name: String,
    pub records: Vec<PnrRecord>,
}

impl PnrZone {
    pub fn new(name: String, records: Vec<PnrRecord>) -> Self {
        Self { name, records }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PnrRecord {
    pub sub_name: Option<String>,
    pub address: String,
    pub ttl: u64,
}

impl PnrRecord {
    pub fn new(sub_name: Option<String>, address: String, ttl: u64) -> Self {
        Self { sub_name, address, ttl }
    }
}