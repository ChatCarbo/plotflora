// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 ChatCarbo

//! État de l'application, opérations fichiers, historique et tâches en ligne.

use crate::panels;
use eframe::egui::{self, Key, KeyboardShortcut, Modifiers, TextureHandle, Vec2};
use plotflora_core::scoring::Recommendation;
use plotflora_core::*;
use plotflora_data::cache::LookupCache;
use plotflora_data::catalog::CatalogStore;
use plotflora_data::eive::EiveIndex;
use plotflora_data::online::{Candidate, Online};
use plotflora_data::store;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};

const UNDO_LIMIT: usize = 100;

pub struct Doc {
    pub project: Project,
    pub path: PathBuf,
    pub saved: Project,
}

impl Doc {
    pub fn dirty(&self) -> bool {
        self.project != self.saved
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    Point,
    Area,
    Line,
    Boundary,
    Obstacle,
    Scale,
    Measure,
    North,
    Detect,
    Sector,
}

impl Tool {
    pub fn hint(self) -> &'static str {
        match self {
            Self::Select => {
                "Clic : sélectionner · glisser : déplacer une zone ou un sommet · molette : zoom · clic milieu : déplacer la vue"
            }
            Self::Point => "Clic : placer une plante isolée",
            Self::Area | Self::Boundary => {
                "Clics : sommets · double-clic ou Entrée : terminer · Retour arrière : annuler le dernier · Échap : abandonner"
            }
            Self::Line => "Clics : points de la haie · double-clic ou Entrée : terminer · Échap : abandonner",
            Self::Obstacle => "Clics : contour de l'obstacle (un clic suffit pour un arbre existant) · Entrée : terminer",
            Self::Scale => "Cliquez les deux extrémités d'une longueur connue",
            Self::Measure => "Cliquez deux points pour mesurer",
            Self::North => "Cliquez dans la direction du Nord (depuis le centre de la vue)",
            Self::Detect => "Cliquez au milieu du jardin : son contour est détecté automatiquement (réglez la tolérance si besoin)",
            Self::Sector => "Clics : contour du secteur de sol · double-clic ou Entrée : terminer · Échap : abandonner",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Garden,
    Zone,
    Catalog,
    Ideas,
}

/// Réglages de l'onglet « Idées ».
#[derive(Debug, Clone, PartialEq, Default)]
pub struct IdeasQuery {
    pub ambiance: Option<Ambiance>,
    pub sun: Option<Sun>,
    pub sector: Option<String>,
    pub category: Option<Category>,
    pub max_height_m: Option<f64>,
}

/// Résultat mis en cache de l'onglet « Idées ».
pub struct IdeasCache {
    pub key: (IdeasQuery, Environment, u64),
    pub list: Vec<Recommendation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drag {
    None,
    Pan,
    Zone,
    Vertex(usize),
    Boundary(usize),
}

#[derive(Debug, Clone, Copy)]
pub struct View {
    pub zoom: f32,
    pub offset: Vec2,
    pub fitted: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Filters {
    pub verified_only: bool,
    pub hide_avoid: bool,
    pub category: Option<Category>,
}

/// Résultats de l'algorithme et contexte qui les a produits (recalcul si le contexte change).
pub struct RecCache {
    pub zone: Zone,
    pub env: Environment,
    pub mpu: Option<f64>,
    pub catalog_rev: u64,
    pub filters: Filters,
    pub list: Vec<Recommendation>,
}

pub struct Editor {
    pub plant: Plant,
    pub is_new: bool,
}

/// Image réduite gardée en mémoire pour la détection du contour.
struct DetectCache {
    path: PathBuf,
    w: usize,
    h: usize,
    kx: f64,
    ky: f64,
    rgb: Vec<u8>,
}

pub enum OnlineMsg {
    Candidates(Result<Vec<Candidate>, String>),
    Draft(Box<Plant>, Vec<String>),
}

pub struct App {
    pub doc: Option<Doc>,
    pub catalog: CatalogStore,
    pub catalog_rev: u64,
    pub online: Online,
    pub eive: Arc<EiveIndex>,
    pub data_dir: PathBuf,
    pub tool: Tool,
    pub obstacle_kind: ObstacleKind,
    pub draft: Vec<Pt>,
    pub selected: Option<String>,
    pub drag: Drag,
    pub view: View,
    textures: HashMap<(PathBuf, u32), Option<TextureHandle>>,
    undo: Vec<Project>,
    redo: Vec<Project>,
    committed: Option<Project>,
    pub tab: Tab,
    pub recs: Option<RecCache>,
    pub filters: Filters,
    pub query: String,
    pub editor: Option<Editor>,
    pub candidates: Vec<Candidate>,
    task: Option<mpsc::Receiver<OnlineMsg>>,
    pub status: String,
    pub status_error: bool,
    pub show_help: bool,
    pub show_credits: bool,
    pub confirm_close: bool,
    allow_close: bool,
    pub scale_points: Option<(Pt, Pt)>,
    /// Télécharger la photo Commons d'une plante quand elle est ajoutée au jardin.
    pub auto_photos: bool,
    photo_queue: Vec<(String, String)>,
    photo_task: Option<mpsc::Receiver<(String, Result<Photo, String>)>>,
    pub ideas: IdeasQuery,
    pub ideas_cache: Option<IdeasCache>,
    pub scale_distance: f64,
    pub detect_tolerance: f64,
    detect_cache: Option<DetectCache>,
    title: String,
    pub icon: Option<TextureHandle>,
    pub logo: Option<TextureHandle>,
}

fn pick_image() -> Option<PathBuf> {
    rfd::FileDialog::new().set_title("Choisir une image").add_filter("Images", &["png", "jpg", "jpeg", "webp"]).pick_file()
}

fn copy_into(dir: &Path, src: &Path) -> std::io::Result<PathBuf> {
    let name = src.file_name().ok_or_else(|| std::io::Error::other("nom de fichier invalide"))?;
    let mut dest = dir.join(name);
    if dest == src {
        return Ok(dest);
    }
    let stem = src.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = src.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    let mut n = 2;
    while dest.exists() {
        dest = dir.join(format!("{stem} ({n}){ext}"));
        n += 1;
    }
    std::fs::copy(src, &dest)?;
    Ok(dest)
}

/// Image PNG intégrée au binaire → texture.
fn embedded_texture(ctx: &egui::Context, name: &str, png: &[u8]) -> Option<TextureHandle> {
    let rgba = image::load_from_memory(png).ok()?.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    Some(ctx.load_texture(name, egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()), egui::TextureOptions::LINEAR))
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let data_dir = plotflora_data::app_data_dir();
        let mut status = String::new();
        let catalog = CatalogStore::load(&data_dir).unwrap_or_else(|e| {
            status = format!("Catalogue utilisateur illisible, catalogue vide utilisé : {e}");
            CatalogStore::empty(&data_dir)
        });
        let cache = Arc::new(Mutex::new(LookupCache::load(&data_dir.join("lookup_cache.json"))));
        Self {
            doc: None,
            catalog,
            catalog_rev: 0,
            online: Online::new(cache),
            eive: Arc::new(EiveIndex::embedded()),
            status_error: !status.is_empty(),
            status,
            data_dir,
            tool: Tool::Select,
            obstacle_kind: ObstacleKind::House,
            draft: Vec::new(),
            selected: None,
            drag: Drag::None,
            view: View { zoom: 1.0, offset: Vec2::ZERO, fitted: false },
            textures: HashMap::new(),
            undo: Vec::new(),
            redo: Vec::new(),
            committed: None,
            tab: Tab::Garden,
            recs: None,
            filters: Filters { hide_avoid: true, ..Filters::default() },
            query: String::new(),
            editor: None,
            candidates: Vec::new(),
            task: None,
            show_help: false,
            show_credits: false,
            confirm_close: false,
            allow_close: false,
            scale_points: None,
            auto_photos: true,
            photo_queue: Vec::new(),
            photo_task: None,
            ideas: IdeasQuery::default(),
            ideas_cache: None,
            scale_distance: 10.0,
            detect_tolerance: 40.0,
            detect_cache: None,
            title: String::new(),
            icon: embedded_texture(&cc.egui_ctx, "icon", include_bytes!("../../../assets/icon.png")),
            logo: embedded_texture(&cc.egui_ctx, "logo", include_bytes!("../../../assets/logo_dark.png")),
        }
    }

    pub fn info(&mut self, msg: impl Into<String>) {
        self.status = msg.into();
        self.status_error = false;
    }

    pub fn error(&mut self, msg: impl Into<String>) {
        self.status = msg.into();
        self.status_error = true;
    }

    // ---------- Fichiers ----------

    fn confirm_discard(&mut self) -> bool {
        if !self.doc.as_ref().is_some_and(Doc::dirty) {
            return true;
        }
        let r = rfd::MessageDialog::new()
            .set_title("Modifications non enregistrées")
            .set_description("Le projet a été modifié. Abandonner les modifications ?")
            .set_buttons(rfd::MessageButtons::YesNo)
            .show();
        r == rfd::MessageDialogResult::Yes
    }

    fn set_doc(&mut self, project: Project, path: PathBuf) {
        self.doc = Some(Doc { saved: project.clone(), project, path });
        self.undo.clear();
        self.redo.clear();
        self.committed = None;
        self.selected = None;
        self.draft.clear();
        self.recs = None;
        self.drag = Drag::None;
        self.view.fitted = false;
        self.scale_points = None;
    }

    pub fn new_project(&mut self, source: MapSource) {
        if !self.confirm_discard() {
            return;
        }
        let image = match source {
            MapSource::Image => match pick_image() {
                Some(p) => Some(p),
                None => return,
            },
            MapSource::Drawing => None,
        };
        let Some(path) = rfd::FileDialog::new()
            .set_title("Enregistrer le nouveau projet")
            .add_filter("Projet PlotFlora", &["json"])
            .set_file_name("projet.json")
            .save_file()
        else {
            return;
        };
        let name = path.file_stem().map_or_else(|| "Mon jardin".into(), |s| s.to_string_lossy().into_owned());
        let mut project = Project::new(&name, source);
        if let Some(img) = image {
            let dir = path.parent().unwrap_or(Path::new("."));
            match copy_into(dir, &img) {
                Ok(dest) => project.map.image_path = Some(store::relativize(&path, &dest)),
                Err(e) => return self.error(format!("Copie de l'image impossible : {e}")),
            }
        }
        match store::save_project(&path, &mut project) {
            Ok(()) => {
                self.set_doc(project, path);
                self.tool = if source == MapSource::Drawing { Tool::Boundary } else { Tool::Detect };
                self.tab = Tab::Garden;
                self.info(match source {
                    MapSource::Image => "Projet créé. Cliquez au milieu du jardin pour détecter ses limites (ou tracez le contour), puis calibrez l'échelle.",
                    MapSource::Drawing => "Projet créé. Tracez le contour du terrain.",
                });
            }
            Err(e) => self.error(e.to_string()),
        }
    }

    pub fn open_project(&mut self) {
        if !self.confirm_discard() {
            return;
        }
        let Some(path) = rfd::FileDialog::new().add_filter("Projet PlotFlora", &["json"]).pick_file() else { return };
        match store::load_project(&path) {
            Ok(p) => {
                self.set_doc(p, path);
                self.info("Projet ouvert.");
            }
            Err(e) => self.error(e.to_string()),
        }
    }

    pub fn save(&mut self) -> bool {
        let Some(doc) = self.doc.as_mut() else { return false };
        match store::save_project(&doc.path, &mut doc.project) {
            Ok(()) => {
                doc.saved = doc.project.clone();
                self.committed = Some(doc.project.clone());
                self.info("Projet enregistré.");
                true
            }
            Err(e) => {
                self.error(e.to_string());
                false
            }
        }
    }

    pub fn save_as(&mut self) {
        let Some(doc) = self.doc.as_mut() else { return };
        let Some(new_path) = rfd::FileDialog::new().add_filter("Projet PlotFlora", &["json"]).set_file_name("projet.json").save_file()
        else {
            return;
        };
        let rebase = |s: &mut Option<String>| {
            if let Some(v) = s.as_mut() {
                *v = store::relativize(&new_path, &store::resolve(&doc.path, v));
            }
        };
        rebase(&mut doc.project.map.image_path);
        for z in &mut doc.project.zones {
            rebase(&mut z.custom_photo_path);
        }
        doc.path = new_path;
        self.save();
    }

    /// Détecte le contour du jardin autour du point `p` (pixels de l'image) et recadre la vue.
    pub fn detect_garden(&mut self, p: Pt) {
        let Some(path) = self.image_path() else { return };
        if self.detect_cache.as_ref().map(|c| &c.path) != Some(&path) {
            let img = match image::open(&path) {
                Ok(i) => i,
                Err(e) => return self.error(format!("Image illisible : {e}")),
            };
            let (ow, oh) = (img.width(), img.height());
            let small = if ow.max(oh) > 640 { img.thumbnail(640, 640) } else { img };
            let rgb = small.to_rgb8();
            let (w, h) = (rgb.width(), rgb.height());
            self.detect_cache = Some(DetectCache {
                path,
                w: w as usize,
                h: h as usize,
                kx: f64::from(ow) / f64::from(w.max(1)),
                ky: f64::from(oh) / f64::from(h.max(1)),
                rgb: rgb.into_raw(),
            });
        }
        let Some((kx, ky)) = self.detect_cache.as_ref().map(|c| (c.kx, c.ky)) else { return };
        let (sx, sy) = (p[0] / kx, p[1] / ky);
        if !(sx >= 0.0 && sy >= 0.0) {
            return self.error("Cliquez à l'intérieur de l'image.");
        }
        let res = match self.detect_cache.as_ref() {
            Some(c) => plotflora_core::detect::detect_region(c.w, c.h, &c.rgb, (sx as usize, sy as usize), self.detect_tolerance),
            None => return,
        };
        match res {
            Ok(poly) => {
                if let Some(d) = self.doc.as_mut() {
                    d.project.map.boundary = Some(poly.into_iter().map(|q| [q[0] * kx, q[1] * ky]).collect());
                }
                self.view.fitted = false;
                self.tool = Tool::Select;
                self.info("Contour du jardin détecté. Ajustez les sommets blancs si besoin, puis calibrez l'échelle.");
            }
            Err(e) => self.error(format!("Détection : {e}.")),
        }
    }

    pub fn relink_image(&mut self) {
        let Some(img) = pick_image() else { return };
        let Some(doc) = self.doc.as_mut() else { return };
        doc.project.map.image_path = Some(store::relativize(&doc.path, &img));
        self.view.fitted = false;
    }

    pub fn image_path(&self) -> Option<PathBuf> {
        let doc = self.doc.as_ref()?;
        Some(store::resolve(&doc.path, doc.project.map.image_path.as_deref()?))
    }

    /// Chemin absolu d'une photo de fiche (relative au dossier de données).
    pub fn photo_path(&self, photo: &Photo) -> PathBuf {
        let p = Path::new(&photo.file);
        if p.is_absolute() {
            p.to_owned()
        } else {
            self.data_dir.join(p)
        }
    }

    // ---------- Textures ----------

    /// Charge (une seule fois) une image en texture ; `max_side = 0` : taille d'origine.
    pub fn texture(&mut self, ctx: &egui::Context, path: &Path, max_side: u32) -> Option<TextureHandle> {
        let key = (path.to_owned(), max_side);
        if let Some(t) = self.textures.get(&key) {
            return t.clone();
        }
        let tex = image::open(path).ok().map(|img| {
            let img = if max_side > 0 { img.thumbnail(max_side, max_side) } else { img };
            let rgba = img.to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            let color = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
            ctx.load_texture(path.to_string_lossy(), color, egui::TextureOptions::LINEAR)
        });
        self.textures.insert(key, tex.clone());
        tex
    }

    pub fn forget_texture(&mut self, path: &Path) {
        self.textures.retain(|(p, _), _| p != path);
    }

    // ---------- Historique ----------

    fn track_history(&mut self, ctx: &egui::Context) {
        let Some(doc) = &self.doc else { return };
        let idle = !ctx.input(|i| i.pointer.any_down()) && ctx.memory(|m| m.focused().is_none());
        if !idle {
            return;
        }
        let changed = self.committed.as_ref().map(|c| *c != doc.project);
        match changed {
            Some(true) => {
                if let Some(prev) = self.committed.replace(doc.project.clone()) {
                    self.undo.push(prev);
                }
                if self.undo.len() > UNDO_LIMIT {
                    self.undo.remove(0);
                }
                self.redo.clear();
            }
            None => self.committed = Some(doc.project.clone()),
            Some(false) => {}
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self) {
        let (Some(doc), Some(prev)) = (self.doc.as_mut(), self.undo.pop()) else { return };
        self.redo.push(std::mem::replace(&mut doc.project, prev));
        self.committed = Some(doc.project.clone());
        self.after_history_jump();
    }

    pub fn redo(&mut self) {
        let (Some(doc), Some(next)) = (self.doc.as_mut(), self.redo.pop()) else { return };
        self.undo.push(std::mem::replace(&mut doc.project, next));
        self.committed = Some(doc.project.clone());
        self.after_history_jump();
    }

    fn after_history_jump(&mut self) {
        self.draft.clear();
        self.drag = Drag::None;
        if let (Some(id), Some(doc)) = (&self.selected, &self.doc) {
            if doc.project.zone(id).is_none() {
                self.selected = None;
            }
        }
    }

    // ---------- Catalogue ----------

    pub fn save_plant(&mut self, plant: Plant) {
        self.catalog.upsert(plant);
        self.catalog_rev += 1;
        match self.catalog.save() {
            Ok(()) => self.info("Fiche enregistrée dans le catalogue."),
            Err(e) => self.error(e.to_string()),
        }
    }

    /// Fiche d'une plante assignée : catalogue, sinon copie du projet.
    pub fn assigned_plant(&self, id: &str) -> Option<&Plant> {
        self.catalog.get(id).or_else(|| self.doc.as_ref()?.project.plants_snapshot.get(id))
    }

    // ---------- Photos à la demande ----------

    /// Met en file le téléchargement de la photo Commons d'une plante (si elle n'en a pas).
    /// `force` ignore le réglage « photos automatiques » (demande explicite).
    pub fn queue_photo(&mut self, plant: &Plant, force: bool) {
        if (!self.auto_photos && !force) || plant.photo.is_some() || plant.id.is_empty() {
            return;
        }
        let Some(file) = plant.links.commons_image.clone() else { return };
        if !self.photo_queue.iter().any(|(id, _)| *id == plant.id) {
            self.photo_queue.push((plant.id.clone(), file));
        }
    }

    pub fn photos_pending(&self) -> usize {
        self.photo_queue.len() + usize::from(self.photo_task.is_some())
    }

    fn pump_photos(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.photo_task {
            match rx.try_recv() {
                Ok((id, res)) => {
                    self.photo_task = None;
                    self.apply_photo(&id, res);
                }
                Err(mpsc::TryRecvError::Empty) => return,
                Err(mpsc::TryRecvError::Disconnected) => self.photo_task = None,
            }
        }
        if self.photo_queue.is_empty() {
            return;
        }
        let (id, file) = self.photo_queue.remove(0);
        let (tx, rx) = mpsc::channel();
        let online = self.online.clone();
        let dir = self.data_dir.join("photos");
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let res = online.commons_photo(&file, &dir, &id, false).map_err(|e| e.to_string());
            let _ = online.save_cache();
            let _ = tx.send((id, res));
            ctx.request_repaint();
        });
        self.photo_task = Some(rx);
    }

    fn apply_photo(&mut self, id: &str, res: Result<Photo, String>) {
        let photo = match res {
            Ok(p) => p,
            Err(e) => return self.error(format!("Photo indisponible : {e}")),
        };
        if let Some(ed) = self.editor.as_mut().filter(|e| e.plant.id == id && e.plant.photo.is_none()) {
            ed.plant.photo = Some(photo.clone());
        }
        if let Some(s) = self.doc.as_mut().and_then(|d| d.project.plants_snapshot.get_mut(id)) {
            s.photo = Some(photo.clone());
        }
        if let Some(mut plant) = self.catalog.get(id).cloned() {
            if plant.photo.is_none() {
                plant.photo = Some(photo);
                let name = plant.display_name();
                self.save_plant(plant);
                self.info(format!("Photo de {name} téléchargée."));
            }
        }
    }

    // ---------- Tâches en ligne ----------

    pub fn busy(&self) -> bool {
        self.task.is_some()
    }

    fn spawn(&mut self, ctx: &egui::Context, job: impl FnOnce(&Online) -> OnlineMsg + Send + 'static) {
        let (tx, rx) = mpsc::channel();
        let online = self.online.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(job(&online));
            ctx.request_repaint();
        });
        self.task = Some(rx);
    }

    pub fn search_online(&mut self, ctx: &egui::Context) {
        let q = self.query.trim().to_owned();
        if q.is_empty() || self.busy() {
            return;
        }
        self.info("Recherche sur Wikidata…");
        self.spawn(ctx, move |o| {
            let r = o.search_taxa(&q, false).map_err(|e| e.to_string());
            let _ = o.save_cache();
            OnlineMsg::Candidates(r)
        });
    }

    pub fn import_candidate(&mut self, ctx: &egui::Context, c: Candidate) {
        if self.busy() {
            return;
        }
        self.info(format!("Import de {}…", c.latin_name));
        let eive = Arc::clone(&self.eive);
        let photos = self.data_dir.join("photos");
        self.spawn(ctx, move |o| {
            let (p, w) = o.build_draft(&c, &eive, Some(&photos), false);
            OnlineMsg::Draft(Box::new(p), w)
        });
    }

    fn poll_task(&mut self) {
        let Some(rx) = &self.task else { return };
        let msg = match rx.try_recv() {
            Ok(m) => m,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.task = None;
                return self.error("La tâche en ligne s'est interrompue.");
            }
        };
        self.task = None;
        match msg {
            OnlineMsg::Candidates(Ok(c)) => {
                let n = c.len();
                self.candidates = c;
                self.info(if n == 0 { "Aucun taxon trouvé sur Wikidata.".to_owned() } else { format!("{n} taxon(s) trouvé(s).") });
            }
            OnlineMsg::Candidates(Err(e)) => self.error(format!("Recherche impossible : {e}")),
            OnlineMsg::Draft(draft, warnings) => {
                let warn = if warnings.is_empty() { String::new() } else { format!(" ({})", warnings.join(" ; ")) };
                if let Some(existing) = self.catalog.get(&draft.id) {
                    let mut plant = existing.clone();
                    let filled = plant.fill_missing_from(&draft);
                    self.editor = Some(Editor { plant, is_new: false });
                    self.info(format!("Fiche existante conservée : {} champ(s) vide(s) complété(s).{warn}", filled.len()));
                } else {
                    self.editor = Some(Editor { plant: *draft, is_new: true });
                    self.info(format!("Brouillon créé : vérifiez les champs marqués ⚠ puis validez.{warn}"));
                }
                self.candidates.clear();
            }
        }
    }

    // ---------- Boucle principale ----------

    fn shortcuts(&mut self, ctx: &egui::Context) {
        let sc = |m, k| KeyboardShortcut::new(m, k);
        if ctx.input_mut(|i| i.consume_shortcut(&sc(Modifiers::COMMAND, Key::S))) {
            self.save();
        }
        let typing = ctx.memory(|m| m.focused().is_some());
        if typing {
            return;
        }
        if ctx.input_mut(|i| {
            i.consume_shortcut(&sc(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z)) || i.consume_shortcut(&sc(Modifiers::COMMAND, Key::Y))
        }) {
            self.redo();
        } else if ctx.input_mut(|i| i.consume_shortcut(&sc(Modifiers::COMMAND, Key::Z))) {
            self.undo();
        }
        if ctx.input(|i| i.key_pressed(Key::Delete)) && self.draft.is_empty() {
            if let (Some(id), Some(doc)) = (self.selected.take(), self.doc.as_mut()) {
                doc.project.remove_zone(&id);
            }
        }
    }

    fn update_title(&mut self, ctx: &egui::Context) {
        let t = match &self.doc {
            Some(d) => format!("{}{} — PlotFlora", if d.dirty() { "● " } else { "" }, d.project.name),
            None => "PlotFlora".into(),
        };
        if t != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(t.clone()));
            self.title = t;
        }
    }

    fn handle_close(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close && self.doc.as_ref().is_some_and(Doc::dirty) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.confirm_close = true;
        }
        if !self.confirm_close {
            return;
        }
        egui::Window::new("Quitter PlotFlora").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO).show(
            ctx,
            |ui| {
                ui.label("Le projet contient des modifications non enregistrées.");
                ui.horizontal(|ui| {
                    let mut quit = false;
                    if ui.button("Enregistrer et quitter").clicked() {
                        quit = self.save();
                    }
                    if ui.button("Quitter sans enregistrer").clicked() {
                        quit = true;
                    }
                    if ui.button("Annuler").clicked() {
                        self.confirm_close = false;
                    }
                    if quit {
                        self.allow_close = true;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            },
        );
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_task();
        self.pump_photos(ctx);
        self.shortcuts(ctx);
        panels::menu_bar(self, ctx);
        panels::status_bar(self, ctx);
        panels::toolbar(self, ctx);
        panels::side_panel(self, ctx);
        egui::CentralPanel::default().frame(egui::Frame::none()).show(ctx, |ui| crate::canvas::show(self, ui));
        panels::windows(self, ctx);
        self.track_history(ctx);
        self.update_title(ctx);
        self.handle_close(ctx);
    }
}
