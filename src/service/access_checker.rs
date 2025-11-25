use crate::model::access_list::AccessList;
use std::collections::HashMap;
use log::debug;

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
        debug!("map: {:?}, address: {}", self.map, address);
        match self.map.get(address) {
            Some(AccessType::Allow) => true,
            Some(AccessType::Deny) => false,
            None => self.is_allowed_default()
        }
    }

    pub fn is_allowed_default(&self) -> bool {
        match self.map.get(&"all".to_string()) {
            Some(AccessType::Deny) => false, // default to deny
            _ => true, // default to allow
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::access_list::AccessList;

    fn create_access_list(allow: Vec<&str>, deny: Vec<&str>) -> AccessList {
        let json = serde_json::json!({
            "allow": allow,
            "deny": deny
        });
        serde_json::from_value(json).unwrap()
    }

    #[test]
    fn test_default_allow() {
        let checker = AccessChecker::new();
        assert!(checker.is_allowed(&"allowed".to_string()));
    }

    #[test]
    fn test_explicit_deny() {
        let mut checker = AccessChecker::new();
        let list = create_access_list(vec![], vec!["denied"]);
        checker.update(&list);
        
        assert!(!checker.is_allowed(&"denied".to_string()));
        assert!(checker.is_allowed(&"allowed".to_string())); // Others still allowed
    }

    #[test]
    fn test_explicit_allow() {
        let mut checker = AccessChecker::new();
        // First deny all to verify explicit allow works
        let list = create_access_list(vec!["allowed"], vec!["all"]);
        checker.update(&list);

        assert!(checker.is_allowed(&"allowed".to_string()));
        assert!(!checker.is_allowed(&"denied".to_string()));
    }

    #[test]
    fn test_deny_all() {
        let mut checker = AccessChecker::new();
        let list = create_access_list(vec![], vec!["all"]);
        checker.update(&list);

        assert!(!checker.is_allowed(&"denied1".to_string()));
        assert!(!checker.is_allowed(&"denied2".to_string()));
    }

    #[test]
    fn test_allow_override_deny_all() {
        let mut checker = AccessChecker::new();
        let list = create_access_list(vec!["allowed"], vec!["all"]);
        checker.update(&list);

        assert!(checker.is_allowed(&"allowed".to_string()));
        assert!(!checker.is_allowed(&"denied".to_string()));
    }

    #[test]
    fn test_deny_override_default_allow() {
        let mut checker = AccessChecker::new();
        let list = create_access_list(vec![], vec!["denied"]);
        checker.update(&list);

        assert!(!checker.is_allowed(&"denied".to_string()));
        assert!(checker.is_allowed(&"allowed".to_string()));
    }
}