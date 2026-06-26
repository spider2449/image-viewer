use crate::browser;
use crate::config::Config;
use crate::editor;
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
    pub browser_state: browser::State,
    pub viewer_state: viewer::State,
    pub textures: HashMap<String, egui::TextureHandle>,
    pub editor_state: editor::State,
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
        let thumbnail_cache = ThumbnailCache::new(512, 4);
        let browser_state = browser::State::new();
        let viewer_state = viewer::State::new();

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

    pub fn scan_folder(&mut self) {
        self.textures.clear();
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

        for path in &self.image_files {
            self.thumbnail_cache.request(path.clone(), 200);
        }
    }

    pub fn switch_to_viewer(&mut self, index: usize) {
        if index < self.image_files.len() {
            self.selected_image_index = index;
            if let Some(p) = self.image_files.get(index) {
                self.editor_state.load_image(p);
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
            }
        }
    }

    pub fn prev_image(&mut self) {
        if self.selected_image_index > 0 {
            self.selected_image_index -= 1;
            self.viewer_state.image_loaded = false;
            if let Some(p) = self.image_files.get(self.selected_image_index) {
                self.editor_state.load_image(p);
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

        ctx.request_repaint();
    }
}
