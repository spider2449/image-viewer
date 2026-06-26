pub mod operations;

use crate::app::App;
use eframe::egui;
use image::{DynamicImage, GenericImageView};
use operations::EditOp;
use std::path::PathBuf;

const MAX_UNDO: usize = 50;

pub struct State {
    pub visible: bool,
    pub undo_stack: Vec<(EditOp, DynamicImage)>,
    pub redo_stack: Vec<(EditOp, DynamicImage)>,
    pub current_image: Option<DynamicImage>,
    #[allow(dead_code)]
    pub crop_active: bool,
    #[allow(dead_code)]
    pub crop_start: Option<egui::Pos2>,
    #[allow(dead_code)]
    pub crop_end: Option<egui::Pos2>,
    pub resize_width: u32,
    pub resize_height: u32,
    pub resize_lock_aspect: bool,
    pub save_format: String,
    pub save_jpeg_quality: u8,
}

impl State {
    pub fn new() -> Self {
        Self {
            visible: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            current_image: None,
            crop_active: false,
            crop_start: None,
            crop_end: None,
            resize_width: 0,
            resize_height: 0,
            resize_lock_aspect: true,
            save_format: "png".to_string(),
            save_jpeg_quality: 90,
        }
    }

    pub fn load_image(&mut self, path: &PathBuf) {
        if let Ok(img) = image::open(path) {
            let (w, h) = img.dimensions();
            self.current_image = Some(img);
            self.resize_width = w;
            self.resize_height = h;
            self.undo_stack.clear();
            self.redo_stack.clear();
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

                ui.horizontal(|ui| {
                    let can_undo = !app.editor_state.undo_stack.is_empty();
                    if ui.add_enabled(can_undo, egui::Button::new("\u{21A9} Undo")).clicked() {
                        undo(app, ctx);
                    }
                    let can_redo = !app.editor_state.redo_stack.is_empty();
                    if ui.add_enabled(can_redo, egui::Button::new("\u{21AA} Redo")).clicked() {
                        redo(app, ctx);
                    }
                    if ui.button("X").clicked() {
                        app.editor_state.visible = false;
                    }
                });

                ui.separator();

                ui.label("Transform");
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

                ui.label("Resize");
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
                if ui.button("Apply").clicked() {
                    apply_op(app, ctx, EditOp::Resize {
                        width: app.editor_state.resize_width,
                        height: app.editor_state.resize_height,
                    });
                }

                ui.separator();

                ui.label("Save As");
                egui::ComboBox::new("save_format", "")
                    .selected_text(&app.editor_state.save_format)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut app.editor_state.save_format, "png".to_string(), "PNG");
                        ui.selectable_value(&mut app.editor_state.save_format, "jpeg".to_string(), "JPEG");
                        ui.selectable_value(&mut app.editor_state.save_format, "bmp".to_string(), "BMP");
                        ui.selectable_value(&mut app.editor_state.save_format, "webp".to_string(), "WEBP");
                    });
                if app.editor_state.save_format == "jpeg" {
                    ui.add(egui::Slider::new(&mut app.editor_state.save_jpeg_quality, 1..=100).text("Quality"));
                }
                if ui.button("Save As...").clicked() {
                    save_as(app);
                }
            });
        });
}

fn apply_op(app: &mut App, ctx: &egui::Context, op: EditOp) {
    let img = match &app.editor_state.current_image {
        Some(i) => i.clone(),
        None => return,
    };

    if app.editor_state.undo_stack.len() >= MAX_UNDO {
        app.editor_state.undo_stack.remove(0);
    }
    app.editor_state.undo_stack.push((op.clone(), img.clone()));
    app.editor_state.redo_stack.clear();

    let result = op.apply(&img);
    let (w, h) = result.dimensions();
    app.editor_state.resize_width = w;
    app.editor_state.resize_height = h;

    let path = match app.image_files.get(app.selected_image_index) {
        Some(p) => p.clone(),
        None => return,
    };
    let tex_key = path.to_string_lossy().to_string();
    let rgba = result.to_rgba8();
    let ci = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
    let tex = ctx.load_texture(&tex_key, ci, egui::TextureOptions::LINEAR);
    app.textures.insert(tex_key, tex);
    app.editor_state.current_image = Some(result);
}

fn undo(app: &mut App, ctx: &egui::Context) {
    if let Some((op, prev_img)) = app.editor_state.undo_stack.pop() {
        let (w, h) = prev_img.dimensions();
        app.editor_state.redo_stack.push((op, prev_img.clone()));
        app.editor_state.resize_width = w;
        app.editor_state.resize_height = h;

        let path = match app.image_files.get(app.selected_image_index) {
            Some(p) => p.clone(),
            None => return,
        };
        let tex_key = path.to_string_lossy().to_string();
        let rgba = prev_img.to_rgba8();
        let ci = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
        let tex = ctx.load_texture(&tex_key, ci, egui::TextureOptions::LINEAR);
        app.textures.insert(tex_key, tex);
        app.editor_state.current_image = Some(prev_img);
    }
}

fn redo(app: &mut App, ctx: &egui::Context) {
    if let Some((op, next_img)) = app.editor_state.redo_stack.pop() {
        let (w, h) = next_img.dimensions();
        app.editor_state.undo_stack.push((op, next_img.clone()));
        app.editor_state.resize_width = w;
        app.editor_state.resize_height = h;

        let path = match app.image_files.get(app.selected_image_index) {
            Some(p) => p.clone(),
            None => return,
        };
        let tex_key = path.to_string_lossy().to_string();
        let rgba = next_img.to_rgba8();
        let ci = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
        let tex = ctx.load_texture(&tex_key, ci, egui::TextureOptions::LINEAR);
        app.textures.insert(tex_key, tex);
        app.editor_state.current_image = Some(next_img);
    }
}

fn save_as(app: &mut App) {
    let img = match &app.editor_state.current_image {
        Some(i) => i.clone(),
        None => return,
    };

    let path = match app.image_files.get(app.selected_image_index) {
        Some(p) => p.clone(),
        None => return,
    };

    let new_ext = &app.editor_state.save_format;
    let new_name = path.with_extension(new_ext);
    let result = match new_ext.as_str() {
        "jpeg" => img.save_with_format(&new_name, image::ImageFormat::Jpeg),
        "bmp" => img.save_with_format(&new_name, image::ImageFormat::Bmp),
        "webp" => img.save_with_format(&new_name, image::ImageFormat::WebP),
        _ => img.save_with_format(&new_name, image::ImageFormat::Png),
    };

    match result {
        Ok(_) => {
            app.scan_folder();
        }
        Err(e) => {
            eprintln!("Save failed: {e}");
        }
    }
}
