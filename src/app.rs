use crate::batch;
use crate::browser;
use crate::config::Config;
use crate::disk_cache::DiskCache;
use crate::editor;
use crate::exif;
use crate::thumbnail_cache::ThumbnailCache;
use crate::viewer;
use eframe::{egui, Frame};
use std::collections::HashMap;
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
    pub disk_cache: DiskCache,
    pub browser_state: browser::State,
    pub viewer_state: viewer::State,
    pub textures: HashMap<String, egui::TextureHandle>,
    pub editor_state: editor::State,
    pub batch_state: batch::State,
    pub exif_state: exif::ExifData,
    pub show_hotkeys: bool,
    pub show_about: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        if let Some(cjk_data) = crate::font_loader::load_cjk_font() {
            fonts.font_data.insert("cjk".to_owned(), std::sync::Arc::new(cjk_data));
            fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap()
                .insert(0, "cjk".to_owned());
            fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap()
                .insert(0, "cjk".to_owned());
        }
        cc.egui_ctx.set_fonts(fonts);
        cc.egui_ctx.set_visuals(crate::theme::theme_visuals());
        cc.egui_ctx.set_style(crate::theme::theme_style());

        let config = Config::load();
        let cache_dir = Self::cache_dir();
        let disk_cache = DiskCache::new(cache_dir.join("thumbnails"));
        let thumbnail_cache = ThumbnailCache::new(512, 4, Some(disk_cache.clone()));
        let browser_state = browser::State::new();
        let viewer_state = viewer::State::new();

        let mut app = Self {
            mode: Mode::Browser,
            config,
            current_folder: None,
            image_files: Vec::new(),
            selected_image_index: 0,
            thumbnail_cache,
            disk_cache,
            browser_state,
            viewer_state,
            textures: HashMap::new(),
            editor_state: editor::State::new(),
            batch_state: batch::State::new(),
            exif_state: exif::ExifData::new(),
            show_hotkeys: false,
            show_about: false,
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

    fn cache_dir() -> PathBuf {
        let mut p = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
        p.pop();
        p.push("cache");
        p
    }

    pub fn scan_folder(&mut self) {
        // Drop old thumbnail cache (channels close → old worker threads exit on Disconnected)
        // and create a fresh one so new folder gets dedicated threads with no stale requests.
        self.thumbnail_cache = ThumbnailCache::new(512, 4, Some(self.disk_cache.clone()));
        self.textures.clear();
        self.browser_state.thumbnails.clear();
        self.browser_state.thumb_textures.clear();
        self.browser_state.tree_nodes.clear(); // rebuild tree on next frame
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

        let decode_size = ((self.config.thumb_size * 1.5).ceil() as u32).max(200);
        self.browser_state.thumb_decode_size = decode_size;
        for path in &self.image_files {
            self.thumbnail_cache.request(path.clone(), decode_size);
        }
    }

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
}

impl eframe::App for App {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.config.save();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        while let Some(result) = self.thumbnail_cache.poll() {
            self.browser_state.thumbnails.insert(result.path, result.image);
        }

        let decode_size = ((self.config.thumb_size * 1.5).ceil() as u32).max(200);
        for path in &self.image_files {
            if !self.browser_state.thumbnails.contains_key(path) {
                self.thumbnail_cache.request(path.clone(), decode_size);
            }
        }

        egui::TopBottomPanel::top("menu_bar")
            .frame(egui::Frame {
                fill: crate::theme::PANEL_BG,
                ..Default::default()
            })
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.style_mut().visuals.widgets.inactive.bg_fill = crate::theme::PANEL_BG;
                    ui.menu_button("File", |ui| {
                        if ui.button("Refresh").clicked() {
                            self.scan_folder();
                            ui.close_menu();
                        }
                        if ui.button("Clear thumbnail cache").clicked() {
                            self.thumbnail_cache.clear_disk_cache();
                            self.browser_state.thumbnails.clear();
                            self.browser_state.thumb_textures.clear();
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
                    ui.menu_button("Help", |ui| {
                        if ui.button("Hotkeys").clicked() {
                            self.show_hotkeys = !self.show_hotkeys;
                            ui.close_menu();
                        }
                        if ui.button("About").clicked() {
                            self.show_about = !self.show_about;
                            ui.close_menu();
                        }
                    });
                });
                // Accent bottom border
                ui.separator();
            });

        match self.mode {
            Mode::Browser => {
                browser::show(self, ctx);
            }
            Mode::Viewer => {
                viewer::show(self, ctx);
                editor::show(self, ctx);
                exif::show(&mut self.exif_state, ctx);
            }
        }

        batch::show(self, ctx);

        if self.show_hotkeys {
            egui::Window::new("Hotkeys")
                .open(&mut self.show_hotkeys)
                .default_size([300.0, 200.0])
                .show(ctx, |ui| {
                    egui::Grid::new("hotkeys_grid").striped(true).show(ui, |ui| {
                        ui.colored_label(crate::theme::ACCENT, "Key");
                        ui.colored_label(crate::theme::ACCENT, "Action");
                        ui.end_row();

                        ui.label("←/→");
                        ui.label("Prev / Next image");
                        ui.end_row();

                        ui.label("Space / F5");
                        ui.label("Toggle slideshow");
                        ui.end_row();

                        ui.label("Esc");
                        ui.label("Exit fullscreen");
                        ui.end_row();

                        ui.label("F11");
                        ui.label("Toggle fullscreen");
                        ui.end_row();

                        ui.label("I");
                        ui.label("Toggle info overlay");
                        ui.end_row();

                        ui.label("F");
                        ui.label("Fit zoom");
                        ui.end_row();

                        ui.label("1");
                        ui.label("100% zoom");
                        ui.end_row();
                    });
                });
        }

        if self.show_about {
            egui::Window::new("About")
                .open(&mut self.show_about)
                .default_size([300.0, 120.0])
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading(&format!("Image Viewer v{}", env!("CARGO_PKG_VERSION")));
                        ui.colored_label(crate::theme::TEXT_SECONDARY, "MIT License");
                        ui.add_space(8.0);
                        ui.label("Copyright (c) 2025 morefunfun11");
                        ui.label("Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the \"Software\"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:");
                        ui.add_space(4.0);
                        ui.label("The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.");
                        ui.add_space(4.0);
                        ui.label("THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.");
                    });
                });
        }

        ctx.request_repaint();
    }
}
