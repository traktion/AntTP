use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AccessList {
    allow: Vec<String>,
    deny: Vec<String>,
}

impl AccessList {
    pub fn allow(&self) -> &Vec<String> {
        &self.allow
    }

    pub fn deny(&self) -> &Vec<String> {
        &self.deny
    }
}