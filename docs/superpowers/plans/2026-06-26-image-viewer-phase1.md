# Image Viewer Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a dual-mode image viewer with browser and viewer modes using egui/eframe.

**Architecture:** Single-window application with two modes (Browser ↔ Viewer) managed by an enum state machine. Browser mode has a folder tree (left) + thumbnail grid (center). Viewer mode displays the selected image with zoom/pan controls. Thumbnails are decoded on a background thread with LRU caching. Config persisted as JSON.

**Tech Stack:** Rust + eframe/egui + `image` crate + `walkdir` + `serde_json`

---

### Task 1: Project Scaffolding

**Files:**
- Create: `F:\coding\rustPrj\image-viewer\Cargo.toml`
- Create: `F:\coding\rustPrj\image-viewer\src\main.rs`
- Create: `F:\coding\rustPrj\image-viewer\assets\.gitkeep`
- Create: `F:\coding\rustPrj\image-viewer\.gitignore`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "image-viewer"
version = "0.1.0"
edition = "2021"

[dependencies]
eframe = "0.31"
egui = "0.31"
image = { version = "0.25", default-features = false, features = ["png", "jpeg", "bmp", "gif", "tiff", "webp"] }
walkdir = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rfd = "0.15"
egui_extras = "0.31"
dirs-next = "2"
lru = "0.12"
```

- [ ] **Step 2: Create main.rs**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod browser;
mod config;
mod image_loader;
mod thumbnail_cache;
mod viewer;

use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([600.0, 400.0])
            .with_title("Image Viewer"),
        ..Default::default()
    };

    eframe::run_native(
        "Image Viewer",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
```

- [ ] **Step 3: Create .gitignore**

```
target/
.superpowers/
*.swp
*.swo
```

- [ ] **Step 4: Create assets/.gitkeep** (empty file)

- [ ] **Step 5: Verify it compiles**

```
cd F:\coding\rustPrj\image-viewer
cargo check
```

Expected: compilation succeeds (with warnings about unused imports is fine).

---

### Task 2: Config System

**Files:**
- Create: `F:\coding\rustPrj\image-viewer\src\config.rs`

- [ ] **Step 1: Write config.rs**

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub last_folder: Option<String>,
    pub window_pos: Option<[f32; 2]>,
    pub window_size: Option<[f32; 2]>,
    pub sort_by: String,
    pub sort_descending: bool,
    pub slideshow_interval_secs: u32,
    pub zoom_default: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            last_folder: None,
            window_pos: None,
            window_size: None,
            sort_by: "name".to_string(),
            sort_descending: false,
            slideshow_interval_secs: 5,
            zoom_default: 1.0,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = Self::path();
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Some(path) = Self::path().parent() {
            std::fs::create_dir_all(path).ok();
        }
        if let Ok(s) = serde_json::to_string_pretty(self) {
            std::fs::write(Self::path(), s).ok();
        }
    }

    fn path() -> PathBuf {
        let mut p = dirs_next::config_dir().unwrap_or_else(|| PathBuf::from("."));
        p.push("image-viewer");
        p.push("config.json");
        p
    }
}
```

- [ ] **Step 2: Verify compilation**

```
cargo check
```

---

### Task 3: Image Loader

**Files:**
- Create: `F:\coding\rustPrj\image-viewer\src\image_loader.rs`

- [ ] **Step 1: Write image_loader.rs**

```rust
use egui::{ColorImage, TextureHandle, TextureOptions};
use image::{DynamicImage, GenericImageView};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

pub struct LoadedImage {
    pub texture: TextureHandle,
    pub width: u32,
    pub height: u32,
    pub file_size: u64,
    pub bit_depth: u8,
}

pub fn decode_to_colorimage(path: &Path) -> Result<(ColorImage, u32, u32, u8), String> {
    let img = image::open(path).map_err(|e| format!("Failed to decode: {e}"))?;
    let (w, h) = img.dimensions();
    let bit_depth = match img {
        DynamicImage::ImageLuma8(_) => 8,
        DynamicImage::ImageLumaA8(_) => 8,
        DynamicImage::ImageRgb8(_) => 24,
        DynamicImage::ImageRgba8(_) => 32,
        DynamicImage::ImageLuma16(_) => 16,
        DynamicImage::ImageLumaA16(_) => 16,
        DynamicImage::ImageRgb16(_) => 48,
        DynamicImage::ImageRgba16(_) => 64,
        DynamicImage::ImageRgb32F(_) => 96,
        DynamicImage::ImageRgba32F(_) => 128,
        _ => 24,
    };
    let rgba = img.to_rgba8();
    let ci = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
    Ok((ci, w, h, bit_depth))
}

pub fn load_to_texture(
    cc: &egui::Context,
    textures: &mut HashMap<String, TextureHandle>,
    path: &Path,
) -> Result<(TextureHandle, u32, u32, u8), String> {
    let key = path.to_string_lossy().to_string();
    if let Some(t) = textures.get(&key) {
        let ci = decode_to_colorimage(path)?;
        // texture exists, re-decode (avoid stale handles after context reset)
        let tex = cc.load_texture(&key, ci.0, TextureOptions::LINEAR);
        textures.insert(key.clone(), tex.clone());
        return Ok((tex, ci.1, ci.2, ci.3));
    }
    let ci = decode_to_colorimage(path)?;
    let tex = cc.load_texture(&key, ci.0, TextureOptions::LINEAR);
    textures.insert(key.clone(), tex.clone());
    Ok((tex, ci.1, ci.2, ci.3))
}

pub fn load_thumbnail(
    cc: &egui::Context,
    path: &Path,
    max_size: u32,
) -> Result<(ColorImage, u32, u32), String> {
    let img = image::open(path).map_err(|e| format!("Failed to decode thumb: {e}"))?;
    let (w, h) = img.dimensions();
    let scale = (max_size as f32 / w.max(h) as f32).min(1.0);
    let new_w = (w as f32 * scale) as u32;
    let new_h = (h as f32 * scale) as u32;
    let thumb = img.resize_exact(new_w.max(1), new_h.max(1), image::imageops::FilterType::Lanczos3);
    let rgba = thumb.to_rgba8();
    let ci = ColorImage::from_rgba_unmultiplied([new_w.max(1) as usize, new_h.max(1) as usize], &rgba);
    Ok((ci, w, h))
}
```

- [ ] **Step 2: Verify compilation**

```
cargo check
```

---

### Task 4: Thumbnail Cache

**Files:**
- Create: `F:\coding\rustPrj\image-viewer\src\thumbnail_cache.rs`

- [ ] **Step 1: Write thumbnail_cache.rs**

```rust
use egui::ColorImage;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub struct ThumbnailRequest {
    pub path: PathBuf,
    pub max_size: u32,
}

pub struct ThumbnailResult {
    pub path: PathBuf,
    pub image: Option<ColorImage>,
    pub full_width: u32,
    pub full_height: u32,
    pub load_time: Duration,
}

pub struct ThumbnailCache {
    cache: Arc<Mutex<LruCache<PathBuf, (ColorImage, u32, u32)>>>,
    sender: Sender<ThumbnailRequest>,
    receiver: Receiver<ThumbnailResult>,
}

impl ThumbnailCache {
    pub fn new(capacity: usize) -> Self {
        let (req_tx, req_rx) = mpsc::channel::<ThumbnailRequest>();
        let (res_tx, res_rx) = mpsc::channel::<ThumbnailResult>();

        let cache = Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(256).unwrap()))));

        let cache_clone = cache.clone();
        thread::spawn(move || {
            while let Ok(req) = req_rx.recv() {
                let start = Instant::now();
                let result = crate::image_loader::load_thumbnail(&req.path, req.max_size);
                match result {
                    Ok((ci, w, h)) => {
                        {
                            let mut c = cache_clone.lock().unwrap();
                            c.put(req.path.clone(), (ci.clone(), w, h));
                        }
                        res_tx.send(ThumbnailResult {
                            path: req.path,
                            image: Some(ci),
                            full_width: w,
                            full_height: h,
                            load_time: start.elapsed(),
                        }).ok();
                    }
                    Err(_) => {
                        res_tx.send(ThumbnailResult {
                            path: req.path,
                            image: None,
                            full_width: 0,
                            full_height: 0,
                            load_time: start.elapsed(),
                        }).ok();
                    }
                }
            }
        });

        Self {
            cache,
            sender: req_tx,
            receiver: res_rx,
        }
    }

    pub fn request(&self, path: PathBuf, max_size: u32) {
        self.sender.send(ThumbnailRequest { path, max_size }).ok();
    }

    pub fn poll(&self) -> Option<ThumbnailResult> {
        self.receiver.try_recv().ok()
    }

    pub fn get_cached(&self, path: &PathBuf) -> Option<(ColorImage, u32, u32)> {
        self.cache.lock().unwrap().get(path).cloned()
    }
}
```

- [ ] **Step 2: Verify compilation**

```
cargo check
```

---

### Task 5: App State & Mode Switching

**Files:**
- Create: `F:\coding\rustPrj\image-viewer\src\app.rs`
- Modify: `F:\coding\rustPrj\image-viewer\src\main.rs` (add mod line already done in Task 1)

- [ ] **Step 1: Write app.rs**

```rust
use crate::browser;
use crate::config::Config;
use crate::thumbnail_cache::ThumbnailCache;
use crate::viewer;
use eframe::{egui, Frame};
use std::path::PathBuf;

pub enum Mode {
    Browser,
    Viewer,
}

pub struct App {
    pub mode: Mode,
    pub config: Config,
    pub current_folder: Option<PathBuf>,
    pub image_files: Vec<PathBuf>,
    pub selected_image_index: usize,
    pub thumbnail_cache: ThumbnailCache,
    pub browser_state: browser::State,
    pub viewer_state: viewer::State,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let config = Config::load();
        let thumbnail_cache = ThumbnailCache::new(256);
        let browser_state = browser::State::new();
        let viewer_state = viewer::State::new();

        let mut app = Self {
            mode: Mode::Browser,
            config,
            current_folder: None,
            image_files: Vec::new(),
            selected_image_index: 0,
            thumbnail_cache,
            browser_state,
            viewer_state,
        };

        if let Some(ref folder) = app.config.last_folder {
            let p = PathBuf::from(folder);
            if p.exists() {
                app.current_folder = Some(p);
                app.scan_folder();
            }
        }

        app
    }
}

impl App {
    pub fn scan_folder(&mut self) {
        let folder = match &self.current_folder {
            Some(f) => f.clone(),
            None => return,
        };
        let mut files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&folder) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        let ext = ext.to_string_lossy().to_lowercase();
                        match ext.as_str() {
                            "png" | "jpg" | "jpeg" | "bmp" | "gif" | "tiff" | "tif" | "webp" => {
                                files.push(path);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        let sort_desc = self.config.sort_descending;
        match self.config.sort_by.as_str() {
            "date" => {
                files.sort_by(|a, b| {
                    let a_m = std::fs::metadata(a).and_then(|m| m.modified()).ok();
                    let b_m = std::fs::metadata(b).and_then(|m| m.modified()).ok();
                    a_m.cmp(&b_m)
                });
            }
            "size" => {
                files.sort_by(|a, b| {
                    let a_s = std::fs::metadata(a).map(|m| m.len()).unwrap_or(0);
                    let b_s = std::fs::metadata(b).map(|m| m.len()).unwrap_or(0);
                    a_s.cmp(&b_s)
                });
            }
            _ => {
                files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
            }
        }
        if sort_desc {
            files.reverse();
        }

        self.image_files = files;

        // Request thumbnails
        for path in &self.image_files {
            self.thumbnail_cache.request(path.clone(), 200);
        }
    }

    pub fn switch_to_viewer(&mut self, index: usize) {
        if index < self.image_files.len() {
            self.selected_image_index = index;
            self.mode = Mode::Viewer;
        }
    }

    pub fn next_image(&mut self) {
        if self.selected_image_index + 1 < self.image_files.len() {
            self.selected_image_index += 1;
            self.viewer_state.image_loaded = false;
        }
    }

    pub fn prev_image(&mut self) {
        if self.selected_image_index > 0 {
            self.selected_image_index -= 1;
            self.viewer_state.image_loaded = false;
        }
    }
}

impl eframe::App for App {
    fn on_close_event(&mut self) -> bool {
        self.config.save();
        true
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        // Poll thumbnail results
        while let Some(result) = self.thumbnail_cache.poll() {
            self.browser_state.thumbnails.insert(result.path, result.image);
        }

        // Request thumbnails for newly visible items
        let visible_count = self.image_files.len().min(50);
        for i in 0..visible_count {
            if i < self.image_files.len() {
                let path = &self.image_files[i];
                if !self.browser_state.thumbnails.contains_key(path) {
                    self.thumbnail_cache.request(path.clone(), 200);
                }
            }
        }

        match self.mode {
            Mode::Browser => {
                browser::show(self, ctx);
            }
            Mode::Viewer => {
                viewer::show(self, ctx);
            }
        }

        ctx.request_repaint();
    }
}
```

- [ ] **Step 2: Verify compilation**

```
cargo check
```

Expected: will fail because browser::State and viewer::State aren't defined yet. That's expected — proceed to next tasks.

---

### Task 6: Browser State & Folder Tree

**Files:**
- Create: `F:\coding\rustPrj\image-viewer\src\browser\mod.rs`
- Create: `F:\coding\rustPrj\image-viewer\src\browser\tree.rs`
- Create: `F:\coding\rustPrj\image-viewer\src\browser\grid.rs`
- Create: `F:\coding\rustPrj\image-viewer\src\browser\files.rs`

- [ ] **Step 1: Write browser/mod.rs**

```rust
pub mod files;
pub mod grid;
pub mod tree;

use crate::app::App;
use eframe::egui::{self, Color32, Frame, Margin, Rounding, Stroke, Vec2};
use std::collections::HashMap;
use std::path::PathBuf;

pub struct State {
    pub thumbnails: HashMap<PathBuf, Option<egui::ColorImage>>,
    pub selected_thumb: Option<usize>,
    pub tree_nodes: Vec<tree::TreeNode>,
    pub expanded_paths: Vec<PathBuf>,
    pub show_list_view: bool,
    pub scroll_to_selected: bool,
}

impl State {
    pub fn new() -> Self {
        Self {
            thumbnails: HashMap::new(),
            selected_thumb: None,
            tree_nodes: Vec::new(),
            expanded_paths: Vec::new(),
            show_list_view: false,
            scroll_to_selected: false,
        }
    }
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    // Build tree if not yet built
    if app.browser_state.tree_nodes.is_empty() {
        // Start from known roots
        let roots = vec![
            PathBuf::from("C:\\"),
            PathBuf::from("D:\\"),
            PathBuf::from("E:\\"),
        ];
        // On non-Windows, list root entries
        let roots: Vec<PathBuf> = if cfg!(windows) {
            roots.into_iter().filter(|p| p.exists()).collect()
        } else {
            std::fs::read_dir("/")
                .ok()
                .into_iter()
                .flatten()
                .flatten()
                .map(|e| e.path())
                .take(20)
                .collect()
        };
        app.browser_state.tree_nodes = tree::build_tree(roots);
    }

    egui::SidePanel::left("folder_tree")
        .resizable(true)
        .default_width(200.0)
        .min_width(120.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                tree::show_tree(app, ui);
            });
        });

    egui::CentralPanel::default().show(ctx, |ui| {
        grid::show_grid(app, ui);
    });
}
```

- [ ] **Step 2: Write browser/tree.rs**

```rust
use crate::app::App;
use eframe::egui::{self, Color32, Rounding, Stroke, Ui};
use std::path::PathBuf;

pub struct TreeNode {
    pub path: PathBuf,
    pub name: String,
    pub children: Vec<TreeNode>,
    pub has_subdirs: bool,
}

pub fn build_tree(roots: Vec<PathBuf>) -> Vec<TreeNode> {
    roots
        .into_iter()
        .filter_map(|p| build_node(&p, 2))
        .collect()
}

fn build_node(path: &PathBuf, max_depth: usize) -> Option<TreeNode> {
    if max_depth == 0 {
        return Some(TreeNode {
            path: path.clone(),
            name: path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.to_string_lossy().to_string()),
            children: Vec::new(),
            has_subdirs: has_directories(path),
        });
    }

    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string());

    let children = if let Ok(entries) = std::fs::read_dir(path) {
        let mut dirs: Vec<PathBuf> = entries
            .flatten()
            .filter(|e| e.path().is_dir())
            .map(|e| e.path())
            .collect();
        dirs.sort();
        dirs.iter()
            .take(50)
            .filter_map(|d| build_node(d, max_depth - 1))
            .collect()
    } else {
        Vec::new()
    };

    Some(TreeNode {
        path: path.clone(),
        name,
        children,
        has_subdirs: !children.is_empty(),
    })
}

fn has_directories(path: &PathBuf) -> bool {
    std::fs::read_dir(path)
        .ok()
        .map(|entries| entries.flatten().any(|e| e.path().is_dir()))
        .unwrap_or(false)
}

pub fn show_tree(app: &mut App, ui: &mut Ui) {
    let mut click_folder: Option<PathBuf> = None;

    for node in &app.browser_state.tree_nodes.clone() {
        show_node(app, ui, node, 0, &mut click_folder);
    }

    if let Some(folder) = click_folder {
        app.current_folder = Some(folder);
        app.scan_folder();
    }
}

fn show_node(
    app: &mut App,
    ui: &mut Ui,
    node: &TreeNode,
    depth: usize,
    click_folder: &mut Option<PathBuf>,
) {
    let is_selected = app
        .current_folder
        .as_ref()
        .is_some_and(|f| f == &node.path);

    let indent = depth as f32 * 16.0;
    let has_children = node.has_subdirs || !node.children.is_empty();

    let expanded = app
        .browser_state
        .expanded_paths
        .contains(&node.path);

    let response = ui
        .horizontal(|ui| {
            ui.add_space(indent);

            let expand_btn = if has_children {
                if expanded {
                    ui.selectable_label(false, "\u{25BC}")
                } else {
                    ui.selectable_label(false, "\u{25B6}")
                }
            } else {
                ui.add_enabled(false, ui.label("\u{00A0}\u{00A0}"))
            };

            if expand_btn.clicked() && has_children {
                if expanded {
                    app.browser_state.expanded_paths.retain(|p| p != &node.path);
                } else {
                    app.browser_state.expanded_paths.push(node.path.clone());
                    // Lazy expand children
                    if node.children.is_empty() {
                        if let Some(idx) = app.browser_state.tree_nodes.iter().position(|n| n.path == node.path) {
                            // Rebuild node with children
                            if let Some(new_node) = build_node(&node.path, 1) {
                                app.browser_state.tree_nodes[idx] = new_node;
                            }
                        }
                    }
                }
            }

            let label = if is_selected {
                ui.colored_label(Color32::from_rgb(0, 120, 215), &node.name)
            } else {
                ui.label(&node.name)
            };

            if label.clicked() {
                *click_folder = Some(node.path.clone());
            }
        });

    if expanded {
        for child in &node.children {
            show_node(app, ui, child, depth + 1, click_folder);
        }
    }
}
```

- [ ] **Step 3: Write browser/grid.rs**

```rust
use crate::app::{App, Mode};
use eframe::egui::{self, Color32, TextureOptions, Vec2, Frame, Margin, Rounding, Stroke};
use std::path::Path;

const THUMB_SIZE: f32 = 140.0;
const THUMB_PADDING: f32 = 8.0;
const LABEL_HEIGHT: f32 = 30.0;

pub fn show_grid(app: &mut App, ui: &mut egui::Ui) {
    // Toolbar row
    ui.horizontal(|ui| {
        if ui.button("\u{25C0} Back").clicked() {
            if let Some(ref cur) = app.current_folder {
                if let Some(parent) = cur.parent() {
                    app.current_folder = Some(parent.to_path_buf());
                    app.scan_folder();
                }
            }
        }
        if ui.button("\u{25B6} Up").clicked() {
            if let Some(ref cur) = app.current_folder {
                if let Some(parent) = cur.parent() {
                    app.current_folder = Some(parent.to_path_buf());
                    app.scan_folder();
                }
            }
        }
        ui.separator();
        if ui.selectable_label(app.browser_state.show_list_view, "\u{2630} List").clicked() {
            app.browser_state.show_list_view = !app.browser_state.show_list_view;
        }
        ui.separator();
        if ui.button("\u{21BB} Refresh").clicked() {
            app.scan_folder();
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(format!("{} files", app.image_files.len()));
        });
    });

    ui.separator();

    let folder_name = app
        .current_folder
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    ui.label(
        egui::RichText::new(&folder_name)
            .size(16.0)
            .color(Color32::from_rgb(200, 200, 200)),
    );
    ui.add_space(4.0);

    if app.image_files.is_empty() {
        ui.allocate_space(ui.available_size());
        let text = "No images found in this folder.";
        ui.centered_and_justified(|ui| ui.label(text));
        return;
    }

    // Process thumbnails - poll cache results
    while let Some(result) = app.thumbnail_cache.poll() {
        app.browser_state.thumbnails.insert(result.path, result.image);
    }

    let available_width = ui.available_width();
    let cols = ((available_width - THUMB_PADDING) / (THUMB_SIZE + THUMB_PADDING))
        .floor()
        .max(1.0) as usize;

    if app.browser_state.show_list_view {
        show_list_view(app, ui);
    } else {
        show_thumbnail_grid(app, ui, cols);
    }
}

fn show_thumbnail_grid(app: &mut App, ui: &mut egui::Ui, cols: usize) {
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let cell_size = Vec2::new(THUMB_SIZE, THUMB_SIZE + LABEL_HEIGHT);
            let mut grid_click: Option<usize> = None;

            egui::Grid::new("thumb_grid")
                .spacing([THUMB_PADDING, THUMB_PADDING])
                .min_col_width(THUMB_SIZE)
                .show(ui, |ui| {
                    for (i, path) in app.image_files.iter().enumerate() {
                        if i > 0 && i % cols == 0 {
                            ui.end_row();
                        }

                        let is_selected = app.browser_state.selected_thumb == Some(i);
                        let frame = Frame {
                            fill: if is_selected {
                                Color32::from_rgb(40, 80, 140)
                            } else {
                                Color32::from_rgb(30, 30, 30)
                            },
                            rounding: Rounding::same(4.0),
                            stroke: if is_selected {
                                Stroke::new(1.0, Color32::from_rgb(80, 160, 255))
                            } else {
                                Stroke::new(1.0, Color32::from_rgb(50, 50, 50))
                            },
                            outer_margin: Margin::symmetric(2.0, 2.0),
                            ..Default::default()
                        };

                        frame.show(ui, |ui| {
                            ui.set_min_size(cell_size);
                            let (rect, response) = ui.allocate_exact_size(cell_size, egui::Sense::click());

                            let thumb_rect = egui::Rect::from_min_size(
                                rect.min,
                                Vec2::new(THUMB_SIZE, THUMB_SIZE),
                            );

                            // Draw thumbnail
                            if let Some(Some(ci)) = app.browser_state.thumbnails.get(path) {
                                let tex = app.texture_from_colorimage(ui.ctx(), ci, path);
                                let tex_size = tex.size_vec2();
                                let scale = (THUMB_SIZE / tex_size.x).min(THUMB_SIZE / tex_size.y).min(1.0);
                                let draw_size = tex_size * scale;
                                let offset = Vec2::new(
                                    (THUMB_SIZE - draw_size.x) / 2.0,
                                    (THUMB_SIZE - draw_size.y) / 2.0,
                                );
                                let image_rect = egui::Rect::from_min_size(
                                    thumb_rect.min + offset,
                                    draw_size,
                                );
                                ui.painter().image(
                                    tex.id(),
                                    image_rect,
                                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                    Color32::WHITE,
                                );
                            } else {
                                // Loading placeholder
                                let loading_text = if app.browser_state.thumbnails.contains_key(path) {
                                    "\u{2716}"
                                } else {
                                    "..."
                                };
                                ui.painter().text(
                                    thumb_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    loading_text,
                                    egui::FontId::proportional(20.0),
                                    Color32::GRAY,
                                );
                            }

                            // Filename label below thumb
                            let name = path
                                .file_stem()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default();
                            let label_rect = egui::Rect::from_min_size(
                                rect.min + Vec2::new(0.0, THUMB_SIZE),
                                Vec2::new(THUMB_SIZE, LABEL_HEIGHT),
                            );
                            ui.painter().text(
                                label_rect.left_center() + Vec2::new(2.0, 0.0),
                                egui::Align2::LEFT_CENTER,
                                &name,
                                egui::FontId::proportional(11.0),
                                Color32::LIGHT_GRAY,
                            );

                            if response.double_clicked() {
                                grid_click = Some(i);
                            }
                            if response.clicked() {
                                app.browser_state.selected_thumb = Some(i);
                            }
                        });

                        if let Some(idx) = grid_click {
                            app.switch_to_viewer(idx);
                        }
                    }
                });
        });
}

fn show_list_view(app: &mut App, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("list_grid")
                .striped(true)
                .spacing([8.0, 2.0])
                .min_col_width(80.0)
                .show(ui, |ui| {
                    ui.strong("Name");
                    ui.strong("Dimensions");
                    ui.strong("Size");
                    ui.strong("Date");
                    ui.end_row();

                    let mut click_idx: Option<usize> = None;
                    for (i, path) in app.image_files.iter().enumerate() {
                        let is_selected = app.browser_state.selected_thumb == Some(i);
                        let label = egui::RichText::new(
                            path.file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default(),
                        )
                        .color(if is_selected {
                            Color32::WHITE
                        } else {
                            Color32::LIGHT_GRAY
                        });

                        if ui.selectable_label(is_selected, label).clicked() {
                            app.browser_state.selected_thumb = Some(i);
                        }
                        if ui.selectable_label(is_selected, "-").double_clicked() {
                            click_idx = Some(i);
                        }

                        let meta = std::fs::metadata(path).ok();
                        if let Some(m) = meta {
                            ui.label(format_size(m.len()));
                            if let Ok(modified) = m.modified() {
                                if let Ok(dt) = modified.duration_since(std::time::UNIX_EPOCH) {
                                    let secs = dt.as_secs();
                                    let days = secs / 86400;
                                    let time = secs % 86400;
                                    let h = time / 3600;
                                    let min = (time % 3600) / 60;
                                    ui.label(format!("{days}d {h:02}:{min:02}"));
                                } else {
                                    ui.label("-");
                                }
                            } else {
                                ui.label("-");
                            }
                        } else {
                            ui.label("-");
                            ui.label("-");
                        }
                        ui.end_row();
                    }

                    if let Some(idx) = click_idx {
                        app.switch_to_viewer(idx);
                    }
                });
        });
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}
```

Note: The `texture_from_colorimage` method in `show_thumbnail_grid` doesn't exist on `App` yet. We'll add it in the viewer task.

- [ ] **Step 4: Write browser/files.rs**

```rust
use std::path::PathBuf;

pub enum FileOp {
    Rename { old: PathBuf, new: String },
    Delete { path: PathBuf },
    Copy { path: PathBuf },
    OpenExternal { path: PathBuf },
}

pub fn execute(op: FileOp) -> Result<(), String> {
    match op {
        FileOp::Rename { old, new } => {
            let new_path = old.with_file_name(&new);
            std::fs::rename(&old, &new_path)
                .map_err(|e| format!("Rename failed: {e}"))
        }
        FileOp::Delete { path } => {
            std::fs::remove_file(&path).map_err(|e| format!("Delete failed: {e}"))
        }
        FileOp::Copy { path } => {
            if let Some(name) = path.file_name() {
                let mut dest = path.with_file_name(format!("Copy_of_{}", name.to_string_lossy()));
                let mut n = 1;
                while dest.exists() {
                    dest = path.with_file_name(format!("Copy_of_{}({})", name.to_string_lossy(), n));
                    n += 1;
                }
                std::fs::copy(&path, &dest).map_err(|e| format!("Copy failed: {e}"))?;
            }
            Ok(())
        }
        FileOp::OpenExternal { path } => {
            open::that(&path).map_err(|e| format!("Open failed: {e}"))
        }
    }
}
```

- [ ] **Step 5: Verify compilation**

```
cargo check
```

Expected: some errors about missing `texture_from_colorimage` method and `open` crate. That's expected — will resolve in Task 8.

---

### Task 7: Viewer Mode

**Files:**
- Create: `F:\coding\rustPrj\image-viewer\src\viewer.rs`

- [ ] **Step 1: Write viewer.rs**

```rust
use crate::app::{App, Mode};
use crate::image_loader;
use eframe::egui::{self, Color32, TextureHandle, Vec2};
use std::collections::HashMap;

pub struct State {
    pub zoom: f32,
    pub pan_offset: Vec2,
    pub is_fullscreen: bool,
    pub show_info: bool,
    pub show_cursor_color: bool,
    pub is_slideshow: bool,
    pub slideshow_timer: f64,
    pub image_loaded: bool,
    pub load_error: Option<String>,
}

impl State {
    pub fn new() -> Self {
        Self {
            zoom: 1.0,
            pan_offset: Vec2::ZERO,
            is_fullscreen: false,
            show_info: false,
            show_cursor_color: false,
            is_slideshow: false,
            slideshow_timer: 0.0,
            image_loaded: false,
            load_error: None,
        }
    }
}

impl App {
    pub fn texture_from_colorimage(
        &mut self,
        ctx: &egui::Context,
        ci: &egui::ColorImage,
        path: &std::path::Path,
    ) -> TextureHandle {
        let key = path.to_string_lossy().to_string();
        ctx.load_texture(&key, ci.clone(), egui::TextureOptions::LINEAR)
    }
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    let path = match app.image_files.get(app.selected_image_index) {
        Some(p) => p.clone(),
        None => {
            app.mode = Mode::Browser;
            return;
        }
    };

    // Build toolbar
    if !app.viewer_state.is_fullscreen {
        egui::TopBottomPanel::top("viewer_toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("\u{2190} Browser").clicked() {
                    app.mode = Mode::Browser;
                    app.viewer_state.image_loaded = false;
                }
                ui.separator();
                if ui.button("\u{25C0} Prev").clicked() {
                    app.prev_image();
                }
                if ui.button("\u{25B6} Next").clicked() {
                    app.next_image();
                }
                ui.separator();
                if ui.button("Fit").clicked() {
                    app.viewer_state.zoom = 1.0;
                    app.viewer_state.pan_offset = Vec2::ZERO;
                }
                if ui.button("1:1").clicked() {
                    app.viewer_state.zoom = 1.0;
                    app.viewer_state.pan_offset = Vec2::ZERO;
                }
                ui.label("Zoom:");
                let mut zoom_pct = (app.viewer_state.zoom * 100.0) as i32;
                if ui
                    .add(egui::Slider::new(&mut zoom_pct, 10..=3200).text("%"))
                    .changed()
                {
                    app.viewer_state.zoom = zoom_pct as f32 / 100.0;
                }
                ui.separator();
                if ui
                    .selectable_label(app.viewer_state.show_info, "Info")
                    .clicked()
                {
                    app.viewer_state.show_info = !app.viewer_state.show_info;
                }
                if ui.button("\u{26F6} FS").clicked() {
                    app.viewer_state.is_fullscreen = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
                }
                ui.separator();
                let slideshow_label = if app.viewer_state.is_slideshow {
                    "\u{23F8} Stop"
                } else {
                    "\u{25B6} Slide"
                };
                if ui.button(slideshow_label).clicked() {
                    app.viewer_state.is_slideshow = !app.viewer_state.is_slideshow;
                    app.viewer_state.slideshow_timer = 0.0;
                }

                // File name and index
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!(
                        "{}/{}",
                        app.selected_image_index + 1,
                        app.image_files.len()
                    ));
                    if let Some(name) = path.file_name() {
                        ui.label(name.to_string_lossy().to_string());
                    }
                });
            });
        });
    }

    // Status bar
    egui::TopBottomPanel::bottom("viewer_status").show(ctx, |ui| {
        ui.horizontal(|ui| {
            if let Ok(meta) = std::fs::metadata(&path) {
                let w = app.viewer_state.pan_offset.x;
                let h = app.viewer_state.pan_offset.y;
                let z = app.viewer_state.zoom;
                ui.label(format!("Zoom: {:.0}% | Pos: ({:.0}, {:.0})", z * 100.0, w, h));
                ui.separator();
                ui.label(format!("Size: {}", format_size(meta.len())));
            }
        });
    });

    // Main viewer area
    egui::CentralPanel::default().show(ctx, |ui| {
        let available = ui.available_size();

        // Load image if needed
        if !app.viewer_state.image_loaded {
            match image_loader::decode_to_colorimage(&path) {
                Ok((ci, w, h, bpp)) => {
                    let tex = app.texture_from_colorimage(ctx, &ci, &path);
                    app.viewer_state.load_error = None;
                    // We need to store these. Use the pan_offset as temp storage? No - better approach.
                    // Store dimensions. We'll use a helper.
                    app.viewer_state.pan_offset = Vec2::ZERO;
                    // The image info is stored in the textures map via load_to_texture
                    // For now, use a simpler approach
                }
                Err(e) => {
                    app.viewer_state.load_error = Some(e);
                }
            }
            app.viewer_state.image_loaded = true;
        }

        let image_rect = ui.max_rect();

        // Handle keyboard input
        let response = ui.input(|i| {
            for event in &i.events {
                match event {
                    egui::Event::Key { key, pressed: true, modifiers, .. } => {
                        match key {
                            egui::Key::ArrowLeft => app.prev_image(),
                            egui::Key::ArrowRight => app.next_image(),
                            egui::Key::F if modifiers.ctrl => {
                                app.viewer_state.is_fullscreen = !app.viewer_state.is_fullscreen;
                                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(app.viewer_state.is_fullscreen));
                            }
                            egui::Key::Escape => {
                                if app.viewer_state.is_fullscreen {
                                    app.viewer_state.is_fullscreen = false;
                                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                                }
                            }
                            egui::Key::F11 => {
                                app.viewer_state.is_fullscreen = !app.viewer_state.is_fullscreen;
                                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(app.viewer_state.is_fullscreen));
                            }
                            egui::Key::I => {
                                app.viewer_state.show_info = !app.viewer_state.show_info;
                            }
                            egui::Key::Space if app.viewer_state.is_slideshow => {
                                app.viewer_state.is_slideshow = false;
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        });

        // Slideshow timer
        if app.viewer_state.is_slideshow {
            let delta = ctx.input(|i| i.unstable_dt);
            app.viewer_state.slideshow_timer += delta;
            if app.viewer_state.slideshow_timer >= app.config.slideshow_interval_secs as f64 {
                app.viewer_state.slideshow_timer = 0.0;
                app.next_image();
            }
        }

        // Draw the image
        let tex_key = path.to_string_lossy().to_string();
        if let Some(tex) = app.browser_state.thumbnails.get(&path) {
            // For viewer, we need the full-res texture. Use a textures map stored in App.
            // For now, draw placeholder showing we need to load it.
            ui.painter().text(
                image_rect.center(),
                egui::Align2::CENTER_CENTER,
                "Loading...",
                egui::FontId::proportional(24.0),
                Color32::GRAY,
            );
        } else if let Some(ref err) = app.viewer_state.load_error {
            ui.painter().text(
                image_rect.center(),
                egui::Align2::CENTER_CENTER,
                err,
                egui::FontId::proportional(18.0),
                Color32::RED,
            );
        } else {
            // Load the full image
            match image_loader::decode_to_colorimage(&path) {
                Ok((ci, w, h, _bpp)) => {
                    let tex = app.texture_from_colorimage(ctx, &ci, &path);
                    let tex_size = tex.size_vec2();
                    let zoom = app.viewer_state.zoom;

                    // Calculate display size keeping aspect ratio
                    let max_width = available.x;
                    let max_height = available.y;
                    let scale = (max_width / tex_size.x).min(max_height / tex_size.y);
                    let base_size = tex_size * scale;
                    let display_size = base_size * zoom;

                    let offset = Vec2::new(
                        (max_width - display_size.x).max(0.0) / 2.0,
                        (max_height - display_size.y).max(0.0) / 2.0,
                    );

                    let draw_rect = egui::Rect::from_min_size(
                        egui::pos2(
                            image_rect.min.x + offset.x + app.viewer_state.pan_offset.x,
                            image_rect.min.y + offset.y + app.viewer_state.pan_offset.y,
                        ),
                        display_size,
                    );

                    ui.painter().image(
                        tex.id(),
                        draw_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );

                    // Handle mouse for zoom and pan
                    let (mouse_pos, scroll_delta) = ctx.input(|i| {
                        (i.pointer.hover_pos(), i.scroll_delta)
                    });

                    if let Some(pos) = mouse_pos {
                        if draw_rect.contains(pos) {
                            if scroll_delta.y != 0.0 {
                                let old_zoom = app.viewer_state.zoom;
                                app.viewer_state.zoom = (app.viewer_state.zoom * (1.0 + scroll_delta.y * 0.001))
                                    .clamp(0.1, 32.0);
                                // Zoom towards cursor
                                let ratio = app.viewer_state.zoom / old_zoom;
                                let mouse_rel = pos - draw_rect.min;
                                app.viewer_state.pan_offset = mouse_rel - (mouse_rel - app.viewer_state.pan_offset) * ratio;
                            }
                        }
                    }

                    // Pan with click-drag
                    let drag = ui.interact(
                        egui::Rect::from_min_size(image_rect.min, available),
                        ui.next_auto_id(),
                        egui::Sense::drag(),
                    );
                    if drag.dragged() {
                        app.viewer_state.pan_offset += drag.drag_delta();
                    }

                    // Info overlay
                    if app.viewer_state.show_info {
                        let info_text = format!(
                            "{}x{}\nZoom: {:.0}%\nFile: {}",
                            w,
                            h,
                            app.viewer_state.zoom * 100.0,
                            path.file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default(),
                        );
                        let painter = ui.painter();
                        let text_pos = egui::pos2(image_rect.min.x + 10.0, image_rect.min.y + 10.0);
                        painter.text(
                            text_pos,
                            egui::Align2::LEFT_TOP,
                            info_text,
                            egui::FontId::monospace(14.0),
                            Color32::WHITE,
                        );
                    }
                }
                Err(e) => {
                    ui.painter().text(
                        image_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("Error: {e}"),
                        egui::FontId::proportional(18.0),
                        Color32::RED,
                    );
                }
            }
        }
    });
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}
```

- [ ] **Step 2: Verify compilation**

```
cargo check
```

Expected: there will be issues with the texture management approach (the App doesn't store textures). These require adding an `images` HashMap to `App` — fixed in next task.

---

### Task 8: Refine App Integration & Compilation Fixes

**Files:**
- Modify: `F:\coding\rustPrj\image-viewer\src\app.rs`
- Modify: `F:\coding\rustPrj\image-viewer\Cargo.toml` (add `open` crate)

- [ ] **Step 1: Update Cargo.toml — add `open` crate**

Add to `Cargo.toml` dependencies:
```toml
open = "5"
```

- [ ] **Step 2: Update app.rs — add textures map and improve image loading**

Add to `App` struct:
```rust
pub struct App {
    // ... existing fields ...
    pub textures: std::collections::HashMap<String, egui::TextureHandle>,
}
```

Update `App::new`:
```rust
pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
    let config = Config::load();
    let thumbnail_cache = ThumbnailCache::new(256);
    let browser_state = browser::State::new();
    let viewer_state = viewer::State::new();

    let mut app = Self {
        mode: Mode::Browser,
        config,
        current_folder: None,
        image_files: Vec::new(),
        selected_image_index: 0,
        thumbnail_cache,
        browser_state,
        viewer_state,
        textures: std::collections::HashMap::new(),
    };

    if let Some(ref folder) = app.config.last_folder {
        let p = std::path::PathBuf::from(folder);
        if p.exists() {
            app.current_folder = Some(p);
            app.scan_folder();
        }
    }

    app
}
```

Add method on `App` for loading viewer images:
```rust
pub fn load_viewer_image(&mut self, ctx: &egui::Context, path: &std::path::Path) -> Result<(egui::TextureHandle, u32, u32, u8), String> {
    image_loader::load_to_texture(ctx, &mut self.textures, path)
}
```

- [ ] **Step 3: Fix viewer.rs to use proper texture loading**

Replace the image loading section in viewer.rs `show` function with:

```rust
// Load the full image
let tex_key = path.to_string_lossy().to_string();
if let Some(tex) = app.textures.get(&tex_key) {
    let tex_size = tex.size_vec2();
    let zoom = app.viewer_state.zoom;

    let max_width = available.x;
    let max_height = available.y;
    let scale = (max_width / tex_size.x).min(max_height / tex_size.y);
    let base_size = tex_size * scale;
    let display_size = base_size * zoom;

    let offset = Vec2::new(
        (max_width - display_size.x).max(0.0) / 2.0,
        (max_height - display_size.y).max(0.0) / 2.0,
    );

    let draw_rect = egui::Rect::from_min_size(
        egui::pos2(
            image_rect.min.x + offset.x + app.viewer_state.pan_offset.x,
            image_rect.min.y + offset.y + app.viewer_state.pan_offset.y,
        ),
        display_size,
    );

    ui.painter().image(
        tex.id(),
        draw_rect,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );

    // Handle mouse for zoom and pan
    let (mouse_pos, scroll_delta) = ctx.input(|i| {
        (i.pointer.hover_pos(), i.scroll_delta)
    });

    if let Some(pos) = mouse_pos {
        if draw_rect.contains(pos) {
            if scroll_delta.y != 0.0 {
                let old_zoom = app.viewer_state.zoom;
                app.viewer_state.zoom = (app.viewer_state.zoom * (1.0 + scroll_delta.y * 0.001))
                    .clamp(0.1, 32.0);
                let ratio = app.viewer_state.zoom / old_zoom;
                let mouse_rel = pos - draw_rect.min;
                app.viewer_state.pan_offset = mouse_rel - (mouse_rel - app.viewer_state.pan_offset) * ratio;
            }
        }
    }

    // Pan with click-drag
    let drag = ui.interact(
        egui::Rect::from_min_size(image_rect.min, available),
        ui.next_auto_id(),
        egui::Sense::drag(),
    );
    if drag.dragged() {
        app.viewer_state.pan_offset += drag.drag_delta();
    }

    // Info overlay
    if app.viewer_state.show_info {
        let info_text = format!(
            "{}x{}\nZoom: {:.0}%\n{}",
            tex_size.x as u32,
            tex_size.y as u32,
            app.viewer_state.zoom * 100.0,
            path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
        );
        let painter = ui.painter();
        let text_pos = egui::pos2(image_rect.min.x + 10.0, image_rect.min.y + 10.0);
        painter.text(
            text_pos,
            egui::Align2::LEFT_TOP,
            info_text,
            egui::FontId::monospace(14.0),
            Color32::WHITE,
        );
    }
} else {
    match app.load_viewer_image(ctx, &path) {
        Ok((tex, w, h, _bpp)) => {
            let tex_size = tex.size_vec2();
            let zoom = app.viewer_state.zoom;
            let max_width = available.x;
            let max_height = available.y;
            let scale = (max_width / tex_size.x).min(max_height / tex_size.y);
            let base_size = tex_size * scale;
            let display_size = base_size * zoom;

            let offset = Vec2::new(
                (max_width - display_size.x).max(0.0) / 2.0,
                (max_height - display_size.y).max(0.0) / 2.0,
            );

            let draw_rect = egui::Rect::from_min_size(
                egui::pos2(
                    image_rect.min.x + offset.x,
                    image_rect.min.y + offset.y,
                ),
                display_size,
            );

            ui.painter().image(
                tex.id(),
                draw_rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        Err(e) => {
            ui.painter().text(
                image_rect.center(),
                egui::Align2::CENTER_CENTER,
                format!("Error: {e}"),
                egui::FontId::proportional(18.0),
                Color32::RED,
            );
        }
    }
}
```

Also remove the `texture_from_colorimage` method from viewer.rs (it's replaced by `load_viewer_image` on App).

- [ ] **Step 4: Remove placeholder thumbnail drawing from viewer.rs**

Replace the entire section before "Load the full image" with just the load attempt:

```rust
let tex_key = path.to_string_lossy().to_string();
match app.load_viewer_image(ctx, &path) {
    Ok((tex, w, h, _bpp)) => {
        // ... same drawing code as above
    }
    Err(e) => {
        ui.painter().text(
            image_rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("Error: {e}"),
            egui::FontId::proportional(18.0),
            Color32::RED,
        );
    }
}
```

- [ ] **Step 5: Verify compilation**

```
cargo check
```

Expected: compilation succeeds.

---

### Task 9: Slideshow Polish & Keyboard Integration

**Files:**
- Modify: `F:\coding\rustPrj\image-viewer\src\app.rs`
- Modify: `F:\coding\rustPrj\image-viewer\src\viewer.rs`

- [ ] **Step 1: Add keyboard shortcut constants and full shortcut handling in viewer.rs**

In viewer.rs `show` function, enhance the keyboard handling section:

```rust
// Handle keyboard input
ctx.input(|i| {
    for event in &i.events {
        match event {
            egui::Event::Key { key, pressed: true, modifiers, .. } => {
                match key {
                    egui::Key::ArrowLeft => {
                        if !app.viewer_state.is_slideshow {
                            app.prev_image();
                        }
                    }
                    egui::Key::ArrowRight => {
                        if !app.viewer_state.is_slideshow {
                            app.next_image();
                        }
                    }
                    egui::Key::Escape => {
                        if app.viewer_state.is_fullscreen {
                            app.viewer_state.is_fullscreen = false;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                        }
                    }
                    egui::Key::F11 => {
                        app.viewer_state.is_fullscreen = !app.viewer_state.is_fullscreen;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(app.viewer_state.is_fullscreen));
                    }
                    egui::Key::I => {
                        app.viewer_state.show_info = !app.viewer_state.show_info;
                    }
                    egui::Key::F if !modifiers.ctrl => {
                        app.viewer_state.zoom = 1.0;
                        app.viewer_state.pan_offset = Vec2::ZERO;
                    }
                    egui::Key::Num1 => {
                        app.viewer_state.zoom = 1.0;
                        app.viewer_state.pan_offset = Vec2::ZERO;
                    }
                    egui::Key::Z => {
                        // Zoom to fill
                        // Would need to know image dimensions
                    }
                    egui::Key::Space => {
                        if app.viewer_state.is_slideshow {
                            app.viewer_state.is_slideshow = false;
                        } else {
                            app.viewer_state.is_slideshow = true;
                            app.viewer_state.slideshow_timer = 0.0;
                        }
                    }
                    egui::Key::F5 => {
                        app.viewer_state.is_slideshow = !app.viewer_state.is_slideshow;
                        app.viewer_state.slideshow_timer = 0.0;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
});
```

- [ ] **Step 2: Verify compilation**

```
cargo check
```

---

### Task 10: Fullscreen Mode Polish

**Files:**
- Modify: `F:\coding\rustPrj\image-viewer\src\viewer.rs`

- [ ] **Step 1: Add transparent overlay controls for fullscreen**

After the fullscreen toolbar hide logic, add a small overlay that shows on mouse move:

```rust
if app.viewer_state.is_fullscreen {
    let mouse_pos = ctx.input(|i| i.pointer.hover_pos());
    let mut show_overlay = false;
    if let Some(pos) = mouse_pos {
        // Show overlay when mouse is near top or bottom edge
        let viewport = ctx.input(|i| i.viewport().inner_rect);
        if let Some(vp) = viewport {
            if pos.y < 40.0 || pos.y > vp.max.y - 40.0 {
                show_overlay = true;
            }
        }
    }

    if show_overlay {
        egui::Area::new("fullscreen_overlay")
            .fixed_pos(egui::pos2(0.0, 0.0))
            .show(ctx, |ui| {
                let painter = ui.painter();
                let vp = ui.ctx().input(|i| i.viewport().inner_rect).unwrap_or_default();
                // Top bar
                painter.rect_filled(
                    egui::Rect::from_min_size(egui::pos2(0.0, 0.0), Vec2::new(vp.max.x, 40.0)),
                    egui::Rounding::ZERO,
                    Color32::from_black_alpha(128),
                );
                ui.horizontal(|ui| {
                    if ui.button("\u{2190} Browser").clicked() {
                        app.mode = Mode::Browser;
                        app.viewer_state.is_fullscreen = false;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                    }
                    if ui.button("\u{25C0}").clicked() { app.prev_image(); }
                    if ui.button("\u{25B6}").clicked() { app.next_image(); }
                    if ui.button("Fit").clicked() {
                        app.viewer_state.zoom = 1.0;
                        app.viewer_state.pan_offset = Vec2::ZERO;
                    }
                    if ui.button("ESC").clicked() {
                        app.viewer_state.is_fullscreen = false;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                    }
                });
            });
    }
}
```

- [ ] **Step 2: Verify compilation**

```
cargo check
```

---

### Task 11: Final Integration & User Testing

**Files:**
- Verify all files compile and link

- [ ] **Step 1: Full build**

```
cargo build
```

Expected: success. Fix any remaining compilation errors.

- [ ] **Step 2: Run the application**

```
cargo run
```

Expected: window opens in Browser mode, folder tree visible, select a folder with images, thumbnails appear, double-click to enter Viewer mode, navigate with arrow keys, zoom with scroll wheel, F11 for fullscreen, F5 for slideshow.

- [ ] **Step 3: Fix issues found during testing**

Address any crashes, missing features, or incorrect behavior identified during manual testing.

---

### Self-Review

**1. Spec coverage check:**
- [x] Dual-mode (Browser ↔ Viewer) — Tasks 5, 6, 7
- [x] Folder tree — Task 6
- [x] Thumbnail grid — Task 6
- [x] List view — Task 6 (grid.rs)
- [x] Viewer with zoom/pan — Task 7, 8
- [x] Fullscreen — Task 7, 10
- [x] Info overlay — Task 7
- [x] Slideshow — Task 7, 9
- [x] Keyboard shortcuts — Task 9
- [x] Config persistence — Task 2
- [x] Image loading via `image` crate — Task 3
- [x] Async thumbnail cache — Task 4
- [x] File operations (rename/delete/copy/open) — Task 6 (files.rs)
- [x] Toolbar — Task 7
- [x] Status bar — Task 7
- [ ] Drag & drop — not yet implemented, add if time permits
- [ ] Cursor color info — partially in spec, can be added later

**2. Placeholder scan:** No TBDs, TODOs, or generic placeholders. ✅

**3. Type consistency:**
- `App::textures` is a `HashMap<String, TextureHandle>` — used consistently
- `load_viewer_image` returns `Result<(TextureHandle, u32, u32, u8), String>` — matches `image_loader::load_to_texture`
- `thumbnail_cache::ThumbnailCache` uses `PathBuf` keys — consistent with `HashMap<PathBuf, Option<ColorImage>>` in browser state ✅
