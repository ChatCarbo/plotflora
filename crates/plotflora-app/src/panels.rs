// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 ChatCarbo

//! Menus, barre d'outils, panneau latéral (Jardin, Zone, Catalogue) et fenêtres.

use crate::app::{App, Editor, RecCache, Tab, Tool};
use crate::canvas::status_color;
use crate::widgets::*;
use eframe::egui::{self, Color32, Context, RichText, Ui, Vec2};
use plotflora_core::convert::{plant_id, unique_id};
use plotflora_core::geometry::{is_simple_polygon, quantity, zone_size_m};
use plotflora_core::scoring::{recommend, AlertLevel, Verdict};
use plotflora_core::search::{normalize, search};
use plotflora_core::*;
use plotflora_data::{store, today};

const ORANGE: Color32 = Color32::from_rgb(255, 167, 38);
const RED: Color32 = Color32::from_rgb(239, 83, 80);
const GREEN: Color32 = Color32::from_rgb(102, 187, 106);

fn verdict_color(v: Verdict) -> Color32 {
    match v {
        Verdict::VeryGood => GREEN,
        Verdict::Reserved => Color32::from_rgb(205, 220, 57),
        Verdict::Poor => ORANGE,
        Verdict::Avoid => RED,
    }
}

// ---------------------------------------------------------------- Menus

pub fn menu_bar(app: &mut App, ctx: &Context) {
    egui::TopBottomPanel::top("menu").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            if let Some(id) = app.icon.as_ref().map(egui::TextureHandle::id) {
                ui.image((id, Vec2::splat(18.0)));
            }
            ui.menu_button("Fichier", |ui| {
                if ui.button("Nouveau projet (image aérienne)…").clicked() {
                    ui.close_menu();
                    app.new_project(MapSource::Image);
                }
                if ui.button("Nouveau projet (plan dessiné)…").clicked() {
                    ui.close_menu();
                    app.new_project(MapSource::Drawing);
                }
                if ui.button("Ouvrir…").clicked() {
                    ui.close_menu();
                    app.open_project();
                }
                ui.separator();
                let has = app.doc.is_some();
                if ui.add_enabled(has, egui::Button::new("Enregistrer (Ctrl+S)")).clicked() {
                    ui.close_menu();
                    app.save();
                }
                if ui.add_enabled(has, egui::Button::new("Enregistrer sous…")).clicked() {
                    ui.close_menu();
                    app.save_as();
                }
                ui.separator();
                if ui.button("Quitter").clicked() {
                    ui.close_menu();
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button("Édition", |ui| {
                if ui.add_enabled(app.can_undo(), egui::Button::new("Annuler (Ctrl+Z)")).clicked() {
                    ui.close_menu();
                    app.undo();
                }
                if ui.add_enabled(app.can_redo(), egui::Button::new("Rétablir (Ctrl+Y)")).clicked() {
                    ui.close_menu();
                    app.redo();
                }
            });
            ui.menu_button("Aide", |ui| {
                if ui.button("Obtenir une vue aérienne IGN").clicked() {
                    ui.close_menu();
                    app.show_help = true;
                }
                if ui.button("À propos & licences").clicked() {
                    ui.close_menu();
                    app.show_credits = true;
                }
            });
        });
    });
}

pub fn status_bar(app: &mut App, ctx: &Context) {
    egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
        ui.horizontal(|ui| {
            if app.busy() {
                ui.spinner();
            }
            let col = if app.status_error { RED } else { ui.visuals().text_color() };
            ui.label(RichText::new(&app.status).color(col));
        });
    });
}

pub fn toolbar(app: &mut App, ctx: &Context) {
    egui::SidePanel::left("tools").resizable(false).exact_width(158.0).show(ctx, |ui| {
        let Some(source) = app.doc.as_ref().map(|d| d.project.map.source) else {
            ui.weak("Aucun projet");
            return;
        };
        let before = app.tool;
        ui.heading("Outils");
        let mut tools = vec![
            (Tool::Select, "🖱 Sélection"),
            (Tool::Point, "📍 Plante isolée"),
            (Tool::Area, "⬟ Massif"),
            (Tool::Line, "〰 Haie / bordure"),
        ];
        tools.push((Tool::Measure, "📏 Mesurer"));
        tools.push((Tool::North, "N  Orienter le Nord"));
        match source {
            MapSource::Image => tools.push((Tool::Scale, "↔ Calibrer l'échelle")),
            MapSource::Drawing => {
                tools.push((Tool::Boundary, "▭ Contour du terrain"));
                tools.push((Tool::Obstacle, "■ Obstacle"));
            }
        }
        for (t, label) in tools {
            ui.selectable_value(&mut app.tool, t, label).on_hover_text(t.hint());
        }
        if app.tool == Tool::Obstacle {
            combo(ui, "obstacle_kind", ObstacleKind::ALL, &mut app.obstacle_kind);
        }
        if app.tool != before {
            app.draft.clear();
            app.scale_points = None;
        }
        ui.separator();
        ui.horizontal(|ui| {
            if ui.add_enabled(app.can_undo(), egui::Button::new("Annuler")).on_hover_text("Annuler").clicked() {
                app.undo();
            }
            if ui.add_enabled(app.can_redo(), egui::Button::new("Rétablir")).on_hover_text("Rétablir").clicked() {
                app.redo();
            }
            if ui.button("🔍").on_hover_text("Recentrer la vue").clicked() {
                app.view.fitted = false;
            }
        });
        ui.separator();
        ui.label(RichText::new("Légende").strong());
        for s in Status::ALL {
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::splat(12.0), egui::Sense::hover());
                ui.painter().circle_filled(r.center(), 5.0, status_color(*s));
                ui.label(RichText::new(s.label()).color(status_color(*s)));
            });
        }
    });
}

pub fn side_panel(app: &mut App, ctx: &Context) {
    egui::SidePanel::right("side").resizable(true).default_width(400.0).min_width(320.0).show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut app.tab, Tab::Garden, "🏡 Jardin");
            ui.selectable_value(&mut app.tab, Tab::Zone, "📍 Zone");
            ui.selectable_value(&mut app.tab, Tab::Catalog, "🌿 Catalogue");
        });
        ui.separator();
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match app.tab {
            Tab::Garden => garden(app, ui),
            Tab::Zone => zone(app, ui, ctx),
            Tab::Catalog => catalog(app, ui, ctx),
        });
    });
}

// ---------------------------------------------------------------- Jardin

fn garden(app: &mut App, ui: &mut Ui) {
    let Some(doc) = app.doc.as_mut() else {
        ui.label("Créez ou ouvrez un projet depuis le menu Fichier.");
        return;
    };
    let mut relink = false;
    let mut tool = None;
    let p = &mut doc.project;
    let mpu = p.map.meters_per_unit();

    ui.heading("Projet");
    egui::Grid::new("project").num_columns(2).show(ui, |ui| {
        ui.label("Nom");
        ui.text_edit_singleline(&mut p.name);
        ui.end_row();
        ui.label("Fichier");
        ui.weak(doc.path.display().to_string());
        ui.end_row();
    });

    ui.add_space(8.0);
    ui.heading("Fond de plan");
    egui::Grid::new("map").num_columns(2).show(ui, |ui| {
        ui.label("Type");
        ui.label(p.map.source.label());
        ui.end_row();
        if p.map.source == MapSource::Image {
            ui.label("Image");
            ui.vertical(|ui| {
                ui.weak(p.map.image_path.clone().unwrap_or_else(|| "—".into()));
                relink = ui.button("Relier / changer l'image…").clicked();
            });
            ui.end_row();
        }
        ui.label("Attribution");
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut p.map.attribution);
            if ui.small_button("IGN").on_hover_text("© IGN – BD ORTHO").clicked() {
                p.map.attribution = "© IGN – BD ORTHO".into();
            }
        });
        ui.end_row();
        ui.label("Nord");
        let mut deg = p.map.north_angle_deg;
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut deg).speed(0.5).suffix(" °").max_decimals(1));
            if ui.small_button("Viser").clicked() {
                tool = Some(Tool::North);
            }
        });
        p.map.set_north(deg);
        ui.end_row();
        ui.label("Échelle");
        ui.horizontal(|ui| {
            match (p.map.source, mpu) {
                (MapSource::Drawing, _) => ui.label("1 unité = 1 m"),
                (_, Some(m)) => ui.label(format!("1 px = {:.3} m", m).replace('.', ",")),
                (_, None) => ui.colored_label(ORANGE, "Non calibrée"),
            };
            if p.map.source == MapSource::Image && ui.small_button("Calibrer").clicked() {
                tool = Some(Tool::Scale);
            }
        });
        ui.end_row();
    });

    if p.map.source == MapSource::Drawing {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(if p.map.boundary.is_some() { "Contour du terrain : tracé" } else { "Contour du terrain : non tracé" });
            if ui.small_button("Tracer").clicked() {
                tool = Some(Tool::Boundary);
            }
        });
        let mut remove = None;
        for (i, o) in p.map.obstacles.iter_mut().enumerate() {
            ui.push_id(i, |ui| {
                ui.horizontal(|ui| {
                    combo(ui, "k", ObstacleKind::ALL, &mut o.kind);
                    ui.label("H");
                    opt_f64(ui, &mut o.height_m, 2.0, (0.0, 60.0), " m");
                    if o.radius_m.is_some() {
                        ui.label("R");
                        opt_f64(ui, &mut o.radius_m, 2.0, (0.1, 30.0), " m");
                    }
                    if ui.small_button("🗑").clicked() {
                        remove = Some(i);
                    }
                });
            });
        }
        if let Some(i) = remove {
            p.map.obstacles.remove(i);
        }
    }

    ui.add_space(8.0);
    ui.heading("Contexte du jardin");
    let e = &mut p.environment;
    egui::Grid::new("env").num_columns(2).show(ui, |ui| {
        ui.label("Température hivernale minimale");
        opt_i32(ui, &mut e.min_winter_temp_c, -10, (-45, 15), " °C");
        ui.end_row();
        ui.label("Risque de sécheresse estivale");
        opt_combo(ui, "drought", Level::ALL, &mut e.summer_drought_risk);
        ui.end_row();
        ui.label("Type de sol");
        opt_combo(ui, "soil", SoilType::ALL, &mut e.soil_type);
        ui.end_row();
        ui.label("Drainage");
        opt_combo(ui, "drainage", Drainage::ALL, &mut e.soil_drainage);
        ui.end_row();
        ui.label("pH");
        opt_combo(ui, "ph", Ph::ALL, &mut e.soil_ph);
        ui.end_row();
        ui.label("Disponibilité d'arrosage");
        opt_combo(ui, "water", Level::ALL, &mut e.watering_availability);
        ui.end_row();
    });

    let outdated: Vec<String> =
        p.plants_snapshot.iter().filter(|(id, snap)| app.catalog.get(id).is_some_and(|c| c != *snap)).map(|(id, _)| id.clone()).collect();
    if !outdated.is_empty() {
        ui.add_space(8.0);
        ui.colored_label(ORANGE, format!("{} fiche(s) du projet diffèrent du catalogue.", outdated.len()));
        if ui.button("Mettre à jour depuis le catalogue").clicked() {
            for id in outdated {
                if let Some(c) = app.catalog.get(&id) {
                    p.plants_snapshot.insert(id, c.clone());
                }
            }
        }
    }

    if let Some(t) = tool {
        app.tool = t;
        app.draft.clear();
    }
    if relink {
        if let Some(path) = app.image_path() {
            app.forget_texture(&path);
        }
        app.relink_image();
    }
}

// ---------------------------------------------------------------- Zone

fn refresh_recs(app: &mut App, zone_id: &str, force: bool) {
    let Some(doc) = &app.doc else { return };
    let Some(zone) = doc.project.zone(zone_id) else { return };
    let env = &doc.project.environment;
    let mpu = doc.project.map.meters_per_unit();
    let stale = app.recs.as_ref().is_some_and(|r| {
        r.zone.id == zone_id
            && (r.zone != *zone || r.env != *env || r.mpu != mpu || r.catalog_rev != app.catalog_rev || r.filters != app.filters)
    });
    if !force && !stale {
        return;
    }
    let f = &app.filters;
    let plants = app
        .catalog
        .plants()
        .into_iter()
        .filter(|p| (!f.verified_only || p.meta.verified) && f.category.is_none_or(|c| p.category == Some(c)));
    let mut list = recommend(env, zone, plants, mpu);
    if f.hide_avoid {
        list.retain(|r| r.verdict != Verdict::Avoid);
    }
    app.recs =
        Some(RecCache { zone: zone.clone(), env: env.clone(), mpu, catalog_rev: app.catalog_rev, filters: app.filters.clone(), list });
}

fn plant_thumb(app: &mut App, ui: &mut Ui, ctx: &Context, plant: Option<&Plant>, custom: Option<std::path::PathBuf>, side: f32) {
    let path = custom.or_else(|| plant.and_then(|p| p.photo.as_ref()).map(|ph| app.photo_path(ph)));
    let tex = path.and_then(|p| app.texture(ctx, &p, 256));
    match tex {
        Some(t) => {
            let s = t.size_vec2();
            ui.image((t.id(), s * (side / s.x.max(s.y))));
        }
        None => {
            let label = plant.and_then(|p| p.category).map_or("🌱", |c| match c {
                Category::Tree => "🌳",
                Category::Shrub => "🌿",
                Category::Climber => "🍃",
                Category::VegetableHerb => "🌾",
                _ => "🌼",
            });
            ui.add_sized(Vec2::splat(side), egui::Label::new(RichText::new(label).size(side * 0.6)));
        }
    }
}

fn zone(app: &mut App, ui: &mut Ui, ctx: &Context) {
    let Some(id) = app.selected.clone() else {
        ui.label("Sélectionnez une zone sur la carte, ou créez-en une avec les outils de gauche.");
        return;
    };
    let Some(doc) = app.doc.as_mut() else { return };
    let mpu = doc.project.map.meters_per_unit();
    let project_file = doc.path.clone();
    let Some(z) = doc.project.zone_mut(&id) else {
        app.selected = None;
        return;
    };
    let (mut delete, mut unassign, mut pick_photo, mut run) = (false, false, false, false);

    ui.heading(format!("{} ({})", z.label, z.kind.label()));
    egui::Grid::new("zone").num_columns(2).show(ui, |ui| {
        ui.label("Libellé");
        ui.text_edit_singleline(&mut z.label);
        ui.end_row();
        ui.label("Statut");
        combo(ui, "status", Status::ALL, &mut z.status);
        ui.end_row();
        ui.label("Exposition");
        opt_combo(ui, "sun", Sun::ALL, &mut z.sun_exposure);
        ui.end_row();
        ui.label("Vent");
        opt_combo(ui, "wind", Wind::ALL, &mut z.wind_exposure);
        ui.end_row();
        ui.label("Humidité du sol");
        opt_combo(ui, "moist", Moisture::ALL, &mut z.moisture);
        ui.end_row();
        ui.label("Hauteur max");
        opt_f64(ui, &mut z.max_height_m, 2.0, (0.05, 60.0), " m");
        ui.end_row();
        match z.kind {
            ZoneKind::Point => {
                ui.label("Rayon disponible");
                opt_f64(ui, &mut z.available_radius_m, 1.0, (0.05, 30.0), " m");
                ui.end_row();
            }
            ZoneKind::Line => {
                ui.label("Largeur max");
                opt_f64(ui, &mut z.max_width_m, 1.0, (0.05, 20.0), " m");
                ui.end_row();
            }
            ZoneKind::Area => {}
        }
        ui.label("Feuillage souhaité");
        combo(ui, "fol", FoliagePref::ALL, &mut z.foliage_preference);
        ui.end_row();
        ui.label("Entretien maximum");
        combo(ui, "maint", MaxMaintenance::ALL, &mut z.max_maintenance);
        ui.end_row();
    });
    ui.label("Usages souhaités");
    checks(ui, Use::ALL, &mut z.desired_uses);
    ui.label("Notes");
    ui.add(egui::TextEdit::multiline(&mut z.notes).desired_rows(2).desired_width(f32::INFINITY));

    match (z.kind, zone_size_m(z, mpu)) {
        (ZoneKind::Area, Some(s)) => ui.label(format!("Surface : {s:.1} m²").replace('.', ",")),
        (ZoneKind::Line, Some(s)) => ui.label(format!("Longueur : {s:.1} m").replace('.', ",")),
        (ZoneKind::Point, _) => ui.label(""),
        (_, None) => ui.colored_label(ORANGE, "Échelle non calibrée : surface et quantités indisponibles."),
    };
    if z.kind == ZoneKind::Area && !is_simple_polygon(&z.points) {
        ui.colored_label(RED, "Polygone invalide (côtés croisés) : corrigez les sommets.");
    }

    ui.separator();
    ui.heading("Plante associée");
    let assigned = z.assigned_plant_id.clone();
    let custom = z.custom_photo_path.as_deref().map(|p| store::resolve(&project_file, p));
    let zone_copy = z.clone();
    let plant = assigned.as_deref().and_then(|pid| app.assigned_plant(pid)).cloned();
    ui.horizontal(|ui| {
        plant_thumb(app, ui, ctx, plant.as_ref(), custom, 96.0);
        ui.vertical(|ui| {
            match &plant {
                Some(p) => {
                    ui.label(RichText::new(p.display_name()).strong());
                    ui.label(RichText::new(&p.latin_name).italics());
                    if let Some(q) = quantity(&zone_copy, p, mpu) {
                        ui.label(format!("Quantité : {q}"));
                    }
                    unassign = ui.small_button("Retirer la plante").clicked();
                }
                None => {
                    ui.weak("Aucune plante assignée.");
                }
            }
            pick_photo = ui.small_button("Photo personnelle…").clicked();
        });
    });
    let mut qty = zone_copy.quantity_override;
    if zone_copy.kind != ZoneKind::Point {
        ui.horizontal(|ui| {
            ui.label("Quantité forcée");
            let mut on = qty.is_some();
            ui.checkbox(&mut on, "");
            if on {
                let mut v = qty.unwrap_or(1);
                ui.add(egui::DragValue::new(&mut v).speed(0.2));
                qty = Some(v.max(1));
            } else {
                qty = None;
            }
        });
    }
    ui.horizontal(|ui| {
        run = ui.button("▶ Lancer l'algorithme").clicked();
        if ui.button("🗑 Supprimer la zone").clicked() {
            delete = true;
        }
    });

    // Application des actions (l'emprunt de la zone est terminé)
    if let Some(z) = app.doc.as_mut().and_then(|d| d.project.zone_mut(&id)) {
        z.quantity_override = qty;
        if unassign {
            z.assigned_plant_id = None;
        }
    }
    if pick_photo {
        if let Some(img) = rfd::FileDialog::new().add_filter("Images", &["png", "jpg", "jpeg", "webp"]).pick_file() {
            if let Some(z) = app.doc.as_mut().and_then(|d| d.project.zone_mut(&id)) {
                z.custom_photo_path = Some(store::relativize(&project_file, &img));
            }
        }
    }
    if delete {
        if let Some(d) = app.doc.as_mut() {
            d.project.remove_zone(&id);
        }
        app.selected = None;
        return;
    }
    if zone_copy.status == Status::Existing {
        ui.weak("Zone existante : pas de recommandation.");
        return;
    }

    ui.separator();
    ui.heading("Recommandations");
    ui.horizontal_wrapped(|ui| {
        ui.checkbox(&mut app.filters.verified_only, "Fiches vérifiées");
        ui.checkbox(&mut app.filters.hide_avoid, "Masquer « Déconseillé »");
        opt_combo(ui, "cat_filter", Category::ALL, &mut app.filters.category);
    });
    refresh_recs(app, &id, run);
    let Some(recs) = app.recs.as_ref().filter(|r| r.zone.id == id) else {
        ui.weak("Cliquez sur « Lancer l'algorithme ».");
        return;
    };
    let list: Vec<_> = recs.list.iter().take(60).cloned().collect();
    if list.is_empty() {
        ui.weak("Aucune plante ne correspond aux filtres.");
    }
    let mut choose = None;
    for r in &list {
        let Some(p) = app.catalog.get(&r.plant_id).cloned() else { continue };
        ui.push_id(&r.plant_id, |ui| {
            let title =
                RichText::new(format!("{:>3}/100 · {} — {}", r.score, p.display_name(), r.verdict.label())).color(verdict_color(r.verdict));
            egui::CollapsingHeader::new(title).show(ui, |ui| {
                ui.horizontal(|ui| {
                    plant_thumb(app, ui, ctx, Some(&p), None, 64.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&p.latin_name).italics());
                        if let Some(c) = p.category {
                            ui.label(c.label());
                        }
                        if !p.meta.verified {
                            ui.colored_label(ORANGE, "Fiche non vérifiée");
                        }
                        if let Some(q) = r.quantity {
                            ui.label(format!("Quantité : {q}"));
                        }
                    });
                });
                for b in &r.breakdown {
                    ui.label(format!("{} : {}/{}", b.criterion.label(), b.points, b.max));
                }
                for reason in &r.reasons {
                    ui.colored_label(GREEN, format!("✔ {reason}"));
                }
                for a in &r.alerts {
                    let (icon, col) = match a.level {
                        AlertLevel::Blocking => ("✖", RED),
                        AlertLevel::Warning => ("⚠", ORANGE),
                        AlertLevel::Info => ("ℹ", Color32::GRAY),
                    };
                    ui.colored_label(col, format!("{icon} {}", a.message));
                }
                if ui.button("Choisir cette plante").clicked() {
                    choose = Some(p.clone());
                }
            });
        });
    }
    if let Some(p) = choose {
        if let Some(d) = app.doc.as_mut() {
            d.project.assign(&id, &p);
            app.info(format!("{} assignée à la zone.", p.display_name()));
        }
    }
}

// ---------------------------------------------------------------- Catalogue

fn catalog(app: &mut App, ui: &mut Ui, ctx: &Context) {
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(&mut app.query).hint_text("Nom commun ou latin").desired_width(200.0));
        if ui.button("Nouvelle fiche").clicked() {
            let mut plant = Plant { common_name: app.query.trim().to_owned(), ..Plant::default() };
            plant.meta.source = DataSource::Manual;
            app.editor = Some(Editor { plant, is_new: true });
        }
    });
    let can_search = !app.busy() && !app.query.trim().is_empty();
    if ui
        .add_enabled(can_search, egui::Button::new("🌐 Rechercher en ligne (Wikidata)"))
        .on_hover_text("Envoie le texte saisi à Wikidata et Wikipédia")
        .clicked()
    {
        app.search_online(ctx);
    }

    if !app.candidates.is_empty() {
        ui.add_space(6.0);
        ui.label(RichText::new("Résultats Wikidata").strong());
        let mut import = None;
        for c in &app.candidates {
            ui.horizontal(|ui| {
                if ui.add_enabled(!app.busy(), egui::Button::new("Importer")).clicked() {
                    import = Some(c.clone());
                }
                let name = c.common_name.as_deref().unwrap_or(&c.latin_name);
                ui.label(format!("{name} — {}", c.latin_name)).on_hover_text(c.description.clone().unwrap_or_default());
            });
        }
        if ui.small_button("Effacer les résultats").clicked() {
            app.candidates.clear();
        }
        if let Some(c) = import {
            app.import_candidate(ctx, c);
        }
        ui.separator();
    }

    let plants = app.catalog.plants();
    let shown: Vec<&Plant> = if app.query.trim().is_empty() {
        let mut v = plants;
        v.sort_by_cached_key(|p| normalize(&p.display_name()));
        v
    } else {
        search(plants, &app.query, 200)
    };
    ui.weak(format!("{} fiche(s)", shown.len()));
    let mut open = None;
    for p in shown {
        let badge = if p.meta.verified { "✔" } else { "⚠" };
        let text = format!("{badge} {} — {}", p.display_name(), p.latin_name);
        if ui.selectable_label(app.editor.as_ref().is_some_and(|e| e.plant.id == p.id), text).clicked() {
            open = Some(p.clone());
        }
    }
    if let Some(plant) = open {
        app.editor = Some(Editor { plant, is_new: false });
    }
}

// ---------------------------------------------------------------- Fenêtres

pub fn windows(app: &mut App, ctx: &Context) {
    editor_window(app, ctx);
    scale_window(app, ctx);
    egui::Window::new("Obtenir une vue aérienne IGN").open(&mut app.show_help).default_width(460.0).show(ctx, |ui| {
        ui.label("1. Sur geoportail.gouv.fr, recherchez votre adresse et affichez la couche « Photographies aériennes ».");
        ui.label("2. Zoomez pour que le jardin occupe la plus grande partie de l'écran, avec une marge.");
        ui.label("3. Gardez le Nord en haut (orientation par défaut).");
        ui.label("4. Avec l'outil de mesure, mesurez un élément net (mur, clôture) et notez la longueur.");
        ui.label("5. Capturez l'image (Windows + Maj + S) et enregistrez-la en PNG.");
        ui.label("6. Dans PlotFlora : Fichier › Nouveau projet (image aérienne), puis calibrez l'échelle avec la longueur notée.");
        ui.add_space(6.0);
        ui.weak("Les orthophotos IGN (BD ORTHO) sont sous Licence Ouverte Etalab 2.0 : citez « © IGN – BD ORTHO ». Une capture Google Maps est tolérée pour un usage strictement personnel.");
        ui.hyperlink_to("Ouvrir Géoportail", "https://www.geoportail.gouv.fr/");
    });
    let mut show = app.show_credits;
    let logo = app.logo.as_ref().map(|t| (t.id(), t.size_vec2()));
    egui::Window::new("À propos & licences").open(&mut show).default_width(480.0).vscroll(true).show(ctx, |ui| {
        if let Some((id, size)) = logo {
            ui.image((id, size * (300.0 / size.x)));
        }
        ui.label(RichText::new(format!("PlotFlora {}", env!("CARGO_PKG_VERSION"))).strong());
        ui.label("Copyright © 2026 ChatCarbo");
        ui.horizontal_wrapped(|ui| {
            ui.label("Logiciel libre distribué sous");
            ui.hyperlink_to("licence GNU GPL version 3 ou ultérieure", "https://www.gnu.org/licenses/gpl-3.0.html");
            ui.label(", sans aucune garantie.");
        });
        ui.add_space(6.0);
        ui.label(RichText::new("Les données gardent leurs propres licences :").strong());
        ui.hyperlink_to("IGN – BD ORTHO (Licence Ouverte Etalab 2.0)", "https://geoservices.ign.fr/bdortho");
        ui.hyperlink_to("Wikidata (CC0 1.0)", "https://www.wikidata.org/");
        ui.hyperlink_to("EIVE – Tichý et al. 2023 (CC BY 4.0)", "https://doi.org/10.5281/zenodo.7427088");
        ui.hyperlink_to("Wikipédia (CC BY-SA 4.0) — valeurs factuelles uniquement", "https://fr.wikipedia.org/");
        ui.add_space(6.0);
        ui.label(RichText::new("Photos").strong());
        let mut any = false;
        for p in app.catalog.plants() {
            let Some(ph) = &p.photo else { continue };
            if ph.source_url.is_none() && ph.author.is_none() {
                continue;
            }
            any = true;
            let credit = format!(
                "{} — {} ({})",
                p.display_name(),
                ph.author.as_deref().unwrap_or("auteur inconnu"),
                ph.license.as_deref().unwrap_or("licence inconnue")
            );
            match &ph.source_url {
                Some(u) => ui.hyperlink_to(credit, u),
                None => ui.label(credit),
            };
        }
        if !any {
            ui.weak("Aucune photo issue de Wikimedia Commons.");
        }
    });
    app.show_credits = show;
}

fn scale_window(app: &mut App, ctx: &Context) {
    let Some((a, b)) = app.scale_points else { return };
    let mut keep = true;
    egui::Window::new("Calibrer l'échelle")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_TOP, Vec2::new(0.0, 60.0))
        .show(ctx, |ui| {
            ui.label(format!("Segment tracé : {:.0} px", geometry::dist(a, b)));
            ui.horizontal(|ui| {
                ui.label("Longueur réelle");
                ui.add(egui::DragValue::new(&mut app.scale_distance).speed(0.05).suffix(" m").max_decimals(2));
            });
            app.scale_distance = app.scale_distance.clamp(0.01, 10_000.0);
            ui.horizontal(|ui| {
                if ui.button("Valider").clicked() {
                    if let Some(d) = app.doc.as_mut() {
                        d.project.map.scale = Some(Scale { p1: a, p2: b, distance_m: app.scale_distance });
                    }
                    keep = false;
                    app.tool = Tool::Select;
                    app.info("Échelle calibrée.");
                }
                if ui.button("Annuler").clicked() {
                    keep = false;
                }
            });
        });
    if !keep {
        app.scale_points = None;
    }
}

/// Libellé de champ, marqué ⚠ si la valeur est à vérifier.
fn field(ui: &mut Ui, text: &str, p: &Plant, key: &str) {
    if p.meta.uncertain_fields.iter().any(|f| f == key) {
        let src = p.meta.field_sources.get(key).map_or_else(|| "source inconnue".to_owned(), |s| format!("source : {s}"));
        ui.label(RichText::new(format!("{text} ⚠")).color(ORANGE)).on_hover_text(format!("À vérifier ({src})"));
    } else {
        ui.label(text);
    }
}

fn editor_window(app: &mut App, ctx: &Context) {
    let Some(mut ed) = app.editor.take() else { return };
    #[derive(PartialEq)]
    enum Action {
        None,
        Save,
        Validate,
        Assign,
        Delete,
        Photo,
    }
    let mut action = Action::None;
    let mut open = true;
    let is_user = app.catalog.is_user(&ed.plant.id);
    let zone_selected = app.selected.is_some() && app.doc.is_some();
    egui::Window::new("Fiche plante").open(&mut open).default_width(480.0).default_height(620.0).vscroll(true).show(ctx, |ui| {
        let p = &mut ed.plant;
        ui.horizontal(|ui| {
            plant_thumb(app, ui, ctx, Some(&*p), None, 96.0);
            ui.vertical(|ui| {
                ui.label(RichText::new(p.display_name()).heading());
                ui.weak(if p.id.is_empty() { "Identifiant attribué à l'enregistrement".to_owned() } else { format!("id : {}", p.id) });
                if p.meta.verified {
                    ui.colored_label(GREEN, "✔ Fiche vérifiée");
                } else {
                    ui.colored_label(ORANGE, "⚠ Fiche non vérifiée");
                }
                if ui.small_button("Choisir une photo…").clicked() {
                    action = Action::Photo;
                }
                if let Some(ph) = &p.photo {
                    let credit = [ph.author.as_deref(), ph.license.as_deref()].into_iter().flatten().collect::<Vec<_>>().join(" — ");
                    if !credit.is_empty() {
                        ui.weak(credit);
                    }
                }
            });
        });
        egui::Grid::new("plant").num_columns(2).striped(true).show(ui, |ui| {
            field(ui, "Nom commun", p, "common_name");
            ui.text_edit_singleline(&mut p.common_name);
            ui.end_row();
            field(ui, "Nom latin", p, "latin_name");
            ui.text_edit_singleline(&mut p.latin_name);
            ui.end_row();
            ui.label("Cultivar");
            opt_text(ui, &mut p.cultivar);
            ui.end_row();
            field(ui, "Catégorie", p, "category");
            opt_combo(ui, "cat", Category::ALL, &mut p.category);
            ui.end_row();
            field(ui, "Rusticité", p, "climate.min_temp_c");
            opt_i32(ui, &mut p.climate.min_temp_c, -15, (-50, 15), " °C");
            ui.end_row();
            field(ui, "Tolérance sécheresse", p, "climate.drought_tolerance");
            opt_combo(ui, "dt", Level::ALL, &mut p.climate.drought_tolerance);
            ui.end_row();
            field(ui, "Tolérance au vent", p, "climate.wind_tolerance");
            opt_combo(ui, "wt", Level::ALL, &mut p.climate.wind_tolerance);
            ui.end_row();
            field(ui, "Hauteur adulte", p, "dimensions.mature_height_m");
            opt_f64(ui, &mut p.dimensions.mature_height_m, 1.0, (0.01, 80.0), " m");
            ui.end_row();
            field(ui, "Envergure adulte", p, "dimensions.mature_spread_m");
            opt_f64(ui, &mut p.dimensions.mature_spread_m, 1.0, (0.01, 50.0), " m");
            ui.end_row();
            ui.label("Espacement de plantation");
            opt_f64(ui, &mut p.dimensions.planting_spacing_m, 0.5, (0.05, 30.0), " m");
            ui.end_row();
            ui.label("Croissance");
            opt_combo(ui, "gr", GrowthRate::ALL, &mut p.dimensions.growth_rate);
            ui.end_row();
            ui.label("Racines");
            opt_combo(ui, "rs", RootSystem::ALL, &mut p.dimensions.root_system);
            ui.end_row();
            ui.label("Feuillage");
            opt_combo(ui, "fo", Foliage::ALL, &mut p.characteristics.foliage);
            ui.end_row();
            ui.label("Entretien");
            opt_combo(ui, "ml", Maintenance::ALL, &mut p.care.maintenance_level);
            ui.end_row();
            ui.label("Besoin en eau");
            opt_combo(ui, "wn", WaterNeed::ALL, &mut p.care.water_need);
            ui.end_row();
            field(ui, "Sans danger (animaux)", p, "care.non_toxic_pets");
            opt_bool(ui, "tox", &mut p.care.non_toxic_pets);
            ui.end_row();
            ui.label("Épines");
            opt_bool(ui, "thorn", &mut p.care.has_thorns);
            ui.end_row();
            ui.label("Couleur des fleurs");
            opt_text(ui, &mut p.characteristics.flower_color);
            ui.end_row();
            ui.label("Couleur d'automne");
            opt_text(ui, &mut p.characteristics.autumn_color);
            ui.end_row();
        });
        field(ui, "Exposition idéale", p, "exposure.ideal");
        checks(ui, Sun::ALL, &mut p.exposure.ideal);
        field(ui, "Exposition tolérée", p, "exposure.tolerated");
        checks(ui, Sun::ALL, &mut p.exposure.tolerated);
        ui.label("Types de sol");
        checks(ui, SoilType::ALL, &mut p.soil.types);
        ui.label("Drainage");
        checks(ui, Drainage::ALL, &mut p.soil.drainage);
        field(ui, "pH", p, "soil.ph");
        checks(ui, Ph::ALL, &mut p.soil.ph);
        field(ui, "Humidité", p, "soil.moisture");
        checks(ui, Moisture::ALL, &mut p.soil.moisture);
        ui.label("Usages");
        checks(ui, Use::ALL, &mut p.characteristics.uses);
        ui.label("Saisons d'intérêt");
        checks(ui, Season::ALL, &mut p.characteristics.interest_seasons);
        ui.label("Floraison");
        months(ui, &mut p.characteristics.flowering_months);
        ui.label("Taille");
        months(ui, &mut p.care.pruning_months);
        ui.label("Notes");
        ui.add(egui::TextEdit::multiline(&mut p.notes).desired_rows(3).desired_width(f32::INFINITY));
        let mut tags = p.tags.join(", ");
        ui.horizontal(|ui| {
            ui.label("Tags");
            if ui.text_edit_singleline(&mut tags).changed() {
                p.tags = tags.split(',').map(|t| t.trim().to_owned()).filter(|t| !t.is_empty()).collect();
            }
        });
        if p.links.wikidata.is_some() || p.links.wikipedia_fr.is_some() {
            ui.horizontal(|ui| {
                if let Some(q) = &p.links.wikidata {
                    ui.hyperlink_to("Wikidata", format!("https://www.wikidata.org/wiki/{q}"));
                }
                if let Some(t) = &p.links.wikipedia_fr {
                    ui.hyperlink_to("Wikipédia", format!("https://fr.wikipedia.org/wiki/{}", t.replace(' ', "_")));
                }
            });
        }
        ui.weak(format!(
            "Source : {} · mise à jour : {}",
            p.meta.source.label(),
            if p.meta.updated_at.is_empty() { "—" } else { p.meta.updated_at.as_str() }
        ));
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            if ui.button("💾 Enregistrer").clicked() {
                action = Action::Save;
            }
            if ui.button("✔ Valider la fiche").on_hover_text("Marque la fiche comme vérifiée").clicked() {
                action = Action::Validate;
            }
            if ui.add_enabled(zone_selected, egui::Button::new("📍 Assigner à la zone")).clicked() {
                action = Action::Assign;
            }
            if is_user && ui.button("🗑 Supprimer du catalogue").clicked() {
                action = Action::Delete;
            }
        });
    });

    match action {
        Action::None => {}
        Action::Photo => {
            if let Some(src) = rfd::FileDialog::new().add_filter("Images", &["png", "jpg", "jpeg", "webp"]).pick_file() {
                let stem =
                    if ed.plant.id.is_empty() { plant_id(&ed.plant.latin_name, ed.plant.cultivar.as_deref()) } else { ed.plant.id.clone() };
                let stem = if stem.is_empty() { "nouvelle_plante".to_owned() } else { stem };
                let ext = src.extension().map_or_else(|| "jpg".to_owned(), |e| e.to_string_lossy().to_lowercase());
                let name = format!("{stem}.{ext}");
                let dir = app.data_dir.join("photos");
                let dest = dir.join(&name);
                match std::fs::create_dir_all(&dir).and_then(|()| std::fs::copy(&src, &dest)) {
                    Ok(_) => {
                        app.forget_texture(&dest);
                        ed.plant.photo =
                            Some(Photo { file: format!("photos/{name}"), license: Some("Photo personnelle".into()), ..Photo::default() });
                    }
                    Err(e) => app.error(format!("Copie de la photo impossible : {e}")),
                }
            }
        }
        Action::Delete => {
            app.catalog.remove_user(&ed.plant.id);
            app.catalog_rev += 1;
            match app.catalog.save() {
                Ok(()) => app.info("Fiche supprimée du catalogue utilisateur."),
                Err(e) => app.error(e.to_string()),
            }
            return;
        }
        Action::Save | Action::Validate | Action::Assign => {
            let p = &mut ed.plant;
            if p.common_name.trim().is_empty() || p.latin_name.trim().is_empty() {
                app.error("Le nom commun et le nom latin sont obligatoires.");
            } else if action == Action::Validate && p.category.is_none() {
                app.error("Choisissez une catégorie avant de valider la fiche.");
            } else {
                if p.id.is_empty() || (ed.is_new && app.catalog.contains(&p.id)) {
                    let base = plant_id(&p.latin_name, p.cultivar.as_deref());
                    p.id = unique_id(&base, |s| app.catalog.contains(s));
                }
                if action == Action::Validate {
                    p.meta.verified = true;
                    p.meta.uncertain_fields.clear();
                }
                p.meta.updated_at = today();
                ed.is_new = false;
                app.save_plant(p.clone());
                if action == Action::Assign {
                    if let (Some(id), Some(d)) = (app.selected.clone(), app.doc.as_mut()) {
                        d.project.assign(&id, p);
                        app.info(format!("{} assignée à la zone.", p.display_name()));
                    }
                }
            }
        }
    }
    if open {
        app.editor = Some(ed);
    }
}
