// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 ChatCarbo

//! Normalisation de texte et recherche floue dans le catalogue (§5.3).

use crate::model::Plant;

/// Minuscules et suppression des accents ; le reste du texte est conservé.
pub fn fold(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars().flat_map(char::to_lowercase) {
        match c {
            'à' | 'â' | 'ä' | 'á' | 'ã' | 'å' => out.push('a'),
            'é' | 'è' | 'ê' | 'ë' => out.push('e'),
            'î' | 'ï' | 'í' | 'ì' => out.push('i'),
            'ô' | 'ö' | 'ó' | 'ò' | 'õ' => out.push('o'),
            'ù' | 'û' | 'ü' | 'ú' => out.push('u'),
            'ÿ' => out.push('y'),
            'ç' => out.push('c'),
            'ñ' => out.push('n'),
            'œ' => out.push_str("oe"),
            'æ' => out.push_str("ae"),
            '−' | '–' | '—' => out.push('-'),
            '\u{a0}' | '\u{202f}' => out.push(' '),
            '’' => out.push('\''),
            _ => out.push(c),
        }
    }
    out
}

/// Clé de recherche : texte replié, ponctuation → espaces, espaces fusionnés.
pub fn normalize(s: &str) -> String {
    fold(s).split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" ")
}

/// Normalise une saisie utilisateur et retire les mots parasites de tête.
pub fn normalize_query(s: &str) -> String {
    let mut q = normalize(s);
    for noise in ["plante ", "plant ", "pied de ", "pied d ", "un ", "une ", "des ", "du ", "de "] {
        if let Some(rest) = q.strip_prefix(noise) {
            q = rest.to_owned();
        }
    }
    q
}

pub fn levenshtein(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for (i, ca) in a.chars().enumerate() {
        cur[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            cur[j + 1] = (prev[j + 1] + 1).min(cur[j] + 1).min(prev[j] + usize::from(ca != *cb));
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

/// Distance d'une requête normalisée à un nom (plus petit = meilleur ; `None` = pas de correspondance).
pub fn match_rank(query: &str, name: &str) -> Option<usize> {
    let n = normalize(name);
    if query.is_empty() || n.is_empty() {
        return None;
    }
    if n == query {
        return Some(0);
    }
    if n.starts_with(query) {
        return Some(1);
    }
    if n.contains(query) {
        return Some(2);
    }
    let tol = (query.chars().count() / 3).max(1);
    let best = std::iter::once(n.as_str()).chain(n.split(' ')).map(|w| levenshtein(query, w)).min()?;
    (best <= tol).then_some(3 + best)
}

/// Recherche floue sur nom commun, nom latin et cultivar ; résultats triés.
pub fn search<'a>(plants: impl IntoIterator<Item = &'a Plant>, query: &str, limit: usize) -> Vec<&'a Plant> {
    let q = normalize_query(query);
    let mut hits: Vec<(usize, String, &Plant)> = plants
        .into_iter()
        .filter_map(|p| {
            let names = [Some(p.common_name.as_str()), Some(p.latin_name.as_str()), p.cultivar.as_deref()];
            let r = names.into_iter().flatten().filter_map(|n| match_rank(&q, n)).min()?;
            Some((r, normalize(&p.common_name), p))
        })
        .collect();
    hits.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)).then_with(|| a.2.id.cmp(&b.2.id)));
    hits.into_iter().take(limit).map(|h| h.2).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plant(id: &str, common: &str, latin: &str) -> Plant {
        Plant { id: id.into(), common_name: common.into(), latin_name: latin.into(), ..Plant::default() }
    }

    #[test]
    fn folding() {
        assert_eq!(fold("Érable du Japon − Œillet"), "erable du japon - oeillet");
        assert_eq!(normalize_query("  Pied de LAVANDE-vraie "), "lavande vraie");
        assert_eq!(levenshtein("lavende", "lavande"), 1);
    }

    #[test]
    fn fuzzy_search() {
        let cat = [
            plant("a", "Lavande vraie", "Lavandula angustifolia"),
            plant("b", "Érable du Japon", "Acer palmatum"),
            plant("c", "Lavandin", "Lavandula x intermedia"),
        ];
        let ids = |q: &str| search(&cat, q, 10).iter().map(|p| p.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids("lavande"), vec!["a", "c"]);
        assert_eq!(ids("erable"), vec!["b"]);
        assert_eq!(ids("acer palmatum"), vec!["b"]);
        assert_eq!(ids("lavendula"), vec!["a", "c"]);
        assert!(ids("zzz").is_empty());
    }
}
