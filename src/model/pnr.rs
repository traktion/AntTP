use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PnrZone {
    pub name: String,
    pub records: Vec<PnrRecord>,
    #[schema(read_only)]
    pub resolver_address: Option<String>,
    #[schema(read_only)]
    pub personal_address: Option<String>,
}

impl PnrZone {
    pub fn new(name: String, records: Vec<PnrRecord>, resolver_address: Option<String>, personal_address: Option<String>) -> Self {
        Self { name, records, resolver_address, personal_address }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PnrRecord {
    pub sub_name: Option<String>,
    pub address: String,
    pub record_type: PnrRecordType,
    pub ttl: u64,
}

impl PnrRecord {
    pub fn new(sub_name: Option<String>, address: String, record_type: PnrRecordType, ttl: u64) -> Self {
        Self { sub_name, address, record_type, ttl }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub enum PnrRecordType {
    A, X
}