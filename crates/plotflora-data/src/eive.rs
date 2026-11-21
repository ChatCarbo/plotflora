// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 ChatCarbo

//! Index EIVE embarqué : nom latin → valeurs indicatrices [lumière, humidité, réaction].
//! Le fichier `assets/eive_index.json` est produit par `plotflora-catalog-builder`.

use plotflora_core::convert::{eive_light, eive_moisture, eive_ph};
use plotflora_core::search::normalize;
use plotflora_core::Plant;
use std::collections::HashMap;

const EMBEDDED: &str = include_str!("../../../assets/eive_index.json");

#[derive(Debug, Default, Clone)]
pub struct EiveIndex(HashMap<String, [Option<f64>; 3]>);

/// Clé : genre + espèce normalisés (sans auteur, hybride ni cultivar).
pub fn key(latin: &str) -> String {
    normalize(latin).split(' ').filter(|w| *w != "x").take(2).collect::<Vec<_>>().join(" ")
}

impl EiveIndex {
    pub fn embedded() -> Self {
        Self::from_json(EMBEDDED).unwrap_or_default()
    }

    pub fn from_json(s: &str) -> serde_json::Result<Self> {
        let raw: HashMap<String, [Option<f64>; 3]> = serde_json::from_str(s)?;
        Ok(Self(raw.into_iter().map(|(k, v)| (key(&k), v)).collect()))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn get(&self, latin: &str) -> Option<[Option<f64>; 3]> {
        self.0.get(&key(latin)).copied()
    }

    /// Complète exposition, humidité et pH s'ils sont vides. Retourne vrai si un champ a été rempli.
    pub fn apply(&self, p: &mut Plant) -> bool {
        let Some([l, m, r]) = self.get(&p.latin_name) else { return false };
        let mut changed = false;
        let mut mark = |p: &mut Plant, field: &str, src: String| {
            p.meta.field_sources.insert(field.into(), src);
            if !p.meta.uncertain_fields.iter().any(|f| f == field) {
                p.meta.uncertain_fields.push(field.into());
            }
            changed = true;
        };
        if let (true, Some(l)) = (p.exposure.ideal.is_empty(), l) {
            if let Some((ideal, tol)) = eive_light(l) {
                p.exposure.ideal = ideal;
                p.exposure.tolerated = tol;
                mark(p, "exposure.ideal", format!("eive:L={l}"));
                mark(p, "exposure.tolerated", format!("eive:L={l}"));
            }
        }
        if let (true, Some(m)) = (p.soil.moisture.is_empty(), m) {
            if let Some(v) = eive_moisture(m) {
                p.soil.moisture = v;
                mark(p, "soil.moisture", format!("eive:M={m}"));
            }
        }
        if let (true, Some(r)) = (p.soil.ph.is_empty(), r) {
            if let Some(v) = eive_ph(r) {
                p.soil.ph = v;
                mark(p, "soil.ph", format!("eive:R={r}"));
            }
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_only_to_empty_fields() {
        let idx = EiveIndex::from_json(r#"{"Lavandula angustifolia Mill.": [8.0, 3.0, null]}"#).unwrap();
        let mut p = Plant { latin_name: "Lavandula angustifolia 'Hidcote'".into(), ..Plant::default() };
        p.soil.moisture = vec![plotflora_core::Moisture::Normal];
        assert!(idx.apply(&mut p));
        assert_eq!(p.exposure.ideal, vec![plotflora_core::Sun::FullSun]);
        assert_eq!(p.soil.moisture, vec![plotflora_core::Moisture::Normal]);
        assert!(p.soil.ph.is_empty());
        assert!(p.meta.uncertain_fields.contains(&"exposure.ideal".to_string()));
    }

    #[test]
    fn embedded_index_parses() {
        assert!(EiveIndex::from_json(EMBEDDED).is_ok());
    }
}
