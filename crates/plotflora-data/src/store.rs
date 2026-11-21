// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 ChatCarbo

//! Lecture / écriture atomique des fichiers JSON et gestion des chemins relatifs.

use crate::{Error, Result};
use plotflora_core::{Project, SCHEMA_VERSION};
use serde::{de::DeserializeOwned, Serialize};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

fn io(path: &Path) -> impl FnOnce(std::io::Error) -> Error + '_ {
    move |source| Error::Io { path: path.to_owned(), source }
}

pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let bytes = std::fs::read(path).map_err(io(path))?;
    serde_json::from_slice(&bytes).map_err(|source| Error::Json { path: path.to_owned(), source })
}

/// Écrit `value` de façon atomique : fichier temporaire du même dossier, `fsync`,
/// copie de l'ancienne version en `.bak`, puis remplacement.
pub fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let json = |source| Error::Json { path: path.to_owned(), source };
    let data = serde_json::to_vec_pretty(value).map_err(json)?;
    serde_json::from_slice::<serde_json::Value>(&data).map_err(json)?; // validation avant écrasement
    let dir = match path.parent() {
        Some(d) if !d.as_os_str().is_empty() => d,
        _ => Path::new("."),
    };
    std::fs::create_dir_all(dir).map_err(io(dir))?;
    let mut tmp = tempfile::NamedTempFile::new_in(dir).map_err(io(dir))?;
    tmp.write_all(&data).map_err(io(tmp.path()))?;
    tmp.as_file().sync_all().map_err(io(tmp.path()))?;
    if path.exists() {
        let mut bak = path.as_os_str().to_owned();
        bak.push(".bak");
        std::fs::copy(path, &bak).map_err(io(path))?;
    }
    tmp.persist(path).map_err(|e| Error::Io { path: path.to_owned(), source: e.error })?;
    Ok(())
}

/// Accepte toute version 1.x.
pub fn check_version(path: &Path, version: &str) -> Result<()> {
    if version.split('.').next() == Some("1") {
        Ok(())
    } else {
        Err(Error::Version { path: path.to_owned(), found: version.to_owned() })
    }
}

pub fn load_project(path: &Path) -> Result<Project> {
    let mut p: Project = read_json(path)?;
    check_version(path, &p.schema_version)?;
    p.schema_version = SCHEMA_VERSION.into();
    for z in &mut p.zones {
        z.points.retain(|pt| pt[0].is_finite() && pt[1].is_finite());
    }
    p.map.set_north(p.map.north_angle_deg);
    Ok(p)
}

pub fn save_project(path: &Path, project: &mut Project) -> Result<()> {
    project.prune_snapshot();
    project.schema_version = SCHEMA_VERSION.into();
    write_json_atomic(path, project)
}

/// Résout un chemin stocké (relatif au dossier de `base_file`, séparateur `/`).
pub fn resolve(base_file: &Path, stored: &str) -> PathBuf {
    let p = Path::new(stored);
    if p.is_absolute() {
        return p.to_owned();
    }
    let mut out = base_file.parent().map(Path::to_owned).unwrap_or_default();
    for part in stored.split(['/', '\\']).filter(|s| !s.is_empty() && *s != ".") {
        out.push(part);
    }
    out
}

/// Chemin à stocker : relatif au dossier de `base_file` si possible, sinon absolu ; toujours avec `/`.
pub fn relativize(base_file: &Path, target: &Path) -> String {
    let rel = base_file.parent().and_then(|dir| target.strip_prefix(dir).ok());
    let p = rel.unwrap_or(target);
    let parts: Vec<String> = p
        .components()
        .filter_map(|c| match c {
            Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
            Component::Prefix(s) => Some(s.as_os_str().to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();
    let joined = parts.join("/");
    if rel.is_none() && p.has_root() && !joined.contains(':') {
        format!("/{joined}")
    } else {
        joined
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plotflora_core::{MapSource, ZoneKind};

    #[test]
    fn project_round_trip_and_backup() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("projet.json");
        let mut p = Project::new("Test", MapSource::Drawing);
        p.add_zone(ZoneKind::Area, vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]);
        save_project(&path, &mut p).unwrap();
        save_project(&path, &mut p).unwrap();
        assert!(dir.path().join("projet.json.bak").exists());
        assert_eq!(load_project(&path).unwrap(), p);
    }

    #[test]
    fn rejects_major_version() {
        assert!(check_version(Path::new("x"), "2.0.0").is_err());
        assert!(check_version(Path::new("x"), "1.0.0").is_ok());
    }

    #[test]
    fn paths() {
        let base = Path::new("/jardin/projet.json");
        assert_eq!(relativize(base, Path::new("/jardin/img/vue.png")), "img/vue.png");
        assert_eq!(resolve(base, "img/vue.png"), Path::new("/jardin/img/vue.png"));
        assert_eq!(resolve(base, "./vue.png"), Path::new("/jardin/vue.png"));
    }
}
