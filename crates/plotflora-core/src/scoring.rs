// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 ChatCarbo

//! Algorithme de recommandation (§7). Arithmétique entière en millièmes.

use crate::geometry::quantity;
use crate::model::{Environment, Plant, Zone};
use crate::search::normalize;
use crate::vocab::*;

/// Facteur appliqué à toute donnée inconnue.
pub const UNKNOWN: u32 = 600;
const FULL: u32 = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Criterion {
    Exposure,
    Hardiness,
    Water,
    Soil,
    Size,
    Uses,
    CareWind,
}

impl Criterion {
    pub const ALL: [Self; 7] = [Self::Exposure, Self::Hardiness, Self::Water, Self::Soil, Self::Size, Self::Uses, Self::CareWind];

    pub const fn weight(self) -> u32 {
        match self {
            Self::Exposure => 25,
            Self::Hardiness | Self::Size => 15,
            Self::Soil => 20,
            Self::Water | Self::Uses => 10,
            Self::CareWind => 5,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Exposure => "Exposition",
            Self::Hardiness => "Rusticité",
            Self::Water => "Sécheresse & arrosage",
            Self::Soil => "Sol & humidité",
            Self::Size => "Dimensions",
            Self::Uses => "Usages & feuillage",
            Self::CareWind => "Entretien & vent",
        }
    }
}

/// Niveau d'alerte ; l'ordre est celui de la gravité décroissante.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AlertLevel {
    Blocking,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alert {
    pub level: AlertLevel,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Verdict {
    VeryGood,
    Reserved,
    Poor,
    Avoid,
}

impl Verdict {
    pub const fn label(self) -> &'static str {
        match self {
            Self::VeryGood => "Très adapté",
            Self::Reserved => "Adapté avec réserves",
            Self::Poor => "Peu adapté",
            Self::Avoid => "Déconseillé",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CriterionScore {
    pub criterion: Criterion,
    pub points: u32,
    pub max: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Recommendation {
    pub plant_id: String,
    pub score: u32,
    pub verdict: Verdict,
    pub breakdown: Vec<CriterionScore>,
    pub reasons: Vec<String>,
    pub alerts: Vec<Alert>,
    pub quantity: Option<u32>,
}

impl Recommendation {
    pub fn is_blocked(&self) -> bool {
        self.alerts.iter().any(|a| a.level == AlertLevel::Blocking)
    }
}

const fn points(weight: u32, c: u32) -> u32 {
    (weight * c + 500) / 1000
}

/// Facteur selon un écart de rangs : ≤ 0 → 1000, 1 → 500, ≥ 2 → 0.
const fn gap_factor(gap: i32) -> u32 {
    if gap <= 0 {
        FULL
    } else if gap == 1 {
        500
    } else {
        0
    }
}

#[derive(Default)]
struct Ctx {
    alerts: Vec<Alert>,
}

impl Ctx {
    fn push(&mut self, level: AlertLevel, m: impl Into<String>) {
        self.alerts.push(Alert { level, message: m.into() });
    }
    fn block(&mut self, m: impl Into<String>) {
        self.push(AlertLevel::Blocking, m)
    }
    fn warn(&mut self, m: impl Into<String>) {
        self.push(AlertLevel::Warning, m)
    }
    fn info(&mut self, m: impl Into<String>) {
        self.push(AlertLevel::Info, m)
    }
    fn has_warning(&self) -> bool {
        self.alerts.iter().any(|a| a.level == AlertLevel::Warning)
    }
}

fn fmt_m(v: f64) -> String {
    format!("{v:.2}").trim_end_matches('0').trim_end_matches('.').replace('.', ",")
}

/// Évalue une plante pour une zone. `meters_per_unit` sert au calcul des quantités.
pub fn evaluate(env: &Environment, zone: &Zone, p: &Plant, meters_per_unit: Option<f64>) -> Recommendation {
    let mut x = Ctx::default();
    let mut reasons = Vec::new();

    // 1. Exposition
    let c1 = match zone.sun_exposure {
        None => {
            x.info("Exposition de la zone inconnue");
            UNKNOWN
        }
        Some(_) if p.exposure.ideal.is_empty() => {
            x.info("Exposition de la plante non renseignée");
            UNKNOWN
        }
        Some(s) if p.exposure.ideal.contains(&s) => {
            reasons.push(format!("{} : exposition idéale", s.label()));
            FULL
        }
        Some(s) if p.exposure.tolerated.contains(&s) => {
            x.warn(format!("{} seulement toléré", s.label()));
            600
        }
        Some(s) => {
            x.block(format!("Exposition inadaptée : {}", s.label().to_lowercase()));
            0
        }
    };

    // 2. Rusticité
    let c2 = match (env.min_winter_temp_c, p.climate.min_temp_c) {
        (Some(g), Some(t)) => {
            let margin = g - t;
            if margin >= 0 {
                reasons.push(format!("Rustique jusqu'à {t} °C (marge de {margin} °C)"));
                FULL
            } else if margin >= -3 {
                x.warn(format!("Protection hivernale nécessaire : rustique jusqu'à {t} °C, jardin à {g} °C"));
                400
            } else {
                x.block(format!("Gèlera : rustique jusqu'à {t} °C, jardin à {g} °C"));
                0
            }
        }
        (None, _) => {
            x.info("Température hivernale du jardin inconnue");
            UNKNOWN
        }
        (_, None) => {
            x.info("Rusticité de la plante inconnue");
            UNKNOWN
        }
    };

    // 3. Sécheresse & arrosage
    const DROUGHT: [[u32; 3]; 3] = [[1000, 500, 0], [1000, 1000, 500], [1000, 1000, 1000]];
    const BONUS: [u32; 3] = [0, 250, 500];
    let avail = env.watering_availability;
    let d = match (p.climate.drought_tolerance, env.summer_drought_risk) {
        (Some(t), Some(r)) => (DROUGHT[t.rank() as usize][r.rank() as usize] + avail.map_or(0, |a| BONUS[a.rank() as usize])).min(FULL),
        _ => {
            x.info("Tolérance à la sécheresse ou risque de sécheresse inconnu");
            UNKNOWN
        }
    };
    let w = match (p.care.water_need, avail) {
        (Some(n), Some(a)) => gap_factor(i32::from(n.rank()) - (i32::from(a.rank()) + 1)),
        _ => {
            x.info("Besoin en eau ou disponibilité d'arrosage inconnu");
            UNKNOWN
        }
    };
    let c3 = (600 * d + 400 * w) / 1000;
    if c3 < 500 {
        x.warn("Arrosage estival indispensable");
    } else if c3 == FULL {
        reasons.push("Besoins en eau compatibles".into());
    }

    // 4. Sol & humidité
    fn member<T: PartialEq>(v: Option<T>, list: &[T], what: &str, x: &mut Ctx) -> u32 {
        match v {
            Some(v) if !list.is_empty() => {
                if list.contains(&v) {
                    FULL
                } else {
                    0
                }
            }
            _ => {
                x.info(format!("{what} inconnu"));
                UNKNOWN
            }
        }
    }
    let subs = [
        member(env.soil_type, &p.soil.types, "Type de sol", &mut x),
        member(env.soil_drainage, &p.soil.drainage, "Drainage", &mut x),
        member(env.soil_ph, &p.soil.ph, "pH", &mut x),
        member(zone.moisture, &p.soil.moisture, "Humidité", &mut x),
    ];
    let c4 = subs.iter().sum::<u32>() / 4;
    if env.soil_ph == Some(Ph::Alkaline) && p.soil.ph == [Ph::Acid] {
        x.block("Sol calcaire incompatible (plante de terre acide)");
    } else if env.soil_ph.is_some_and(|ph| !p.soil.ph.is_empty() && !p.soil.ph.contains(&ph)) {
        x.warn("pH du sol hors des préférences de la plante");
    }
    if env.soil_drainage == Some(Drainage::Poor) && !p.soil.drainage.is_empty() && !p.soil.drainage.contains(&Drainage::Poor) {
        x.warn("Risque d'asphyxie des racines (sol mal drainé)");
    }
    if c4 == FULL {
        reasons.push("Sol et humidité adaptés".into());
    }

    // 5. Dimensions
    let ratio = |value: Option<f64>, limit: Option<f64>, what: &str, x: &mut Ctx| -> u32 {
        let Some(l) = limit.filter(|l| l.is_finite() && *l > 0.0) else { return FULL };
        let Some(v) = value.filter(|v| v.is_finite() && *v >= 0.0) else {
            x.info(format!("{what} adulte inconnue"));
            return UNKNOWN;
        };
        let r = ((v / l) * 1000.0).round() as u64;
        match r {
            0..=1000 => FULL,
            1001..=1250 => {
                x.warn(format!("{what} : taille régulière nécessaire ({} m pour {} m)", fmt_m(v), fmt_m(l)));
                600
            }
            1251..=1500 => {
                x.warn(format!("{what} : dépassement important ({} m pour {} m)", fmt_m(v), fmt_m(l)));
                200
            }
            _ => {
                x.block(format!("Beaucoup trop grand : {} {} m pour {} m", what.to_lowercase(), fmt_m(v), fmt_m(l)));
                200
            }
        }
    };
    let dims = &p.dimensions;
    let fh = ratio(dims.mature_height_m, zone.max_height_m, "Hauteur", &mut x);
    let fw = match zone.kind {
        ZoneKind::Point => ratio(dims.mature_spread_m.map(|s| s / 2.0), zone.available_radius_m, "Rayon", &mut x),
        ZoneKind::Line => ratio(dims.mature_spread_m, zone.max_width_m, "Largeur", &mut x),
        ZoneKind::Area => FULL,
    };
    let c5 = fh.min(fw);
    if c5 == FULL {
        match (dims.mature_height_m, dims.mature_spread_m) {
            (Some(h), Some(s)) => reasons.push(format!("Taille adulte ({} m × {} m) compatible", fmt_m(h), fmt_m(s))),
            _ => reasons.push("Aucune contrainte de taille dépassée".into()),
        }
    }

    // 6. Usages & feuillage
    let mut desired = zone.desired_uses.clone();
    desired.sort();
    desired.dedup();
    let u = if desired.is_empty() {
        FULL
    } else {
        let hit = desired.iter().filter(|d| p.characteristics.uses.contains(d)).count() as u32;
        let missing: Vec<&str> = desired.iter().filter(|d| !p.characteristics.uses.contains(d)).map(|d| d.label()).collect();
        if !missing.is_empty() {
            x.info(format!("Usage non couvert : {}", missing.join(", ")));
        }
        FULL * hit / desired.len() as u32
    };
    let f = match (zone.foliage_preference, p.characteristics.foliage) {
        (FoliagePref::Any, _) => FULL,
        (_, None) => {
            x.info("Feuillage de la plante inconnu");
            UNKNOWN
        }
        (FoliagePref::Evergreen, Some(Foliage::Evergreen)) | (FoliagePref::Deciduous, Some(Foliage::Deciduous)) => FULL,
        (_, Some(Foliage::SemiEvergreen)) => 500,
        (pref, Some(fol)) => {
            x.info(format!("Feuillage {} (souhaité : {})", fol.label().to_lowercase(), pref.label().to_lowercase()));
            0
        }
    };
    let c6 = (700 * u + 300 * f) / 1000;
    if c6 == FULL {
        reasons.push("Usages et feuillage conformes".into());
    }

    // 7. Entretien & vent
    let m = match (zone.max_maintenance, p.care.maintenance_level) {
        (MaxMaintenance::Any, _) => FULL,
        (_, None) => {
            x.info("Niveau d'entretien inconnu");
            UNKNOWN
        }
        (max, Some(lvl)) => {
            let f = gap_factor(i32::from(lvl.rank()) - (i32::from(max.rank()) - 1));
            if f < FULL {
                x.info(format!("Entretien {} (maximum souhaité : {})", lvl.label().to_lowercase(), max.label().to_lowercase()));
            }
            f
        }
    };
    let v = match (zone.wind_exposure, p.climate.wind_tolerance) {
        (Some(e), Some(t)) => {
            let f = gap_factor(i32::from(e.rank()) - i32::from(t.rank()));
            if f == 0 {
                x.warn("Trop exposé au vent");
            }
            f
        }
        _ => {
            x.info("Exposition au vent ou tolérance inconnue");
            UNKNOWN
        }
    };
    let c7 = (m + v) / 2;
    if c7 == FULL {
        reasons.push("Entretien et vent compatibles".into());
    }

    if !p.meta.verified {
        x.info("Fiche non vérifiée");
    }

    let factors = [c1, c2, c3, c4, c5, c6, c7];
    let breakdown: Vec<CriterionScore> = Criterion::ALL
        .iter()
        .zip(factors)
        .map(|(c, f)| CriterionScore { criterion: *c, points: points(c.weight(), f), max: c.weight() })
        .collect();
    let score = breakdown.iter().map(|b| b.points).sum();
    let blocked = x.alerts.iter().any(|a| a.level == AlertLevel::Blocking);
    let verdict = match score {
        _ if blocked => Verdict::Avoid,
        80.. if !x.has_warning() => Verdict::VeryGood,
        55.. => Verdict::Reserved,
        30.. => Verdict::Poor,
        _ => Verdict::Avoid,
    };
    let mut alerts = x.alerts;
    alerts.sort_by_key(|a| a.level);
    Recommendation { plant_id: p.id.clone(), score, verdict, breakdown, reasons, alerts, quantity: quantity(zone, p, meters_per_unit) }
}

/// Classe les plantes pour une zone (§7.6). Vide pour une zone `existing`.
pub fn recommend<'a>(
    env: &Environment,
    zone: &Zone,
    plants: impl IntoIterator<Item = &'a Plant>,
    meters_per_unit: Option<f64>,
) -> Vec<Recommendation> {
    if zone.status == Status::Existing {
        return Vec::new();
    }
    let mut v: Vec<(String, Recommendation)> =
        plants.into_iter().map(|p| (normalize(&p.display_name()), evaluate(env, zone, p, meters_per_unit))).collect();
    v.sort_by(|(na, a), (nb, b)| {
        a.is_blocked().cmp(&b.is_blocked()).then(b.score.cmp(&a.score)).then_with(|| na.cmp(nb)).then_with(|| a.plant_id.cmp(&b.plant_id))
    });
    v.into_iter().map(|(_, r)| r).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn env() -> Environment {
        Environment {
            min_winter_temp_c: Some(-12),
            summer_drought_risk: Some(Level::Medium),
            soil_type: Some(SoilType::Loamy),
            soil_drainage: Some(Drainage::Good),
            soil_ph: Some(Ph::Neutral),
            watering_availability: Some(Level::Medium),
        }
    }

    fn zone() -> Zone {
        Zone {
            sun_exposure: Some(Sun::FullSun),
            wind_exposure: Some(Wind::Sheltered),
            moisture: Some(Moisture::Normal),
            available_radius_m: Some(1.2),
            max_height_m: Some(1.8),
            desired_uses: vec![Use::Ornamental, Use::Pollinator],
            max_maintenance: MaxMaintenance::Low,
            ..Zone::default()
        }
    }

    fn lavender() -> Plant {
        let mut p = Plant { id: "lavandula_angustifolia".into(), common_name: "Lavande vraie".into(), ..Plant::default() };
        p.meta.verified = true;
        p.climate = Climate { min_temp_c: Some(-15), drought_tolerance: Some(Level::High), wind_tolerance: Some(Level::Medium) };
        p.exposure.ideal = vec![Sun::FullSun];
        p.soil = Soil {
            types: vec![SoilType::Sandy, SoilType::Loamy, SoilType::Chalky],
            drainage: vec![Drainage::Fast, Drainage::Good],
            ph: vec![Ph::Neutral, Ph::Alkaline],
            moisture: vec![Moisture::Dry, Moisture::Normal],
        };
        p.dimensions.mature_height_m = Some(0.8);
        p.dimensions.mature_spread_m = Some(0.9);
        p.characteristics.foliage = Some(Foliage::Evergreen);
        p.characteristics.uses = vec![Use::Ornamental, Use::Pollinator, Use::Aromatic];
        p.care.maintenance_level = Some(Maintenance::Low);
        p.care.water_need = Some(WaterNeed::VeryLow);
        p
    }

    fn maple() -> Plant {
        let mut p = Plant { id: "acer_palmatum".into(), common_name: "Érable du Japon".into(), ..Plant::default() };
        p.meta.verified = true;
        p.climate = Climate { min_temp_c: Some(-18), drought_tolerance: Some(Level::Low), wind_tolerance: Some(Level::Low) };
        p.exposure = Exposure { ideal: vec![Sun::PartialShade, Sun::LightShade], tolerated: vec![Sun::FullSun] };
        p.soil = Soil {
            types: vec![SoilType::Loamy, SoilType::Humus],
            drainage: vec![Drainage::Good, Drainage::Moderate],
            ph: vec![Ph::Acid, Ph::Neutral],
            moisture: vec![Moisture::Normal, Moisture::Moist],
        };
        p.dimensions.mature_height_m = Some(4.0);
        p.dimensions.mature_spread_m = Some(4.0);
        p.characteristics.foliage = Some(Foliage::Deciduous);
        p.characteristics.uses = vec![Use::Ornamental];
        p.care.maintenance_level = Some(Maintenance::Medium);
        p.care.water_need = Some(WaterNeed::Medium);
        p
    }

    fn pts(r: &Recommendation) -> Vec<u32> {
        r.breakdown.iter().map(|b| b.points).collect()
    }

    #[test]
    fn spec_example_lavender() {
        let r = evaluate(&env(), &zone(), &lavender(), None);
        assert_eq!(pts(&r), vec![25, 15, 10, 20, 15, 10, 5]);
        assert_eq!((r.score, r.verdict), (100, Verdict::VeryGood));
        assert!(r.alerts.is_empty());
        assert_eq!(r.quantity, Some(1));
    }

    #[test]
    fn spec_example_maple() {
        let r = evaluate(&env(), &zone(), &maple(), None);
        assert_eq!(pts(&r), vec![15, 15, 9, 20, 3, 7, 4]);
        assert_eq!((r.score, r.verdict), (73, Verdict::Avoid));
        assert!(r.is_blocked());
        assert_eq!(r.alerts[0].level, AlertLevel::Blocking);
    }

    #[test]
    fn everything_unknown_never_panics() {
        let r = evaluate(&Environment::default(), &Zone::default(), &Plant::default(), None);
        // 0,6 partout sauf dimensions (aucune limite) et usages/préférences « indifférent »
        assert_eq!(pts(&r), vec![15, 9, 6, 12, 15, 10, 4]);
        assert!(!r.is_blocked());
        assert_eq!(r.verdict, Verdict::Reserved);
    }

    #[test]
    fn size_thresholds_are_exact() {
        let mut z = zone();
        let mut p = lavender();
        z.available_radius_m = None;
        for (h, max, expect, blocked) in [(1.25, 1.0, 9, false), (1.5, 1.2, 9, false), (1.5, 1.0, 3, false), (1.51, 1.0, 3, true)] {
            p.dimensions.mature_height_m = Some(h);
            z.max_height_m = Some(max);
            let r = evaluate(&env(), &z, &p, None);
            assert_eq!((r.breakdown[4].points, r.is_blocked()), (expect, blocked), "h={h} max={max}");
        }
    }

    #[test]
    fn hardiness_margin() {
        let mut p = lavender();
        for (t, pts_, blocked) in [(-12, 15, false), (-9, 6, false), (-8, 0, true)] {
            p.climate.min_temp_c = Some(t);
            let r = evaluate(&env(), &zone(), &p, None);
            assert_eq!((r.breakdown[1].points, r.is_blocked()), (pts_, blocked), "t={t}");
        }
    }

    #[test]
    fn calcifuge_on_chalk_is_blocked() {
        let mut e = env();
        e.soil_ph = Some(Ph::Alkaline);
        let mut p = lavender();
        p.soil.ph = vec![Ph::Acid];
        assert!(evaluate(&e, &zone(), &p, None).is_blocked());
    }

    #[test]
    fn ordering_blocked_last_and_existing_empty() {
        let (l, m) = (lavender(), maple());
        let recs = recommend(&env(), &zone(), [&m, &l], None);
        assert_eq!(recs.iter().map(|r| r.plant_id.as_str()).collect::<Vec<_>>(), ["lavandula_angustifolia", "acer_palmatum"]);
        let mut z = zone();
        z.status = Status::Existing;
        assert!(recommend(&env(), &z, [&l], None).is_empty());
    }

    #[test]
    fn warning_caps_verdict() {
        let mut p = lavender();
        p.exposure = Exposure { ideal: vec![Sun::PartialShade], tolerated: vec![Sun::FullSun] };
        let r = evaluate(&env(), &zone(), &p, None);
        assert_eq!((r.score, r.verdict), (90, Verdict::Reserved));
    }
}
