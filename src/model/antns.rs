use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RecordType {
    A,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AntNs {
    name: String,
    address: String,
    record_type: RecordType,
    ttl: u32,
}

impl AntNs {
    pub fn new(name: String, address: String, record_type: RecordType, ttl: u32) -> Self {
        Self {
            name,
            address,
            record_type,
            ttl,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    pub fn record_type(&self) -> &RecordType {
        &self.record_type
    }

    pub fn ttl(&self) -> u32 {
        self.ttl
    }
}
