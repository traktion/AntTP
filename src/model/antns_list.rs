use serde::{Deserialize, Serialize};
use crate::model::antns::AntNs;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AntNsList {
    antns: Vec<AntNs>,
}

impl AntNsList {
    pub fn new(antns: Vec<AntNs>) -> Self {
        Self { antns }
    }

    pub fn antns(&self) -> &Vec<AntNs> {
        &self.antns
    }
}