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

    pub fn set_allow(&mut self, allow: Vec<String>) {
        self.allow = allow;
    }

    pub fn set_deny(&mut self, deny: Vec<String>) {
        self.deny = deny;
    }
}