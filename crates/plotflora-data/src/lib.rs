// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 ChatCarbo

//! Couche données de PlotFlora : fichiers JSON, catalogue, cache, EIVE et sources en ligne.
#![forbid(unsafe_code)]

pub mod cache;
pub mod catalog;
pub mod eive;
pub mod online;
pub mod store;

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{} : {source}", path.display())]
    Io { path: PathBuf, source: std::io::Error },
    #[error("{} : JSON invalide ({source})", path.display())]
    Json { path: PathBuf, source: serde_json::Error },
    #[error("{} : version de schéma {found} non prise en charge (attendu 1.x)", path.display())]
    Version { path: PathBuf, found: String },
    #[error("réseau : {0}")]
    Network(String),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Dossier de données de l'application (`%APPDATA%\PlotFlora` sous Windows).
pub fn app_data_dir() -> PathBuf {
    dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("PlotFlora")
}

/// Secondes depuis l'époque Unix.
pub fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// Date du jour (UTC) au format ISO.
pub fn today() -> String {
    plotflora_core::convert::iso_date(now_secs())
}
