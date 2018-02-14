pub trait Storage {
    fn put(&mut self, key: String, value: String) -> Option<String>;
    fn get(&self, key: &String) -> Option<String>;
    fn len(&self) -> usize;
}
