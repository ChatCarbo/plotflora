// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 ChatCarbo

//! Carte : affichage du plan et interactions (zoom, déplacement, outils de tracé).

use crate::app::{App, Drag, Tab, Tool};
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Key, Mesh, PointerButton, Pos2, Rect, Sense, Shape, Stroke, Ui, Vec2};
use plotflora_core::geometry::{dist, hit_test, is_simple_polygon, label_anchor, polyline_length, triangulate};
use plotflora_core::*;
use std::f32::consts::FRAC_PI_2;

#[derive(Clone, Copy)]
struct Tr {
    origin: Pos2,
    zoom: f32,
}

impl Tr {
    fn new(app: &App, rect: Rect) -> Self {
        Self { origin: rect.min + app.view.offset, zoom: app.view.zoom }
    }
    fn s(self, p: Pt) -> Pos2 {
        self.origin + vec2(p[0] as f32 * self.zoom, p[1] as f32 * self.zoom)
    }
    fn p(self, s: Pos2) -> Pt {
        let d = (s - self.origin) / self.zoom;
        [f64::from(d.x), f64::from(d.y)]
    }
}

pub fn status_color(s: Status) -> Color32 {
    match s {
        Status::ToPlant => Color32::from_rgb(76, 175, 80),
        Status::Existing => Color32::from_rgb(120, 144, 156),
        Status::ToReplace => Color32::from_rgb(255, 152, 0),
    }
}

const SELECT: Color32 = Color32::from_rgb(255, 235, 59);

/// Emprise du contenu en unités du plan.
fn bounds(app: &App, image: Option<Vec2>) -> (Pt, Pt) {
    if let Some(sz) = image {
        return ([0.0, 0.0], [f64::from(sz.x), f64::from(sz.y)]);
    }
    let Some(doc) = &app.doc else { return ([0.0, 0.0], [20.0, 15.0]) };
    let m = &doc.project.map;
    let pts = m
        .boundary
        .iter()
        .flatten()
        .chain(m.obstacles.iter().flat_map(|o| o.points.iter().chain(o.center.iter())))
        .chain(doc.project.zones.iter().flat_map(|z| z.points.iter()));
    let (mut lo, mut hi) = ([0.0_f64, 0.0], [20.0_f64, 15.0]);
    for p in pts {
        lo = [lo[0].min(p[0]), lo[1].min(p[1])];
        hi = [hi[0].max(p[0]), hi[1].max(p[1])];
    }
    (lo, hi)
}

fn fill_polygon(painter: &egui::Painter, tr: Tr, pts: &[Pt], color: Color32) {
    let tris = triangulate(pts);
    if tris.is_empty() {
        return;
    }
    let mut mesh = Mesh::default();
    for p in pts {
        mesh.colored_vertex(tr.s(*p), color);
    }
    for t in tris {
        mesh.add_triangle(t[0] as u32, t[1] as u32, t[2] as u32);
    }
    painter.add(Shape::mesh(mesh));
}

fn text_box(painter: &egui::Painter, pos: Pos2, anchor: Align2, text: String, size: f32) {
    let galley = painter.layout_no_wrap(text, FontId::proportional(size), Color32::WHITE);
    let r = anchor.anchor_size(pos, galley.size()).expand(3.0);
    painter.rect_filled(r, 3.0, Color32::from_black_alpha(170));
    painter.galley(r.min + vec2(3.0, 3.0), galley, Color32::WHITE);
}

fn fmt_len(m: f64) -> String {
    if m < 1.0 {
        format!("{:.0} cm", m * 100.0)
    } else {
        format!("{:.2} m", m).replace('.', ",")
    }
}

pub fn show(app: &mut App, ui: &mut Ui) {
    let (resp, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
    let rect = resp.rect;
    painter.rect_filled(rect, 0.0, Color32::from_gray(38));
    let Some(source) = app.doc.as_ref().map(|d| d.project.map.source) else {
        if let Some(t) = &app.logo {
            let size = t.size_vec2() * (rect.width() * 0.5 / t.size_vec2().x).min(1.0);
            let r = Rect::from_center_size(rect.center() - vec2(0.0, 40.0), size);
            painter.image(t.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        }
        painter.text(
            rect.center() + vec2(0.0, 60.0),
            Align2::CENTER_CENTER,
            "Fichier › Nouveau projet ou Ouvrir…",
            FontId::proportional(18.0),
            Color32::GRAY,
        );
        return;
    };

    // Fond de plan
    let ctx = ui.ctx().clone();
    let mut tex = None;
    let mut image_missing = false;
    if source == MapSource::Image {
        match app.image_path() {
            Some(path) => {
                tex = app.texture(&ctx, &path, 0);
                image_missing = tex.is_none();
            }
            None => image_missing = true,
        }
    }
    if !app.view.fitted && (tex.is_some() || source == MapSource::Drawing) {
        let (lo, hi) = bounds(app, tex.as_ref().map(egui::TextureHandle::size_vec2));
        let ext = vec2((hi[0] - lo[0]).max(1.0) as f32, (hi[1] - lo[1]).max(1.0) as f32);
        let zoom = (rect.width() / ext.x).min(rect.height() / ext.y) * 0.92;
        app.view.zoom = zoom;
        app.view.offset = (rect.size() - ext * zoom) / 2.0 - vec2(lo[0] as f32, lo[1] as f32) * zoom;
        app.view.fitted = true;
    }

    // Zoom (molette) et déplacement de la vue
    if let Some(pos) = resp.hover_pos() {
        let s = ui.input(|i| i.smooth_scroll_delta.y);
        if s != 0.0 {
            let plan = Tr::new(app, rect).p(pos);
            let z = (app.view.zoom * (s * 0.002).exp()).clamp(0.005, 400.0);
            app.view.zoom = z;
            app.view.offset = pos - rect.min - vec2(plan[0] as f32, plan[1] as f32) * z;
        }
    }
    if resp.dragged_by(PointerButton::Middle) || (app.drag == Drag::Pan && resp.dragged_by(PointerButton::Primary)) {
        app.view.offset += resp.drag_delta();
    }

    handle_tools(app, &resp, rect);
    draw(app, &ctx, &painter, &resp, rect, tex.as_ref(), image_missing);
}

fn handle_tools(app: &mut App, resp: &egui::Response, rect: Rect) {
    let tr = Tr::new(app, rect);
    let tol = f64::from(8.0 / tr.zoom);
    let click = resp.clicked().then(|| resp.interact_pointer_pos()).flatten().map(|p| tr.p(p));
    let (enter, back, esc) = resp.ctx.input(|i| (i.key_pressed(Key::Enter), i.key_pressed(Key::Backspace), i.key_pressed(Key::Escape)));
    let typing = resp.ctx.memory(|m| m.focused().is_some());

    if esc && !typing {
        if app.draft.is_empty() && app.scale_points.is_none() {
            app.tool = Tool::Select;
        }
        app.draft.clear();
        app.scale_points = None;
    }
    let Some(doc) = app.doc.as_mut() else { return };
    let project = &mut doc.project;

    match app.tool {
        Tool::Select => {
            if resp.drag_started_by(PointerButton::Primary) {
                if let Some(p) = resp.interact_pointer_pos().map(|s| tr.p(s)) {
                    let vertex = app
                        .selected
                        .as_deref()
                        .and_then(|id| project.zone(id))
                        .and_then(|z| z.points.iter().position(|v| dist(*v, p) <= tol));
                    let hit = project.zones.iter().rev().find(|z| hit_test(z, p, tol)).map(|z| z.id.clone());
                    app.drag = match (vertex, hit) {
                        (Some(i), _) => Drag::Vertex(i),
                        (None, Some(id)) => {
                            app.selected = Some(id);
                            Drag::Zone
                        }
                        (None, None) => Drag::Pan,
                    };
                }
            }
            if resp.dragged_by(PointerButton::Primary) {
                let d = resp.drag_delta() / tr.zoom;
                let d = [f64::from(d.x), f64::from(d.y)];
                if let Some(z) = app.selected.as_deref().and_then(|id| project.zone_mut(id)) {
                    match app.drag {
                        Drag::Vertex(i) => {
                            if let Some(v) = z.points.get_mut(i) {
                                *v = [v[0] + d[0], v[1] + d[1]];
                            }
                        }
                        Drag::Zone => z.points.iter_mut().for_each(|v| *v = [v[0] + d[0], v[1] + d[1]]),
                        Drag::Pan | Drag::None => {}
                    }
                }
            }
            if resp.drag_stopped() {
                app.drag = Drag::None;
            }
            if let Some(p) = click {
                app.selected = project.zones.iter().rev().find(|z| hit_test(z, p, tol)).map(|z| z.id.clone());
                if app.selected.is_some() {
                    app.tab = Tab::Zone;
                }
            }
        }
        Tool::Point => {
            if let Some(p) = click {
                let id = project.add_zone(ZoneKind::Point, vec![p]).id.clone();
                app.selected = Some(id);
                app.tab = Tab::Zone;
            }
        }
        Tool::Area | Tool::Line | Tool::Boundary | Tool::Obstacle => {
            if app.tool == Tool::Obstacle && app.obstacle_kind == ObstacleKind::ExistingTree {
                if let Some(p) = click {
                    project.map.obstacles.push(Obstacle {
                        kind: ObstacleKind::ExistingTree,
                        center: Some(p),
                        radius_m: Some(2.0),
                        ..Obstacle::default()
                    });
                }
                return;
            }
            if resp.double_clicked() || (enter && !typing) {
                let pts = std::mem::take(&mut app.draft);
                let mut msg: Option<Result<&str, &str>> = None;
                match app.tool {
                    Tool::Area if is_simple_polygon(&pts) => {
                        app.selected = Some(project.add_zone(ZoneKind::Area, pts).id.clone());
                        (app.tool, app.tab) = (Tool::Select, Tab::Zone);
                    }
                    Tool::Line if pts.len() >= 2 && polyline_length(&pts) > 0.0 => {
                        app.selected = Some(project.add_zone(ZoneKind::Line, pts).id.clone());
                        (app.tool, app.tab) = (Tool::Select, Tab::Zone);
                    }
                    Tool::Boundary if is_simple_polygon(&pts) => {
                        project.map.boundary = Some(pts);
                        app.tool = Tool::Select;
                        msg = Some(Ok("Contour du terrain enregistré."));
                    }
                    Tool::Obstacle if is_simple_polygon(&pts) => {
                        project.map.obstacles.push(Obstacle { kind: app.obstacle_kind, points: pts, ..Obstacle::default() });
                    }
                    Tool::Line => msg = Some(Err("Une haie demande au moins deux points distincts.")),
                    _ => msg = Some(Err("Polygone invalide : au moins trois sommets, sans côtés croisés.")),
                }
                match msg {
                    Some(Ok(m)) => app.info(m),
                    Some(Err(m)) => app.error(m),
                    None => {}
                }
            } else if let Some(p) = click {
                app.draft.push(p);
            } else if back && !typing {
                app.draft.pop();
            }
        }
        Tool::Scale | Tool::Measure => {
            if let Some(p) = click {
                app.draft.push(p);
            }
            if let [a, b] = app.draft[..] {
                app.draft.clear();
                if app.tool == Tool::Scale {
                    app.scale_points = Some((a, b));
                } else {
                    let d = dist(a, b);
                    let msg = match project.map.meters_per_unit() {
                        Some(m) => format!("Distance mesurée : {}", fmt_len(d * m)),
                        None => format!("Distance : {d:.0} px (échelle non calibrée)"),
                    };
                    app.info(msg);
                }
            }
        }
        Tool::North => {
            if let Some(pos) = resp.clicked().then(|| resp.interact_pointer_pos()).flatten() {
                let d = pos - rect.center();
                if d.length() > 4.0 {
                    project.map.set_north(f64::from(d.x.atan2(-d.y).to_degrees()));
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw(
    app: &App,
    ctx: &egui::Context,
    painter: &egui::Painter,
    resp: &egui::Response,
    rect: Rect,
    tex: Option<&egui::TextureHandle>,
    image_missing: bool,
) {
    let Some(doc) = &app.doc else { return };
    let project = &doc.project;
    let map = &project.map;
    let tr = Tr::new(app, rect);
    let mpu = map.meters_per_unit();

    if let Some(t) = tex {
        let r = Rect::from_min_size(tr.s([0.0, 0.0]), t.size_vec2() * tr.zoom);
        painter.image(t.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
    } else if image_missing {
        text_box(painter, rect.center(), Align2::CENTER_CENTER, "Image introuvable — onglet Jardin › « Relier l'image »".into(), 16.0);
    }

    // Grille (mode dessin), contour du terrain et obstacles
    if map.source == MapSource::Drawing {
        let step = [0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0].into_iter().find(|s| s * f64::from(tr.zoom) >= 14.0).unwrap_or(200.0);
        let (lo, hi) = (tr.p(rect.min), tr.p(rect.max));
        let stroke = Stroke::new(1.0_f32, Color32::from_gray(55));
        let mut x = (lo[0] / step).floor() * step;
        while x <= hi[0] {
            painter.line_segment([pos2(tr.s([x, 0.0]).x, rect.top()), pos2(tr.s([x, 0.0]).x, rect.bottom())], stroke);
            x += step;
        }
        let mut y = (lo[1] / step).floor() * step;
        while y <= hi[1] {
            painter.line_segment([pos2(rect.left(), tr.s([0.0, y]).y), pos2(rect.right(), tr.s([0.0, y]).y)], stroke);
            y += step;
        }
    }
    if let Some(b) = &map.boundary {
        fill_polygon(painter, tr, b, Color32::from_rgba_unmultiplied(110, 140, 80, 40));
        painter.add(Shape::closed_line(b.iter().map(|p| tr.s(*p)).collect(), Stroke::new(2.0_f32, Color32::from_gray(230))));
    }
    for o in &map.obstacles {
        match (o.center, o.radius_m) {
            (Some(c), Some(r)) => {
                let rad = (r / mpu.unwrap_or(1.0)) as f32 * tr.zoom;
                painter.circle(
                    tr.s(c),
                    rad,
                    Color32::from_rgba_unmultiplied(60, 110, 50, 120),
                    Stroke::new(1.5_f32, Color32::from_rgb(40, 80, 35)),
                );
            }
            _ => {
                fill_polygon(painter, tr, &o.points, Color32::from_rgba_unmultiplied(150, 150, 150, 140));
                painter.add(Shape::closed_line(o.points.iter().map(|p| tr.s(*p)).collect(), Stroke::new(1.5_f32, Color32::from_gray(90))));
            }
        }
    }

    // Zones (la zone sélectionnée en dernier)
    let selected = app.selected.as_deref();
    let mut zones: Vec<&Zone> = project.zones.iter().collect();
    zones.sort_by_key(|z| Some(z.id.as_str()) == selected);
    for z in &zones {
        let col = status_color(z.status);
        let sel = Some(z.id.as_str()) == selected;
        let stroke = Stroke::new(if sel { 3.0_f32 } else { 2.0 }, if sel { SELECT } else { col });
        let pts: Vec<Pos2> = z.points.iter().map(|p| tr.s(*p)).collect();
        match z.kind {
            ZoneKind::Area if pts.len() >= 3 => {
                fill_polygon(painter, tr, &z.points, col.gamma_multiply(0.35));
                painter.add(Shape::closed_line(pts.clone(), stroke));
            }
            ZoneKind::Line if pts.len() >= 2 => {
                painter.add(Shape::line(pts.clone(), Stroke::new(stroke.width + 3.0, stroke.color)));
            }
            ZoneKind::Point if !pts.is_empty() => {
                if let (Some(r), Some(m)) = (z.available_radius_m, mpu) {
                    painter.circle_stroke(pts[0], (r / m) as f32 * tr.zoom, Stroke::new(1.0_f32, col.gamma_multiply(0.8)));
                }
                painter.circle(pts[0], 7.0, col, Stroke::new(2.0_f32, if sel { SELECT } else { Color32::WHITE }));
            }
            _ => {}
        }
        if sel && z.kind != ZoneKind::Point {
            for p in &pts {
                painter.rect_filled(Rect::from_center_size(*p, vec2(8.0, 8.0)), 1.0, SELECT);
            }
        }
        if let Some(a) = label_anchor(z) {
            let mut text = z.label.clone();
            if let Some(p) = z.assigned_plant_id.as_deref().and_then(|id| app.assigned_plant(id)) {
                text.push('\n');
                text.push_str(&p.display_name());
            }
            let offset = if z.kind == ZoneKind::Point { vec2(0.0, -12.0) } else { Vec2::ZERO };
            text_box(
                painter,
                tr.s(a) + offset,
                if z.kind == ZoneKind::Point { Align2::CENTER_BOTTOM } else { Align2::CENTER_CENTER },
                text,
                12.0,
            );
        }
    }

    // Tracé en cours
    if !app.draft.is_empty() {
        let mut pts: Vec<Pos2> = app.draft.iter().map(|p| tr.s(*p)).collect();
        if let Some(h) = resp.hover_pos() {
            pts.push(h);
        }
        painter.add(Shape::line(pts.clone(), Stroke::new(2.0_f32, SELECT)));
        for p in &pts[..app.draft.len()] {
            painter.circle_filled(*p, 4.0, SELECT);
        }
    }
    if let Some((a, b)) = app.scale_points {
        painter.line_segment([tr.s(a), tr.s(b)], Stroke::new(3.0_f32, Color32::from_rgb(33, 150, 243)));
    }

    // Survol : résumé de la zone
    if app.tool == Tool::Select && app.drag == Drag::None {
        if let Some(h) = resp.hover_pos() {
            let p = tr.p(h);
            if let Some(z) = project.zones.iter().rev().find(|z| hit_test(z, p, f64::from(8.0 / tr.zoom))) {
                let mut t = format!("{} — {}", z.label, z.status.label());
                if let Some(pl) = z.assigned_plant_id.as_deref().and_then(|id| app.assigned_plant(id)) {
                    t.push_str(&format!("\n{} ({})", pl.display_name(), pl.latin_name));
                    let score = app.recs.as_ref().filter(|r| r.zone.id == z.id).and_then(|r| r.list.iter().find(|x| x.plant_id == pl.id));
                    if let Some(s) = score {
                        t.push_str(&format!("\nScore : {}/100 — {}", s.score, s.verdict.label()));
                    }
                }
                text_box(painter, h + vec2(14.0, 14.0), Align2::LEFT_TOP, t, 13.0);
            }
        }
    }

    // Rose des vents
    let c = rect.right_top() + vec2(-52.0, 52.0);
    let r = 30.0;
    painter.circle_filled(c, r + 10.0, Color32::from_black_alpha(150));
    let th = (map.north_angle_deg as f32).to_radians();
    painter.line_segment([c, c + vec2(th.sin(), -th.cos()) * (r - 12.0)], Stroke::new(3.0_f32, Color32::from_rgb(229, 57, 53)));
    for (k, l) in ["N", "E", "S", "O"].into_iter().enumerate() {
        let a = th + k as f32 * FRAC_PI_2;
        let col = if k == 0 { Color32::from_rgb(229, 57, 53) } else { Color32::WHITE };
        painter.text(c + vec2(a.sin(), -a.cos()) * r, Align2::CENTER_CENTER, l, FontId::proportional(14.0), col);
    }

    // Barre d'échelle
    if let Some(m) = mpu {
        let target = 120.0 / f64::from(tr.zoom) * m;
        let p10 = 10f64.powf(target.log10().floor());
        let nice = [5.0, 2.0, 1.0].into_iter().map(|k| k * p10).find(|v| *v <= target).unwrap_or(p10);
        let px = (nice / m) as f32 * tr.zoom;
        let o = rect.left_bottom() + vec2(16.0, -22.0);
        painter.rect_filled(Rect::from_min_size(o - vec2(6.0, 22.0), vec2(px + 12.0, 30.0)), 3.0, Color32::from_black_alpha(150));
        painter.line_segment([o, o + vec2(px, 0.0)], Stroke::new(3.0_f32, Color32::WHITE));
        painter.text(o + vec2(px / 2.0, -4.0), Align2::CENTER_BOTTOM, fmt_len(nice), FontId::proportional(12.0), Color32::WHITE);
    }

    if !map.attribution.trim().is_empty() {
        text_box(painter, rect.right_bottom() - vec2(6.0, 6.0), Align2::RIGHT_BOTTOM, map.attribution.clone(), 11.0);
    }
    text_box(painter, rect.left_top() + vec2(8.0, 8.0), Align2::LEFT_TOP, app.tool.hint().into(), 12.0);
    if app.busy() {
        ctx.request_repaint_after(std::time::Duration::from_millis(200));
    }
}
