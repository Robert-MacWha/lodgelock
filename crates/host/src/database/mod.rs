pub mod fs;
pub mod memory;

pub use fs::FsDatabase;
pub use memory::MemoryDatabase;

use tlock_hdk::{tlock_api::entities::EntityId, wasmi_plugin_hdk::plugin_id::PluginId};

use crate::host_state::PluginSource;

#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Unsupported version: {0}")]
    UnsupportedVersion(u32),
    #[error("Storage error: {0}")]
    StorageError(String),
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
pub trait Database: Send + Sync {
    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, DatabaseError>;
    async fn set(&self, key: &[u8], value: &[u8]) -> Result<(), DatabaseError>;
    async fn delete(&self, key: &[u8]) -> Result<(), DatabaseError>;
}

// --- Key helpers ---

pub(crate) fn plugins_key() -> &'static [u8] {
    b"plugins"
}

pub(crate) fn plugin_name_key(id: PluginId) -> Vec<u8> {
    format!("plugin_name:{id:#}").into_bytes()
}

pub(crate) fn plugin_source_key(id: PluginId) -> Vec<u8> {
    format!("plugin_source:{id:#}").into_bytes()
}

pub(crate) fn entities_key() -> &'static [u8] {
    b"entities"
}

pub(crate) fn plugin_state_key(id: PluginId, key: &str) -> Vec<u8> {
    format!("state:{id:#}:{key}").into_bytes()
}

// --- HostDatabase extension trait ---

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
pub(crate) trait HostDatabase: Database {
    async fn get_plugins(&self) -> Result<Vec<PluginId>, DatabaseError> {
        match self.get(plugins_key()).await? {
            None => Ok(vec![]),
            Some(bytes) => Ok(serde_json::from_slice(&bytes)?),
        }
    }

    async fn set_plugins(&self, ids: &[PluginId]) -> Result<(), DatabaseError> {
        self.set(plugins_key(), &serde_json::to_vec(ids)?).await
    }

    async fn get_plugin_name(&self, id: PluginId) -> Result<Option<String>, DatabaseError> {
        match self.get(&plugin_name_key(id)).await? {
            None => Ok(None),
            Some(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
        }
    }

    async fn set_plugin_name(&self, id: PluginId, name: &str) -> Result<(), DatabaseError> {
        self.set(&plugin_name_key(id), &serde_json::to_vec(name)?).await
    }

    async fn get_plugin_source(&self, id: PluginId) -> Result<Option<PluginSource>, DatabaseError> {
        match self.get(&plugin_source_key(id)).await? {
            None => Ok(None),
            Some(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
        }
    }

    async fn set_plugin_source(
        &self,
        id: PluginId,
        source: &PluginSource,
    ) -> Result<(), DatabaseError> {
        self.set(&plugin_source_key(id), &serde_json::to_vec(source)?).await
    }

    async fn get_entities(&self) -> Result<Vec<(EntityId, PluginId)>, DatabaseError> {
        match self.get(entities_key()).await? {
            None => Ok(vec![]),
            Some(bytes) => Ok(serde_json::from_slice(&bytes)?),
        }
    }

    async fn set_entities(&self, entities: &[(EntityId, PluginId)]) -> Result<(), DatabaseError> {
        self.set(entities_key(), &serde_json::to_vec(entities)?).await
    }

    async fn get_plugin_state(
        &self,
        id: PluginId,
        key: &str,
    ) -> Result<Option<Vec<u8>>, DatabaseError> {
        self.get(&plugin_state_key(id, key)).await
    }

    async fn set_plugin_state(
        &self,
        id: PluginId,
        key: &str,
        value: &[u8],
    ) -> Result<(), DatabaseError> {
        self.set(&plugin_state_key(id, key), value).await
    }
}

impl<D: Database + ?Sized> HostDatabase for D {}
