// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 ChatCarbo

//! Conversions vers le vocabulaire contrôlé (§5.5), identifiants et extraction
//! de valeurs factuelles dans un texte encyclopédique.

use crate::search::fold;
use crate::vocab::{Moisture, Ph, Sun};

/// Identifiant `snake_case` ASCII.
pub fn slug(s: &str) -> String {
    let f = fold(s);
    let parts: Vec<&str> = f.split(|c: char| !c.is_ascii_alphanumeric()).filter(|w| !w.is_empty()).collect();
    parts.join("_")
}

/// Identifiant de plante : nom latin + cultivar.
pub fn plant_id(latin: &str, cultivar: Option<&str>) -> String {
    let base = slug(latin);
    match cultivar.map(slug).filter(|c| !c.is_empty()) {
        Some(c) => format!("{base}_{c}"),
        None => base,
    }
}

/// Rend `base` unique en ajoutant `_2`, `_3`…
pub fn unique_id(base: &str, exists: impl Fn(&str) -> bool) -> String {
    let base = if base.is_empty() { "plante" } else { base };
    if !exists(base) {
        return base.to_owned();
    }
    (2..).map(|n| format!("{base}_{n}")).find(|c| !exists(c)).expect("suite infinie")
}

/// Zone USDA (« 8 », « 8a », « 8b ») → température minimale (°C, arrondie vers le bas).
pub fn usda_min_temp_c(zone: &str) -> Option<i32> {
    let z = zone.trim().to_ascii_lowercase();
    let (num, half) = match z.strip_suffix('a') {
        Some(n) => (n, 0.0),
        None => match z.strip_suffix('b') {
            Some(n) => (n, 5.0),
            None => (z.as_str(), 0.0),
        },
    };
    let n: i32 = num.parse().ok().filter(|n| (1..=13).contains(n))?;
    let f = -60.0 + 10.0 * f64::from(n - 1) + half;
    Some(((f - 32.0) * 5.0 / 9.0).floor() as i32)
}

/// Lumière EIVE (1–9) → (idéal, toléré).
pub fn eive_light(l: f64) -> Option<(Vec<Sun>, Vec<Sun>)> {
    use Sun::*;
    if !l.is_finite() {
        return None;
    }
    Some(match l.round().clamp(1.0, 9.0) as u8 {
        9 => (vec![FullSun], vec![]),
        8 => (vec![FullSun], vec![PartialShade]),
        7 => (vec![FullSun, PartialShade], vec![LightShade]),
        6 => (vec![PartialShade], vec![FullSun, LightShade]),
        5 => (vec![PartialShade, LightShade], vec![FullSun]),
        4 => (vec![LightShade], vec![PartialShade, DenseShade]),
        3 => (vec![LightShade, DenseShade], vec![PartialShade]),
        _ => (vec![DenseShade], vec![LightShade]),
    })
}

/// Humidité EIVE (1–12).
pub fn eive_moisture(m: f64) -> Option<Vec<Moisture>> {
    use Moisture::*;
    m.is_finite().then(|| match m.round().clamp(1.0, 12.0) as u8 {
        1..=3 => vec![Dry],
        4..=5 => vec![Dry, Normal],
        6 => vec![Normal],
        7 => vec![Normal, Moist],
        8..=9 => vec![Moist],
        _ => vec![Moist, Wet],
    })
}

/// Réaction du sol EIVE (1–9).
pub fn eive_ph(r: f64) -> Option<Vec<Ph>> {
    use Ph::*;
    r.is_finite().then(|| match r.round().clamp(1.0, 9.0) as u8 {
        1..=3 => vec![Acid],
        4..=5 => vec![Acid, Neutral],
        6 => vec![Neutral],
        7 => vec![Neutral, Alkaline],
        _ => vec![Alkaline, Neutral],
    })
}

/// Valeurs factuelles extraites d'un texte (jamais le texte lui-même).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Facts {
    pub min_temp_c: Option<i32>,
    pub height_m: Option<f64>,
    pub spread_m: Option<f64>,
    /// `Some(false)` si le texte signale une toxicité.
    pub non_toxic: Option<bool>,
}

/// Nombre (virgule ou point décimal) se terminant à l'octet `end` exclu.
fn number_before(s: &str, end: usize) -> Option<(f64, usize)> {
    let b = s.as_bytes();
    let mut i = end;
    while i > 0 && b[i - 1] == b' ' {
        i -= 1;
    }
    let stop = i;
    while i > 0 && (b[i - 1].is_ascii_digit() || ((b[i - 1] == b',' || b[i - 1] == b'.') && i > 1 && b[i - 2].is_ascii_digit())) {
        i -= 1;
    }
    if i == stop {
        return None;
    }
    let v: f64 = s[i..stop].replace(',', ".").trim_end_matches('.').parse().ok()?;
    Some((v, i))
}

/// Températures « -15 °C » (signe facultatif) avec leur position.
fn temperatures(s: &str) -> Vec<(usize, i32)> {
    let mut out = Vec::new();
    for (pos, _) in s.match_indices('°') {
        let after = s[pos + '°'.len_utf8()..].trim_start();
        if !after.starts_with('c') {
            continue;
        }
        if let Some((v, start)) = number_before(s, pos) {
            let neg = s[..start].trim_end().ends_with('-');
            out.push((start, if neg { -(v.round() as i32) } else { v.round() as i32 }));
        }
    }
    out
}

/// Longueurs « 1,5 m » / « 60 cm » dans `s[from..to]`, converties en mètres.
fn lengths(s: &str, from: usize, to: usize) -> Vec<f64> {
    let w = &s[from..to];
    let mut out = Vec::new();
    for (unit, factor) in [("cm", 0.01), ("m", 1.0)] {
        for (pos, _) in w.match_indices(unit) {
            let next = w[pos + unit.len()..].chars().next();
            let prev = w[..pos].chars().next_back();
            if next.is_some_and(char::is_alphanumeric) || (unit == "m" && prev == Some('c')) {
                continue;
            }
            if let Some((v, _)) = number_before(w, pos) {
                out.push(v * factor);
            }
        }
    }
    out
}

fn floor_char(s: &str, i: usize) -> usize {
    let mut i = i.min(s.len());
    while !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

fn ceil_char(s: &str, i: usize) -> usize {
    let mut i = i.min(s.len());
    while !s.is_char_boundary(i) {
        i += 1;
    }
    i
}

fn dimension(s: &str, keys: &[&str]) -> Option<f64> {
    let mut best: Option<f64> = None;
    for k in keys {
        for (pos, _) in s.match_indices(k) {
            let before = lengths(s, floor_char(s, pos.saturating_sub(18)), pos);
            let end = pos + k.len();
            let after = lengths(s, end, ceil_char(s, end + 28));
            for v in before.into_iter().chain(after).filter(|v| (0.02..=60.0).contains(v)) {
                best = Some(best.map_or(v, |b| b.max(v)));
            }
        }
    }
    best
}

/// Extrait rusticité, hauteur, envergure et toxicité d'un texte libre (§5.4).
pub fn extract_facts(text: &str) -> Facts {
    let s = fold(text);
    let mut f = Facts::default();

    // Rusticité : températures négatives proches d'un mot-clé, puis zones USDA.
    let keys = ["rusti", "resist", "supporte", "tolere", "gel", "froid"];
    let anchors: Vec<usize> = keys.iter().flat_map(|k| s.match_indices(k).map(|(p, _)| p)).collect();
    f.min_temp_c =
        temperatures(&s).into_iter().filter(|(p, t)| *t < 0 && anchors.iter().any(|a| *p >= *a && *p - *a <= 80)).map(|(_, t)| t).min();
    if f.min_temp_c.is_none() {
        f.min_temp_c = s
            .match_indices("zone")
            .filter_map(|(p, _)| {
                let rest = s[p + 4..].trim_start();
                let rest = rest.strip_prefix("usda").unwrap_or(rest).trim_start();
                let code: String = rest.chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
                usda_min_temp_c(&code)
            })
            .min();
    }

    f.height_m = dimension(&s, &["de haut", "hauteur", "atteint", "mesure"]);
    f.spread_m = dimension(&s, &["de large", "largeur", "envergure", "etalement", "diametre"]);

    let negated = ["non toxique", "pas toxique", "aucune toxicite"].iter().any(|n| s.contains(n));
    if !negated && (s.contains("toxique") || s.contains("toxicite")) {
        f.non_toxic = Some(false);
    }
    f
}

/// Date ISO (AAAA-MM-JJ) à partir d'un nombre de secondes depuis l'époque Unix (UTC).
pub fn iso_date(unix_secs: u64) -> String {
    // Algorithme « days from civil » de H. Hinnant.
    let z = (unix_secs / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids() {
        assert_eq!(plant_id("Lavandula angustifolia", Some("Hidcote Blue")), "lavandula_angustifolia_hidcote_blue");
        assert_eq!(plant_id("Rubus × fruticosus", None), "rubus_fruticosus");
        assert_eq!(unique_id("a", |s| s == "a" || s == "a_2"), "a_3");
    }

    #[test]
    fn usda() {
        assert_eq!(usda_min_temp_c("8"), Some(-13));
        assert_eq!(usda_min_temp_c("8b"), Some(-10));
        assert_eq!(usda_min_temp_c("5"), Some(-29));
        assert_eq!(usda_min_temp_c("14"), None);
        assert_eq!(usda_min_temp_c("x"), None);
    }

    #[test]
    fn eive() {
        assert_eq!(eive_light(7.4).unwrap().0, vec![Sun::FullSun, Sun::PartialShade]);
        assert_eq!(eive_light(f64::NAN), None);
        assert_eq!(eive_moisture(2.0), Some(vec![Moisture::Dry]));
        assert_eq!(eive_ph(8.6), Some(vec![Ph::Alkaline, Ph::Neutral]));
    }

    #[test]
    fn facts() {
        let t = "Arbuste rustique jusqu'à −15 °C. Il atteint 1,5 m de haut et 80 cm de large. \
                 Toutes les parties sont toxiques. Floraison à 20 °C.";
        let f = extract_facts(t);
        assert_eq!(f.min_temp_c, Some(-15));
        assert_eq!(f.height_m, Some(1.5));
        assert_eq!(f.spread_m, Some(0.8));
        assert_eq!(f.non_toxic, Some(false));
        assert_eq!(extract_facts("Convient en zone USDA 7b.").min_temp_c, Some(-15));
        assert_eq!(extract_facts("Plante non toxique.").non_toxic, None);
        assert_eq!(extract_facts("").min_temp_c, None);
    }

    #[test]
    fn dates() {
        assert_eq!(iso_date(0), "1970-01-01");
        assert_eq!(iso_date(1_795_219_200), "2026-11-21");
    }
}
