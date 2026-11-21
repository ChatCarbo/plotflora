// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 ChatCarbo

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![forbid(unsafe_code)]

mod app;
mod canvas;
mod panels;
mod widgets;

/// Icône de la fenêtre (barre de titre et barre des tâches).
fn window_icon() -> eframe::egui::IconData {
    image::load_from_memory(include_bytes!("../../../assets/icon.png"))
        .map(|img| {
            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();
            eframe::egui::IconData { rgba: rgba.into_raw(), width, height }
        })
        .unwrap_or_default()
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1360.0, 860.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("PlotFlora")
            .with_icon(window_icon()),
        ..Default::default()
    };
    eframe::run_native("PlotFlora", options, Box::new(|cc| Ok(Box::new(app::App::new(cc)))))
}
