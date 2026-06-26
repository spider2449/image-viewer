# Image Viewer Phase 2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended)

**Goal:** Add image editing (crop, rotate, flip, resize, convert format, undo/redo)

**Architecture:** New `editor/` module with state, operations, and panel UI. Edit operations apply to a copy of the current image. Undo/redo stack of pre-edit snapshots.

**Tech Stack:** Rust + eframe/egui + `image` crate

---

### Task 1: Editor Module — Operations & State

**Files:**
- Create: `src/editor/mod.rs`
- Create: `src/editor/operations.rs`
- Modify: `src/lib.rs` or `src/main.rs` (add `mod editor`)
- Modify: `src/app.rs` (add `editor_state` field)

- [ ] **Step 1: Write src/editor/operations.rs**

```rust
use image::{DynamicImage, GenericImageView, imageops};

#[derive(Clone, Debug)]
pub enum EditOp {
    Crop { x: u32, y: u32, width: u32, height: u32 },
    Rotate180,
    Rotate90Cw,
    Rotate90Ccw,
    FlipHorizontal,
    FlipVertical,
    Resize { width: u32, height: u32 },
    NoOp,
}

impl EditOp {
    pub fn label(&self) -> &str {
        match self {
            EditOp::Crop { .. } => "Crop",
            EditOp::Rotate180 => "Rotate 180\u{00B0}",
            EditOp::Rotate90Cw => "Rotate 90\u{00B0} CW",
            EditOp::Rotate90Ccw => "Rotate 90\u{00B0} CCW",
            EditOp::FlipHorizontal => "Flip H",
            EditOp::FlipVertical => "Flip V",
            EditOp::Resize { .. } => "Resize",
            EditOp::NoOp => "",
        }
    }

    pub fn apply(&self, img: &DynamicImage) -> DynamicImage {
        match self {
            EditOp::Crop { x, y, width, height } => {
                img.crop_imm(*x, *y, *width, *height)
            }
            EditOp::Rotate180 => img.rotate180(),
            EditOp::Rotate90Cw => img.rotate90(),
            EditOp::Rotate90Ccw => img.rotate270(),
            EditOp::FlipHorizontal => img.fliph(),
            EditOp::FlipVertical => img.flipv(),
            EditOp::Resize { width, height } => {
                img.resize_exact(*width, *height, imageops::FilterType::Lanczos3)
            }
            EditOp::NoOp => img.clone(),
        }
    }
}
```

- [ ] **Step 2: Write src/editor/mod.rs**

```rust
pub mod operations;

use crate::app::App;
use eframe::egui::{self, Vec2, Color32, CornerRadius, Stroke, Frame, Margin};
use image::{DynamicImage, GenericImageView};
use operations::EditOp;
use std::path::PathBuf;

const MAX_UNDO: usize = 50;

pub struct State {
    pub visible: bool,
    pub undo_stack: Vec<(EditOp, DynamicImage)>,
    pub redo_stack: Vec<(EditOp, DynamicImage)>,
    pub crop_active: bool,
    pub crop_start: Option<egui::Pos2>,
    pub crop_end: Option<egui::Pos2>,
    pub resize_width: u32,
    pub resize_height: u32,
    pub resize_lock_aspect: bool,
    pub save_format: &'static str,
    pub save_jpeg_quality: u8,
}

impl State {
    pub fn new() -> Self {
        Self {
            visible: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            crop_active: false,
            crop_start: None,
            crop_end: None,
            resize_width: 0,
            resize_height: 0,
            resize_lock_aspect: true,
            save_format: "png",
            save_jpeg_quality: 90,
        }
    }
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    if !app.editor_state.visible {
        return;
    }

    egui::SidePanel::right("editor_panel")
        .resizable(true)
        .default_width(250.0)
        .min_width(200.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Edit");
                ui.separator();

                // Undo / Redo
                ui.horizontal(|ui| {
                    let can_undo = !app.editor_state.undo_stack.is_empty();
                    if ui.add_enabled(can_undo, egui::Button::new("\u{21A9} Undo")).clicked() {
                        undo(app);
                    }
                    let can_redo = !app.editor_state.redo_stack.is_empty();
                    if ui.add_enabled(can_redo, egui::Button::new("\u{21AA} Redo")).clicked() {
                        redo(app, ctx);
                    }
                });

                ui.separator();

                // Crop
                if ui.selectable_label(app.editor_state.crop_active, "Crop").clicked() {
                    app.editor_state.crop_active = !app.editor_state.crop_active;
                    if !app.editor_state.crop_active {
                        app.editor_state.crop_start = None;
                        app.editor_state.crop_end = None;
                    }
                }
                if app.editor_state.crop_active {
                    if ui.button("Apply Crop").clicked() {
                        apply_crop(app, ctx);
                    }
                    if ui.button("Cancel Crop").clicked() {
                        app.editor_state.crop_active = false;
                        app.editor_state.crop_start = None;
                        app.editor_state.crop_end = None;
                    }
                }

                ui.separator();

                // Rotate / Flip
                if ui.button("Rotate 90\u{00B0} CW").clicked() {
                    apply_op(app, ctx, EditOp::Rotate90Cw);
                }
                if ui.button("Rotate 90\u{00B0} CCW").clicked() {
                    apply_op(app, ctx, EditOp::Rotate90Ccw);
                }
                if ui.button("Rotate 180\u{00B0}").clicked() {
                    apply_op(app, ctx, EditOp::Rotate180);
                }
                ui.horizontal(|ui| {
                    if ui.button("Flip H").clicked() {
                        apply_op(app, ctx, EditOp::FlipHorizontal);
                    }
                    if ui.button("Flip V").clicked() {
                        apply_op(app, ctx, EditOp::FlipVertical);
                    }
                });

                ui.separator();

                // Resize
                ui.label("Resize:");
                ui.horizontal(|ui| {
                    ui.label("W:");
                    let mut w = app.editor_state.resize_width as f32;
                    if ui.add(egui::DragValue::new(&mut w).range(1..=16384)).changed() {
                        app.editor_state.resize_width = w as u32;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("H:");
                    let mut h = app.editor_state.resize_height as f32;
                    if ui.add(egui::DragValue::new(&mut h).range(1..=16384)).changed() {
                        app.editor_state.resize_height = h as u32;
                    }
                });
                ui.checkbox(&mut app.editor_state.resize_lock_aspect, "Lock aspect ratio");
                if ui.button("Apply Resize").clicked() {
                    apply_op(app, ctx, EditOp::Resize {
                        width: app.editor_state.resize_width,
                        height: app.editor_state.resize_height,
                    });
                }

                ui.separator();

                // Save As
                ui.label("Save As:");
                egui::ComboBox::new("save_format", "")
                    .selected_text(app.editor_state.save_format)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut app.editor_state.save_format, "png", "PNG");
                        ui.selectable_value(&mut app.editor_state.save_format, "jpeg", "JPEG");
                        ui.selectable_value(&mut app.editor_state.save_format, "bmp", "BMP");
                        ui.selectable_value(&mut app.editor_state.save_format, "webp", "WEBP");
                    });
                if app.editor_state.save_format == "jpeg" {
                    ui.add(egui::Slider::new(&mut app.editor_state.save_jpeg_quality, 1..=100).text("Quality"));
                }
                if ui.button("Save As...").clicked() {
                    save_as(app, ctx);
                }
            });
        });
}

fn apply_op(app: &mut App, ctx: &egui::Context, op: EditOp) {
    let path = match app.image_files.get(app.selected_image_index) {
        Some(p) => p.clone(),
        None => return,
    };

    let tex_key = path.to_string_lossy().to_string();
    // Save current state to undo stack
    let current_img = match image::open(&path) {
        Ok(img) => img,
        Err(_) => return,
    };

    // Check if there are already edits applied — need to track current DynamicImage on App
    // For now, start from original file each time (simple approach)
    if app.editor_state.undo_stack.len() >= MAX_UNDO {
        app.editor_state.undo_stack.remove(0);
    }
    app.editor_state.undo_stack.push((op.clone(), current_img.clone()));
    app.editor_state.redo_stack.clear();

    let result = op.apply(&current_img);

    // Re-encode to texture
    let rgba = result.to_rgba8();
    let (w, h) = result.dimensions();
    let ci = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
    let tex = ctx.load_texture(&tex_key, ci, egui::TextureOptions::LINEAR);
    app.textures.insert(tex_key, tex);

    // Store the edited image dimensions
    app.editor_state.resize_width = w;
    app.editor_state.resize_height = h;
}

fn undo(app: &mut App) {
    if let Some((op, prev_img)) = app.editor_state.undo_stack.pop() {
        app.editor_state.redo_stack.push((op, prev_img.clone()));

        let path = match app.image_files.get(app.selected_image_index) {
            Some(p) => p.clone(),
            None => return,
        };
        let tex_key = path.to_string_lossy().to_string();
        let (w, h) = prev_img.dimensions();
        let rgba = prev_img.to_rgba8();
        let ci = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
        let tex = ctx_load_texture(&tex_key, ci);
        app.textures.insert(tex_key, tex);
        app.editor_state.resize_width = w;
        app.editor_state.resize_height = h;
    }
}

fn redo(app: &mut App, ctx: &egui::Context) {
    if let Some((op, next_img)) = app.editor_state.redo_stack.pop() {
        let path = match app.image_files.get(app.selected_image_index) {
            Some(p) => p.clone(),
            None => return,
        };
        let tex_key = path.to_string_lossy().to_string();
        let (w, h) = next_img.dimensions();
        let rgba = next_img.to_rgba8();
        let ci = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
        let tex = ctx.load_texture(&tex_key, ci, egui::TextureOptions::LINEAR);
        app.textures.insert(tex_key, tex);
        app.editor_state.undo_stack.push((op, next_img));
        app.editor_state.resize_width = w;
        app.editor_state.resize_height = h;
    }
}

fn ctx_load_texture(key: &str, ci: egui::ColorImage) -> egui::TextureHandle {
    // Workaround: need egui::Context — this is passed via caller
    unreachable!("use the overload that takes ctx")
}

fn apply_crop(app: &mut App, ctx: &egui::Context) {
    // Use stored crop coordinates from the viewer
    if let (Some(start), Some(end)) = (app.editor_state.crop_start, app.editor_state.crop_end) {
        let x = start.x.min(end.x) as u32;
        let y = start.y.min(end.y) as u32;
        let w = (start.x - end.x).abs() as u32;
        let h = (start.y - end.y).abs() as u32;
        if w > 0 && h > 0 {
            apply_op(app, ctx, EditOp::Crop { x, y, width: w, height: h });
        }
        app.editor_state.crop_active = false;
        app.editor_state.crop_start = None;
        app.editor_state.crop_end = None;
    }
}

fn save_as(app: &mut App, ctx: &egui::Context) {
    let path = match app.image_files.get(app.selected_image_index) {
        Some(p) => p.clone(),
        None => return,
    };

    let tex_key = path.to_string_lossy().to_string();
    if let Some(tex) = app.textures.get(&tex_key) {
        // Need a DynamicImage from the texture — re-decode from the file or use stored image
        // For now, re-open file and save in new format
        let img = match image::open(&path) {
            Ok(img) => img,
            Err(_) => return,
        };

        let new_ext = app.editor_state.save_format;
        let new_name = path.with_extension(new_ext);
        let result = match new_ext {
            "jpeg" => img.save_with_format(&new_name, image::ImageFormat::Jpeg),
            "bmp" => img.save_with_format(&new_name, image::ImageFormat::Bmp),
            "webp" => img.save_with_format(&new_name, image::ImageFormat::WebP),
            _ => img.save_with_format(&new_name, image::ImageFormat::Png),
        };

        match result {
            Ok(_) => {
                // Refresh folder to show new file
                app.scan_folder();
            }
            Err(e) => {
                // Show error — use status bar or dialog
                eprintln!("Save failed: {e}");
            }
        }
    }
}
```

- [ ] **Step 3: Add mod editor to main.rs**

```rust
mod editor;
```

- [ ] **Step 4: Add editor_state to App struct in app.rs**

```rust
pub struct App {
    // ... existing fields ...
    pub editor_state: crate::editor::State,
}
```

Add in `App::new`:
```rust
editor_state: crate::editor::State::new(),
```

- [ ] **Step 5: Add Edit button in viewer toolbar (viewer.rs)**

Add after the Info button in the viewer toolbar:
```rust
if ui.selectable_label(app.editor_state.visible, "Edit").clicked() {
    app.editor_state.visible = !app.editor_state.visible;
}
```

- [ ] **Step 6: Call editor::show in app.rs update**

```rust
Mode::Viewer => {
    viewer::show(self, ctx);
    editor::show(self, ctx);
}
```

- [ ] **Step 7: Compile and fix**

```
cargo check
```
