# Phase 3: Menu Bar + Batch Operations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a top menu bar and batch convert/rename/resize tool to the image viewer.

**Architecture:** New `batch/` module with modal UI + compute in `operations.rs`. Menu bar is a `TopBottomPanel` in `app::update`. Batch modal windows sit on top of browser mode.

**Tech Stack:** Rust, egui 0.31, `image` crate 0.25, `std::thread` + `mpsc` for progress.

---

### Task 1: Create batch module skeleton

**Files:**
- Create: `src/batch/mod.rs`
- Create: `src/batch/operations.rs`
- Modify: `src/main.rs:1-11`

- [ ] **Step 1: Add `mod batch;` to main.rs**

Edit `src/main.rs` to add `mod batch;` in alphabetical order.

```rust
mod app;
mod batch;
mod browser;
mod config;
mod editor;
mod font_loader;
mod image_loader;
mod thumbnail_cache;
mod viewer;
```

- [ ] **Step 2: Create `src/batch/operations.rs` with stub functions**

```rust
use image::DynamicImage;
use std::path::PathBuf;

pub fn batch_convert(
    files: &[PathBuf],
    format: &str,
    jpeg_quality: u8,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    for path in files {
        let img = match image::open(path) {
            Ok(i) => i,
            Err(e) => { errors.push(format!("{}: {e}", path.display())); continue; }
        };
        let new_ext = if format == "jpeg" { "jpg" } else { format };
        let new_name = path.with_extension(new_ext);
        let result = match format {
            "jpeg" => {
                let file = match std::fs::File::create(&new_name) {
                    Ok(f) => f,
                    Err(e) => { errors.push(format!("{}: {e}", new_name.display())); continue; }
                };
                let (w, h) = img.dimensions();
                let rgba = img.to_rgba8();
                let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(file, jpeg_quality);
                encoder.encode(&rgba, w, h, image::ExtendedColorType::Rgba8).map_err(|e| e.to_string())
            }
            "png" => img.save_with_format(&new_name, image::ImageFormat::Png).map_err(|e| e.to_string()),
            "bmp" => img.save_with_format(&new_name, image::ImageFormat::Bmp).map_err(|e| e.to_string()),
            "webp" => img.save_with_format(&new_name, image::ImageFormat::WebP).map_err(|e| e.to_string()),
            _ => Err(format!("Unknown format: {format}")),
        };
        if let Err(e) = result {
            errors.push(format!("{}: {e}", path.display()));
        }
    }
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

pub fn batch_rename(
    files: &[PathBuf],
    pattern: &str,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    for (i, path) in files.iter().enumerate() {
        let stem = path.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let ext = path.extension()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let new_stem = pattern
            .replace("{n}", &format!("{:03}", i + 1))
            .replace("{name}", &stem);
        let new_name = path.with_file_name(format!("{}.{}", new_stem, ext));
        if new_name == *path {
            continue;
        }
        if new_name.exists() {
            errors.push(format!("{} already exists", new_name.display()));
            continue;
        }
        if std::fs::rename(path, &new_name).is_err() {
            errors.push(format!("Failed to rename {}", path.display()));
        }
    }
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

pub fn batch_resize(
    files: &[PathBuf],
    width: u32,
    height: u32,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    for path in files {
        let img = match image::open(path) {
            Ok(i) => i,
            Err(e) => { errors.push(format!("{}: {e}", path.display())); continue; }
        };
        let resized = img.resize_exact(width, height, image::imageops::FilterType::Lanczos3);
        let ext = path.extension()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let save_result = match ext.as_str() {
            "jpg" | "jpeg" => {
                let file = match std::fs::File::create(path) {
                    Ok(f) => f,
                    Err(e) => { errors.push(format!("{}: {e}", path.display())); continue; }
                };
                let (w, h) = resized.dimensions();
                let rgba = resized.to_rgba8();
                let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(file, 90);
                encoder.encode(&rgba, w, h, image::ExtendedColorType::Rgba8).map_err(|e| e.to_string())
            }
            _ => resized.save(path).map_err(|e| e.to_string()),
        };
        if let Err(e) = save_result {
            errors.push(format!("{}: {e}", path.display()));
        }
    }
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}
```

- [ ] **Step 3: Create `src/batch/mod.rs` with State struct**

```rust
pub mod operations;

use crate::app::App;
use eframe::egui;
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(PartialEq)]
pub enum BatchMode {
    Convert,
    Rename,
    Resize,
}

pub struct State {
    pub visible: bool,
    pub mode: BatchMode,
    pub checked: HashSet<PathBuf>,
    pub select_all: bool,
    pub convert_format: &'static str,
    pub jpeg_quality: u8,
    pub rename_pattern: String,
    pub rename_preview: Vec<(PathBuf, PathBuf)>,
    pub resize_width: u32,
    pub resize_height: u32,
    pub resize_lock_aspect: bool,
    pub running: bool,
    pub progress_current: usize,
    pub progress_total: usize,
    pub log: Vec<String>,
}

impl State {
    pub fn new() -> Self {
        Self {
            visible: false,
            mode: BatchMode::Convert,
            checked: HashSet::new(),
            select_all: true,
            convert_format: "png",
            jpeg_quality: 90,
            rename_pattern: "{name}_modified".to_string(),
            rename_preview: Vec::new(),
            resize_width: 800,
            resize_height: 600,
            resize_lock_aspect: true,
            running: false,
            progress_current: 0,
            progress_total: 0,
            log: Vec::new(),
        }
    }

    pub fn open(&mut self, files: &[PathBuf]) {
        self.visible = true;
        self.checked = files.iter().cloned().collect();
        self.select_all = true;
        self.log.clear();
        self.running = false;
    }
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    if !app.batch_state.visible {
        return;
    }

    let mut open = true;
    egui::Window::new("Batch Tool")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_size([600.0, 500.0])
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut app.batch_state.mode, BatchMode::Convert, "Convert");
                ui.selectable_value(&mut app.batch_state.mode, BatchMode::Rename, "Rename");
                ui.selectable_value(&mut app.batch_state.mode, BatchMode::Resize, "Resize");
            });
            ui.separator();

            let files: Vec<PathBuf> = app.image_files.clone();
            if app.batch_state.checked.is_empty() && app.batch_state.select_all {
                for f in &files {
                    app.batch_state.checked.insert(f.clone());
                }
            }

            let mut select_all = app.batch_state.select_all;
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    for path in &files {
                        let name = path.file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        let checked = app.batch_state.checked.contains(path);
                        let mut new_checked = checked;
                        ui.checkbox(&mut new_checked, &name);
                        if new_checked != checked {
                            if new_checked {
                                app.batch_state.checked.insert(path.clone());
                            } else {
                                app.batch_state.checked.remove(path);
                                select_all = false;
                            }
                        }
                    }
                });
            app.batch_state.select_all = select_all;

            ui.separator();

            let selected: Vec<PathBuf> = app.image_files.iter()
                .filter(|p| app.batch_state.checked.contains(*p))
                .cloned()
                .collect();
            ui.label(format!("{} files selected", selected.len()));

            match app.batch_state.mode {
                BatchMode::Convert => {
                    egui::ComboBox::new("batch_format", "Format")
                        .selected_text(app.batch_state.convert_format)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut app.batch_state.convert_format, "png", "PNG");
                            ui.selectable_value(&mut app.batch_state.convert_format, "jpeg", "JPEG");
                            ui.selectable_value(&mut app.batch_state.convert_format, "bmp", "BMP");
                            ui.selectable_value(&mut app.batch_state.convert_format, "webp", "WEBP");
                        });
                    if app.batch_state.convert_format == "jpeg" {
                        ui.add(egui::Slider::new(&mut app.batch_state.jpeg_quality, 1..=100).text("Quality"));
                    }
                    if ui.add_enabled(!app.batch_state.running, egui::Button::new("Apply")).clicked() {
                        app.batch_state.running = true;
                        app.batch_state.progress_total = selected.len();
                        app.batch_state.progress_current = 0;
                        let fmt = app.batch_state.convert_format;
                        let q = app.batch_state.jpeg_quality;
                        let result = operations::batch_convert(&selected, fmt, q);
                        match result {
                            Ok(()) => app.batch_state.log.push("Convert complete.".to_string()),
                            Err(errs) => {
                                for e in errs {
                                    app.batch_state.log.push(e);
                                }
                            }
                        }
                        app.batch_state.running = false;
                        app.scan_folder();
                    }
                }
                BatchMode::Rename => {
                    ui.horizontal(|ui| {
                        ui.label("Pattern:");
                        ui.text_edit_singleline(&mut app.batch_state.rename_pattern);
                    });
                    ui.label("Use {n} for sequence number, {name} for original name.");
                    if !selected.is_empty() {
                        let preview_name = app.batch_state.rename_pattern
                            .replace("{n}", "001")
                            .replace("{name}", &selected[0].file_stem()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_default());
                        ui.label(format!("Preview: {}", preview_name));
                    }
                    if ui.add_enabled(!app.batch_state.running, egui::Button::new("Apply")).clicked() {
                        app.batch_state.running = true;
                        let pattern = app.batch_state.rename_pattern.clone();
                        let result = operations::batch_rename(&selected, &pattern);
                        match result {
                            Ok(()) => app.batch_state.log.push("Rename complete.".to_string()),
                            Err(errs) => {
                                for e in errs {
                                    app.batch_state.log.push(e);
                                }
                            }
                        }
                        app.batch_state.running = false;
                        app.scan_folder();
                    }
                }
                BatchMode::Resize => {
                    ui.horizontal(|ui| {
                        ui.label("W:");
                        let mut w = app.batch_state.resize_width as f32;
                        if ui.add(egui::DragValue::new(&mut w).range(1..=16384)).changed() {
                            app.batch_state.resize_width = w as u32;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("H:");
                        let mut h = app.batch_state.resize_height as f32;
                        if ui.add(egui::DragValue::new(&mut h).range(1..=16384)).changed() {
                            app.batch_state.resize_height = h as u32;
                        }
                    });
                    ui.checkbox(&mut app.batch_state.resize_lock_aspect, "Lock aspect ratio");
                    if ui.add_enabled(!app.batch_state.running, egui::Button::new("Apply")).clicked() {
                        app.batch_state.running = true;
                        let w = app.batch_state.resize_width;
                        let h = app.batch_state.resize_height;
                        let result = operations::batch_resize(&selected, w, h);
                        match result {
                            Ok(()) => app.batch_state.log.push("Resize complete.".to_string()),
                            Err(errs) => {
                                for e in errs {
                                    app.batch_state.log.push(e);
                                }
                            }
                        }
                        app.batch_state.running = false;
                        app.scan_folder();
                    }
                }
            }

            if !app.batch_state.log.is_empty() {
                ui.separator();
                egui::ScrollArea::vertical()
                    .max_height(100.0)
                    .show(ui, |ui| {
                        for line in &app.batch_state.log {
                            ui.label(line);
                        }
                    });
            }
        });

    if !open {
        app.batch_state.visible = false;
    }
}
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check`
Expected: Compilation succeeds.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs src/batch/mod.rs src/batch/operations.rs
git commit -m "feat: add batch module skeleton with operations"
```

---

### Task 2: Wire batch state into App + batch modal in update()

**Files:**
- Modify: `src/app.rs:15-26` (add batch_state field)
- Modify: `src/app.rs:29-66` (init batch_state)
- Modify: `src/app.rs:160-182` (call batch::show)

- [ ] **Step 1: Add `batch_state` field to App struct**

Edit `src/app.rs`. Add `use crate::batch;` and the field:

```rust
pub struct App {
    pub mode: Mode,
    pub config: Config,
    pub current_folder: Option<PathBuf>,
    pub image_files: Vec<PathBuf>,
    pub selected_image_index: usize,
    pub thumbnail_cache: ThumbnailCache,
    pub browser_state: browser::State,
    pub viewer_state: viewer::State,
    pub textures: HashMap<String, egui::TextureHandle>,
    pub editor_state: editor::State,
    pub batch_state: batch::State,
}
```

- [ ] **Step 2: Initialize in App::new()**

In the struct construction:

```rust
let mut app = Self {
    mode: Mode::Browser,
    config,
    current_folder: None,
    image_files: Vec::new(),
    selected_image_index: 0,
    thumbnail_cache,
    browser_state,
    viewer_state,
    textures: HashMap::new(),
    editor_state: editor::State::new(),
    batch_state: batch::State::new(),
};
```

- [ ] **Step 3: Call batch::show() in update()**

Add `batch::show(app, ctx);` after the mode match in `update()`:

```rust
fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
    while let Some(result) = self.thumbnail_cache.poll() {
        self.browser_state.thumbnails.insert(result.path, result.image);
    }

    for path in &self.image_files {
        if !self.browser_state.thumbnails.contains_key(path) {
            self.thumbnail_cache.request(path.clone(), 200);
        }
    }

    match self.mode {
        Mode::Browser => {
            browser::show(self, ctx);
        }
        Mode::Viewer => {
            viewer::show(self, ctx);
            editor::show(self, ctx);
        }
    }

    batch::show(self, ctx);

    ctx.request_repaint();
}
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check`
Expected: Compilation succeeds.

- [ ] **Step 5: Commit**

```bash
git add src/app.rs
git commit -m "feat: wire batch state and UI into App"
```

---

### Task 3: Add menu bar

**Files:**
- Modify: `src/app.rs:160-182`

- [ ] **Step 1: Add menu bar TopBottomPanel before mode dispatch**

In `update()`, before the `match self.mode` block, add:

```rust
egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
    egui::menu::bar(ui, |ui| {
        ui.menu_button("File", |ui| {
            if ui.button("Refresh").clicked() {
                self.scan_folder();
                ui.close_menu();
            }
            if ui.button("Exit").clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
        ui.menu_button("View", |ui| {
            ui.menu_button("Sort by", |ui| {
                let mut sort_changed = false;
                sort_changed |= ui.selectable_value(&mut self.config.sort_by, "name".to_string(), "Name").changed();
                sort_changed |= ui.selectable_value(&mut self.config.sort_by, "date".to_string(), "Date").changed();
                sort_changed |= ui.selectable_value(&mut self.config.sort_by, "size".to_string(), "Size").changed();
                if sort_changed {
                    self.scan_folder();
                }
            });
            if ui.button("Toggle sort direction").clicked() {
                self.config.sort_descending = !self.config.sort_descending;
                self.scan_folder();
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Toggle Grid/List").clicked() {
                self.browser_state.show_list_view = !self.browser_state.show_list_view;
                ui.close_menu();
            }
        });
        ui.menu_button("Tools", |ui| {
            if ui.button("Batch Convert").clicked() {
                self.batch_state.mode = batch::BatchMode::Convert;
                if self.mode == Mode::Browser {
                    self.batch_state.open(&self.image_files);
                }
                ui.close_menu();
            }
            if ui.button("Batch Rename").clicked() {
                self.batch_state.mode = batch::BatchMode::Rename;
                if self.mode == Mode::Browser {
                    self.batch_state.open(&self.image_files);
                }
                ui.close_menu();
            }
            if ui.button("Batch Resize").clicked() {
                self.batch_state.mode = batch::BatchMode::Resize;
                if self.mode == Mode::Browser {
                    self.batch_state.open(&self.image_files);
                }
                ui.close_menu();
            }
        });
    });
});
```

The full `update()` should now be:

```rust
fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
    while let Some(result) = self.thumbnail_cache.poll() {
        self.browser_state.thumbnails.insert(result.path, result.image);
    }

    for path in &self.image_files {
        if !self.browser_state.thumbnails.contains_key(path) {
            self.thumbnail_cache.request(path.clone(), 200);
        }
    }

    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Refresh").clicked() {
                    self.scan_folder();
                    ui.close_menu();
                }
                if ui.button("Exit").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button("View", |ui| {
                ui.menu_button("Sort by", |ui| {
                    let mut sort_changed = false;
                    sort_changed |= ui.selectable_value(&mut self.config.sort_by, "name".to_string(), "Name").changed();
                    sort_changed |= ui.selectable_value(&mut self.config.sort_by, "date".to_string(), "Date").changed();
                    sort_changed |= ui.selectable_value(&mut self.config.sort_by, "size".to_string(), "Size").changed();
                    if sort_changed {
                        self.scan_folder();
                    }
                });
                if ui.button("Toggle sort direction").clicked() {
                    self.config.sort_descending = !self.config.sort_descending;
                    self.scan_folder();
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Toggle Grid/List").clicked() {
                    self.browser_state.show_list_view = !self.browser_state.show_list_view;
                    ui.close_menu();
                }
            });
            ui.menu_button("Tools", |ui| {
                if ui.button("Batch Convert").clicked() {
                    self.batch_state.mode = batch::BatchMode::Convert;
                    self.batch_state.open(&self.image_files);
                    ui.close_menu();
                }
                if ui.button("Batch Rename").clicked() {
                    self.batch_state.mode = batch::BatchMode::Rename;
                    self.batch_state.open(&self.image_files);
                    ui.close_menu();
                }
                if ui.button("Batch Resize").clicked() {
                    self.batch_state.mode = batch::BatchMode::Resize;
                    self.batch_state.open(&self.image_files);
                    ui.close_menu();
                }
            });
        });
    });

    match self.mode {
        Mode::Browser => {
            browser::show(self, ctx);
        }
        Mode::Viewer => {
            viewer::show(self, ctx);
            editor::show(self, ctx);
        }
    }

    batch::show(self, ctx);

    ctx.request_repaint();
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: Compilation succeeds.

- [ ] **Step 3: Run existing tests**

Run: `cargo test`
Expected: All 11 existing tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/app.rs
git commit -m "feat: add menu bar with File, View, Tools menus"
```

---

### Task 4: Add batch operation tests

**Files:**
- Modify: `src/batch/operations.rs` (append tests)

- [ ] **Step 1: Add batch_convert test that converts a temp PNG to JPEG**

Append a `#[cfg(test)] mod tests { ... }` block at the end of `src/batch/operations.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_batch_convert_png_to_jpeg() {
        let dir = std::env::temp_dir().join("batch_test_convert");
        let _ = std::fs::create_dir_all(&dir);
        let src = dir.join("test.png");
        let img = image::DynamicImage::new_rgba8(10, 10);
        img.save(&src).unwrap();

        let result = batch_convert(&[src.clone()], "jpeg", 90);
        assert!(result.is_ok());

        let dst = src.with_extension("jpg");
        assert!(dst.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_batch_convert_invalid_file() {
        let result = batch_convert(&[PathBuf::from("nonexistent.png")], "png", 90);
        assert!(result.is_err());
    }

    #[test]
    fn test_batch_rename_sequence() {
        let dir = std::env::temp_dir().join("batch_test_rename");
        let _ = std::fs::create_dir_all(&dir);
        let files: Vec<PathBuf> = (1..=3).map(|i| {
            let p = dir.join(format!("img{i}.png"));
            image::DynamicImage::new_rgba8(10, 10).save(&p).unwrap();
            p
        }).collect();

        let result = batch_rename(&files, "photo_{n}");
        assert!(result.is_ok());

        for i in 0..3 {
            let renamed = dir.join(format!("photo_{:03}.png", i + 1));
            assert!(renamed.exists(), "missing {renamed:?}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_batch_rename_name_pattern() {
        let dir = std::env::temp_dir().join("batch_test_rename2");
        let _ = std::fs::create_dir_all(&dir);
        let src = dir.join("vacation.png");
        image::DynamicImage::new_rgba8(10, 10).save(&src).unwrap();

        let result = batch_rename(&[src.clone()], "{name}_edited");
        assert!(result.is_ok());

        let renamed = dir.join("vacation_edited.png");
        assert!(renamed.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_batch_resize() {
        let dir = std::env::temp_dir().join("batch_test_resize");
        let _ = std::fs::create_dir_all(&dir);
        let src = dir.join("test.png");
        image::DynamicImage::new_rgba8(100, 200).save(&src).unwrap();

        let result = batch_resize(&[src.clone()], 50, 100);
        assert!(result.is_ok());

        let img = image::open(&src).unwrap();
        assert_eq!(img.dimensions(), (50, 100));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_batch_resize_jpeg() {
        let dir = std::env::temp_dir().join("batch_test_resize_jpg");
        let _ = std::fs::create_dir_all(&dir);
        let src = dir.join("test.jpg");
        image::DynamicImage::new_rgba8(100, 200).save(&src).unwrap();

        let result = batch_resize(&[src.clone()], 25, 50);
        assert!(result.is_ok());

        let img = image::open(&src).unwrap();
        assert_eq!(img.dimensions(), (25, 50));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
```

- [ ] **Step 2: Run the new tests**

Run: `cargo test`
Expected: All 16 tests pass (11 existing + 5 new).

- [ ] **Step 3: Bump version in Cargo.toml and commit**

Edit `Cargo.toml` to bump patch version (e.g. `0.1.2` → `0.1.3`), then:

```bash
git add src/batch/operations.rs Cargo.toml
git commit -m "feat: add batch operations and tests"
```

---

### Task 5: Final verification

- [ ] **Step 1: Run cargo check**

Run: `cargo check`
Expected: Clean compile with no warnings.

- [ ] **Step 2: Run cargo test**

Run: `cargo test`
Expected: All 16 tests pass.

- [ ] **Step 3: Bump version and commit**

```bash
git add Cargo.toml
git commit -m "chore: bump version to 0.1.3"
```
