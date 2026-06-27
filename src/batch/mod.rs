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
    #[allow(dead_code)]
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
                            }
                        }
                    }
                });

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
                    if ui.add_enabled(!app.batch_state.running, egui::Button::new(
                        egui::RichText::new("Apply").color(crate::theme::ACCENT)
                    )).clicked() {
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
                    if ui.add_enabled(!app.batch_state.running, egui::Button::new(
                        egui::RichText::new("Apply").color(crate::theme::ACCENT)
                    )).clicked() {
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
                    if ui.add_enabled(!app.batch_state.running, egui::Button::new(
                        egui::RichText::new("Apply").color(crate::theme::ACCENT)
                    )).clicked() {
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
