use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct PathDetail {
    pub path: String,
    pub display: String,
    pub modified: u64,
    pub size: u64,
    pub path_type: PathDetailType,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, ToSchema)]
pub enum PathDetailType {
    FILE, DIRECTORY
}
