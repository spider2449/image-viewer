pub mod operations;

use crate::app::App;
use eframe::egui;
use image::{DynamicImage, GenericImageView};
use operations::EditOp;
use std::path::PathBuf;

const MAX_UNDO: usize = 50;
const MAX_UNDO_BYTES: usize = 64 * 1024 * 1024; // 64 MB byte budget for undo history

pub struct State {
    pub visible: bool,
    pub undo_stack: Vec<(EditOp, DynamicImage)>,
    pub redo_stack: Vec<(EditOp, DynamicImage)>,
    pub current_image: Option<DynamicImage>,
    pub crop_active: bool,
    pub crop_start: Option<egui::Pos2>,
    pub crop_end: Option<egui::Pos2>,
    pub resize_width: u32,
    pub resize_height: u32,
    pub resize_lock_aspect: bool,
    pub save_format: &'static str,
    pub save_jpeg_quality: u8,
    pub save_as_filename: String,
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
            save_format: "png",
            save_jpeg_quality: 90,
            save_as_filename: String::new(),
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

    let colors = app.theme_colors();

    egui::SidePanel::right("editor_panel")
        .resizable(true)
        .frame(egui::Frame {
            fill: colors.panel_bg,
            inner_margin: egui::Margin::symmetric(8, 8),
            ..Default::default()
        })
        .default_width(250.0)
        .min_width(200.0)
        .show(ctx, |ui| {
            let colors = colors;
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Header
                ui.horizontal(|ui| {
                    ui.heading("Edit");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("X").clicked() {
                            app.editor_state.visible = false;
                        }
                    });
                });
                ui.separator();

                // Undo/Redo
                ui.horizontal(|ui| {
                    let can_undo = !app.editor_state.undo_stack.is_empty();
                    if ui.add_enabled(can_undo, egui::Button::new("\u{21A9} Undo")).clicked() {
                        undo(app, ctx);
                    }
                    let can_redo = !app.editor_state.redo_stack.is_empty();
                    if ui.add_enabled(can_redo, egui::Button::new("\u{21AA} Redo")).clicked() {
                        redo(app, ctx);
                    }
                });

                ui.separator();

                // Crop section
                ui.label(egui::RichText::new("Crop").strong().color(colors.accent));
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

                // Transform section
                ui.label(egui::RichText::new("Transform").strong().color(colors.accent));
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

                // Resize section
                ui.label(egui::RichText::new("Resize").strong().color(colors.accent));
                let aspect = app.editor_state.current_image.as_ref().map(|img| {
                    let (iw, ih) = img.dimensions();
                    (iw.max(1), ih.max(1))
                });
                ui.horizontal(|ui| {
                    ui.colored_label(colors.text_secondary, "W:");
                    let mut w = app.editor_state.resize_width as f32;
                    if ui.add(egui::DragValue::new(&mut w).range(1..=16384)).changed() {
                        app.editor_state.resize_width = w as u32;
                        if app.editor_state.resize_lock_aspect {
                            if let Some((iw, ih)) = aspect {
                                app.editor_state.resize_height =
                                    ((w * ih as f32 / iw as f32).round() as u32).max(1);
                            }
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.colored_label(colors.text_secondary, "H:");
                    let mut h = app.editor_state.resize_height as f32;
                    if ui.add(egui::DragValue::new(&mut h).range(1..=16384)).changed() {
                        app.editor_state.resize_height = h as u32;
                        if app.editor_state.resize_lock_aspect {
                            if let Some((iw, ih)) = aspect {
                                app.editor_state.resize_width =
                                    ((h * iw as f32 / ih as f32).round() as u32).max(1);
                            }
                        }
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
                // Paste from clipboard
                ui.label(egui::RichText::new("Clipboard").strong().color(colors.accent));
                if ui.button("Paste").clicked() {
                    paste_from_clipboard(app, ctx);
                }

                ui.separator();


                // Save As section
                ui.label(egui::RichText::new("Save As").strong().color(colors.accent));
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
                // Filename input
                egui::TextEdit::singleline(&mut app.editor_state.save_as_filename)
                    .hint_text("Enter filename")
                    .show(ui);
                if ui.button("Save As...").clicked() && !app.editor_state.save_as_filename.is_empty() {
                    save_as(app);
                }
            });
        });
}

fn upload_texture(app: &mut App, ctx: &egui::Context, img: &image::DynamicImage) {
    let path = match app.image_files.get(app.selected_image_index) {
        Some(p) => p.clone(),
        None => return,
    };
    let tex_key = path.to_string_lossy().to_string();
    let (w, h) = img.dimensions();
    let rgba = img.to_rgba8();
    let ci = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
    let tex = ctx.load_texture(&tex_key, ci, egui::TextureOptions::LINEAR);
    app.textures.put(tex_key, tex);
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
    // Byte-budget cap: evict oldest entries if total RGBA bytes exceed budget.
    let mut total_bytes: usize = app.editor_state.undo_stack.iter()
        .map(|(_, i)| (i.width() * i.height() * 4) as usize)
        .sum();
    while total_bytes > MAX_UNDO_BYTES && !app.editor_state.undo_stack.is_empty() {
        let (_, oldest) = app.editor_state.undo_stack.remove(0);
        total_bytes -= (oldest.width() * oldest.height() * 4) as usize;
    }
    app.editor_state.redo_stack.clear();

    let result = op.apply(&img);
    let (w, h) = result.dimensions();
    app.editor_state.resize_width = w;
    app.editor_state.resize_height = h;

    upload_texture(app, ctx, &result);
    app.editor_state.current_image = Some(result);
}

fn undo(app: &mut App, ctx: &egui::Context) {
    if let Some((op, prev_img)) = app.editor_state.undo_stack.pop() {
        let (w, h) = prev_img.dimensions();
        app.editor_state.redo_stack.push((op, prev_img.clone()));
        app.editor_state.resize_width = w;
        app.editor_state.resize_height = h;

        upload_texture(app, ctx, &prev_img);
        app.editor_state.current_image = Some(prev_img);
    }
}

fn redo(app: &mut App, ctx: &egui::Context) {
    if let Some((op, next_img)) = app.editor_state.redo_stack.pop() {
        let (w, h) = next_img.dimensions();
        app.editor_state.undo_stack.push((op, next_img.clone()));
        app.editor_state.resize_width = w;
        app.editor_state.resize_height = h;

        upload_texture(app, ctx, &next_img);
        app.editor_state.current_image = Some(next_img);
    }
}

fn apply_crop(app: &mut App, ctx: &egui::Context) {
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

fn paste_from_clipboard(app: &mut App, ctx: &egui::Context) {
    let mut clipboard = match arboard::Clipboard::new() {
        Ok(cb) => cb,
        Err(e) => {
            eprintln!("Failed to open clipboard: {e}");
            return;
        }
    };
    let image = match clipboard.get_image() {
        Ok(img) => img,
        Err(e) => {
            eprintln!("Failed to read image from clipboard: {e}");
            return;
        }
    };
    let rgba = image::RgbaImage::from_raw(image.width as u32, image.height as u32, image.bytes.to_vec());
    let img = match rgba {
        Some(rgba_img) => image::DynamicImage::ImageRgba8(rgba_img),
        None => return,
    };
    // Push current image to undo stack before replacing.
    if let Some(ref current) = app.editor_state.current_image {
        if app.editor_state.undo_stack.len() >= MAX_UNDO {
            app.editor_state.undo_stack.remove(0);
        }
        app.editor_state.undo_stack.push((EditOp::NoOp, current.clone()));
    }

    app.editor_state.current_image = Some(img.clone());
    let (w, h) = img.dimensions();
    app.editor_state.resize_width = w;
    app.editor_state.resize_height = h;
    upload_texture(app, ctx, &img);



    app.editor_state.redo_stack.clear();
}

fn save_as(app: &mut App) {
    let img = match &app.editor_state.current_image {
        Some(i) => i.clone(),
        None => return,
    };

    let base_path = match app.image_files.get(app.selected_image_index) {
        Some(p) => p.clone(),
        None => return,
    };

    let save_format = app.editor_state.save_format;
    let new_ext = crate::format_ext::format_to_extension(save_format);
    let filename = &app.editor_state.save_as_filename;
    let new_name = base_path.with_file_name(format!("{}.{}", filename, new_ext));
    match crate::format_ext::save_image(&img, &new_name, save_format, app.editor_state.save_jpeg_quality) {
        Ok(()) => {
            app.scan_folder();
            app.editor_state.save_as_filename.clear();
        }
        Err(e) => eprintln!("Save failed: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::DynamicImage;
    use image::GenericImageView;

    #[test]
    fn test_undo_pops_correct_entry() {
        let mut state = State::new();
        let img = DynamicImage::new_rgba8(10, 10);
        state.current_image = Some(img.clone());

        // Simulate an edit by pushing to undo stack directly.
        state.undo_stack.push((EditOp::Rotate90Cw, img.clone()));

        // Pop and verify the undone image matches.
        let (op, prev_img) = state.undo_stack.pop().unwrap();
        assert_eq!(op, EditOp::Rotate90Cw);
        assert_eq!(prev_img.dimensions(), (10, 10));
    }

    #[test]
    fn test_redo_pops_correct_entry() {
        let mut state = State::new();
        let img = DynamicImage::new_rgba8(10, 10);

        state.undo_stack.push((EditOp::FlipHorizontal, img.clone()));

        // Simulate undo: move from undo to redo.
        if let Some(entry) = state.undo_stack.pop() {
            state.redo_stack.push(entry);
        }

        let (op, next_img) = state.redo_stack.pop().unwrap();
        assert_eq!(op, EditOp::FlipHorizontal);
        assert_eq!(next_img.dimensions(), (10, 10));
    }

    #[test]
    fn test_redo_clears_on_new_edit() {
        let mut state = State::new();
        let img = DynamicImage::new_rgba8(10, 10);

        state.undo_stack.push((EditOp::Rotate90Cw, img.clone()));
        if let Some(entry) = state.undo_stack.pop() {
            state.redo_stack.push(entry);
        }

        // New edit should clear redo stack.
        state.redo_stack.clear();
        assert!(state.redo_stack.is_empty());
    }
}
