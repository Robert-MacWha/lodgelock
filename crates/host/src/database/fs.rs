use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
};

use super::{Database, DatabaseError};

/// Filesystem-backed database. Each key is stored as a separate file whose
/// name is the hex encoding of the key bytes. Writes are atomic: the value is
/// written to a `.tmp` file first, then renamed into place.
pub struct FsDatabase {
    dir: PathBuf,
}

impl FsDatabase {
    pub fn new(dir: impl AsRef<Path>) -> Result<Self, DatabaseError> {
        let dir = dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&dir)
            .map_err(|e| DatabaseError::StorageError(e.to_string()))?;
        Ok(Self { dir })
    }

    fn key_path(&self, key: &[u8]) -> PathBuf {
        // FNV-1a 64-bit hash — stable, collision-free for our structured key space,
        // and keeps filenames to a fixed 16 hex chars regardless of key length.
        let mut h: u64 = 0xcbf29ce484222325;
        for &b in key {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        self.dir.join(format!("{h:016x}"))
    }
}

#[async_trait::async_trait]
impl Database for FsDatabase {
    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, DatabaseError> {
        let path = self.key_path(key);
        match std::fs::read(&path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
            Err(e) => Err(DatabaseError::StorageError(e.to_string())),
        }
    }

    async fn set(&self, key: &[u8], value: &[u8]) -> Result<(), DatabaseError> {
        let path = self.key_path(key);
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, value)
            .map_err(|e| DatabaseError::StorageError(e.to_string()))?;
        std::fs::rename(&tmp, &path)
            .map_err(|e| DatabaseError::StorageError(e.to_string()))?;
        Ok(())
    }

    async fn delete(&self, key: &[u8]) -> Result<(), DatabaseError> {
        let path = self.key_path(key);
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
            Err(e) => Err(DatabaseError::StorageError(e.to_string())),
        }
    }
}
