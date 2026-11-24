// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 ChatCarbo

//! Outil développeur (§5.2) : construit `eive_index.json` à partir du classeur EIVE
//! et complète `plants_catalog.seed.json` à partir d'une liste d'espèces.
//!
//! ```text
//! plotflora-catalog-builder --species assets/seed_species.txt [--eive EIVE.xlsx]
//!     [--sheet NOM] [--taxon-col Taxon] [--l-col L] [--m-col M] [--r-col R]
//!     [--out assets] [--offline]
//! ```
#![forbid(unsafe_code)]

use calamine::{open_workbook_auto, Data, Reader};
use plotflora_core::convert::plant_id;
use plotflora_core::search::normalize;
use plotflora_core::{Ambiance, Catalog, DataSource, Plant, SCHEMA_VERSION};
use plotflora_data::cache::LookupCache;
use plotflora_data::eive::EiveIndex;
use plotflora_data::online::{Candidate, Online};
use plotflora_data::{store, today};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
struct Args {
    species: Option<PathBuf>,
    eive: Option<PathBuf>,
    sheet: Option<String>,
    cols: [String; 4],
    out: PathBuf,
    offline: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut a = Args {
        species: None,
        eive: None,
        sheet: None,
        cols: ["taxon".into(), "light".into(), "moisture".into(), "reaction".into()],
        out: PathBuf::from("assets"),
        offline: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        let mut val = || it.next().ok_or_else(|| format!("valeur manquante après {k}"));
        match k.as_str() {
            "--species" => a.species = Some(val()?.into()),
            "--eive" => a.eive = Some(val()?.into()),
            "--sheet" => a.sheet = Some(val()?),
            "--taxon-col" => a.cols[0] = val()?,
            "--l-col" => a.cols[1] = val()?,
            "--m-col" => a.cols[2] = val()?,
            "--r-col" => a.cols[3] = val()?,
            "--out" => a.out = val()?.into(),
            "--offline" => a.offline = true,
            "-h" | "--help" => return Err("usage : voir l'en-tête de src/main.rs".into()),
            other => return Err(format!("option inconnue : {other}")),
        }
    }
    Ok(a)
}

fn cell_f64(d: &Data) -> Option<f64> {
    match d {
        Data::Float(f) => Some(*f),
        Data::Int(i) => Some(*i as f64),
        Data::String(s) => s.trim().replace(',', ".").parse().ok(),
        _ => None,
    }
    .filter(|v: &f64| v.is_finite())
}

fn find_col(header: &[String], want: &str) -> Option<usize> {
    let w = want.trim().to_lowercase();
    header
        .iter()
        .position(|h| *h == w)
        .or_else(|| header.iter().position(|h| h.starts_with(&w)))
        .or_else(|| header.iter().position(|h| h.contains(&w)))
}

fn cells(row: &[Data]) -> Vec<String> {
    row.iter().map(|c| c.to_string().trim().to_lowercase()).collect()
}

/// En-têtes de la ligne `r`, combinés avec ceux de la ligne précédente (en-têtes sur deux
/// lignes, cellules fusionnées propagées vers la droite) : « light » + « average » → « light average ».
fn header_at(rows: &[&[Data]], r: usize) -> Vec<String> {
    let cur = cells(rows[r]);
    let Some(prev_row) = r.checked_sub(1).map(|p| cells(rows[p])) else { return cur };
    let mut fill = String::new();
    cur.iter()
        .enumerate()
        .map(|(i, c)| {
            if let Some(p) = prev_row.get(i).filter(|p| !p.is_empty()) {
                fill.clone_from(p);
            }
            format!("{fill} {c}").trim().to_owned()
        })
        .collect()
}

/// Lit le classeur EIVE et renvoie nom latin → [L, M, R].
/// Parcourt les onglets (ou celui imposé par `--sheet`) et leurs 30 premières lignes
/// pour trouver la ligne d'en-têtes contenant les quatre colonnes demandées.
fn read_eive(path: &Path, sheet: Option<&str>, cols: &[String; 4]) -> Result<BTreeMap<String, [Option<f64>; 3]>, String> {
    let mut wb = open_workbook_auto(path).map_err(|e| format!("{} : {e}", path.display()))?;
    let sheets = match sheet {
        Some(s) => vec![s.to_owned()],
        None => wb.sheet_names(),
    };
    let mut candidates = Vec::new();
    for name in &sheets {
        let range = wb.worksheet_range(name).map_err(|e| format!("onglet « {name} » : {e}"))?;
        let rows: Vec<&[Data]> = range.rows().collect();
        for r in 0..rows.len().min(30) {
            let header = header_at(&rows, r);
            let Some(idx) = cols.iter().map(|c| find_col(&header, c)).collect::<Option<Vec<usize>>>() else {
                let filled: Vec<&str> = header.iter().map(String::as_str).filter(|h| !h.is_empty()).collect();
                if filled.len() >= 3 && r < 3 && candidates.len() < 40 {
                    let line: String = filled.join(" | ").chars().take(300).collect();
                    candidates.push(format!("  [{name}] ligne {} : {line}", r + 1));
                }
                continue;
            };
            let mut out = BTreeMap::new();
            let mut ranges = [(f64::INFINITY, f64::NEG_INFINITY); 3];
            for data in &rows[r + 1..] {
                let Some(taxon) = data.get(idx[0]).map(|c| c.to_string().trim().to_owned()).filter(|t| !t.is_empty()) else { continue };
                let v = [1, 2, 3].map(|i| data.get(idx[i]).and_then(cell_f64));
                for (k, x) in v.iter().enumerate() {
                    if let Some(x) = x {
                        ranges[k] = (ranges[k].0.min(*x), ranges[k].1.max(*x));
                    }
                }
                if v.iter().any(Option::is_some) {
                    out.insert(taxon, v);
                }
            }
            if out.is_empty() {
                continue;
            }
            println!(
                "Onglet « {name} », en-têtes ligne {} : taxon = « {} », L = « {} », M = « {} », R = « {} »",
                r + 1,
                header[idx[0]],
                header[idx[1]],
                header[idx[2]],
                header[idx[3]]
            );
            for (k, (lo, hi)) in ["L (attendu 1–9)", "M (attendu 1–12)", "R (attendu 1–9)"].iter().zip(ranges) {
                if lo <= hi {
                    println!("  {k} : valeurs de {lo} à {hi}");
                }
            }
            return Ok(out);
        }
    }
    Err(format!(
        "aucun onglet ne contient les colonnes « {} ». Onglets : {}. Lignes d'en-têtes possibles :\n{}\n\
         Précisez --sheet, --taxon-col, --l-col, --m-col, --r-col.",
        cols.join(" », « "),
        sheets.join(", "),
        candidates.join("\n")
    ))
}

/// Cherche un taxon sur Wikidata ; compare les noms normalisés (le « × » des hybrides est ignoré)
/// et réessaie sans « × » si besoin.
fn find_taxon(o: &Online, latin: &str) -> plotflora_data::Result<Option<Candidate>> {
    let key = normalize(latin);
    let pick = |list: Vec<Candidate>| list.into_iter().find(|c| normalize(&c.latin_name) == key);
    if let Some(c) = pick(o.search_taxa(latin, false)?) {
        return Ok(Some(c));
    }
    if latin.contains('×') {
        return Ok(pick(o.search_taxa(&latin.replace('×', " "), false)?));
    }
    Ok(None)
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let err = |e: plotflora_data::Error| e.to_string();
    std::fs::create_dir_all(&args.out).map_err(|e| e.to_string())?;
    let eive_path = args.out.join("eive_index.json");

    let eive = if let Some(x) = &args.eive {
        let map = read_eive(x, args.sheet.as_deref(), &args.cols)?;
        store::write_json_atomic(&eive_path, &map).map_err(err)?;
        println!("Index EIVE : {} taxons → {}", map.len(), eive_path.display());
        EiveIndex::from_json(&serde_json::to_string(&map).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?
    } else {
        let s = std::fs::read_to_string(&eive_path).unwrap_or_else(|_| "{}".into());
        EiveIndex::from_json(&s).map_err(|e| e.to_string())?
    };

    let Some(species) = &args.species else { return Ok(()) };
    let seed_path = args.out.join("plants_catalog.seed.json");
    let mut catalog: Catalog = store::read_json(&seed_path).unwrap_or_default();
    catalog.schema_version = SCHEMA_VERSION.into();
    let list = std::fs::read_to_string(species).map_err(|e| format!("{} : {e}", species.display()))?;
    let online = (!args.offline).then(|| Online::new(Arc::new(Mutex::new(LookupCache::load(Path::new("builder_cache.json"))))));
    let entries: Vec<(&str, Vec<Ambiance>)> = list
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let (name, tags) = l.split_once('|').unwrap_or((l, ""));
            let ambiances = tags
                .split(',')
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .filter_map(|t| t.parse().map_err(|e| eprintln!("{name} : {e}")).ok())
                .collect();
            (name.trim(), ambiances)
        })
        .collect();
    let (mut added, mut updated, mut missing, mut failed) = (0, 0, Vec::new(), Vec::new());
    let total = entries.len();

    for (n, (latin, ambiances)) in entries.into_iter().enumerate() {
        let id = plant_id(latin, None);
        let existing = catalog.plants.iter().position(|p| p.id == id || normalize(&p.latin_name) == normalize(latin));
        let needs_links =
            existing.is_none_or(|i| catalog.plants[i].links.commons_image.is_none() || catalog.plants[i].links.wikidata.is_none());
        let candidate = match (&online, needs_links) {
            (Some(o), true) => match find_taxon(o, latin) {
                Ok(c) => c,
                Err(e) => {
                    println!("[{}/{total}] {latin} : échec réseau ({e})", n + 1);
                    failed.push(latin.to_owned());
                    continue;
                }
            },
            _ => None,
        };
        if let Some(i) = existing {
            // Fiche existante : on complète seulement les ambiances et les liens.
            let p = &mut catalog.plants[i];
            let mut changed = false;
            if p.characteristics.ambiances.is_empty() && !ambiances.is_empty() {
                p.characteristics.ambiances = ambiances;
                changed = true;
            }
            if let Some(c) = &candidate {
                for (slot, value) in [
                    (&mut p.links.wikidata, Some(c.qid.clone())),
                    (&mut p.links.wikipedia_fr, c.frwiki.clone()),
                    (&mut p.links.commons_image, c.image.clone()),
                ] {
                    if slot.is_none() && value.is_some() {
                        *slot = value;
                        changed = true;
                    }
                }
            }
            if changed {
                updated += 1;
            }
            println!("[{}/{total}] {latin} : {}", n + 1, if changed { "complétée" } else { "déjà à jour" });
        } else {
            let mut plant = match (&online, candidate) {
                (Some(o), Some(c)) => {
                    let (p, warnings) = o.build_draft(&c, &eive, None, false);
                    for w in warnings {
                        eprintln!("{latin} : {w}");
                    }
                    println!("[{}/{total}] {latin} : ajoutée", n + 1);
                    p
                }
                _ => {
                    if online.is_some() {
                        missing.push(latin.to_owned());
                    }
                    println!("[{}/{total}] {latin} : fiche minimale", n + 1);
                    let mut p = Plant { common_name: latin.to_owned(), latin_name: latin.to_owned(), ..Plant::default() };
                    eive.apply(&mut p);
                    p
                }
            };
            plant.id = id;
            plant.characteristics.ambiances = ambiances;
            plant.meta.source = DataSource::Seed;
            plant.meta.verified = false;
            plant.meta.updated_at = today();
            catalog.plants.push(plant);
            added += 1;
        }
        if (n + 1) % 25 == 0 {
            store::write_json_atomic(&seed_path, &catalog).map_err(err)?;
            if let Some(o) = &online {
                o.save_cache().map_err(err)?;
            }
        }
    }
    if let Some(o) = &online {
        o.save_cache().map_err(err)?;
    }
    store::write_json_atomic(&seed_path, &catalog).map_err(err)?;
    println!(
        "Catalogue : {added} fiche(s) ajoutée(s), {updated} complétée(s), {} au total → {}",
        catalog.plants.len(),
        seed_path.display()
    );
    if !missing.is_empty() {
        println!("Taxons introuvables sur Wikidata (fiches minimales créées) : {}", missing.join(", "));
    }
    if !failed.is_empty() {
        println!("Échecs réseau, à relancer (les espèces déjà traitées sont ignorées) : {}", failed.join(", "));
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Erreur : {e}");
        std::process::exit(1);
    }
}
