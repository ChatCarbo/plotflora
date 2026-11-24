// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 ChatCarbo

//! Détection approximative des limites du jardin sur une vue aérienne (« baguette magique ») :
//! croissance de région depuis un clic, fermeture morphologique, bouchage des trous,
//! extraction du contour extérieur puis simplification. Le résultat est à ajuster par l'utilisateur.

use crate::geometry::{dist_point_segment, is_simple_polygon, polygon_area, signed_area};
use crate::model::Pt;
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectError {
    /// Image vide, dimensions incohérentes ou clic hors de l'image.
    BadInput,
    /// Zone trouvée trop petite : augmenter la tolérance.
    TooSmall,
    /// Zone trop étendue (débordement sur le voisinage) : réduire la tolérance.
    TooLarge,
    /// Contour inexploitable : tracer le contour à la main.
    Invalid,
}

impl std::fmt::Display for DetectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::BadInput => "image ou point de départ invalide",
            Self::TooSmall => "zone détectée trop petite : augmentez la tolérance",
            Self::TooLarge => "la détection déborde du jardin : réduisez la tolérance",
            Self::Invalid => "contour inexploitable : tracez le contour à la main",
        })
    }
}

impl std::error::Error for DetectError {}

/// Masque binaire `w × h`.
struct Mask {
    w: usize,
    h: usize,
    v: Vec<bool>,
}

impl Mask {
    fn get(&self, x: i64, y: i64) -> bool {
        x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h && self.v[y as usize * self.w + x as usize]
    }

    /// Dilatation (`grow = true`) ou érosion par un carré de rayon `r` (séparable).
    fn morph(&self, r: usize, grow: bool) -> Mask {
        let (w, h) = (self.w, self.h);
        let pass = |src: &[bool], horizontal: bool| -> Vec<bool> {
            let mut out = vec![false; w * h];
            for y in 0..h {
                for x in 0..w {
                    let (c, n) = if horizontal { (x, w) } else { (y, h) };
                    let (lo, hi) = (c.saturating_sub(r), (c + r).min(n - 1));
                    let at = |k: usize| if horizontal { src[y * w + k] } else { src[k * w + x] };
                    out[y * w + x] = if grow { (lo..=hi).any(at) } else { (lo..=hi).all(at) };
                }
            }
            out
        };
        let tmp = pass(&self.v, true);
        Mask { w, h, v: pass(&tmp, false) }
    }

    /// Bouche les trous : tout ce qui n'est pas relié au bord par le fond devient plein.
    fn fill_holes(&mut self) {
        let (w, h) = (self.w, self.h);
        let mut outside = vec![false; w * h];
        let mut q = VecDeque::new();
        for x in 0..w {
            q.push_back((x, 0));
            q.push_back((x, h - 1));
        }
        for y in 0..h {
            q.push_back((0, y));
            q.push_back((w - 1, y));
        }
        while let Some((x, y)) = q.pop_front() {
            let i = y * w + x;
            if self.v[i] || outside[i] {
                continue;
            }
            outside[i] = true;
            if x > 0 {
                q.push_back((x - 1, y));
            }
            if x + 1 < w {
                q.push_back((x + 1, y));
            }
            if y > 0 {
                q.push_back((x, y - 1));
            }
            if y + 1 < h {
                q.push_back((x, y + 1));
            }
        }
        for (v, o) in self.v.iter_mut().zip(outside) {
            *v = !o;
        }
    }

    fn count(&self) -> usize {
        self.v.iter().filter(|b| **b).count()
    }
}

/// Croissance de région 4-connexe : pixels dont la couleur est à moins de `tol` de la couleur
/// moyenne autour du clic.
fn grow(w: usize, h: usize, rgb: &[u8], seed: (usize, usize), tol: f64) -> Mask {
    let px = |x: usize, y: usize| {
        let i = (y * w + x) * 3;
        [f64::from(rgb[i]), f64::from(rgb[i + 1]), f64::from(rgb[i + 2])]
    };
    let (mut sum, mut n) = ([0.0; 3], 0.0);
    for y in seed.1.saturating_sub(3)..=(seed.1 + 3).min(h - 1) {
        for x in seed.0.saturating_sub(3)..=(seed.0 + 3).min(w - 1) {
            let c = px(x, y);
            for k in 0..3 {
                sum[k] += c[k];
            }
            n += 1.0;
        }
    }
    let mean = sum.map(|s| s / n);
    let tol2 = tol * tol;
    let close = |x: usize, y: usize| {
        let c = px(x, y);
        (0..3).map(|k| (c[k] - mean[k]).powi(2)).sum::<f64>() <= tol2
    };
    let mut m = Mask { w, h, v: vec![false; w * h] };
    let mut seen = vec![false; w * h];
    let mut q = VecDeque::from([seed]);
    seen[seed.1 * w + seed.0] = true;
    while let Some((x, y)) = q.pop_front() {
        if !close(x, y) {
            continue;
        }
        m.v[y * w + x] = true;
        let mut push = |nx: usize, ny: usize| {
            if !seen[ny * w + nx] {
                seen[ny * w + nx] = true;
                q.push_back((nx, ny));
            }
        };
        if x > 0 {
            push(x - 1, y);
        }
        if x + 1 < w {
            push(x + 1, y);
        }
        if y > 0 {
            push(x, y - 1);
        }
        if y + 1 < h {
            push(x, y + 1);
        }
    }
    m
}

/// Contour extérieur du masque (sommets aux coins des pixels). Parmi les boucles obtenues,
/// garde celle de plus grande aire.
fn outer_contour(m: &Mask) -> Vec<Pt> {
    // Arêtes orientées, région à droite (repère Y vers le bas).
    let mut edges: Vec<((i64, i64), (i64, i64))> = Vec::new();
    for y in 0..m.h as i64 {
        for x in 0..m.w as i64 {
            if !m.get(x, y) {
                continue;
            }
            if !m.get(x, y - 1) {
                edges.push(((x, y), (x + 1, y)));
            }
            if !m.get(x + 1, y) {
                edges.push(((x + 1, y), (x + 1, y + 1)));
            }
            if !m.get(x, y + 1) {
                edges.push(((x + 1, y + 1), (x, y + 1)));
            }
            if !m.get(x - 1, y) {
                edges.push(((x, y + 1), (x, y)));
            }
        }
    }
    let mut from: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    for (i, e) in edges.iter().enumerate() {
        from.entry(e.0).or_default().push(i);
    }
    let mut used = vec![false; edges.len()];
    let mut best: Vec<Pt> = Vec::new();
    for start in 0..edges.len() {
        if used[start] {
            continue;
        }
        let mut loop_pts = Vec::new();
        let mut e = start;
        loop {
            used[e] = true;
            let (a, b) = edges[e];
            loop_pts.push([a.0 as f64, a.1 as f64]);
            if b == edges[start].0 {
                break;
            }
            let dir = (b.0 - a.0, b.1 - a.1);
            // Au point de pincement, on tourne à droite en priorité (boucles séparées).
            let next = from.get(&b).into_iter().flatten().copied().filter(|i| !used[*i]).max_by_key(|i| {
                let (c, d) = edges[*i];
                dir.0 * (d.1 - c.1) - dir.1 * (d.0 - c.0)
            });
            match next {
                Some(n) => e = n,
                None => break,
            }
        }
        if polygon_area(&loop_pts) > polygon_area(&best) {
            best = loop_pts;
        }
    }
    best
}

/// Simplification de Douglas-Peucker sur un polygone fermé.
pub fn simplify_closed(poly: &[Pt], eps: f64) -> Vec<Pt> {
    fn dp(pts: &[Pt], eps: f64, out: &mut Vec<Pt>) {
        let (a, b) = (pts[0], pts[pts.len() - 1]);
        let (mut idx, mut dmax) = (0, 0.0);
        for (i, p) in pts.iter().enumerate().take(pts.len() - 1).skip(1) {
            let d = dist_point_segment(*p, a, b);
            if d > dmax {
                (idx, dmax) = (i, d);
            }
        }
        if dmax > eps {
            dp(&pts[..=idx], eps, out);
            dp(&pts[idx..], eps, out);
        } else {
            out.push(a);
        }
    }
    if poly.len() < 4 {
        return poly.to_vec();
    }
    // Découpe au sommet le plus éloigné du premier pour traiter la boucle en deux chaînes.
    let far = (1..poly.len())
        .max_by(|i, j| crate::geometry::dist(poly[0], poly[*i]).total_cmp(&crate::geometry::dist(poly[0], poly[*j])))
        .unwrap_or(1);
    let mut out = Vec::new();
    dp(&poly[..=far], eps, &mut out);
    let mut back: Vec<Pt> = poly[far..].to_vec();
    back.push(poly[0]);
    dp(&back, eps, &mut out);
    out
}

/// Détecte le contour de la zone autour du point `seed` (coordonnées en pixels) d'une image
/// RVB `w × h` (`rgb.len() == w * h * 3`). `tolerance` : écart de couleur accepté (0–255,
/// 40 par défaut). Renvoie un polygone simple en pixels de l'image.
pub fn detect_region(w: usize, h: usize, rgb: &[u8], seed: (usize, usize), tolerance: f64) -> Result<Vec<Pt>, DetectError> {
    if w < 8 || h < 8 || rgb.len() != w * h * 3 || seed.0 >= w || seed.1 >= h || !tolerance.is_finite() {
        return Err(DetectError::BadInput);
    }
    let mut m = grow(w, h, rgb, seed, tolerance.clamp(1.0, 255.0));
    let total = (w * h) as f64;
    if (m.count() as f64) < total * 0.002 {
        return Err(DetectError::TooSmall);
    }
    let r = (w.max(h) / 200).max(2);
    m = m.morph(r, true).morph(r, false);
    m.fill_holes();
    if m.count() as f64 > total * 0.85 {
        return Err(DetectError::TooLarge);
    }
    let contour = outer_contour(&m);
    if contour.len() < 3 {
        return Err(DetectError::Invalid);
    }
    let diag = (w as f64).hypot(h as f64);
    let mut eps = (diag * 0.004).max(1.0);
    let mut poly = simplify_closed(&contour, eps);
    while poly.len() > 40 {
        eps *= 1.5;
        poly = simplify_closed(&contour, eps);
    }
    if signed_area(&poly) < 0.0 {
        poly.reverse();
    }
    if is_simple_polygon(&poly) {
        Ok(poly)
    } else {
        Err(DetectError::Invalid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Image 200 × 150 grise avec un jardin vert en L et un peu de bruit.
    fn l_garden() -> (usize, usize, Vec<u8>) {
        let (w, h) = (200, 150);
        let mut rgb = vec![0u8; w * h * 3];
        for y in 0..h {
            for x in 0..w {
                let garden = (40..160).contains(&x) && (30..120).contains(&y) && !(x >= 100 && y < 70);
                let noise = ((x * 7 + y * 13) % 11) as u8;
                let c = if garden { [60 + noise, 140 + noise, 50] } else { [150 + noise, 150, 150 - noise] };
                rgb[(y * w + x) * 3..][..3].copy_from_slice(&c);
            }
        }
        // petite allée claire au milieu du jardin (doit être absorbée par le bouchage des trous)
        for y in 80..84 {
            for x in 60..70 {
                rgb[(y * w + x) * 3..][..3].copy_from_slice(&[220, 210, 190]);
            }
        }
        (w, h, rgb)
    }

    #[test]
    fn detects_l_shaped_garden() {
        let (w, h, rgb) = l_garden();
        let poly = detect_region(w, h, &rgb, (60, 100), 40.0).unwrap();
        let expected = 120.0 * 90.0 - 60.0 * 40.0;
        let area = polygon_area(&poly);
        assert!((area - expected).abs() / expected < 0.05, "aire {area} attendue {expected}");
        assert!(poly.len() <= 40 && poly.len() >= 6, "{} sommets", poly.len());
        assert!(is_simple_polygon(&poly));
    }

    #[test]
    fn leak_and_bad_input() {
        let (w, h) = (64, 64);
        let uniform = vec![100u8; w * h * 3];
        assert_eq!(detect_region(w, h, &uniform, (10, 10), 40.0), Err(DetectError::TooLarge));
        assert_eq!(detect_region(w, h, &uniform, (99, 10), 40.0), Err(DetectError::BadInput));
        assert_eq!(detect_region(w, h, &uniform[1..], (10, 10), 40.0), Err(DetectError::BadInput));
        let (w, h, rgb) = l_garden();
        assert_eq!(detect_region(w, h, &rgb, (65, 81), 5.0), Err(DetectError::TooSmall));
    }

    #[test]
    fn simplify_keeps_corners() {
        let mut sq = Vec::new();
        for i in 0..10 {
            sq.push([f64::from(i), 0.0]);
        }
        for i in 0..10 {
            sq.push([10.0, f64::from(i)]);
        }
        for i in 0..10 {
            sq.push([10.0 - f64::from(i), 10.0]);
        }
        for i in 0..10 {
            sq.push([0.0, 10.0 - f64::from(i)]);
        }
        let s = simplify_closed(&sq, 0.5);
        assert_eq!(s.len(), 4);
        assert_eq!(polygon_area(&s), 100.0);
    }
}
