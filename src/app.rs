use crate::batch;
use crate::browser;
use crate::config::Config;
use crate::disk_cache::DiskCache;
use crate::editor;
use crate::exif;
use crate::file_cache::FileCache;
use crate::thumbnail_cache::ThumbnailCache;
use crate::viewer;
use eframe::{egui, Frame};
use lru::LruCache;
use std::num::NonZeroUsize;
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
    pub file_cache: FileCache,
    pub browser_state: browser::State,
    pub viewer_state: viewer::State,
    pub textures: LruCache<String, egui::TextureHandle>,
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
        let config = Config::load();
        let theme = crate::theme::Theme::from_str(&config.theme);
        crate::theme::apply_theme(&cc.egui_ctx, theme);

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
            file_cache: FileCache::new(),
            browser_state,
            viewer_state,
            textures: LruCache::new(NonZeroUsize::new(8).unwrap()),
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

    pub fn theme_colors(&self) -> crate::theme::ThemeColors {
        crate::theme::palette(crate::theme::Theme::from_str(&self.config.theme))
    }

    fn cache_dir() -> PathBuf {
        let mut p = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
        p.pop();
        p.push("cache");
        p
    }

    pub fn scan_folder(&mut self) {
        self.thumbnail_cache.clear();
        self.textures.clear();
        self.browser_state.thumbnails.clear();
        self.browser_state.thumb_textures.clear();
        self.browser_state.tree_nodes.clear(); // rebuild tree on next frame
        let folder = match &self.current_folder {
            Some(f) => f.clone(),
            None => return,
        };
        let folder_str = folder.to_string_lossy().to_string();
        if self.config.last_folder.as_deref() != Some(folder_str.as_str()) {
            self.config.last_folder = Some(folder_str);
            self.config.save();
        }
        let mut files: Vec<PathBuf> = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&folder) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && crate::format_ext::is_supported_extension(&path) {
                    files.push(path);
                }
            }
        }

        let sort_desc = self.config.sort_descending;
        match self.config.sort_by.as_str() {
            "date" => {
                let mtimes: Vec<Option<std::time::SystemTime>> = files
                    .iter()
                    .map(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok())
                    .collect();
                let mut indices: Vec<usize> = (0..files.len()).collect();
                indices.sort_by(|&a, &b| mtimes[a].cmp(&mtimes[b]));
                let sorted: Vec<PathBuf> = indices.iter().map(|&i| files[i].clone()).collect();
                files = sorted;
            }
            "size" => {
                let sizes: Vec<u64> = files
                    .iter()
                    .map(|p| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0))
                    .collect();
                let mut indices: Vec<usize> = (0..files.len()).collect();
                indices.sort_by(|&a, &b| sizes[a].cmp(&sizes[b]));
                let sorted: Vec<PathBuf> = indices.iter().map(|&i| files[i].clone()).collect();
                files = sorted;
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

    /// Point the viewer at `index`: update the selection and refresh the
    /// editor image and EXIF data so every navigation path stays consistent.
    pub fn select_image(&mut self, index: usize) {
        if index >= self.image_files.len() {
            return;
        }
        self.selected_image_index = index;
        if let Some(p) = self.image_files.get(index) {
            if self.editor_state.visible {
                self.editor_state.load_image(p);
            } else {
                self.editor_state.current_image = None;
            }
            self.exif_state.parse(p);
        }
    }

    pub fn switch_to_viewer(&mut self, index: usize) {
        if index < self.image_files.len() {
            self.select_image(index);
            self.mode = Mode::Viewer;
        }
    }

    pub fn create_new_file(&mut self) {
        let folder = match &self.current_folder {
            Some(f) => f.clone(),
            None => return,
        };
        let mut counter = 1;
        loop {
            let filename = format!("untitled_{}.png", counter);
            let path = folder.join(&filename);
            if !path.exists() {
                let img = image::DynamicImage::new_rgba8(800, 600);
                if let Ok(()) = img.save(&path) {
                    self.image_files.push(path);
                    let index = self.image_files.len() - 1;
                    self.switch_to_viewer(index);
                    if !self.editor_state.visible {
                        self.editor_state.visible = true;
                    }
                    self.editor_state.load_image(&self.image_files[index]);
                }
                return;
            }
            counter += 1;
        }
    }

    pub fn next_image(&mut self) {
        if self.selected_image_index + 1 < self.image_files.len() {
            self.select_image(self.selected_image_index + 1);
        }
    }

    pub fn prev_image(&mut self) {
        if self.selected_image_index > 0 {
            self.select_image(self.selected_image_index - 1);
        }
    }
}

impl eframe::App for App {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.config.save();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        while let Some(result) = self.thumbnail_cache.poll() {
            self.browser_state.thumbnails.put(result.path, result.image);
        }

        let decode_size = ((self.config.thumb_size * 1.5).ceil() as u32).max(200);
        for path in &self.image_files {
            if !self.browser_state.thumbnails.contains(path) {
                self.thumbnail_cache.request(path.clone(), decode_size);
            }
        }

        egui::TopBottomPanel::top("menu_bar")
            .frame(egui::Frame {
                fill: self.theme_colors().panel_bg,
                ..Default::default()
            })
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    let panel_bg = self.theme_colors().panel_bg;
                    ui.style_mut().visuals.widgets.inactive.bg_fill = panel_bg;
                    ui.menu_button("File", |ui| {
                        if ui.button("New File").clicked() {
                            self.create_new_file();
                            ui.close_menu();
                        }
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
                        ui.separator();
                        let next_theme = if self.config.theme == "light" { "Dark" } else { "Light" };
                        if ui.button(format!("Switch to {next_theme} Theme")).clicked() {
                            self.config.theme = next_theme.to_lowercase();
                            self.config.save();
                            crate::theme::apply_theme(
                                ctx,
                                crate::theme::Theme::from_str(&self.config.theme),
                            );
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
                let colors = self.theme_colors();
                viewer::show(self, ctx);
                editor::show(self, ctx);
                exif::show(&mut self.exif_state, ctx, &colors);
            }
        }

        batch::show(self, ctx);

        if self.show_hotkeys {
            let colors = self.theme_colors();
            egui::Window::new("Hotkeys")
                .open(&mut self.show_hotkeys)
                .default_size([300.0, 200.0])
                .show(ctx, |ui| {
                    egui::Grid::new("hotkeys_grid").striped(true).show(ui, |ui| {
                        let accent = colors.accent;
                        ui.colored_label(accent, "Key");
                        ui.colored_label(accent, "Action");
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
            let colors = self.theme_colors();
            egui::Window::new("About")
                .open(&mut self.show_about)
                .default_size([300.0, 120.0])
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading(&format!("Image Viewer v{}", env!("CARGO_PKG_VERSION")));
                        ui.colored_label(colors.text_secondary, "MIT License");
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

        let slideshow_active = matches!(self.mode, Mode::Viewer) && self.viewer_state.is_slideshow;
        let pending_thumbs = !self.image_files.is_empty() && self.thumbnail_cache.has_pending();
        if slideshow_active || pending_thumbs {
            ctx.request_repaint();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_textures_lru_evicts_old_entries() {
        let mut cache: LruCache<String, i32> = LruCache::new(NonZeroUsize::new(2).unwrap());
        cache.put("a".to_string(), 1);
        cache.put("b".to_string(), 2);
        cache.put("c".to_string(), 3);
        assert!(!cache.contains("a"), "a should be evicted");
        assert!(cache.contains("b"));
        assert!(cache.contains("c"));
        assert_eq!(cache.len(), 2);
    }
}
