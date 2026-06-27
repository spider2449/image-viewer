use crate::app::App;
use eframe::egui::Ui;
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

fn expand_node(nodes: &mut [TreeNode], target: &PathBuf) -> bool {
    for node in nodes.iter_mut() {
        if node.path == *target {
            if node.children.is_empty() {
                if let Some(new_node) = build_node(target, 1) {
                    node.children = new_node.children;
                    node.has_subdirs = new_node.has_subdirs;
                }
            }
            return true;
        }
        if expand_node(&mut node.children, target) {
            return true;
        }
    }
    false
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

    let bg = if is_selected {
        crate::theme::SELECTED_BG
    } else {
        crate::theme::PANEL_BG
    };

    let response = egui::Frame {
        fill: bg,
        corner_radius: egui::CornerRadius::same(4),
        inner_margin: egui::Margin::symmetric(2, 2),
        ..Default::default()
    }
    .show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.add_space(indent);

            if has_children {
                let expand_label = if expanded { "\u{25BC}" } else { "\u{25B6}" };
                if ui.selectable_label(false, expand_label).clicked() {
                    if expanded {
                        app.browser_state.expanded_paths.retain(|p| p != &node.path);
                    } else {
                        app.browser_state.expanded_paths.push(node.path.clone());
                        if node.children.is_empty() {
                            expand_node(&mut app.browser_state.tree_nodes, &node.path);
                        }
                    }
                }
            } else {
                ui.add_space(16.0);
            }

            let icon_color = if depth == 0 {
                crate::theme::ACCENT
            } else if depth == 1 {
                egui::Color32::from_rgb(0xf0, 0xc0, 0x40)
            } else {
                egui::Color32::from_rgb(0x80, 0xc0, 0x80)
            };
            ui.label(egui::RichText::new("\u{1F4C1}").color(icon_color));
            ui.add_space(4.0);

            let label_color = if is_selected {
                crate::theme::TEXT_PRIMARY
            } else if depth == 0 {
                crate::theme::ACCENT
            } else if depth == 1 {
                egui::Color32::from_rgb(0xe0, 0xe0, 0xc0)
            } else {
                egui::Color32::from_rgb(0xc0, 0xd0, 0xc0)
            };
            let label = ui.colored_label(label_color, &node.name)
                .on_hover_cursor(egui::CursorIcon::PointingHand);

            if label.clicked() {
                *click_folder = Some(node.path.clone());
            }
        });
    });

    if !is_selected && response.response.hovered() {
        ui.painter().rect_filled(
            response.response.rect,
            egui::CornerRadius::same(4),
            egui::Color32::from_rgba_premultiplied(0x4a, 0x9e, 0xff, 20),
        );
    }

    if expanded {
        for child in &node.children {
            show_node(app, ui, child, depth + 1, click_folder);
        }
    }
}
