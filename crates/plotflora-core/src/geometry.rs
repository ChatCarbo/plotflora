// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 ChatCarbo

//! Géométrie du plan : mesures, tests de sélection, triangulation, quantités (§3.3).

use crate::model::{Plant, Pt, Zone};
use crate::vocab::ZoneKind;

pub fn dist(a: Pt, b: Pt) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

fn cross(o: Pt, a: Pt, b: Pt) -> f64 {
    (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
}

/// Aire signée (positive si les sommets tournent dans le sens trigonométrique, repère Y vers le haut).
pub fn signed_area(poly: &[Pt]) -> f64 {
    let n = poly.len();
    (0..n)
        .map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum::<f64>()
        / 2.0
}

pub fn polygon_area(poly: &[Pt]) -> f64 {
    if poly.len() < 3 {
        0.0
    } else {
        signed_area(poly).abs()
    }
}

pub fn polyline_length(pts: &[Pt]) -> f64 {
    pts.windows(2).map(|w| dist(w[0], w[1])).sum()
}

/// Intersection stricte ou par contact de deux segments.
pub fn segments_intersect(p1: Pt, p2: Pt, q1: Pt, q2: Pt) -> bool {
    let (d1, d2) = (cross(q1, q2, p1), cross(q1, q2, p2));
    let (d3, d4) = (cross(p1, p2, q1), cross(p1, p2, q2));
    let on = |a: Pt, b: Pt, c: Pt| c[0] >= a[0].min(b[0]) && c[0] <= a[0].max(b[0]) && c[1] >= a[1].min(b[1]) && c[1] <= a[1].max(b[1]);
    if ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0)) && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0)) {
        return true;
    }
    (d1 == 0.0 && on(q1, q2, p1)) || (d2 == 0.0 && on(q1, q2, p2)) || (d3 == 0.0 && on(p1, p2, q1)) || (d4 == 0.0 && on(p1, p2, q2))
}

/// Polygone valide : au moins 3 sommets, aire non nulle, côtés non croisés.
pub fn is_simple_polygon(poly: &[Pt]) -> bool {
    let n = poly.len();
    if n < 3 || polygon_area(poly) <= f64::EPSILON {
        return false;
    }
    for i in 0..n {
        for j in i + 1..n {
            let adjacent = j == i + 1 || (i == 0 && j == n - 1);
            if !adjacent && segments_intersect(poly[i], poly[(i + 1) % n], poly[j], poly[(j + 1) % n]) {
                return false;
            }
        }
    }
    true
}

pub fn point_in_polygon(p: Pt, poly: &[Pt]) -> bool {
    let mut inside = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + n - 1) % n]);
        if (a[1] > p[1]) != (b[1] > p[1]) && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0] {
            inside = !inside;
        }
    }
    inside
}

pub fn dist_point_segment(p: Pt, a: Pt, b: Pt) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len2 = dx * dx + dy * dy;
    if len2 == 0.0 {
        return dist(p, a);
    }
    let t = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len2).clamp(0.0, 1.0);
    dist(p, [a[0] + t * dx, a[1] + t * dy])
}

/// La zone est-elle sous le point `p` (tolérance en unités du plan) ?
pub fn hit_test(zone: &Zone, p: Pt, tol: f64) -> bool {
    let pts = &zone.points;
    match zone.kind {
        ZoneKind::Point => pts.first().is_some_and(|c| dist(*c, p) <= tol),
        ZoneKind::Line => pts.windows(2).any(|w| dist_point_segment(p, w[0], w[1]) <= tol),
        ZoneKind::Area => {
            point_in_polygon(p, pts) || (0..pts.len()).any(|i| dist_point_segment(p, pts[i], pts[(i + 1) % pts.len()]) <= tol)
        }
    }
}

/// Centre de gravité d'un polygone (peut tomber à l'extérieur s'il est concave).
pub fn polygon_centroid(pts: &[Pt]) -> Option<Pt> {
    let a = signed_area(pts);
    if pts.len() < 3 || a.abs() <= f64::EPSILON {
        return mean(pts);
    }
    let n = pts.len();
    let (mut cx, mut cy) = (0.0, 0.0);
    for i in 0..n {
        let (p, q) = (pts[i], pts[(i + 1) % n]);
        let f = p[0] * q[1] - q[0] * p[1];
        cx += (p[0] + q[0]) * f;
        cy += (p[1] + q[1]) * f;
    }
    Some([cx / (6.0 * a), cy / (6.0 * a)])
}

/// Point situé à l'intérieur du polygone : le centre de gravité s'il convient, sinon le milieu
/// du plus large segment intérieur sur l'horizontale passant par lui.
pub fn polygon_label_anchor(pts: &[Pt]) -> Option<Pt> {
    let c = polygon_centroid(pts)?;
    if pts.len() < 3 || point_in_polygon(c, pts) {
        return Some(c);
    }
    let y = c[1];
    let n = pts.len();
    let mut xs: Vec<f64> = (0..n)
        .filter_map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            ((a[1] > y) != (b[1] > y)).then(|| a[0] + (y - a[1]) * (b[0] - a[0]) / (b[1] - a[1]))
        })
        .collect();
    xs.sort_by(f64::total_cmp);
    xs.chunks_exact(2).max_by(|p, q| (p[1] - p[0]).total_cmp(&(q[1] - q[0]))).map(|p| [(p[0] + p[1]) / 2.0, y]).or(Some(c))
}

/// Point d'ancrage du libellé.
pub fn label_anchor(zone: &Zone) -> Option<Pt> {
    let pts = &zone.points;
    match zone.kind {
        ZoneKind::Area if pts.len() >= 3 => polygon_label_anchor(pts),
        ZoneKind::Line if pts.len() >= 2 => {
            let half = polyline_length(pts) / 2.0;
            let mut acc = 0.0;
            for w in pts.windows(2) {
                let l = dist(w[0], w[1]);
                if acc + l >= half && l > 0.0 {
                    let t = (half - acc) / l;
                    return Some([w[0][0] + t * (w[1][0] - w[0][0]), w[0][1] + t * (w[1][1] - w[0][1])]);
                }
                acc += l;
            }
            pts.first().copied()
        }
        _ => mean(pts),
    }
}

fn mean(pts: &[Pt]) -> Option<Pt> {
    (!pts.is_empty()).then(|| {
        let n = pts.len() as f64;
        [pts.iter().map(|p| p[0]).sum::<f64>() / n, pts.iter().map(|p| p[1]).sum::<f64>() / n]
    })
}

/// Triangulation par oreilles d'un polygone simple (concave autorisé).
pub fn triangulate(poly: &[Pt]) -> Vec<[usize; 3]> {
    let n = poly.len();
    if n < 3 {
        return Vec::new();
    }
    let ccw = signed_area(poly) > 0.0;
    let mut idx: Vec<usize> = (0..n).collect();
    let mut out = Vec::with_capacity(n - 2);
    let mut guard = 0;
    while idx.len() > 3 && guard < n * n {
        guard += 1;
        let m = idx.len();
        let ear = (0..m).find(|&i| {
            let (a, b, c) = (idx[(i + m - 1) % m], idx[i], idx[(i + 1) % m]);
            let cr = cross(poly[a], poly[b], poly[c]);
            let convex = if ccw { cr > 0.0 } else { cr < 0.0 };
            convex
                && !idx.iter().any(|&k| {
                    k != a && k != b && k != c && {
                        let p = poly[k];
                        let (s1, s2, s3) = (cross(poly[a], poly[b], p), cross(poly[b], poly[c], p), cross(poly[c], poly[a], p));
                        if ccw {
                            s1 >= 0.0 && s2 >= 0.0 && s3 >= 0.0
                        } else {
                            s1 <= 0.0 && s2 <= 0.0 && s3 <= 0.0
                        }
                    }
                })
        });
        match ear {
            Some(i) => {
                out.push([idx[(i + m - 1) % m], idx[i], idx[(i + 1) % m]]);
                idx.remove(i);
            }
            None => break, // polygone dégénéré : on s'arrête proprement
        }
    }
    if idx.len() == 3 {
        out.push([idx[0], idx[1], idx[2]]);
    }
    out
}

/// Surface (m²) d'un massif ou longueur (m) d'une haie, si l'échelle est connue.
pub fn zone_size_m(zone: &Zone, meters_per_unit: Option<f64>) -> Option<f64> {
    let m = meters_per_unit?;
    match zone.kind {
        ZoneKind::Point => None,
        ZoneKind::Area => Some(polygon_area(&zone.points) * m * m),
        ZoneKind::Line => Some(polyline_length(&zone.points) * m),
    }
}

/// Nombre de plants (§3.3). `None` si l'échelle ou l'espacement est inconnu.
pub fn quantity(zone: &Zone, plant: &Plant, meters_per_unit: Option<f64>) -> Option<u32> {
    if let Some(q) = zone.quantity_override {
        return Some(q);
    }
    if zone.kind == ZoneKind::Point {
        return Some(1);
    }
    let e = plant.dimensions.planting_spacing_m.or(plant.dimensions.mature_spread_m).filter(|e| e.is_finite() && *e > 0.0)?;
    let size = zone_size_m(zone, meters_per_unit)?;
    let q = match zone.kind {
        ZoneKind::Line => (size / e).floor() + 1.0,
        _ => (size / (e * e)).ceil().max(1.0),
    };
    Some(q.min(u32::MAX as f64) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SQUARE: [Pt; 4] = [[0.0, 0.0], [4.0, 0.0], [4.0, 3.0], [0.0, 3.0]];
    const L_SHAPE: [Pt; 6] = [[0.0, 0.0], [4.0, 0.0], [4.0, 1.0], [1.0, 1.0], [1.0, 3.0], [0.0, 3.0]];

    #[test]
    fn measures() {
        assert_eq!(polygon_area(&SQUARE), 12.0);
        assert_eq!(polygon_area(&L_SHAPE), 6.0);
        assert_eq!(polyline_length(&[[0.0, 0.0], [3.0, 4.0], [3.0, 10.0]]), 11.0);
        assert!(is_simple_polygon(&L_SHAPE));
        assert!(!is_simple_polygon(&[[0.0, 0.0], [2.0, 2.0], [2.0, 0.0], [0.0, 2.0]]));
        assert!(!is_simple_polygon(&[[0.0, 0.0], [1.0, 1.0], [2.0, 2.0]]));
    }

    #[test]
    fn triangulation_covers_area() {
        for poly in [&SQUARE[..], &L_SHAPE[..]] {
            for p in [poly.to_vec(), poly.iter().rev().copied().collect()] {
                let tris = triangulate(&p);
                assert_eq!(tris.len(), p.len() - 2);
                let sum: f64 = tris.iter().map(|t| polygon_area(&[p[t[0]], p[t[1]], p[t[2]]])).sum();
                assert!((sum - polygon_area(&p)).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn hits_and_anchor() {
        let area = Zone { kind: ZoneKind::Area, points: L_SHAPE.to_vec(), ..Zone::default() };
        assert!(hit_test(&area, [0.5, 2.0], 0.1));
        assert!(!hit_test(&area, [3.0, 2.5], 0.1));
        let line = Zone { kind: ZoneKind::Line, points: vec![[0.0, 0.0], [10.0, 0.0]], ..Zone::default() };
        assert!(hit_test(&line, [5.0, 0.4], 0.5));
        assert_eq!(label_anchor(&line), Some([5.0, 0.0]));
        let sq = Zone { kind: ZoneKind::Area, points: SQUARE.to_vec(), ..Zone::default() };
        assert_eq!(label_anchor(&sq), Some([2.0, 1.5]));
        // L très ouvert : le centre de gravité tombe hors du polygone, l'ancrage reste dedans.
        let l: Vec<Pt> = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 1.0], [1.0, 1.0], [1.0, 10.0], [0.0, 10.0]];
        assert!(!point_in_polygon(polygon_centroid(&l).unwrap(), &l));
        assert!(point_in_polygon(polygon_label_anchor(&l).unwrap(), &l));
    }

    #[test]
    fn quantities() {
        let mut plant = Plant::default();
        plant.dimensions.mature_spread_m = Some(3.0);
        plant.dimensions.planting_spacing_m = Some(0.8);
        let hedge = Zone { kind: ZoneKind::Line, points: vec![[0.0, 0.0], [120.0, 0.0]], ..Zone::default() };
        assert_eq!(quantity(&hedge, &plant, Some(0.1)), Some(16)); // 12 m / 0,8 + 1
        assert_eq!(quantity(&hedge, &plant, None), None);
        let bed = Zone { kind: ZoneKind::Area, points: SQUARE.to_vec(), ..Zone::default() };
        plant.dimensions.planting_spacing_m = None;
        assert_eq!(quantity(&bed, &plant, Some(1.0)), Some(2)); // 12 / 9 → 2
        let pt = Zone { quantity_override: Some(5), ..Zone::default() };
        assert_eq!(quantity(&pt, &plant, None), Some(5));
    }
}
