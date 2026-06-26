# EXIF Viewer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a right-side EXIF metadata panel in Viewer mode.

**Architecture:** New `exif.rs` module parses EXIF via `kamadak-exif` crate. Right-side `SidePanel` toggled by a button in the viewer toolbar. Parsing happens on image load, stored in `ExifData` struct on `App`.

**Tech Stack:** Rust, egui 0.31, `exif` crate 1.x

---

### Task 1: Add dependency and create EXIF module

**Files:**
- Modify: `Cargo.toml`
- Create: `src/exif.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Add exif dependency to Cargo.toml**

```toml
[dependencies]
eframe = "0.31"
egui = "0.31"
exif = "1"
image = { version = "0.25", default-features = false, features = ["png", "jpeg", "bmp", "gif", "tiff", "webp"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
dirs-next = "2"
lru = "0.12"
open = "5"
```

- [ ] **Step 2: Create `src/exif.rs`**

```rust
use std::path::Path;

pub struct ExifData {
    pub visible: bool,
    pub entries: Vec<(String, String)>,
}

impl ExifData {
    pub fn new() -> Self {
        Self {
            visible: false,
            entries: Vec::new(),
        }
    }

    pub fn parse(&mut self, path: &Path) {
        self.entries.clear();

        let ext = path.extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if ext != "jpg" && ext != "jpeg" && ext != "tif" && ext != "tiff" {
            self.entries.push(("Info".to_string(), "EXIF not available for this format".to_string()));
            return;
        }

        let file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(e) => {
                self.entries.push(("Error".to_string(), format!("Cannot read file: {e}")));
                return;
            }
        };
        let mut reader = std::io::BufReader::new(file);
        let exif = match exif::Reader::new().read_from_container(&mut reader) {
            Ok(e) => e,
            Err(_) => {
                self.entries.push(("Info".to_string(), "No EXIF data found".to_string()));
                return;
            }
        };

        let fields: Vec<(&str, exif::Tag)> = vec![
            ("Camera Make", exif::Tag::Make),
            ("Camera Model", exif::Tag::Model),
            ("Lens", exif::Tag::LensModel),
            ("Software", exif::Tag::Software),
            ("Date/Time", exif::Tag::DateTimeOriginal),
            ("Image Width", exif::Tag::PixelXDimension),
            ("Image Height", exif::Tag::PixelYDimension),
            ("Orientation", exif::Tag::Orientation),
            ("Exposure Time", exif::Tag::ExposureTime),
            ("F-Number", exif::Tag::FNumber),
            ("ISO", exif::Tag::PhotographicSensitivity),
            ("Focal Length", exif::Tag::FocalLength),
            ("Flash", exif::Tag::Flash),
        ];

        for (label, tag) in fields {
            if let Some(field) = exif.get_field(tag, exif::In::PRIMARY) {
                let value = format_exif_value(field);
                self.entries.push((label.to_string(), value));
            }
        }

        // GPS: need ref field for N/S/E/W
        if let Some(lat) = exif.get_field(exif::Tag::GPSLatitude, exif::In::PRIMARY) {
            let ref_val = exif.get_field(exif::Tag::GPSLatitudeRef, exif::In::PRIMARY)
                .and_then(|f| f.value.display_as(f.tag).to_string().into())
                .unwrap_or_default();
            let dir = if ref_val == "S" { "S" } else { "N" };
            self.entries.push(("GPS Latitude".to_string(), format_gps_coords(&lat, dir)));
        }
        if let Some(lon) = exif.get_field(exif::Tag::GPSLongitude, exif::In::PRIMARY) {
            let ref_val = exif.get_field(exif::Tag::GPSLongitudeRef, exif::In::PRIMARY)
                .and_then(|f| f.value.display_as(f.tag).to_string().into())
                .unwrap_or_default();
            let dir = if ref_val == "W" { "W" } else { "E" };
            self.entries.push(("GPS Longitude".to_string(), format_gps_coords(&lon, dir)));
        }

        if self.entries.is_empty() {
            self.entries.push(("Info".to_string(), "No EXIF data found".to_string()));
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

fn format_exif_value(field: &exif::Field) -> String {
    match field.tag {
        exif::Tag::Orientation => {
            match field.value.get_uint(0) {
                Some(1) => "Normal".to_string(),
                Some(2) => "Mirrored".to_string(),
                Some(3) => "Rotated 180\u{00B0}".to_string(),
                Some(4) => "Mirrored & rotated 180\u{00B0}".to_string(),
                Some(5) => "Mirrored & rotated 90\u{00B0} CW".to_string(),
                Some(6) => "Rotated 90\u{00B0} CW".to_string(),
                Some(7) => "Mirrored & rotated 90\u{00B0} CCW".to_string(),
                Some(8) => "Rotated 90\u{00B0} CCW".to_string(),
                _ => format!("{}", field.value.display_as(field.tag)),
            }
        }
        exif::Tag::ExposureTime => {
            if let exif::Value::Rational(rats) = &field.value {
                if let Some(r) = rats.first() {
                    let num = r.to_f64();
                    if num > 0.0 {
                        let denom = (1.0 / num).round() as u64;
                        return format!("1/{denom} sec");
                    }
                }
            }
            format!("{}", field.value.display_as(field.tag))
        }
        exif::Tag::FNumber => {
            if let exif::Value::Rational(rats) = &field.value {
                if let Some(r) = rats.first() {
                    let val = r.to_f64();
                    return format!("F/{val:.1}");
                }
            }
            format!("{}", field.value.display_as(field.tag))
        }
        exif::Tag::FocalLength => {
            if let exif::Value::Rational(rats) = &field.value {
                if let Some(r) = rats.first() {
                    let val = r.to_f64();
                    return format!("{val:.1} mm");
                }
            }
            format!("{}", field.value.display_as(field.tag))
        }
        exif::Tag::Flash => {
            match field.value.get_uint(0) {
                Some(0) => "Did not fire".to_string(),
                Some(1) => "Fired".to_string(),
                Some(5) => "Fired (return light detected)".to_string(),
                Some(7) => "Fired (return light not detected)".to_string(),
                Some(16) => "Did not fire (auto)".to_string(),
                _ => format!("{}", field.value.display_as(field.tag)),
            }
        }
        _ => {
            format!("{}", field.value.display_as(field.tag))
        }
    }
}

fn format_gps_coords(field: &exif::Field, dir: &str) -> String {
    if let exif::Value::Rational(rats) = &field.value {
        if rats.len() >= 3 {
            let deg = rats[0].to_f64();
            let min = rats[1].to_f64();
            let sec = rats[2].to_f64();
            return format!("{deg}\u{00B0} {min}' {sec:.1}\" {dir}");
        }
    }
    format!("{} {}", field.value.display_as(field.tag), dir)
}

pub fn show(app: &mut crate::app::App, ctx: &egui::Context) {
    if !app.exif_state.visible {
        return;
    }

    egui::SidePanel::right("exif_panel")
        .resizable(true)
        .default_width(280.0)
        .min_width(200.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("EXIF Data");
                ui.separator();
                for (label, value) in &app.exif_state.entries {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("{label}:")).strong());
                        ui.label(value);
                    });
                }
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_exif_for_png() {
        let mut data = ExifData::new();
        data.parse(&std::path::PathBuf::from("test.png"));
        assert!(!data.entries.is_empty());
        assert!(data.entries[0].1.contains("not available"));
    }

    #[test]
    fn test_no_exif_for_bmp() {
        let mut data = ExifData::new();
        data.parse(&std::path::PathBuf::from("test.bmp"));
        assert!(!data.entries.is_empty());
        assert!(data.entries[0].1.contains("not available"));
    }

    #[test]
    fn test_no_exif_for_missing_file() {
        let mut data = ExifData::new();
        data.parse(&std::path::PathBuf::from("nonexistent.jpg"));
        assert!(!data.entries.is_empty());
        assert!(data.entries[0].1.contains("Cannot read"));
    }

    #[test]
    fn test_clear() {
        let mut data = ExifData::new();
        data.entries.push(("Test".to_string(), "Value".to_string()));
        data.clear();
        assert!(data.entries.is_empty());
    }
}
```

- [ ] **Step 3: Add `mod exif;` to `src/main.rs`**

```rust
mod app;
mod batch;
mod browser;
mod config;
mod editor;
mod exif;
mod font_loader;
mod image_loader;
mod thumbnail_cache;
mod viewer;
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check`
Expected: Compilation succeeds.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock src/exif.rs src/main.rs
git commit -m "feat: add EXIF module with parsing and panel UI"
```

---

### Task 2: Wire EXIF state into App

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 1: Add exif_state field to App**

Add `use crate::exif;` and the field:

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
    pub exif_state: exif::ExifData,
}
```

- [ ] **Step 2: Initialize in App::new()**

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
    exif_state: exif::ExifData::new(),
};
```

- [ ] **Step 3: Parse EXIF in switch_to_viewer and image navigation**

The EXIF data should be parsed whenever an image is loaded. The existing `editor_state.load_image(path)` calls in `switch_to_viewer()`, `next_image()`, and `prev_image()` are the right hooks. Add `self.exif_state.parse(path)` after each `editor_state.load_image(path)`.

Edit `switch_to_viewer()`:

```rust
pub fn switch_to_viewer(&mut self, index: usize) {
    if index < self.image_files.len() {
        self.selected_image_index = index;
        if let Some(p) = self.image_files.get(index) {
            self.editor_state.load_image(p);
            self.exif_state.parse(p);
        }
        self.mode = Mode::Viewer;
    }
}
```

Edit `next_image()`:

```rust
pub fn next_image(&mut self) {
    if self.selected_image_index + 1 < self.image_files.len() {
        self.selected_image_index += 1;
        self.viewer_state.image_loaded = false;
        if let Some(p) = self.image_files.get(self.selected_image_index) {
            self.editor_state.load_image(p);
            self.exif_state.parse(p);
        }
    }
}
```

Edit `prev_image()`:

```rust
pub fn prev_image(&mut self) {
    if self.selected_image_index > 0 {
        self.selected_image_index -= 1;
        self.viewer_state.image_loaded = false;
        if let Some(p) = self.image_files.get(self.selected_image_index) {
            self.editor_state.load_image(p);
            self.exif_state.parse(p);
        }
    }
}
```

- [ ] **Step 4: Add exif::show() call in update()**

In `App::update()`, add `exif::show(self, ctx);` after `editor::show(self, ctx);`:

```rust
Mode::Viewer => {
    viewer::show(self, ctx);
    editor::show(self, ctx);
    exif::show(self, ctx);
}
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo check`
Expected: Compilation succeeds.

- [ ] **Step 6: Run tests**

Run: `cargo test`
Expected: All tests pass (17 existing + 4 new = 21 total).

- [ ] **Step 7: Commit**

```bash
git add src/app.rs
git commit -m "feat: wire EXIF state into App and trigger parse on image load"
```

---

### Task 3: Add Exif toggle button to viewer toolbar

**Files:**
- Modify: `src/viewer.rs`

- [ ] **Step 1: Add "Exif" toggle button to viewer toolbar**

Read the viewer toolbar section. Add the Exif toggle button between the Info button and the Edit button:

```rust
if ui.selectable_label(app.viewer_state.show_info, "Info").clicked() {
    app.viewer_state.show_info = !app.viewer_state.show_info;
}
if ui.selectable_label(app.exif_state.visible, "Exif").clicked() {
    app.exif_state.visible = !app.exif_state.visible;
}
if ui.selectable_label(app.editor_state.visible, "Edit").clicked() {
    app.editor_state.visible = !app.editor_state.visible;
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: Compilation succeeds.

- [ ] **Step 3: Run all tests**

Run: `cargo test`
Expected: All 21 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/viewer.rs
git commit -m "feat: add Exif toggle button to viewer toolbar"
```

---

### Task 4: Bump version and final verification

- [ ] **Step 1: Bump version in Cargo.toml**

`0.1.3` → `0.1.4`

- [ ] **Step 2: Run cargo check**

`cargo check` — clean compile, no warnings.

- [ ] **Step 3: Run cargo test**

`cargo test` — 21 tests pass.

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml
git commit -m "chore: bump version to 0.1.4"
```
