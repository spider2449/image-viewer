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

/// Strip the Windows verbatim prefix (`\\?\`) produced by `canonicalize`
/// so tree paths (`D:\...`) match the startup folder (`\\?\D:\...`).
pub fn normalize_path(path: &std::path::Path) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(stripped) = s.strip_prefix(r"\\?\") {
        PathBuf::from(stripped)
    } else {
        path.to_path_buf()
    }
}

/// Ancestors of `folder` from filesystem root down to `folder` itself,
/// excluding entries already present in `expanded`. Pure helper so the
/// expansion chain is unit-testable without egui state.
pub fn missing_ancestors(
    folder: &std::path::Path,
    expanded: &[PathBuf],
) -> Vec<PathBuf> {
    let folder = normalize_path(folder);
    let mut chain: Vec<PathBuf> = folder.ancestors().map(|p| p.to_path_buf()).collect();
    chain.reverse();
    chain
        .into_iter()
        .filter(|p| !expanded.iter().any(|e| normalize_path(e) == *p))
        .collect()
}

/// Ensure every ancestor of the current folder is marked expanded and
/// materialized via `expand_node`, so deep folders become visible.
/// Idempotent: re-running with the same folder is a no-op.
pub fn sync_tree_to_current_folder(app: &mut App) {
    let Some(folder) = app.current_folder.clone() else {
        return;
    };
    let folder = normalize_path(&folder);
    if app.current_folder.as_ref() != Some(&folder) {
        app.current_folder = Some(folder.clone());
    }
    for ancestor in missing_ancestors(&folder, &app.browser_state.expanded_paths) {
        app.browser_state.expanded_paths.push(ancestor.clone());
        expand_node(&mut app.browser_state.tree_nodes, &ancestor);
    }
}

pub fn show_tree(app: &mut App, ui: &mut Ui) {
    sync_tree_to_current_folder(app);
    let mut click_folder: Option<PathBuf> = None;

    for node in &app.browser_state.tree_nodes.clone() {
        show_node(app, ui, node, 0, &mut click_folder);
    }

    // One-shot auto-scroll: only jump on folder change / return from viewer,
    // otherwise the user stays in control of the tree scroll position.
    app.browser_state.scroll_to_selected = false;

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
    let colors = app.theme_colors();

    let bg = if is_selected {
        colors.selected_bg
    } else {
        colors.panel_bg
    };

    let frame_resp = egui::Frame {
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

            // Folder icon stays in the theme family: accent for roots,
            // muted secondary tones for deeper levels.
            let icon_color = if depth == 0 {
                colors.accent
            } else if depth == 1 {
                colors.text_secondary
            } else {
                colors.text_secondary.gamma_multiply(0.7)
            };
            ui.label(egui::RichText::new("\u{1F4C1}").color(icon_color));
            ui.add_space(4.0);

            let label_color = if is_selected {
                colors.text_primary
            } else if depth == 0 {
                colors.accent
            } else {
                colors.text_primary
            };
            // Truncate to the remaining row width so long folder names never
            // overflow the side panel; full name is shown on hover.
            let body_font = ui
                .style()
                .text_styles
                .get(&egui::TextStyle::Body)
                .cloned()
                .unwrap_or_else(|| egui::FontId::proportional(14.0));
            let avail_w = ui.available_width().max(10.0);
            let display_name =
                crate::browser::grid::truncate_to_fit(&node.name, avail_w, |s| {
                    ui.painter()
                        .layout_no_wrap(s.to_string(), body_font.clone(), label_color)
                        .size()
                        .x
                });
            let label = ui
                .colored_label(label_color, &display_name)
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            if display_name != node.name {
                label.clone().on_hover_text(&node.name);
            }

            if label.clicked() {
                *click_folder = Some(node.path.clone());
            }

            if depth > 0 {
                label.context_menu(|ui| {
                    if ui.button("Delete folder").clicked() {
                        ui.close_menu();
                        app.browser_state.request_delete(node.path.clone());
                    }
                });
            }
        });
    });

    if !is_selected && frame_resp.response.hovered() {
        ui.painter().rect_filled(
            frame_resp.response.rect,
            egui::CornerRadius::same(4),
            colors.hover_bg,
        );
    }

    if is_selected && app.browser_state.scroll_to_selected {
        ui.scroll_to_rect(frame_resp.response.rect, Some(egui::Align::Center));
    }

    if expanded {
        for child in &node.children {
            show_node(app, ui, child, depth + 1, click_folder);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_missing_ancestors_returns_root_to_leaf_chain() {
        let folder = Path::new("/tmp").join("imgview_sync").join("a").join("b");
        let missing = missing_ancestors(&folder, &[]);
        let expected = vec![
            Path::new("/").to_path_buf(),
            Path::new("/tmp").join("imgview_sync"),
            Path::new("/tmp").join("imgview_sync").join("a"),
            folder.clone(),
        ];
        // Filter expected to ancestors that are actually ancestors of folder
        // (root handling differs per platform, so compare suffix chain).
        assert!(missing.ends_with(&expected[1..]));
        assert_eq!(missing.last(), Some(&folder));
    }

    #[test]
    fn test_missing_ancestors_skips_already_expanded() {
        let folder = Path::new("/tmp").join("imgview_sync2").join("sub");
        let parent = Path::new("/tmp").join("imgview_sync2");
        let missing = missing_ancestors(&folder, &[parent.clone()]);
        assert!(!missing.contains(&parent));
        assert_eq!(missing.last(), Some(&folder));
    }

    #[test]
    fn test_normalize_strips_verbatim_prefix() {
        let verbatim = Path::new(r"\\?\D:\photos\trip");
        assert_eq!(
            normalize_path(verbatim),
            PathBuf::from(r"D:\photos\trip")
        );
    }
}
