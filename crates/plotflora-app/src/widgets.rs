// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 ChatCarbo

//! Petits éditeurs réutilisables pour les champs du modèle.

use eframe::egui::{self, Ui};
use std::fmt::Display;

pub const UNKNOWN: &str = "Inconnu";

/// Liste déroulante pour une valeur facultative (« Inconnu » = `None`).
pub fn opt_combo<T: Copy + PartialEq + Display>(ui: &mut Ui, id: &str, all: &[T], v: &mut Option<T>) {
    egui::ComboBox::new(id, "").selected_text(v.map_or_else(|| UNKNOWN.to_owned(), |x| x.to_string())).show_ui(ui, |ui| {
        ui.selectable_value(v, None, UNKNOWN);
        for a in all {
            ui.selectable_value(v, Some(*a), a.to_string());
        }
    });
}

pub fn combo<T: Copy + PartialEq + Display>(ui: &mut Ui, id: &str, all: &[T], v: &mut T) {
    egui::ComboBox::new(id, "").selected_text(v.to_string()).show_ui(ui, |ui| {
        for a in all {
            ui.selectable_value(v, *a, a.to_string());
        }
    });
}

/// Cases à cocher pour un ensemble de valeurs (liste maintenue triée).
pub fn checks<T: Copy + Ord + Display>(ui: &mut Ui, all: &[T], v: &mut Vec<T>) {
    ui.horizontal_wrapped(|ui| {
        for a in all {
            let mut on = v.contains(a);
            if ui.checkbox(&mut on, a.to_string()).changed() {
                if on {
                    v.push(*a);
                    v.sort();
                    v.dedup();
                } else {
                    v.retain(|x| x != a);
                }
            }
        }
    });
}

/// Nombre facultatif : case « renseigné » + valeur bornée.
pub fn opt_f64(ui: &mut Ui, v: &mut Option<f64>, default: f64, range: (f64, f64), suffix: &str) {
    ui.horizontal(|ui| {
        let mut on = v.is_some();
        ui.checkbox(&mut on, "");
        if on {
            let mut x = v.unwrap_or(default);
            ui.add(egui::DragValue::new(&mut x).speed(0.05).suffix(suffix).max_decimals(2));
            *v = Some(if x.is_finite() { x.clamp(range.0, range.1) } else { default });
        } else {
            *v = None;
            ui.weak(UNKNOWN);
        }
    });
}

pub fn opt_i32(ui: &mut Ui, v: &mut Option<i32>, default: i32, range: (i32, i32), suffix: &str) {
    ui.horizontal(|ui| {
        let mut on = v.is_some();
        ui.checkbox(&mut on, "");
        if on {
            let mut x = v.unwrap_or(default);
            ui.add(egui::DragValue::new(&mut x).speed(0.2).suffix(suffix));
            *v = Some(x.clamp(range.0, range.1));
        } else {
            *v = None;
            ui.weak(UNKNOWN);
        }
    });
}

pub fn opt_bool(ui: &mut Ui, id: &str, v: &mut Option<bool>) {
    let text = match v {
        Some(true) => "Oui",
        Some(false) => "Non",
        None => UNKNOWN,
    };
    egui::ComboBox::new(id, "").selected_text(text).show_ui(ui, |ui| {
        ui.selectable_value(v, None, UNKNOWN);
        ui.selectable_value(v, Some(true), "Oui");
        ui.selectable_value(v, Some(false), "Non");
    });
}

/// Mois (1–12) sous forme de cases.
pub fn months(ui: &mut Ui, v: &mut Vec<u8>) {
    const M: [&str; 12] = ["J", "F", "M", "A", "M", "J", "J", "A", "S", "O", "N", "D"];
    ui.horizontal_wrapped(|ui| {
        for (i, name) in M.iter().enumerate() {
            let m = i as u8 + 1;
            let mut on = v.contains(&m);
            if ui.toggle_value(&mut on, *name).changed() {
                if on {
                    v.push(m);
                    v.sort_unstable();
                    v.dedup();
                } else {
                    v.retain(|x| *x != m);
                }
            }
        }
    });
}

/// Texte facultatif : vide = `None`.
pub fn opt_text(ui: &mut Ui, v: &mut Option<String>) {
    let mut s = v.clone().unwrap_or_default();
    if ui.text_edit_singleline(&mut s).changed() {
        *v = (!s.trim().is_empty()).then_some(s);
    }
}
