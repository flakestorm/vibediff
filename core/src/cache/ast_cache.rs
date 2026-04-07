use std::path::PathBuf;

use anyhow::Result;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::ast::entity_mapper::EntityChange;

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheKey(pub String);

pub struct AstCache {
    mem: DashMap<CacheKey, Vec<EntityChange>>,
    db: sled::Db,
}

impl AstCache {
    pub fn new(path: PathBuf) -> Result<Self> {
        let db = sled::open(path)?;
        Ok(Self {
            mem: DashMap::new(),
            db,
        })
    }

    pub fn get(&self, key: &CacheKey) -> Result<Option<Vec<EntityChange>>> {
        if let Some(v) = self.mem.get(key) {
            return Ok(Some(v.clone()));
        }
        if let Some(bytes) = self.db.get(&key.0)? {
            let entities: Vec<EntityChange> = serde_json::from_slice(&bytes)?;
            self.mem.insert(key.clone(), entities.clone());
            return Ok(Some(entities));
        }
        Ok(None)
    }

    pub fn set(&self, key: CacheKey, value: Vec<EntityChange>) -> Result<()> {
        let bytes = serde_json::to_vec(&value)?;
        self.db.insert(key.0.as_bytes(), bytes)?;
        self.mem.insert(key, value);
        Ok(())
    }

    pub fn clear(&self) -> Result<()> {
        self.mem.clear();
        self.db.clear()?;
        Ok(())
    }

    pub fn prune(&self, max_entries: usize) -> Result<usize> {
        let mut removed = 0usize;
        let mut keys = Vec::new();
        for item in self.db.iter().keys() {
            keys.push(item?);
        }
        if keys.len() <= max_entries {
            return Ok(0);
        }
        let to_remove = keys.len() - max_entries;
        for k in keys.into_iter().take(to_remove) {
            self.db.remove(k)?;
            removed += 1;
        }
        Ok(removed)
    }
}
