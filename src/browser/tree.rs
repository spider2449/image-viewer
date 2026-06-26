use crate::app::App;
use eframe::egui::{Color32, CursorIcon, Ui};
use std::path::PathBuf;

#[derive(Clone)]
pub struct TreeNode {
    pub path: PathBuf,
    pub name: String,
    pub children: Vec<TreeNode>,
    pub has_subdirs: bool,
}

pub fn build_tree(roots: Vec<PathBuf>) -> Vec<TreeNode> {
    roots
        .into_iter()
        .filter_map(|p| build_node(&p, 2))
        .collect()
}

fn build_node(path: &PathBuf, max_depth: usize) -> Option<TreeNode> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string());

    if max_depth == 0 {
        return Some(TreeNode {
            path: path.clone(),
            name,
            children: Vec::new(),
            has_subdirs: has_directories(path),
        });
    }

    let children = if let Ok(entries) = std::fs::read_dir(path) {
        let mut dirs: Vec<PathBuf> = entries
            .flatten()
            .filter(|e| e.path().is_dir())
            .map(|e| e.path())
            .collect();
        dirs.sort();
        dirs.iter()
            .take(50)
            .filter_map(|d| build_node(d, max_depth - 1))
            .collect()
    } else {
        Vec::new()
    };

    let has_subdirs = !children.is_empty();
    Some(TreeNode {
        path: path.clone(),
        name,
        children,
        has_subdirs,
    })
}

fn has_directories(path: &PathBuf) -> bool {
    std::fs::read_dir(path)
        .ok()
        .map(|entries| entries.flatten().any(|e| e.path().is_dir()))
        .unwrap_or(false)
}

pub fn show_tree(app: &mut App, ui: &mut Ui) {
    let mut click_folder: Option<PathBuf> = None;

    for node in &app.browser_state.tree_nodes.clone() {
        show_node(app, ui, node, 0, &mut click_folder);
    }

    if let Some(folder) = click_folder {
        app.current_folder = Some(folder);
        app.scan_folder();
    }
}

fn show_node(
    app: &mut App,
    ui: &mut Ui,
    node: &TreeNode,
    depth: usize,
    click_folder: &mut Option<PathBuf>,
) {
    let is_selected = app
        .current_folder
        .as_ref()
        .is_some_and(|f| f == &node.path);

    let indent = depth as f32 * 16.0;
    let has_children = node.has_subdirs || !node.children.is_empty();

    let expanded = app.browser_state.expanded_paths.contains(&node.path);

    ui.horizontal(|ui| {
        ui.add_space(indent);

        if has_children {
            let expand_label = if expanded { "\u{25BC} " } else { "\u{25B6} " };
            if ui.selectable_label(false, expand_label).clicked() {
                if expanded {
                    app.browser_state.expanded_paths.retain(|p| p != &node.path);
                } else {
                    app.browser_state.expanded_paths.push(node.path.clone());
                    if node.children.is_empty() {
                        if let Some(idx) = app
                            .browser_state
                            .tree_nodes
                            .iter()
                            .position(|n| n.path == node.path)
                        {
                            if let Some(new_node) = build_node(&node.path, 1) {
                                app.browser_state.tree_nodes[idx] = new_node;
                            }
                        }
                    }
                }
            }
        } else {
            ui.add_space(16.0);
        }

        let label = if is_selected {
            ui.colored_label(Color32::from_rgb(0, 120, 215), &node.name)
                .on_hover_cursor(CursorIcon::PointingHand)
        } else {
            ui.label(&node.name)
                .on_hover_cursor(CursorIcon::PointingHand)
        };

        if label.clicked() {
            *click_folder = Some(node.path.clone());
        }
    });

    if expanded {
        for child in &node.children {
            show_node(app, ui, child, depth + 1, click_folder);
        }
    }
}
