// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 ChatCarbo

//! Modèle de données (§6). Convention : `None` / `null` = inconnu.

use crate::vocab::*;
use std::collections::BTreeMap;

/// Point en unités du plan (pixels de l'image ou mètres en mode dessin).
pub type Pt = [f64; 2];

/// Version de schéma écrite par cette version du logiciel.
pub const SCHEMA_VERSION: &str = "1.2.0";

macro_rules! data {
    ($($item:item)+) => {$(
        #[derive(Debug, Clone, PartialEq, Default)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
        $item
    )+};
}

data! {
    pub struct Photo {
        /// Chemin relatif au dossier de données de l'application (ou absolu).
        pub file: String,
        pub source_url: Option<String>,
        pub author: Option<String>,
        pub license: Option<String>,
        pub license_url: Option<String>,
    }

    pub struct Climate {
        pub min_temp_c: Option<i32>,
        pub drought_tolerance: Option<Level>,
        pub wind_tolerance: Option<Level>,
    }

    pub struct Exposure {
        pub ideal: Vec<Sun>,
        pub tolerated: Vec<Sun>,
    }

    pub struct Soil {
        pub types: Vec<SoilType>,
        pub drainage: Vec<Drainage>,
        pub ph: Vec<Ph>,
        pub moisture: Vec<Moisture>,
    }

    pub struct Dimensions {
        pub mature_height_m: Option<f64>,
        pub mature_spread_m: Option<f64>,
        pub planting_spacing_m: Option<f64>,
        pub growth_rate: Option<GrowthRate>,
        pub root_system: Option<RootSystem>,
    }

    pub struct Characteristics {
        pub foliage: Option<Foliage>,
        pub uses: Vec<Use>,
        pub flowering_months: Vec<u8>,
        pub interest_seasons: Vec<Season>,
        pub flower_color: Option<String>,
        pub autumn_color: Option<String>,
        /// Ambiances de jardin auxquelles la plante convient.
        pub ambiances: Vec<Ambiance>,
    }

    pub struct Care {
        pub maintenance_level: Option<Maintenance>,
        pub water_need: Option<WaterNeed>,
        pub pruning_months: Vec<u8>,
        pub non_toxic_pets: Option<bool>,
        pub has_thorns: Option<bool>,
    }

    pub struct Links {
        pub wikidata: Option<String>,
        pub wikipedia_fr: Option<String>,
        /// Nom du fichier photo sur Wikimedia Commons, téléchargé seulement à l'ajout au jardin.
        pub commons_image: Option<String>,
    }

    pub struct Meta {
        pub source: DataSource,
        pub verified: bool,
        pub uncertain_fields: Vec<String>,
        pub field_sources: BTreeMap<String, String>,
        pub updated_at: String,
    }

    pub struct Plant {
        pub id: String,
        pub common_name: String,
        pub latin_name: String,
        pub cultivar: Option<String>,
        pub category: Option<Category>,
        pub photo: Option<Photo>,
        pub climate: Climate,
        pub exposure: Exposure,
        pub soil: Soil,
        pub dimensions: Dimensions,
        pub characteristics: Characteristics,
        pub care: Care,
        pub notes: String,
        pub tags: Vec<String>,
        pub links: Links,
        pub meta: Meta,
    }

    pub struct Catalog {
        pub schema_version: String,
        pub plants: Vec<Plant>,
    }

    pub struct Environment {
        pub min_winter_temp_c: Option<i32>,
        pub summer_drought_risk: Option<Level>,
        pub soil_type: Option<SoilType>,
        pub soil_drainage: Option<Drainage>,
        pub soil_ph: Option<Ph>,
        pub watering_availability: Option<Level>,
        /// Ambiance recherchée pour le jardin (`None` = aucune).
        pub ambiance: Option<Ambiance>,
    }

    pub struct Zone {
        pub id: String,
        pub kind: ZoneKind,
        pub label: String,
        pub points: Vec<Pt>,
        pub status: Status,
        pub sun_exposure: Option<Sun>,
        pub wind_exposure: Option<Wind>,
        pub moisture: Option<Moisture>,
        pub max_height_m: Option<f64>,
        pub available_radius_m: Option<f64>,
        pub max_width_m: Option<f64>,
        pub desired_uses: Vec<Use>,
        pub foliage_preference: FoliagePref,
        pub max_maintenance: MaxMaintenance,
        pub assigned_plant_id: Option<String>,
        pub quantity_override: Option<u32>,
        pub custom_photo_path: Option<String>,
        /// Ambiance propre à la zone (`None` = celle du jardin).
        pub ambiance: Option<Ambiance>,
        pub notes: String,
    }

    pub struct Scale {
        pub p1: Pt,
        pub p2: Pt,
        pub distance_m: f64,
    }

    pub struct Obstacle {
        pub kind: ObstacleKind,
        pub points: Vec<Pt>,
        pub center: Option<Pt>,
        pub radius_m: Option<f64>,
        pub height_m: Option<f64>,
    }

    /// Secteur de sol : polygone dont les valeurs renseignées remplacent celles du jardin
    /// pour les zones qu'il contient (`None` = valeur du jardin).
    pub struct SoilSector {
        pub id: String,
        pub label: String,
        pub points: Vec<Pt>,
        pub soil_type: Option<SoilType>,
        pub soil_drainage: Option<Drainage>,
        pub soil_ph: Option<Ph>,
    }

    pub struct MapDef {
        pub source: MapSource,
        pub image_path: Option<String>,
        pub attribution: String,
        pub north_angle_deg: f64,
        pub scale: Option<Scale>,
        pub boundary: Option<Vec<Pt>>,
        pub obstacles: Vec<Obstacle>,
    }
}

impl Default for Project {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION.into(),
            project_id: String::new(),
            name: String::new(),
            map: MapDef::default(),
            environment: Environment::default(),
            next_zone_seq: 1,
            zones: Vec::new(),
            soil_sectors: Vec::new(),
            plants_snapshot: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct Project {
    pub schema_version: String,
    pub project_id: String,
    pub name: String,
    pub map: MapDef,
    pub environment: Environment,
    pub next_zone_seq: u32,
    pub zones: Vec<Zone>,
    /// Secteurs de sol ; en cas de chevauchement, le dernier tracé l'emporte.
    pub soil_sectors: Vec<SoilSector>,
    /// Copie des fiches assignées, pour que le projet reste lisible sans le catalogue.
    pub plants_snapshot: BTreeMap<String, Plant>,
}

impl MapDef {
    /// Mètres par unité du plan, si l'échelle est connue.
    pub fn meters_per_unit(&self) -> Option<f64> {
        match self.source {
            MapSource::Drawing => Some(1.0),
            MapSource::Image => self.scale.as_ref().and_then(|s| {
                let d = crate::geometry::dist(s.p1, s.p2);
                let m = s.distance_m / d;
                (d > 0.0 && s.distance_m > 0.0 && m.is_finite()).then_some(m)
            }),
        }
    }

    /// Fixe l'angle du Nord, normalisé dans [0, 360).
    pub fn set_north(&mut self, deg: f64) {
        self.north_angle_deg = if deg.is_finite() { deg.rem_euclid(360.0) } else { 0.0 };
        if self.north_angle_deg >= 360.0 {
            self.north_angle_deg = 0.0;
        }
    }
}

impl Project {
    pub fn new(name: &str, source: MapSource) -> Self {
        let mut p = Self { name: name.to_owned(), project_id: crate::convert::slug(name), ..Self::default() };
        p.map.source = source;
        p
    }

    pub fn zone(&self, id: &str) -> Option<&Zone> {
        self.zones.iter().find(|z| z.id == id)
    }

    pub fn zone_mut(&mut self, id: &str) -> Option<&mut Zone> {
        self.zones.iter_mut().find(|z| z.id == id)
    }

    /// Ajoute une zone avec un identifiant jamais réutilisé.
    pub fn add_zone(&mut self, kind: ZoneKind, points: Vec<Pt>) -> &mut Zone {
        let mut seq = self.next_zone_seq.max(1);
        while self.zones.iter().any(|z| z.id == format!("z_{seq}")) {
            seq += 1;
        }
        self.next_zone_seq = seq + 1;
        let label = format!("{} {seq}", kind.label());
        self.zones.push(Zone { id: format!("z_{seq}"), kind, label, points, ..Zone::default() });
        self.zones.last_mut().expect("zone ajoutée")
    }

    pub fn remove_zone(&mut self, id: &str) -> Option<Zone> {
        let i = self.zones.iter().position(|z| z.id == id)?;
        Some(self.zones.remove(i))
    }

    /// Assigne une plante à une zone et en garde une copie dans le projet.
    pub fn assign(&mut self, zone_id: &str, plant: &Plant) -> bool {
        let Some(z) = self.zone_mut(zone_id) else { return false };
        z.assigned_plant_id = Some(plant.id.clone());
        self.plants_snapshot.insert(plant.id.clone(), plant.clone());
        true
    }

    /// Ajoute un secteur de sol (identifiant `s_N` libre).
    pub fn add_sector(&mut self, points: Vec<Pt>) -> &mut SoilSector {
        let n =
            (self.soil_sectors.len() + 1..).find(|n| !self.soil_sectors.iter().any(|s| s.id == format!("s_{n}"))).expect("suite infinie");
        self.soil_sectors.push(SoilSector { id: format!("s_{n}"), label: format!("Secteur {n}"), points, ..SoilSector::default() });
        self.soil_sectors.last_mut().expect("secteur ajouté")
    }

    /// Secteur de sol contenant la zone (point d'ancrage de la zone), le plus récent d'abord.
    pub fn sector_for(&self, zone: &Zone) -> Option<&SoilSector> {
        let a = crate::geometry::label_anchor(zone)?;
        self.soil_sectors.iter().rev().find(|s| s.points.len() >= 3 && crate::geometry::point_in_polygon(a, &s.points))
    }

    /// Contexte effectif d'une zone : celui du jardin, corrigé par son secteur de sol.
    pub fn environment_for(&self, zone: &Zone) -> Environment {
        self.environment_with(self.sector_for(zone))
    }

    /// Contexte du jardin corrigé par un secteur de sol donné (ou aucun).
    pub fn environment_with(&self, sector: Option<&SoilSector>) -> Environment {
        let mut e = self.environment.clone();
        if let Some(s) = sector {
            e.soil_type = s.soil_type.or(e.soil_type);
            e.soil_drainage = s.soil_drainage.or(e.soil_drainage);
            e.soil_ph = s.soil_ph.or(e.soil_ph);
        }
        e
    }

    /// Supprime les copies de fiches qui ne sont plus assignées.
    pub fn prune_snapshot(&mut self) {
        let used: std::collections::HashSet<&str> = self.zones.iter().filter_map(|z| z.assigned_plant_id.as_deref()).collect();
        self.plants_snapshot.retain(|id, _| used.contains(id.as_str()));
    }
}

impl Catalog {
    pub fn get(&self, id: &str) -> Option<&Plant> {
        self.plants.iter().find(|p| p.id == id)
    }

    /// Insère ou remplace une fiche (même `id`).
    pub fn upsert(&mut self, plant: Plant) {
        match self.plants.iter_mut().find(|p| p.id == plant.id) {
            Some(slot) => *slot = plant,
            None => self.plants.push(plant),
        }
    }
}

impl Plant {
    /// Nom affiché : « Nom commun 'Cultivar' ».
    pub fn display_name(&self) -> String {
        let mut chars = self.common_name.trim().chars();
        let name: String = chars.next().map(|c| c.to_uppercase().chain(chars).collect()).unwrap_or_default();
        match &self.cultivar {
            Some(c) if !c.is_empty() => format!("{name} '{c}'"),
            _ => name,
        }
    }

    /// Complète uniquement les champs vides à partir de `other` (§5.8).
    /// Retourne les chemins des champs complétés.
    pub fn fill_missing_from(&mut self, o: &Plant) -> Vec<&'static str> {
        fn opt<T: Clone>(a: &mut Option<T>, b: &Option<T>, n: &'static str, out: &mut Vec<&'static str>) {
            if a.is_none() && b.is_some() {
                a.clone_from(b);
                out.push(n);
            }
        }
        fn list<T: Clone>(a: &mut Vec<T>, b: &[T], n: &'static str, out: &mut Vec<&'static str>) {
            if a.is_empty() && !b.is_empty() {
                *a = b.to_vec();
                out.push(n);
            }
        }
        fn text(a: &mut String, b: &str, n: &'static str, out: &mut Vec<&'static str>) {
            if a.trim().is_empty() && !b.trim().is_empty() {
                *a = b.to_owned();
                out.push(n);
            }
        }
        let mut o_ = Vec::new();
        let out = &mut o_;
        text(&mut self.common_name, &o.common_name, "common_name", out);
        text(&mut self.latin_name, &o.latin_name, "latin_name", out);
        opt(&mut self.cultivar, &o.cultivar, "cultivar", out);
        opt(&mut self.category, &o.category, "category", out);
        opt(&mut self.photo, &o.photo, "photo", out);
        opt(&mut self.climate.min_temp_c, &o.climate.min_temp_c, "climate.min_temp_c", out);
        opt(&mut self.climate.drought_tolerance, &o.climate.drought_tolerance, "climate.drought_tolerance", out);
        opt(&mut self.climate.wind_tolerance, &o.climate.wind_tolerance, "climate.wind_tolerance", out);
        list(&mut self.exposure.ideal, &o.exposure.ideal, "exposure.ideal", out);
        list(&mut self.exposure.tolerated, &o.exposure.tolerated, "exposure.tolerated", out);
        list(&mut self.soil.types, &o.soil.types, "soil.types", out);
        list(&mut self.soil.drainage, &o.soil.drainage, "soil.drainage", out);
        list(&mut self.soil.ph, &o.soil.ph, "soil.ph", out);
        list(&mut self.soil.moisture, &o.soil.moisture, "soil.moisture", out);
        let (d, od) = (&mut self.dimensions, &o.dimensions);
        opt(&mut d.mature_height_m, &od.mature_height_m, "dimensions.mature_height_m", out);
        opt(&mut d.mature_spread_m, &od.mature_spread_m, "dimensions.mature_spread_m", out);
        opt(&mut d.planting_spacing_m, &od.planting_spacing_m, "dimensions.planting_spacing_m", out);
        opt(&mut d.growth_rate, &od.growth_rate, "dimensions.growth_rate", out);
        opt(&mut d.root_system, &od.root_system, "dimensions.root_system", out);
        let (c, oc) = (&mut self.characteristics, &o.characteristics);
        opt(&mut c.foliage, &oc.foliage, "characteristics.foliage", out);
        list(&mut c.uses, &oc.uses, "characteristics.uses", out);
        list(&mut c.flowering_months, &oc.flowering_months, "characteristics.flowering_months", out);
        list(&mut c.interest_seasons, &oc.interest_seasons, "characteristics.interest_seasons", out);
        opt(&mut c.flower_color, &oc.flower_color, "characteristics.flower_color", out);
        opt(&mut c.autumn_color, &oc.autumn_color, "characteristics.autumn_color", out);
        let (k, ok) = (&mut self.care, &o.care);
        opt(&mut k.maintenance_level, &ok.maintenance_level, "care.maintenance_level", out);
        opt(&mut k.water_need, &ok.water_need, "care.water_need", out);
        list(&mut k.pruning_months, &ok.pruning_months, "care.pruning_months", out);
        opt(&mut k.non_toxic_pets, &ok.non_toxic_pets, "care.non_toxic_pets", out);
        opt(&mut k.has_thorns, &ok.has_thorns, "care.has_thorns", out);
        text(&mut self.notes, &o.notes, "notes", out);
        list(&mut self.tags, &o.tags, "tags", out);
        opt(&mut self.links.wikidata, &o.links.wikidata, "links.wikidata", out);
        opt(&mut self.links.wikipedia_fr, &o.links.wikipedia_fr, "links.wikipedia_fr", out);
        opt(&mut self.links.commons_image, &o.links.commons_image, "links.commons_image", out);
        list(&mut self.characteristics.ambiances, &o.characteristics.ambiances, "characteristics.ambiances", out);
        for f in out.iter() {
            if let Some(src) = o.meta.field_sources.get(*f) {
                self.meta.field_sources.insert((*f).to_owned(), src.clone());
            }
            if o.meta.uncertain_fields.iter().any(|u| u == f) && !self.meta.uncertain_fields.iter().any(|u| u == f) {
                self.meta.uncertain_fields.push((*f).to_owned());
            }
        }
        o_
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_name_is_capitalized() {
        let p = Plant { common_name: " érable du Japon".into(), cultivar: Some("Osakazuki".into()), ..Plant::default() };
        assert_eq!(p.display_name(), "Érable du Japon 'Osakazuki'");
        assert_eq!(Plant::default().display_name(), "");
    }

    #[test]
    fn zone_ids_never_reused() {
        let mut p = Project::new("Mon jardin", MapSource::Image);
        assert_eq!(p.project_id, "mon_jardin");
        let a = p.add_zone(ZoneKind::Point, vec![[0.0, 0.0]]).id.clone();
        let b = p.add_zone(ZoneKind::Line, vec![]).id.clone();
        p.remove_zone(&b);
        let c = p.add_zone(ZoneKind::Area, vec![]).id.clone();
        assert_eq!((a.as_str(), c.as_str()), ("z_1", "z_3"));
        p.next_zone_seq = 1; // fichier incohérent
        assert_eq!(p.add_zone(ZoneKind::Point, vec![]).id, "z_2");
    }

    #[test]
    fn north_and_scale() {
        let mut m = MapDef::default();
        m.set_north(-90.0);
        assert_eq!(m.north_angle_deg, 270.0);
        m.set_north(f64::NAN);
        assert_eq!(m.north_angle_deg, 0.0);
        assert_eq!(m.meters_per_unit(), None);
        m.scale = Some(Scale { p1: [0.0, 0.0], p2: [30.0, 40.0], distance_m: 10.0 });
        assert_eq!(m.meters_per_unit(), Some(0.2));
        m.scale = Some(Scale { p1: [1.0, 1.0], p2: [1.0, 1.0], distance_m: 10.0 });
        assert_eq!(m.meters_per_unit(), None);
    }

    #[test]
    fn fill_missing_never_overwrites() {
        let mut a = Plant { common_name: "A".into(), ..Plant::default() };
        a.climate.min_temp_c = Some(-10);
        let mut b = Plant { common_name: "B".into(), ..Plant::default() };
        b.climate.min_temp_c = Some(-20);
        b.care.water_need = Some(WaterNeed::Low);
        b.meta.uncertain_fields.push("care.water_need".into());
        let filled = a.fill_missing_from(&b);
        assert_eq!(filled, vec!["care.water_need"]);
        assert_eq!((a.common_name.as_str(), a.climate.min_temp_c), ("A", Some(-10)));
        assert_eq!(a.meta.uncertain_fields, vec!["care.water_need".to_string()]);
    }

    #[test]
    fn soil_sectors_override_garden() {
        let mut p = Project::default();
        p.environment.soil_type = Some(SoilType::Loamy);
        p.environment.soil_ph = Some(Ph::Neutral);
        let inside = p.add_zone(ZoneKind::Point, vec![[5.0, 5.0]]).clone();
        let outside = p.add_zone(ZoneKind::Point, vec![[50.0, 5.0]]).clone();
        let s = p.add_sector(vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]]);
        s.soil_type = Some(SoilType::Clay);
        assert_eq!(s.id, "s_1");
        let e = p.environment_for(&inside);
        assert_eq!((e.soil_type, e.soil_ph), (Some(SoilType::Clay), Some(Ph::Neutral)));
        assert_eq!(p.environment_for(&outside).soil_type, Some(SoilType::Loamy));
        // Le secteur tracé en dernier l'emporte.
        p.add_sector(vec![[4.0, 4.0], [6.0, 4.0], [6.0, 6.0], [4.0, 6.0]]).soil_type = Some(SoilType::Sandy);
        assert_eq!(p.environment_for(&inside).soil_type, Some(SoilType::Sandy));
    }

    #[test]
    fn snapshot_is_pruned() {
        let mut p = Project::default();
        let id = p.add_zone(ZoneKind::Point, vec![]).id.clone();
        let plant = Plant { id: "x".into(), ..Plant::default() };
        assert!(p.assign(&id, &plant));
        p.prune_snapshot();
        assert!(p.plants_snapshot.contains_key("x"));
        p.zone_mut(&id).unwrap().assigned_plant_id = None;
        p.prune_snapshot();
        assert!(p.plants_snapshot.is_empty());
    }
}
