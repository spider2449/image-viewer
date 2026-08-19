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
    pub undo_stack: Vec<DynamicImage>,
    pub redo_stack: Vec<DynamicImage>,
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
            self.save_as_filename = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
        }
    }

    fn image_bytes(image: &DynamicImage) -> usize {
        image.as_bytes().len()
    }

    fn push_bounded(stack: &mut Vec<DynamicImage>, image: DynamicImage) {
        stack.push(image);
        while stack.len() > MAX_UNDO {
            stack.remove(0);
        }
        let mut total: usize = stack.iter().map(Self::image_bytes).sum();
        while total > MAX_UNDO_BYTES && stack.len() > 1 {
            total -= Self::image_bytes(&stack.remove(0));
        }
    }

    fn install_edit(&mut self, result: DynamicImage) -> bool {
        let Some(current) = self.current_image.take() else {
            return false;
        };
        self.current_image = Some(result);
        Self::push_bounded(&mut self.undo_stack, current);
        self.redo_stack.clear();
        true
    }

    fn apply_operation(&mut self, op: &EditOp) -> bool {
        let Some(current) = self.current_image.as_ref() else {
            return false;
        };
        self.install_edit(op.apply(current))
    }

    fn replace_image(&mut self, image: DynamicImage) -> bool {
        self.install_edit(image)
    }

    fn undo_edit(&mut self) -> bool {
        if self.current_image.is_none() {
            return false;
        }
        let Some(previous) = self.undo_stack.pop() else {
            return false;
        };
        let current = self.current_image.take().unwrap();
        Self::push_bounded(&mut self.redo_stack, current);
        self.current_image = Some(previous);
        true
    }

    fn redo_edit(&mut self) -> bool {
        if self.current_image.is_none() {
            return false;
        }
        let Some(next) = self.redo_stack.pop() else {
            return false;
        };
        let current = self.current_image.take().unwrap();
        Self::push_bounded(&mut self.undo_stack, current);
        self.current_image = Some(next);
        true
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
                    if ui
                        .add_enabled(can_undo, egui::Button::new("\u{21A9} Undo"))
                        .clicked()
                    {
                        undo(app, ctx);
                    }
                    let can_redo = !app.editor_state.redo_stack.is_empty();
                    if ui
                        .add_enabled(can_redo, egui::Button::new("\u{21AA} Redo"))
                        .clicked()
                    {
                        redo(app, ctx);
                    }
                });

                ui.separator();

                // Crop section
                ui.label(egui::RichText::new("Crop").strong().color(colors.accent));
                if ui
                    .selectable_label(app.editor_state.crop_active, "Crop")
                    .clicked()
                {
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
                ui.label(
                    egui::RichText::new("Transform")
                        .strong()
                        .color(colors.accent),
                );
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
                    if ui
                        .add(egui::DragValue::new(&mut w).range(1..=16384))
                        .changed()
                    {
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
                    if ui
                        .add(egui::DragValue::new(&mut h).range(1..=16384))
                        .changed()
                    {
                        app.editor_state.resize_height = h as u32;
                        if app.editor_state.resize_lock_aspect {
                            if let Some((iw, ih)) = aspect {
                                app.editor_state.resize_width =
                                    ((h * iw as f32 / ih as f32).round() as u32).max(1);
                            }
                        }
                    }
                });
                ui.checkbox(
                    &mut app.editor_state.resize_lock_aspect,
                    "Lock aspect ratio",
                );
                if ui.button("Apply").clicked() {
                    apply_op(
                        app,
                        ctx,
                        EditOp::Resize {
                            width: app.editor_state.resize_width,
                            height: app.editor_state.resize_height,
                        },
                    );
                }

                ui.separator();
                // Paste from clipboard
                ui.label(
                    egui::RichText::new("Clipboard")
                        .strong()
                        .color(colors.accent),
                );
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
                    ui.add(
                        egui::Slider::new(&mut app.editor_state.save_jpeg_quality, 1..=100)
                            .text("Quality"),
                    );
                }
                // Filename input
                egui::TextEdit::singleline(&mut app.editor_state.save_as_filename)
                    .hint_text("Enter filename")
                    .show(ui);
                if ui.button("Save As...").clicked()
                    && !app.editor_state.save_as_filename.is_empty()
                {
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
    if app.editor_state.apply_operation(&op) {
        sync_editor_image(app, ctx);
    }
}

fn undo(app: &mut App, ctx: &egui::Context) {
    if app.editor_state.undo_edit() {
        sync_editor_image(app, ctx);
    }
}

fn redo(app: &mut App, ctx: &egui::Context) {
    if app.editor_state.redo_edit() {
        sync_editor_image(app, ctx);
    }
}

fn sync_editor_image(app: &mut App, ctx: &egui::Context) {
    let Some(image) = app.editor_state.current_image.clone() else {
        return;
    };
    let (w, h) = image.dimensions();
    app.editor_state.resize_width = w;
    app.editor_state.resize_height = h;
    upload_texture(app, ctx, &image);
}

fn apply_crop(app: &mut App, ctx: &egui::Context) {
    if let (Some(start), Some(end)) = (app.editor_state.crop_start, app.editor_state.crop_end) {
        let x = start.x.min(end.x) as u32;
        let y = start.y.min(end.y) as u32;
        let w = (start.x - end.x).abs() as u32;
        let h = (start.y - end.y).abs() as u32;
        if w > 0 && h > 0 {
            apply_op(
                app,
                ctx,
                EditOp::Crop {
                    x,
                    y,
                    width: w,
                    height: h,
                },
            );
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
    let rgba = image::RgbaImage::from_raw(
        image.width as u32,
        image.height as u32,
        image.bytes.to_vec(),
    );
    let img = match rgba {
        Some(rgba_img) => image::DynamicImage::ImageRgba8(rgba_img),
        None => return,
    };
    if app.editor_state.replace_image(img) {
        sync_editor_image(app, ctx);
    }
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
    let filename = app.editor_state.save_as_filename.clone();
    let new_name = base_path.with_file_name(format!("{}.{}", filename, new_ext));
    match crate::format_ext::save_image(
        &img,
        &new_name,
        save_format,
        app.editor_state.save_jpeg_quality,
    ) {
        Ok(()) => {
            app.scan_folder();
            // Update the filename field to reflect the saved file.
            app.editor_state.save_as_filename = filename.clone();
            // Switch viewer to the newly saved file — load_image populates the filename field.
            if let Some(new_index) = app.image_files.iter().position(|p| p == &new_name) {
                app.switch_to_viewer(new_index);
            }
        }
        Err(e) => eprintln!("Save failed: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::DynamicImage;
    use image::GenericImageView;

    fn colored_image(width: u32, height: u32, value: u8) -> DynamicImage {
        DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            width,
            height,
            image::Rgba([value, 0, 0, 255]),
        ))
    }

    #[test]
    fn test_rotate_undo_redo_restores_both_states() {
        let mut state = State::new();
        state.current_image = Some(colored_image(2, 3, 10));
        assert!(state.apply_operation(&EditOp::Rotate90Cw));
        assert_eq!(state.current_image.as_ref().unwrap().dimensions(), (3, 2));
        assert!(state.undo_edit());
        assert_eq!(state.current_image.as_ref().unwrap().dimensions(), (2, 3));
        assert!(state.redo_edit());
        assert_eq!(state.current_image.as_ref().unwrap().dimensions(), (3, 2));
    }

    #[test]
    fn test_resize_undo_redo_restores_both_states() {
        let mut state = State::new();
        state.current_image = Some(colored_image(10, 20, 10));
        assert!(state.apply_operation(&EditOp::Resize {
            width: 4,
            height: 6
        }));
        assert!(state.undo_edit());
        assert_eq!(state.current_image.as_ref().unwrap().dimensions(), (10, 20));
        assert!(state.redo_edit());
        assert_eq!(state.current_image.as_ref().unwrap().dimensions(), (4, 6));
    }

    #[test]
    fn test_replacement_undo_redo_restores_pasted_pixels() {
        let mut state = State::new();
        state.current_image = Some(colored_image(2, 2, 10));
        assert!(state.replace_image(colored_image(3, 3, 20)));
        assert!(state.undo_edit());
        assert_eq!(
            state
                .current_image
                .as_ref()
                .unwrap()
                .to_rgba8()
                .get_pixel(0, 0)[0],
            10
        );
        assert!(state.redo_edit());
        assert_eq!(
            state
                .current_image
                .as_ref()
                .unwrap()
                .to_rgba8()
                .get_pixel(0, 0)[0],
            20
        );
    }

    #[test]
    fn test_new_edit_after_undo_clears_redo() {
        let mut state = State::new();
        state.current_image = Some(colored_image(2, 3, 10));
        assert!(state.apply_operation(&EditOp::Rotate90Cw));
        assert!(state.undo_edit());
        assert!(!state.redo_stack.is_empty());
        assert!(state.apply_operation(&EditOp::FlipHorizontal));
        assert!(state.redo_stack.is_empty());
    }

    #[test]
    fn test_history_limit_keeps_newest_usable_state() {
        let mut state = State::new();
        state.current_image = Some(colored_image(1, 1, 0));
        for value in 1..=60 {
            assert!(state.replace_image(colored_image(1, 1, value)));
        }
        assert_eq!(state.undo_stack.len(), MAX_UNDO);
        assert!(state.undo_edit());
        assert_eq!(
            state
                .current_image
                .as_ref()
                .unwrap()
                .to_rgba8()
                .get_pixel(0, 0)[0],
            59
        );
    }
}
