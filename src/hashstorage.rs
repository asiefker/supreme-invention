use std::collections::HashMap;
use storage::Storage;

pub struct HashStorage {
    data: HashMap<String, String>,
}

impl HashStorage {
    pub fn new() -> HashStorage {
        HashStorage { data: HashMap::new() }
    }
}


impl Storage for HashStorage {
    fn put(&mut self, key: String, value: String) -> Option<String> {
        self.data.insert(key, value)
    }

    fn get(&self, key: &String) -> Option<String> {
        self.data.get(key).map(|s| {s.clone()})
    }

    fn len(&self) -> usize {
        self.data.len()
    }
}
