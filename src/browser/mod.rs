pub mod files;
pub mod grid;
pub mod tree;

use crate::app::App;
use eframe::egui;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct State {
    pub thumbnails: HashMap<PathBuf, Option<egui::ColorImage>>,
    pub selected_thumb: Option<usize>,
    pub tree_nodes: Vec<tree::TreeNode>,
    pub expanded_paths: Vec<PathBuf>,
    pub show_list_view: bool,
    #[allow(dead_code)]
    pub scroll_to_selected: bool,
}

impl State {
    pub fn new() -> Self {
        Self {
            thumbnails: HashMap::new(),
            selected_thumb: None,
            tree_nodes: Vec::new(),
            expanded_paths: Vec::new(),
            show_list_view: false,
            scroll_to_selected: false,
        }
    }
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    if app.browser_state.tree_nodes.is_empty() {
        let roots: Vec<PathBuf> = if cfg!(windows) {
            vec![
                PathBuf::from("C:\\"),
                PathBuf::from("D:\\"),
                PathBuf::from("E:\\"),
            ]
            .into_iter()
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

    egui::SidePanel::left("folder_tree")
        .resizable(true)
        .default_width(200.0)
        .min_width(120.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                tree::show_tree(app, ui);
            });
        });

    egui::CentralPanel::default().show(ctx, |ui| {
        grid::show_grid(app, ui);
    });
}
