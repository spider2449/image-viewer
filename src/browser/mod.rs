pub mod files;
pub mod grid;
pub mod tree;

use crate::app::App;
use eframe::egui;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct State {
    pub thumbnails: HashMap<PathBuf, Option<egui::ColorImage>>,
    pub thumb_textures: HashMap<PathBuf, egui::TextureHandle>,
    pub selected_thumb: Option<usize>,
    pub tree_nodes: Vec<tree::TreeNode>,
    pub expanded_paths: Vec<PathBuf>,
    pub show_list_view: bool,
    pub tree_width: f32,
    #[allow(dead_code)]
    pub scroll_to_selected: bool,
    pub thumb_decode_size: u32,
}

impl State {
    pub fn new() -> Self {
        Self {
            thumbnails: HashMap::new(),
            thumb_textures: HashMap::new(),
            selected_thumb: None,
            tree_nodes: Vec::new(),
            expanded_paths: Vec::new(),
            show_list_view: false,
            tree_width: 200.0,
            scroll_to_selected: false,
            thumb_decode_size: 0,
        }
    }
}

pub fn show(app: &mut App, ctx: &egui::Context) {
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
        let tree_rect = egui::Rect::from_min_size(
            full_rect.min,
            egui::vec2(tree_w, full_rect.height()),
        );
        ui.painter().rect_filled(tree_rect, egui::CornerRadius::same(0), crate::theme::PANEL_BG);

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
        let resp = ui.interact(handle_rect, egui::Id::new("tree_resize"), egui::Sense::click_and_drag());
        if resp.dragged() {
            app.browser_state.tree_width = (app.browser_state.tree_width + resp.drag_delta().x).max(120.0);
        }
        if resp.drag_started() || resp.dragged() || resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeColumn);
        }

        ui.painter().vline(handle_x, tree_rect.top()..=tree_rect.bottom(), egui::Stroke::new(1.0, crate::theme::BORDER));

        // ── Grid / list view (right side) ───────────────────
        let grid_rect = egui::Rect::from_min_size(
            egui::pos2(handle_x + 1.0, full_rect.top()),
            egui::vec2((full_rect.width() - tree_w - 1.0).max(0.0), full_rect.height()),
        );
        #[allow(deprecated)]
        ui.allocate_ui_at_rect(grid_rect, |ui| {
            grid::show_grid(app, ui);
        });
    });
}
