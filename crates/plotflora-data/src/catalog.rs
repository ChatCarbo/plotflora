// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 ChatCarbo

//! Catalogue : fiches embarquées + catalogue utilisateur (prioritaire, même `id`).

use crate::{store, Result};
use plotflora_core::{Catalog, Plant, SCHEMA_VERSION};
use std::path::{Path, PathBuf};

const SEED: &str = include_str!("../../../assets/plants_catalog.seed.json");

#[derive(Debug)]
pub struct CatalogStore {
    path: PathBuf,
    user: Catalog,
    seed: Vec<Plant>,
}

/// Catalogue embarqué dans le binaire.
pub fn seed() -> Vec<Plant> {
    serde_json::from_str::<Catalog>(SEED).map(|c| c.plants).unwrap_or_default()
}

impl CatalogStore {
    /// Charge le catalogue utilisateur (`plants_catalog.json` du dossier `dir`), absent = vide.
    pub fn load(dir: &Path) -> Result<Self> {
        let path = dir.join("plants_catalog.json");
        let user = if path.exists() {
            let c: Catalog = store::read_json(&path)?;
            store::check_version(&path, &c.schema_version)?;
            c
        } else {
            Catalog { schema_version: SCHEMA_VERSION.into(), plants: Vec::new() }
        };
        Ok(Self { path, user, seed: seed() })
    }

    /// Catalogue utilisateur vide (repli si le fichier existant est illisible ;
    /// l'ancien fichier est conservé en `.bak` à la première sauvegarde).
    pub fn empty(dir: &Path) -> Self {
        Self {
            path: dir.join("plants_catalog.json"),
            user: Catalog { schema_version: SCHEMA_VERSION.into(), plants: Vec::new() },
            seed: seed(),
        }
    }

    /// Toutes les fiches, la version utilisateur masquant la version embarquée.
    pub fn plants(&self) -> Vec<&Plant> {
        let mut v: Vec<&Plant> = self.user.plants.iter().collect();
        v.extend(self.seed.iter().filter(|s| self.user.get(&s.id).is_none()));
        v
    }

    pub fn get(&self, id: &str) -> Option<&Plant> {
        self.user.get(id).or_else(|| self.seed.iter().find(|p| p.id == id))
    }

    pub fn contains(&self, id: &str) -> bool {
        self.get(id).is_some()
    }

    pub fn is_user(&self, id: &str) -> bool {
        self.user.get(id).is_some()
    }

    pub fn upsert(&mut self, plant: Plant) {
        self.user.upsert(plant);
    }

    /// Supprime une fiche utilisateur (la version embarquée éventuelle redevient visible).
    pub fn remove_user(&mut self, id: &str) -> bool {
        let n = self.user.plants.len();
        self.user.plants.retain(|p| p.id != id);
        n != self.user.plants.len()
    }

    pub fn save(&mut self) -> Result<()> {
        self.user.schema_version = SCHEMA_VERSION.into();
        store::write_json_atomic(&self.path, &self.user)
    }

    pub fn dir(&self) -> &Path {
        self.path.parent().unwrap_or(Path::new("."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_is_valid() {
        let s = seed();
        assert_eq!(s.len(), 15);
        assert!(s.iter().all(|p| !p.id.is_empty() && p.category.is_some()));
    }

    #[test]
    fn user_overrides_seed() {
        let dir = tempfile::tempdir().unwrap();
        let mut c = CatalogStore::load(dir.path()).unwrap();
        let mut p = c.get("lavandula_angustifolia").unwrap().clone();
        p.notes = "modifié".into();
        c.upsert(p);
        c.save().unwrap();
        let c = CatalogStore::load(dir.path()).unwrap();
        assert_eq!(c.plants().len(), 15);
        assert_eq!(c.get("lavandula_angustifolia").unwrap().notes, "modifié");
    }
}
