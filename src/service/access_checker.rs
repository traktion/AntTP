use crate::config::access_list::AccessList;
use std::collections::HashMap;
use log::info;

#[derive(Debug)]
enum AccessType {
    Allow, Deny
}

pub struct AccessChecker {
    map: HashMap<String, AccessType>,
}

impl AccessChecker {

    pub fn new() -> AccessChecker {
        let map = HashMap::new();
        AccessChecker { map }
    }
    pub fn update(&mut self, access_list: &AccessList) {
        for allow_address in access_list.allow() {
            self.map.insert(allow_address.clone(), AccessType::Allow);
        }
        for deny_address in access_list.deny() {
            self.map.insert(deny_address.clone(), AccessType::Deny);
        }
    }

    pub fn is_allowed(&self, address: &String) -> bool {
        info!("map: {:?}, address: {}", self.map, address);
        match self.map.get(address) {
            Some(AccessType::Allow) => true,
            Some(AccessType::Deny) => false,
            None => match self.map.get(&"all".to_string()) {
                Some(AccessType::Deny) => false, // default to deny
                _ => true, // default to allow
            }
        }
    }
}