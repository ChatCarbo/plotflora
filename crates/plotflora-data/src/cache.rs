// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 ChatCarbo

//! Cache horodaté des réponses en ligne (`lookup_cache.json`, §5.9).

use crate::{now_secs, store, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Durée de validité : 180 jours.
pub const TTL_SECS: u64 = 180 * 86_400;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Entry {
    pub fetched_at: u64,
    pub body: String,
}

#[derive(Debug, Default)]
pub struct LookupCache {
    path: PathBuf,
    entries: BTreeMap<String, Entry>,
    dirty: bool,
}

impl LookupCache {
    /// Charge le cache ; un fichier absent ou illisible donne un cache vide.
    pub fn load(path: &Path) -> Self {
        let entries = store::read_json(path).unwrap_or_default();
        Self { path: path.to_owned(), entries, dirty: false }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).filter(|e| now_secs().saturating_sub(e.fetched_at) < TTL_SECS).map(|e| e.body.as_str())
    }

    pub fn put(&mut self, key: String, body: String) {
        self.entries.insert(key, Entry { fetched_at: now_secs(), body });
        self.dirty = true;
    }

    pub fn save(&mut self) -> Result<()> {
        if self.dirty {
            let now = now_secs();
            self.entries.retain(|_, e| now.saturating_sub(e.fetched_at) < TTL_SECS);
            store::write_json_atomic(&self.path, &self.entries)?;
            self.dirty = false;
        }
        Ok(())
    }
}
