use serde::{Deserialize, Serialize};

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct PathDetail {
    pub path: String,
    pub display: String,
    pub modified: u64,
    pub size: u64
}