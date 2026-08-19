pub mod files;
pub mod grid;
pub mod tree;

use crate::app::App;
use eframe::egui;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::path::PathBuf;

// Decoded thumbnail bitmaps are bounded by an LRU so that browsing a folder
// with thousands of images does not retain every bitmap in memory. Kept a bit
// larger than `thumb_textures` so a texture rebuild rarely needs a re-decode.
// Eviction is safe: a miss is re-requested by `App::update` on the next frame.
const THUMBNAIL_CACHE_CAP: usize = 1024;

pub struct State {
    pub thumbnails: LruCache<PathBuf, Option<egui::ColorImage>>,
    pub thumb_textures: LruCache<PathBuf, egui::TextureHandle>,
    pub selected_thumb: Option<usize>,
    pub tree_nodes: Vec<tree::TreeNode>,
    pub expanded_paths: Vec<PathBuf>,
    pub show_list_view: bool,
    pub tree_width: f32,
    #[allow(dead_code)]
    pub scroll_to_selected: bool,
    pub thumb_decode_size: u32,
    /// Index of the thumbnail currently being renamed inline, if any.
    pub rename_target: Option<usize>,
    /// Edit buffer holding the new file stem while renaming.
    pub rename_buffer: String,
    /// Set when a rename has just started so the text field grabs focus once.
    pub rename_focus: bool,
    pub pending_delete: Option<PathBuf>,
    pub error_message: Option<String>,
}

impl State {
    pub fn new() -> Self {
        Self {
            thumbnails: LruCache::new(NonZeroUsize::new(THUMBNAIL_CACHE_CAP).unwrap()),
            thumb_textures: LruCache::new(NonZeroUsize::new(512).unwrap()),
            selected_thumb: None,
            tree_nodes: Vec::new(),
            expanded_paths: Vec::new(),
            show_list_view: false,
            tree_width: 200.0,
            scroll_to_selected: false,
            thumb_decode_size: 0,
            rename_target: None,
            rename_buffer: String::new(),
            rename_focus: false,
            pending_delete: None,
            error_message: None,
        }
    }

    pub fn request_delete(&mut self, path: PathBuf) {
        self.pending_delete = Some(path);
        self.error_message = None;
    }

    pub fn cancel_delete(&mut self) {
        self.pending_delete = None;
    }

    pub fn record_delete_error(&mut self, error: String) {
        self.error_message = Some(error);
    }
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    let colors = app.theme_colors();
    if app.browser_state.tree_nodes.is_empty() {
        let roots: Vec<PathBuf> = if cfg!(windows) {
            (b'A'..=b'Z')
                .map(|c| PathBuf::from(format!(r"{}:\", c as char)))
                .filter(|p| p.exists())
                .collect()
        } else {
            std::fs::read_dir("/")
                .ok()
                .into_iter()
                .flatten()
                .flatten()
                .map(|e| e.path())
                .take(20)
                .collect()
        };
        app.browser_state.tree_nodes = tree::build_tree(roots);
    }

    egui::CentralPanel::default().show(ctx, |ui| {
        let tree_w = app.browser_state.tree_width.max(120.0);
        let full_rect = ui.max_rect();

        // ── Tree panel (left side) ──────────────────────────
        let tree_rect =
            egui::Rect::from_min_size(full_rect.min, egui::vec2(tree_w, full_rect.height()));
        ui.painter()
            .rect_filled(tree_rect, egui::CornerRadius::same(0), colors.panel_bg);

        #[allow(deprecated)]
        ui.allocate_ui_at_rect(tree_rect.shrink2(egui::vec2(4.0, 4.0)), |ui| {
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .id_salt("tree_scroll")
                .show(ui, |ui| {
                    tree::show_tree(app, ui);
                });
        });

        // ── Resize handle ───────────────────────────────────
        let handle_x = tree_rect.right();
        let handle_rect = egui::Rect::from_min_size(
            egui::pos2(handle_x - 3.0, tree_rect.top()),
            egui::vec2(6.0, tree_rect.height()),
        );
        let resp = ui.interact(
            handle_rect,
            egui::Id::new("tree_resize"),
            egui::Sense::click_and_drag(),
        );
        if resp.dragged() {
            app.browser_state.tree_width =
                (app.browser_state.tree_width + resp.drag_delta().x).max(120.0);
        }
        if resp.drag_started() || resp.dragged() || resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeColumn);
        }

        ui.painter().vline(
            handle_x,
            tree_rect.top()..=tree_rect.bottom(),
            egui::Stroke::new(1.0, colors.border),
        );

        // ── Grid / list view (right side) ───────────────────
        let grid_rect = egui::Rect::from_min_size(
            egui::pos2(handle_x + 1.0, full_rect.top()),
            egui::vec2(
                (full_rect.width() - tree_w - 1.0).max(0.0),
                full_rect.height(),
            ),
        );
        #[allow(deprecated)]
        ui.allocate_ui_at_rect(grid_rect, |ui| {
            grid::show_grid(app, ui);
        });
    });

    show_delete_confirmation(app, ctx);
}

fn show_delete_confirmation(app: &mut App, ctx: &egui::Context) {
    let Some(path) = app.browser_state.pending_delete.clone() else {
        return;
    };
    let mut confirm = false;
    let mut cancel = false;
    egui::Window::new("Delete folder?")
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label("Move this folder and all of its contents to the recycle bin/trash?");
            ui.monospace(path.display().to_string());
            if let Some(error) = &app.browser_state.error_message {
                ui.colored_label(egui::Color32::RED, error);
            }
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
                if ui.button("Move to Trash").clicked() {
                    confirm = true;
                }
            });
        });

    if cancel {
        app.browser_state.cancel_delete();
    } else if confirm {
        match files::delete_to_trash(&path) {
            Ok(()) => finish_folder_delete(app, &path),
            Err(error) => app.browser_state.record_delete_error(error),
        }
    }
}

fn finish_folder_delete(app: &mut App, deleted_path: &std::path::Path) {
    app.browser_state.pending_delete = None;
    app.browser_state.error_message = None;
    app.browser_state.tree_nodes.clear();
    app.browser_state
        .expanded_paths
        .retain(|path| !path.starts_with(deleted_path));
    app.thumbnail_cache.clear();
    app.browser_state.thumbnails.clear();
    app.browser_state.thumb_textures.clear();
    app.textures.clear();
    app.file_cache = crate::file_cache::FileCache::new();

    if selection_is_deleted(app.current_folder.as_deref(), deleted_path) {
        app.current_folder = None;
        app.config.last_folder = None;
        app.config.save();
        app.image_files.clear();
        app.selected_image_index = 0;
        app.browser_state.selected_thumb = None;
        app.editor_state = crate::editor::State::new();
        app.exif_state = crate::exif::ExifData::new();
    }
}

fn selection_is_deleted(current: Option<&std::path::Path>, deleted: &std::path::Path) -> bool {
    current.is_some_and(|path| path.starts_with(deleted))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delete_request_and_cancel_do_not_touch_folder() {
        let dir = std::env::temp_dir().join("browser_delete_request_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut state = State::new();
        state.request_delete(dir.clone());
        assert!(dir.exists());
        state.cancel_delete();
        assert!(dir.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_deleted_selection_matches_exact_and_nested_paths() {
        let deleted = std::path::Path::new("C:\\photos");
        assert!(selection_is_deleted(Some(deleted), deleted));
        assert!(selection_is_deleted(
            Some(std::path::Path::new("C:\\photos\\trip")),
            deleted
        ));
        assert!(!selection_is_deleted(
            Some(std::path::Path::new("C:\\other")),
            deleted
        ));
    }


    #[test]
    fn test_delete_failure_retains_request_and_sets_visible_error() {
        let mut state = State::new();
        let path = PathBuf::from("C:\\protected");
        state.request_delete(path.clone());
        state.record_delete_error("Delete failed: access denied".to_string());
        assert_eq!(state.pending_delete, Some(path));
        assert_eq!(state.error_message.as_deref(), Some("Delete failed: access denied"));
    }
}
