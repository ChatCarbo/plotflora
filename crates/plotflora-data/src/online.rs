// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 ChatCarbo

//! Recherche en ligne sur action explicite (§5.4) : Wikidata, Wikipédia FR, Wikimedia Commons.

use crate::cache::LookupCache;
use crate::eive::EiveIndex;
use crate::{today, Error, Result};
use plotflora_core::convert::{extract_facts, plant_id};
use plotflora_core::{DataSource, Photo, Plant};
use serde_json::Value;
use std::io::Read;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

const WIKIDATA: &str = "https://www.wikidata.org/w/api.php";
const WIKIPEDIA: &str = "https://fr.wikipedia.org/w/api.php";
const COMMONS: &str = "https://commons.wikimedia.org/w/api.php";
const MAX_IMAGE_BYTES: u64 = 15 * 1024 * 1024;

/// En-tête `User-Agent` conforme aux règles Wikimedia. Le contact (URL ou e-mail) vient de la
/// variable d'environnement `PLOTFLORA_CONTACT`, lue à l'exécution puis, à défaut, à la compilation.
pub fn user_agent() -> String {
    let contact = std::env::var("PLOTFLORA_CONTACT")
        .ok()
        .filter(|c| !c.trim().is_empty())
        .or_else(|| option_env!("PLOTFLORA_CONTACT").map(str::to_owned))
        .unwrap_or_else(|| "contact non renseigné".to_owned());
    format!("PlotFlora/{} ({contact})", env!("CARGO_PKG_VERSION"))
}

/// Taxon candidat renvoyé par Wikidata.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Candidate {
    pub qid: String,
    pub latin_name: String,
    pub common_name: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
    pub frwiki: Option<String>,
}

#[derive(Clone)]
pub struct Online {
    agent: ureq::Agent,
    cache: Arc<Mutex<LookupCache>>,
}

static LAST_REQUEST: Mutex<Option<Instant>> = Mutex::new(None);

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Au plus une requête par seconde.
fn throttle() {
    let mut last = lock(&LAST_REQUEST);
    if let Some(t) = *last {
        if let Some(wait) = Duration::from_secs(1).checked_sub(t.elapsed()) {
            std::thread::sleep(wait);
        }
    }
    *last = Some(Instant::now());
}

fn net(e: impl std::fmt::Display) -> Error {
    Error::Network(e.to_string())
}

fn str_at<'a>(v: &'a Value, path: &[&str]) -> Option<&'a str> {
    path.iter().try_fold(v, |v, k| v.get(*k))?.as_str().filter(|s| !s.is_empty())
}

fn claim_values<'a>(entity: &'a Value, prop: &str) -> impl Iterator<Item = &'a Value> {
    entity["claims"][prop].as_array().into_iter().flatten().map(|c| &c["mainsnak"]["datavalue"]["value"])
}

/// Retire les balises HTML et décode les entités courantes (métadonnées Commons).
pub fn strip_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    let out = out
        .replace("&nbsp;", " ")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&");
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

impl Online {
    pub fn new(cache: Arc<Mutex<LookupCache>>) -> Self {
        let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(10)).user_agent(&user_agent()).build();
        Self { agent, cache }
    }

    fn get_json(&self, base: &str, params: &[(&str, &str)], force: bool) -> Result<Value> {
        let query: Vec<String> = params.iter().map(|(k, v)| format!("{k}={v}")).collect();
        let key = format!("{base}?{}", query.join("&"));
        if !force {
            if let Some(v) = lock(&self.cache).get(&key).and_then(|b| serde_json::from_str(b).ok()) {
                return Ok(v);
            }
        }
        // Réessais sur 429 / 503 en respectant l'en-tête Retry-After (Wikimedia).
        let mut attempt: u64 = 0;
        let body = loop {
            throttle();
            let req = params.iter().fold(self.agent.get(base), |r, (k, v)| r.query(k, v));
            match req.call() {
                Ok(resp) => break resp.into_string().map_err(net)?,
                Err(ureq::Error::Status(429 | 503, resp)) if attempt < 4 => {
                    attempt += 1;
                    let wait = resp.header("Retry-After").and_then(|s| s.trim().parse::<u64>().ok()).unwrap_or(5 * attempt);
                    std::thread::sleep(Duration::from_secs(wait.clamp(1, 120)));
                }
                Err(e) => return Err(net(e)),
            }
        };
        let v: Value = serde_json::from_str(&body).map_err(|e| net(format!("réponse invalide : {e}")))?;
        if let Some(err) = v.get("error") {
            return Err(net(err["info"].as_str().unwrap_or("erreur de l'API")));
        }
        lock(&self.cache).put(key, body);
        Ok(v)
    }

    /// Persiste le cache sur disque.
    pub fn save_cache(&self) -> Result<()> {
        lock(&self.cache).save()
    }

    /// Recherche de taxons par nom commun ou latin. Ne garde que les éléments ayant un nom de taxon (P225).
    pub fn search_taxa(&self, query: &str, force: bool) -> Result<Vec<Candidate>> {
        let q = query.trim();
        if q.is_empty() {
            return Ok(Vec::new());
        }
        let found = self.get_json(
            WIKIDATA,
            &[
                ("action", "wbsearchentities"),
                ("search", q),
                ("language", "fr"),
                ("uselang", "fr"),
                ("type", "item"),
                ("limit", "15"),
                ("format", "json"),
            ],
            force,
        )?;
        let ids: Vec<&str> = found["search"].as_array().into_iter().flatten().filter_map(|i| i["id"].as_str()).collect();
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let joined = ids.join("|");
        let ents = self.get_json(
            WIKIDATA,
            &[
                ("action", "wbgetentities"),
                ("ids", &joined),
                ("props", "labels|descriptions|claims|sitelinks"),
                ("languages", "fr"),
                ("sitefilter", "frwiki"),
                ("format", "json"),
            ],
            force,
        )?;
        Ok(ids
            .iter()
            .filter_map(|id| {
                let e = &ents["entities"][*id];
                let latin = claim_values(e, "P225").find_map(Value::as_str)?.to_owned();
                let common = claim_values(e, "P1843")
                    .find(|v| v["language"] == "fr")
                    .and_then(|v| v["text"].as_str())
                    .or_else(|| str_at(e, &["labels", "fr", "value"]).filter(|l| !l.eq_ignore_ascii_case(&latin)))
                    .map(str::to_owned);
                Some(Candidate {
                    qid: (*id).to_owned(),
                    latin_name: latin,
                    common_name: common,
                    description: str_at(e, &["descriptions", "fr", "value"]).map(str::to_owned),
                    image: claim_values(e, "P18").find_map(Value::as_str).map(str::to_owned),
                    frwiki: str_at(e, &["sitelinks", "frwiki", "title"]).map(str::to_owned),
                })
            })
            .collect())
    }

    /// Texte brut d'un article de Wikipédia FR (utilisé uniquement pour en extraire des valeurs).
    pub fn wikipedia_text(&self, title: &str, force: bool) -> Result<String> {
        let v = self.get_json(
            WIKIPEDIA,
            &[
                ("action", "query"),
                ("prop", "extracts"),
                ("explaintext", "1"),
                ("redirects", "1"),
                ("titles", title),
                ("format", "json"),
                ("formatversion", "2"),
            ],
            force,
        )?;
        Ok(v["query"]["pages"][0]["extract"].as_str().unwrap_or_default().to_owned())
    }

    /// Télécharge une vignette Commons (800 px) dans `photos_dir` et renvoie ses crédits.
    pub fn commons_photo(&self, file_name: &str, photos_dir: &Path, stem: &str, force: bool) -> Result<Photo> {
        let title = format!("File:{file_name}");
        let v = self.get_json(
            COMMONS,
            &[
                ("action", "query"),
                ("titles", &title),
                ("prop", "imageinfo"),
                ("iiprop", "url|extmetadata"),
                ("iiurlwidth", "800"),
                ("format", "json"),
                ("formatversion", "2"),
            ],
            force,
        )?;
        let info = &v["query"]["pages"][0]["imageinfo"][0];
        let url = str_at(info, &["thumburl"]).or_else(|| str_at(info, &["url"])).ok_or_else(|| net("image introuvable sur Commons"))?;
        let meta = |k: &str| str_at(info, &["extmetadata", k, "value"]).map(strip_html).filter(|s| !s.is_empty());
        let ext = url
            .rsplit('.')
            .next()
            .map(str::to_ascii_lowercase)
            .filter(|e| ["jpg", "jpeg", "png", "webp"].contains(&e.as_str()))
            .unwrap_or_else(|| "jpg".into());
        let name = format!("{stem}.{ext}");
        throttle();
        let mut bytes = Vec::new();
        self.agent.get(url).call().map_err(net)?.into_reader().take(MAX_IMAGE_BYTES).read_to_end(&mut bytes).map_err(net)?;
        std::fs::create_dir_all(photos_dir).map_err(|source| Error::Io { path: photos_dir.to_owned(), source })?;
        let dest = photos_dir.join(&name);
        std::fs::write(&dest, &bytes).map_err(|source| Error::Io { path: dest.clone(), source })?;
        Ok(Photo {
            file: format!("photos/{name}"),
            source_url: str_at(info, &["descriptionurl"]).map(str::to_owned),
            author: meta("Artist"),
            license: meta("LicenseShortName"),
            license_url: meta("LicenseUrl"),
        })
    }

    /// Construit un brouillon de fiche (`verified = false`). Les échecs non bloquants
    /// (article ou photo indisponible) sont renvoyés comme avertissements.
    pub fn build_draft(&self, c: &Candidate, eive: &EiveIndex, photos_dir: Option<&Path>, force: bool) -> (Plant, Vec<String>) {
        let mut warnings = Vec::new();
        let mut p = Plant {
            id: plant_id(&c.latin_name, None),
            common_name: c.common_name.clone().unwrap_or_else(|| c.latin_name.clone()),
            latin_name: c.latin_name.clone(),
            ..Plant::default()
        };
        p.links.wikidata = Some(c.qid.clone());
        p.links.wikipedia_fr = c.frwiki.clone();
        p.links.commons_image = c.image.clone();
        p.meta.source = DataSource::Mixed;
        p.meta.verified = false;
        p.meta.updated_at = today();
        let wd = format!("wikidata:{}", c.qid);
        p.meta.field_sources.insert("latin_name".into(), wd.clone());
        p.meta.field_sources.insert("common_name".into(), wd);
        p.meta.uncertain_fields.push("category".into());

        if let Some(title) = &c.frwiki {
            match self.wikipedia_text(title, force) {
                Ok(text) => {
                    let f = extract_facts(&text);
                    let src = format!("wikipedia_fr:{}", title.replace(' ', "_"));
                    let mark = |p: &mut Plant, field: &str| {
                        p.meta.field_sources.insert(field.into(), src.clone());
                        p.meta.uncertain_fields.push(field.into());
                    };
                    if let Some(t) = f.min_temp_c {
                        p.climate.min_temp_c = Some(t);
                        mark(&mut p, "climate.min_temp_c");
                    }
                    if let Some(h) = f.height_m {
                        p.dimensions.mature_height_m = Some(h);
                        mark(&mut p, "dimensions.mature_height_m");
                    }
                    if let Some(s) = f.spread_m {
                        p.dimensions.mature_spread_m = Some(s);
                        mark(&mut p, "dimensions.mature_spread_m");
                    }
                    if let Some(t) = f.non_toxic {
                        p.care.non_toxic_pets = Some(t);
                        mark(&mut p, "care.non_toxic_pets");
                    }
                }
                Err(e) => warnings.push(format!("Article Wikipédia indisponible : {e}")),
            }
        }
        eive.apply(&mut p);
        if let (Some(img), Some(dir)) = (&c.image, photos_dir) {
            match self.commons_photo(img, dir, &p.id, force) {
                Ok(photo) => {
                    p.photo = Some(photo);
                    p.meta.field_sources.insert("photo".into(), "commons".into());
                }
                Err(e) => warnings.push(format!("Photo indisponible : {e}")),
            }
        }
        if let Err(e) = self.save_cache() {
            warnings.push(format!("Cache non enregistré : {e}"));
        }
        (p, warnings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_is_stripped() {
        assert_eq!(strip_html("<a href=\"x\">Jean&nbsp;Dupont</a> &amp; co"), "Jean Dupont & co");
    }

    #[test]
    fn json_paths() {
        let v: Value = serde_json::from_str(r#"{"claims":{"P225":[{"mainsnak":{"datavalue":{"value":"Acer palmatum"}}}]}}"#).unwrap();
        assert_eq!(claim_values(&v, "P225").find_map(Value::as_str), Some("Acer palmatum"));
        assert_eq!(claim_values(&v, "P18").count(), 0);
    }
}
