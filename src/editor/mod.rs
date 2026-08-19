pub mod operations;

use crate::app::App;
use eframe::egui;
use image::{DynamicImage, GenericImageView};
use operations::{ColorAdjustments, EditOp, ResizeFilter, ResizeMode};
use std::path::PathBuf;
use std::sync::Arc;

pub struct State {
    pub visible: bool,
    pub history: Vec<EditOp>,
    pub history_index: usize,
    pub pending_image_index: Option<usize>,
    pub pending_browser_exit: bool,
    pub original_image: Option<DynamicImage>,
    pub current_image: Option<DynamicImage>,
    pub crop_active: bool,
    pub crop_start: Option<egui::Pos2>,
    pub crop_end: Option<egui::Pos2>,
    pub resize_width: u32,
    pub resize_height: u32,
    pub resize_lock_aspect: bool,
    pub resize_mode: ResizeMode,
    pub resize_filter: ResizeFilter,
    pub adjustments: ColorAdjustments,
    pub blur_sigma: f32,
    pub sharpen_sigma: f32,
    pub sharpen_threshold: i32,
    pub save_format: &'static str,
    pub save_jpeg_quality: u8,
    pub save_as_filename: String,
}

impl State {
    pub fn new() -> Self {
        Self {
            visible: false,
            history: Vec::new(),
            history_index: 0,
            pending_image_index: None,
            pending_browser_exit: false,
            original_image: None,
            current_image: None,
            crop_active: false,
            crop_start: None,
            crop_end: None,
            resize_width: 0,
            resize_height: 0,
            resize_lock_aspect: true,
            resize_mode: ResizeMode::Exact,
            resize_filter: ResizeFilter::Lanczos3,
            adjustments: ColorAdjustments::default(),
            blur_sigma: 2.0,
            sharpen_sigma: 1.0,
            sharpen_threshold: 2,
            save_format: "png",
            save_jpeg_quality: 90,
            save_as_filename: String::new(),
        }
    }

    pub fn load_image(&mut self, path: &PathBuf) {
        if let Ok(img) = image::open(path) {
            let (w, h) = img.dimensions();
            self.original_image = Some(img.clone());
            self.current_image = Some(img);
            self.resize_width = w;
            self.resize_height = h;
            self.adjustments = ColorAdjustments::default();
            self.history.clear();
            self.history_index = 0;
            self.save_as_filename = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
        }
    }

    fn apply_operation(&mut self, op: &EditOp) -> bool {
        let Some(current) = self.current_image.as_ref() else {
            return false;
        };
        let result = op.apply(current);
        self.history.truncate(self.history_index);
        self.history.push(op.clone());
        self.history_index += 1;
        self.current_image = Some(result);
        true
    }

    fn replace_image(&mut self, image: DynamicImage) -> bool {
        self.apply_operation(&EditOp::Replace(Arc::new(image)))
    }

    fn commit_adjustments(&mut self) -> bool {
        if self.adjustments.is_neutral() {
            return false;
        }
        let adjustments = self.adjustments;
        let committed = self.apply_operation(&EditOp::Adjust(adjustments));
        if committed {
            self.adjustments = ColorAdjustments::default();
        }
        committed
    }

    fn undo_edit(&mut self) -> bool {
        if !self.can_undo() {
            return false;
        }
        self.history_index -= 1;
        self.rebuild_current();
        true
    }

    fn redo_edit(&mut self) -> bool {
        if !self.can_redo() {
            return false;
        }
        let operation = &self.history[self.history_index];
        let Some(current) = self.current_image.as_ref() else {
            return false;
        };
        self.current_image = Some(operation.apply(current));
        self.history_index += 1;
        true
    }

    fn revert_all(&mut self) -> bool {
        if !self.can_undo() {
            return false;
        }
        self.history_index = 0;
        self.rebuild_current();
        true
    }

    fn rebuild_current(&mut self) {
        let Some(mut image) = self.original_image.clone() else {
            return;
        };
        for operation in &self.history[..self.history_index] {
            image = operation.apply(&image);
        }
        self.current_image = Some(image);
    }

    fn can_undo(&self) -> bool {
        self.history_index > 0
    }

    fn can_redo(&self) -> bool {
        self.history_index < self.history.len()
    }

    pub(crate) fn has_unsaved_changes(&self) -> bool {
        self.history_index > 0 || !self.adjustments.is_neutral()
    }

    fn mark_saved(&mut self) {
        self.original_image = self.current_image.clone();
        self.history.clear();
        self.history_index = 0;
        self.adjustments = ColorAdjustments::default();
    }

    fn discard_changes(&mut self) {
        self.current_image = self.original_image.clone();
        self.history.clear();
        self.history_index = 0;
        self.adjustments = ColorAdjustments::default();
    }
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    if !app.editor_state.visible {
        return;
    }

    handle_history_shortcuts(app, ctx);
    let colors = app.theme_colors();
    let mut preview_changed = false;

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
                ui.horizontal(|ui| {
                    ui.heading("Edit");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("X").clicked() {
                            discard_adjustment_preview(app);
                            app.editor_state.visible = false;
                        }
                    });
                });
                ui.separator();

                ui.horizontal_wrapped(|ui| {
                    let has_preview = !app.editor_state.adjustments.is_neutral();
                    let can_undo = has_preview || app.editor_state.can_undo();
                    if ui
                        .add_enabled(can_undo, egui::Button::new("\u{21A9} Undo"))
                        .clicked()
                    {
                        undo(app, ctx);
                    }
                    let can_redo = !has_preview && app.editor_state.can_redo();
                    if ui
                        .add_enabled(can_redo, egui::Button::new("\u{21AA} Redo"))
                        .clicked()
                    {
                        redo(app, ctx);
                    }
                    if ui
                        .add_enabled(can_undo, egui::Button::new("Revert All"))
                        .on_hover_text("Return to the image loaded from disk")
                        .clicked()
                    {
                        revert_all(app, ctx);
                    }
                });
                let edit_count = app.editor_state.history_index;
                if edit_count > 0 || !app.editor_state.adjustments.is_neutral() {
                    let preview_suffix = if app.editor_state.adjustments.is_neutral() {
                        ""
                    } else {
                        " + live preview"
                    };
                    ui.colored_label(
                        colors.text_secondary,
                        format!("{edit_count} unsaved edit(s){preview_suffix}"),
                    );
                }

                ui.separator();
                egui::CollapsingHeader::new(
                    egui::RichText::new("Adjustments")
                        .strong()
                        .color(colors.accent),
                )
                .default_open(true)
                .show(ui, |ui| {
                    preview_changed |= adjustment_controls(ui, &mut app.editor_state.adjustments);
                    ui.horizontal(|ui| {
                        let can_apply = !app.editor_state.adjustments.is_neutral();
                        if ui
                            .add_enabled(can_apply, egui::Button::new("Apply"))
                            .clicked()
                        {
                            if app.editor_state.commit_adjustments() {
                                sync_editor_image(app, ctx);
                            }
                        }
                        if ui
                            .add_enabled(can_apply, egui::Button::new("Reset"))
                            .clicked()
                        {
                            reset_adjustment_preview(app, ctx);
                        }
                    });
                });

                ui.separator();
                egui::CollapsingHeader::new(
                    egui::RichText::new("Quick Effects")
                        .strong()
                        .color(colors.accent),
                )
                .default_open(true)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for (label, op) in [
                            ("Auto Contrast", EditOp::AutoContrast),
                            ("Grayscale", EditOp::Grayscale),
                            ("Sepia", EditOp::Sepia),
                            ("Invert", EditOp::Invert),
                        ] {
                            if ui.button(label).clicked() {
                                apply_op(app, ctx, op);
                            }
                        }
                    });
                    ui.add(
                        egui::Slider::new(&mut app.editor_state.blur_sigma, 0.1..=20.0)
                            .text("Blur radius"),
                    );
                    if ui.button("Apply Blur").clicked() {
                        apply_op(app, ctx, EditOp::Blur(app.editor_state.blur_sigma));
                    }
                    ui.add(
                        egui::Slider::new(&mut app.editor_state.sharpen_sigma, 0.1..=10.0)
                            .text("Sharpen radius"),
                    );
                    ui.add(
                        egui::Slider::new(&mut app.editor_state.sharpen_threshold, 0..=50)
                            .text("Threshold"),
                    );
                    if ui.button("Apply Sharpen").clicked() {
                        apply_op(
                            app,
                            ctx,
                            EditOp::Sharpen {
                                sigma: app.editor_state.sharpen_sigma,
                                threshold: app.editor_state.sharpen_threshold,
                            },
                        );
                    }
                });

                ui.separator();
                egui::CollapsingHeader::new(
                    egui::RichText::new("Transform")
                        .strong()
                        .color(colors.accent),
                )
                .default_open(true)
                .show(ui, |ui| transform_controls(app, ctx, ui));

                ui.separator();
                egui::CollapsingHeader::new(
                    egui::RichText::new("Resize").strong().color(colors.accent),
                )
                .default_open(true)
                .show(ui, |ui| {
                    resize_controls(app, ctx, ui, colors.text_secondary)
                });

                ui.separator();
                egui::CollapsingHeader::new(
                    egui::RichText::new("Clipboard")
                        .strong()
                        .color(colors.accent),
                )
                .show(ui, |ui| {
                    if ui.button("Paste Image").clicked() {
                        paste_from_clipboard(app, ctx);
                    }
                });

                ui.separator();
                egui::CollapsingHeader::new(
                    egui::RichText::new("Save As").strong().color(colors.accent),
                )
                .default_open(true)
                .show(ui, |ui| save_controls(app, ui));
            });
        });

    if preview_changed {
        preview_adjustments(app, ctx);
    }
    show_unsaved_navigation_dialog(app, ctx);
}

fn show_unsaved_navigation_dialog(app: &mut App, ctx: &egui::Context) {
    let target_index = app.editor_state.pending_image_index;
    let browser_exit = app.editor_state.pending_browser_exit;
    if target_index.is_none() && !browser_exit {
        return;
    }
    egui::Window::new("Unsaved edits")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.label("This image has edits that have not been saved.");
            ui.label("Save them first, keep editing, or explicitly discard them.");
            ui.horizontal(|ui| {
                if ui.button("Keep Editing").clicked() {
                    app.editor_state.pending_image_index = None;
                    app.editor_state.pending_browser_exit = false;
                }
                if browser_exit && ui.button("Browser (Keep Edits)").clicked() {
                    app.editor_state.pending_browser_exit = false;
                    app.mode = crate::app::Mode::Browser;
                }
                let discard_label = if browser_exit {
                    "Discard and Return"
                } else {
                    "Discard and Open"
                };
                if ui.button(discard_label).clicked() {
                    if let Some(path) = app.image_files.get(app.selected_image_index) {
                        app.textures.pop(path.to_string_lossy().as_ref());
                    }
                    app.editor_state.pending_image_index = None;
                    app.editor_state.pending_browser_exit = false;
                    app.editor_state.discard_changes();
                    if browser_exit {
                        app.mode = crate::app::Mode::Browser;
                    } else if let Some(target_index) = target_index {
                        app.select_image(target_index);
                    }
                }
            });
        });
}

fn handle_history_shortcuts(app: &mut App, ctx: &egui::Context) {
    if ctx.wants_keyboard_input() {
        return;
    }
    let (undo_requested, redo_requested) = ctx.input(|input| {
        let ctrl = input.modifiers.ctrl;
        let shift = input.modifiers.shift;
        let redo =
            ctrl && ((shift && input.key_pressed(egui::Key::Z)) || input.key_pressed(egui::Key::Y));
        let undo = ctrl && !shift && input.key_pressed(egui::Key::Z);
        (undo, redo)
    });
    if redo_requested && app.editor_state.adjustments.is_neutral() {
        redo(app, ctx);
    } else if undo_requested {
        undo(app, ctx);
    }
}

fn adjustment_controls(ui: &mut egui::Ui, adjustments: &mut ColorAdjustments) -> bool {
    let mut changed = false;
    changed |= ui
        .add(
            egui::Slider::new(&mut adjustments.exposure, -4.0..=4.0)
                .text("Exposure")
                .suffix(" EV"),
        )
        .changed();
    changed |= ui
        .add(egui::Slider::new(&mut adjustments.brightness, -100.0..=100.0).text("Brightness"))
        .changed();
    changed |= ui
        .add(egui::Slider::new(&mut adjustments.contrast, -100.0..=100.0).text("Contrast"))
        .changed();
    changed |= ui
        .add(egui::Slider::new(&mut adjustments.saturation, -100.0..=100.0).text("Saturation"))
        .changed();
    changed |= ui
        .add(
            egui::Slider::new(&mut adjustments.hue, -180..=180)
                .text("Hue")
                .suffix("\u{00B0}"),
        )
        .changed();
    changed |= ui
        .add(egui::Slider::new(&mut adjustments.temperature, -100.0..=100.0).text("Temperature"))
        .changed();
    changed |= ui
        .add(egui::Slider::new(&mut adjustments.tint, -100.0..=100.0).text("Tint"))
        .changed();
    changed |= ui
        .add(egui::Slider::new(&mut adjustments.gamma, 0.1..=3.0).text("Gamma"))
        .changed();
    changed
}

fn transform_controls(app: &mut App, ctx: &egui::Context, ui: &mut egui::Ui) {
    ui.horizontal_wrapped(|ui| {
        if ui.button("Rotate 90\u{00B0} CW").clicked() {
            apply_op(app, ctx, EditOp::Rotate90Cw);
        }
        if ui.button("Rotate 90\u{00B0} CCW").clicked() {
            apply_op(app, ctx, EditOp::Rotate90Ccw);
        }
        if ui.button("Rotate 180\u{00B0}").clicked() {
            apply_op(app, ctx, EditOp::Rotate180);
        }
        if ui.button("Flip H").clicked() {
            apply_op(app, ctx, EditOp::FlipHorizontal);
        }
        if ui.button("Flip V").clicked() {
            apply_op(app, ctx, EditOp::FlipVertical);
        }
    });
    if ui
        .selectable_label(app.editor_state.crop_active, "Crop Tool")
        .clicked()
    {
        app.editor_state.crop_active = !app.editor_state.crop_active;
        if !app.editor_state.crop_active {
            app.editor_state.crop_start = None;
            app.editor_state.crop_end = None;
        }
    }
    if app.editor_state.crop_active {
        ui.label("Drag a rectangle over the image.");
        ui.horizontal(|ui| {
            if ui.button("Apply Crop").clicked() {
                apply_crop(app, ctx);
            }
            if ui.button("Cancel").clicked() {
                app.editor_state.crop_active = false;
                app.editor_state.crop_start = None;
                app.editor_state.crop_end = None;
            }
        });
    }
}

fn resize_controls(
    app: &mut App,
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    secondary: egui::Color32,
) {
    let source_size = app
        .editor_state
        .current_image
        .as_ref()
        .map(|image| image.dimensions());
    ui.horizontal_wrapped(|ui| {
        for percent in [25, 50, 100, 200] {
            if ui.button(format!("{percent}%")).clicked() {
                if let Some((width, height)) = source_size {
                    app.editor_state.resize_width = ((width as u64 * percent) / 100).max(1) as u32;
                    app.editor_state.resize_height =
                        ((height as u64 * percent) / 100).max(1) as u32;
                }
            }
        }
    });
    ui.horizontal_wrapped(|ui| {
        for (label, width, height) in [("HD", 1280, 720), ("FHD", 1920, 1080), ("4K", 3840, 2160)] {
            if ui
                .button(label)
                .on_hover_text(format!("{width} x {height}"))
                .clicked()
            {
                app.editor_state.resize_width = width;
                app.editor_state.resize_height = height;
            }
        }
    });

    let aspect = source_size.map(|(width, height)| (width.max(1), height.max(1)));
    ui.horizontal(|ui| {
        ui.colored_label(secondary, "Width:");
        let changed = ui
            .add(egui::DragValue::new(&mut app.editor_state.resize_width).range(1..=32768))
            .changed();
        if changed && app.editor_state.resize_lock_aspect {
            if let Some((width, height)) = aspect {
                app.editor_state.resize_height =
                    ((app.editor_state.resize_width as f64 * height as f64 / width as f64).round()
                        as u32)
                        .max(1);
            }
        }
    });
    ui.horizontal(|ui| {
        ui.colored_label(secondary, "Height:");
        let changed = ui
            .add(egui::DragValue::new(&mut app.editor_state.resize_height).range(1..=32768))
            .changed();
        if changed && app.editor_state.resize_lock_aspect {
            if let Some((width, height)) = aspect {
                app.editor_state.resize_width =
                    ((app.editor_state.resize_height as f64 * width as f64 / height as f64).round()
                        as u32)
                        .max(1);
            }
        }
    });
    ui.checkbox(
        &mut app.editor_state.resize_lock_aspect,
        "Lock source aspect ratio",
    );
    egui::ComboBox::from_label("Mode")
        .selected_text(app.editor_state.resize_mode.label())
        .show_ui(ui, |ui| {
            for mode in [ResizeMode::Exact, ResizeMode::Fit, ResizeMode::Fill] {
                ui.selectable_value(&mut app.editor_state.resize_mode, mode, mode.label());
            }
        });
    egui::ComboBox::from_label("Resampling")
        .selected_text(app.editor_state.resize_filter.label())
        .show_ui(ui, |ui| {
            for filter in [
                ResizeFilter::Nearest,
                ResizeFilter::Triangle,
                ResizeFilter::CatmullRom,
                ResizeFilter::Gaussian,
                ResizeFilter::Lanczos3,
            ] {
                ui.selectable_value(&mut app.editor_state.resize_filter, filter, filter.label());
            }
        });
    if ui.button("Apply Resize").clicked() {
        apply_op(
            app,
            ctx,
            EditOp::Resize {
                width: app.editor_state.resize_width,
                height: app.editor_state.resize_height,
                mode: app.editor_state.resize_mode,
                filter: app.editor_state.resize_filter,
            },
        );
    }
}

fn save_controls(app: &mut App, ui: &mut egui::Ui) {
    egui::ComboBox::new("save_format", "")
        .selected_text(app.editor_state.save_format.to_uppercase())
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut app.editor_state.save_format, "png", "PNG");
            ui.selectable_value(&mut app.editor_state.save_format, "jpeg", "JPEG");
            ui.selectable_value(&mut app.editor_state.save_format, "bmp", "BMP");
            ui.selectable_value(&mut app.editor_state.save_format, "webp", "WEBP");
        });
    if app.editor_state.save_format == "jpeg" {
        ui.add(egui::Slider::new(&mut app.editor_state.save_jpeg_quality, 1..=100).text("Quality"));
    }
    egui::TextEdit::singleline(&mut app.editor_state.save_as_filename)
        .hint_text("Enter filename")
        .show(ui);
    if ui.button("Save As...").clicked() && !app.editor_state.save_as_filename.is_empty() {
        save_as(app);
    }
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
        app.editor_state.adjustments = ColorAdjustments::default();
        sync_editor_image(app, ctx);
    }
}

fn undo(app: &mut App, ctx: &egui::Context) {
    if !app.editor_state.adjustments.is_neutral() {
        reset_adjustment_preview(app, ctx);
        return;
    }
    if app.editor_state.undo_edit() {
        sync_editor_image(app, ctx);
    }
}

fn redo(app: &mut App, ctx: &egui::Context) {
    if app.editor_state.redo_edit() {
        app.editor_state.adjustments = ColorAdjustments::default();
        sync_editor_image(app, ctx);
    }
}

fn revert_all(app: &mut App, ctx: &egui::Context) {
    app.editor_state.adjustments = ColorAdjustments::default();
    if app.editor_state.revert_all() {
        sync_editor_image(app, ctx);
    } else {
        reset_adjustment_preview(app, ctx);
    }
}

fn preview_adjustments(app: &mut App, ctx: &egui::Context) {
    let Some(image) = app.editor_state.current_image.as_ref() else {
        return;
    };
    let preview = EditOp::Adjust(app.editor_state.adjustments).apply(image);
    upload_texture(app, ctx, &preview);
}

fn reset_adjustment_preview(app: &mut App, ctx: &egui::Context) {
    app.editor_state.adjustments = ColorAdjustments::default();
    let Some(image) = app.editor_state.current_image.clone() else {
        return;
    };
    upload_texture(app, ctx, &image);
}

pub(crate) fn discard_adjustment_preview(app: &mut App) {
    if app.editor_state.adjustments.is_neutral() {
        return;
    }
    app.editor_state.adjustments = ColorAdjustments::default();
    if let Some(path) = app.image_files.get(app.selected_image_index) {
        app.textures.pop(path.to_string_lossy().as_ref());
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
        app.editor_state.adjustments = ColorAdjustments::default();
        sync_editor_image(app, ctx);
    }
}

fn save_as(app: &mut App) {
    app.editor_state.commit_adjustments();
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
            app.editor_state.mark_saved();
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

    fn state_with_image(image: DynamicImage) -> State {
        let mut state = State::new();
        state.original_image = Some(image.clone());
        state.current_image = Some(image);
        state
    }

    #[test]
    fn test_rotate_undo_redo_restores_both_states() {
        let mut state = state_with_image(colored_image(2, 3, 10));
        assert!(state.apply_operation(&EditOp::Rotate90Cw));
        assert_eq!(state.current_image.as_ref().unwrap().dimensions(), (3, 2));
        assert!(state.undo_edit());
        assert_eq!(state.current_image.as_ref().unwrap().dimensions(), (2, 3));
        assert!(state.redo_edit());
        assert_eq!(state.current_image.as_ref().unwrap().dimensions(), (3, 2));
    }

    #[test]
    fn test_resize_undo_redo_restores_both_states() {
        let mut state = state_with_image(colored_image(10, 20, 10));
        assert!(state.apply_operation(&EditOp::Resize {
            width: 4,
            height: 6,
            mode: ResizeMode::Exact,
            filter: ResizeFilter::Lanczos3,
        }));
        assert!(state.undo_edit());
        assert_eq!(state.current_image.as_ref().unwrap().dimensions(), (10, 20));
        assert!(state.redo_edit());
        assert_eq!(state.current_image.as_ref().unwrap().dimensions(), (4, 6));
    }

    #[test]
    fn test_replacement_undo_redo_restores_pasted_pixels() {
        let mut state = state_with_image(colored_image(2, 2, 10));
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
        let mut state = state_with_image(colored_image(2, 3, 10));
        assert!(state.apply_operation(&EditOp::Rotate90Cw));
        assert!(state.undo_edit());
        assert!(state.can_redo());
        assert!(state.apply_operation(&EditOp::FlipHorizontal));
        assert!(!state.can_redo());
        assert_eq!(state.history.len(), 1);
    }

    #[test]
    fn test_adjustment_undo_redo_restores_pixels() {
        let mut state = state_with_image(colored_image(1, 1, 40));
        assert!(state.apply_operation(&EditOp::Adjust(ColorAdjustments {
            brightness: 20.0,
            ..Default::default()
        })));
        let adjusted = state
            .current_image
            .as_ref()
            .unwrap()
            .to_rgba8()
            .get_pixel(0, 0)[0];
        assert_ne!(adjusted, 40);
        assert!(state.undo_edit());
        assert_eq!(
            state
                .current_image
                .as_ref()
                .unwrap()
                .to_rgba8()
                .get_pixel(0, 0)[0],
            40
        );
        assert!(state.redo_edit());
        assert_eq!(
            state
                .current_image
                .as_ref()
                .unwrap()
                .to_rgba8()
                .get_pixel(0, 0)[0],
            adjusted
        );
    }

    #[test]
    fn test_all_unsaved_edits_remain_undoable() {
        let mut state = state_with_image(colored_image(1, 1, 0));
        for value in 1..=60 {
            assert!(state.replace_image(colored_image(1, 1, value)));
        }
        assert_eq!(state.history.len(), 60);
        for _ in 0..60 {
            assert!(state.undo_edit());
        }
        assert_eq!(
            state
                .current_image
                .as_ref()
                .unwrap()
                .to_rgba8()
                .get_pixel(0, 0)[0],
            0
        );
        assert!(!state.can_undo());
        assert!(state.can_redo());
    }

    #[test]
    fn test_revert_all_preserves_redo_history() {
        let mut state = state_with_image(colored_image(1, 1, 10));
        assert!(state.replace_image(colored_image(1, 1, 20)));
        assert!(state.replace_image(colored_image(1, 1, 30)));
        assert!(state.revert_all());
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
        assert!(state.redo_edit());
        assert_eq!(
            state
                .current_image
                .as_ref()
                .unwrap()
                .to_rgba8()
                .get_pixel(0, 0)[0],
            30
        );
    }

    #[test]
    fn test_pending_adjustment_is_committed_as_one_undoable_edit() {
        let mut state = state_with_image(colored_image(1, 1, 40));
        state.adjustments.brightness = 20.0;
        assert!(state.has_unsaved_changes());
        assert!(state.commit_adjustments());
        assert!(state.adjustments.is_neutral());
        assert_eq!(state.history_index, 1);
        assert!(state.undo_edit());
        assert_eq!(
            state
                .current_image
                .as_ref()
                .unwrap()
                .to_rgba8()
                .get_pixel(0, 0)[0],
            40
        );
    }

    #[test]
    fn test_discard_and_save_update_the_session_baseline() {
        let mut state = state_with_image(colored_image(1, 1, 10));
        assert!(state.replace_image(colored_image(1, 1, 20)));
        state.discard_changes();
        assert!(!state.has_unsaved_changes());
        assert_eq!(
            state
                .current_image
                .as_ref()
                .unwrap()
                .to_rgba8()
                .get_pixel(0, 0)[0],
            10
        );

        assert!(state.replace_image(colored_image(1, 1, 30)));
        state.mark_saved();
        assert!(!state.has_unsaved_changes());
        assert!(state.replace_image(colored_image(1, 1, 40)));
        assert!(state.undo_edit());
        assert_eq!(
            state
                .current_image
                .as_ref()
                .unwrap()
                .to_rgba8()
                .get_pixel(0, 0)[0],
            30
        );
    }
}
