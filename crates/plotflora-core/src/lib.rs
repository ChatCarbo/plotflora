// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 ChatCarbo

//! Cœur métier de PlotFlora : vocabulaire contrôlé, modèle, géométrie,
//! recherche, conversions et scoring. Aucune entrée/sortie.
#![forbid(unsafe_code)]
#![warn(missing_debug_implementations)]

pub mod convert;
pub mod geometry;
pub mod model;
pub mod scoring;
pub mod search;
pub mod vocab;

pub use model::*;
pub use vocab::*;
