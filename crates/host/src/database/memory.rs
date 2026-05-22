use std::{collections::HashMap, sync::Mutex};

use super::{Database, DatabaseError};

#[derive(Default)]
pub struct MemoryDatabase {
    store: Mutex<HashMap<Vec<u8>, Vec<u8>>>,
}

#[async_trait::async_trait]
impl Database for MemoryDatabase {
    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, DatabaseError> {
        Ok(self.store.lock().unwrap().get(key).cloned())
    }

    async fn set(&self, key: &[u8], value: &[u8]) -> Result<(), DatabaseError> {
        self.store.lock().unwrap().insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    async fn delete(&self, key: &[u8]) -> Result<(), DatabaseError> {
        self.store.lock().unwrap().remove(key);
        Ok(())
    }
}
