# Visual Polish Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refine the entire image-viewer GUI with a consistent visual theme, improved colors, spacing, and per-module styling — no new features.

**Architecture:** New `src/theme.rs` module provides centralized color palette and egui visuals/style, applied in `App::new()`. Each module then uses theme constants for targeted visual refinements: card-based thumbnails, styled toolbars, checkerboard alpha background, collapsible editor sections, table-style EXIF layout, and styled batch modal.

**Tech Stack:** Rust, egui 0.31, `image` crate 0.25.

---
### Task 1: Create theme.rs with color palette and style builder

**Files:**
- Create: `src/theme.rs`
- Modify: `src/main.rs:1-15`

- [ ] **Step 1: Add `mod theme;` to main.rs**

Edit `src/main.rs` to add `mod theme;` in alphabetical order:

```rust
mod app;
mod batch;
mod browser;
mod config;
mod editor;
mod exif;
mod font_loader;
mod image_loader;
mod theme;
mod thumbnail_cache;
mod viewer;
```

- [ ] **Step 2: Create `src/theme.rs` with color constants**

```rust
use eframe::egui::{self, Color32, Rounding, Stroke, Style, Visuals, Margin, Vec2};

// ── Color palette ──────────────────────────────────────────
pub const BG_DARK: Color32 = Color32::from_rgb(0x1a, 0x1a, 0x1a);
pub const PANEL_BG: Color32 = Color32::from_rgb(0x22, 0x22, 0x22);
pub const CARD_BG: Color32 = Color32::from_rgb(0x2a, 0x2a, 0x2a);
pub const HOVER_BG: Color32 = Color32::from_rgb(0x35, 0x35, 0x35);
pub const ACCENT: Color32 = Color32::from_rgb(0x4a, 0x9e, 0xff);
pub const SELECTED_BG: Color32 = Color32::from_rgb(0x2d, 0x5a, 0x8e);
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xe0, 0xe0, 0xe0);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(0x88, 0x88, 0x88);
pub const BORDER: Color32 = Color32::from_rgb(0x3a, 0x3a, 0x3a);
pub const DANGER: Color32 = Color32::from_rgb(0xe7, 0x4c, 0x3c);
pub const SUCCESS: Color32 = Color32::from_rgb(0x2e, 0xcc, 0x71);

// ── Convenient icon wrapper ────────────────────────────────
pub fn styled_icon(codepoint: &str) -> egui::RichText {
    egui::RichText::new(codepoint).size(14.0).color(ACCENT)
}

// ── Build the global Visuals ───────────────────────────────
pub fn theme_visuals() -> Visuals {
    Visuals {
        dark_mode: true,
        override_text_color: Some(TEXT_PRIMARY),
        window_rounding: Rounding::same(6.0),
        window_stroke: Stroke::new(1.0, BORDER),
        panel_fill: PANEL_BG,
        faint_bg_color: BG_DARK,
        extreme_bg_color: BG_DARK,
        code_bg_color: CARD_BG,
        warn_fg_color: DANGER,
        error_fg_color: DANGER,
        hyperlink_color: ACCENT,
        selection: egui::style::Selection {
            bg_fill: SELECTED_BG,
            stroke: Stroke::new(1.0, ACCENT),
        },
        widgets: egui::style::Widgets {
            noninteractive: egui::style::WidgetVisuals {
                bg_fill: CARD_BG,
                weak_bg_fill: PANEL_BG,
                bg_stroke: Stroke::new(1.0, BORDER),
                corner_radius: Rounding::same(4.0),
                fg_stroke: Stroke::new(1.0, TEXT_SECONDARY),
                expansion: 0.0,
            },
            inactive: egui::style::WidgetVisuals {
                bg_fill: PANEL_BG,
                weak_bg_fill: CARD_BG,
                bg_stroke: Stroke::new(1.0, BORDER),
                corner_radius: Rounding::same(4.0),
                fg_stroke: Stroke::new(1.0, TEXT_PRIMARY),
                expansion: 0.0,
            },
            hovered: egui::style::WidgetVisuals {
                bg_fill: HOVER_BG,
                weak_bg_fill: HOVER_BG,
                bg_stroke: Stroke::new(1.0, ACCENT),
                corner_radius: Rounding::same(4.0),
                fg_stroke: Stroke::new(1.5, ACCENT),
                expansion: 1.0,
            },
            active: egui::style::WidgetVisuals {
                bg_fill: SELECTED_BG,
                weak_bg_fill: SELECTED_BG,
                bg_stroke: Stroke::new(1.0, ACCENT),
                corner_radius: Rounding::same(4.0),
                fg_stroke: Stroke::new(2.0, ACCENT),
                expansion: 1.0,
            },
            open: egui::style::WidgetVisuals {
                bg_fill: CARD_BG,
                weak_bg_fill: CARD_BG,
                bg_stroke: Stroke::new(1.0, BORDER),
                corner_radius: Rounding::same(4.0),
                fg_stroke: Stroke::new(1.0, TEXT_PRIMARY),
                expansion: 0.0,
            },
        },
        ..Default::default()
    }
}

// ── Build the global Style ─────────────────────────────────
pub fn theme_style() -> Style {
    Style {
        spacing: egui::style::Spacing {
            item_spacing: Vec2::new(8.0, 8.0),
            button_padding: Vec2::new(8.0, 4.0),
            indent: 16.0,
            scroll_bar_width: 6.0,
            scroll_bar_rounding: Rounding::same(3.0),
            ..Default::default()
        },
        interaction: egui::style::Interaction {
            resize_grab_radius_side: 4.0,
            resize_grab_radius_corner: 4.0,
            ..Default::default()
        },
        ..Default::default()
    }
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check`
Expected: Compilation succeeds.

- [ ] **Step 4: Commit**

```bash
git add src/theme.rs src/main.rs
git commit -m "feat: add theme module with color palette and style builder"
```

---

### Task 2: Apply theme in App init + menu bar styling

**Files:**
- Modify: `src/app.rs:33-42` (init) and `src/app.rs:180-230` (menu bar)

- [ ] **Step 1: Apply theme visuals and style in `App::new()`**

At the end of the font setup block in `App::new()`, add after `cc.egui_ctx.set_fonts(fonts);`:

Edit `src/app.rs` to insert after `cc.egui_ctx.set_fonts(fonts);` (around line 42):

```rust
        cc.egui_ctx.set_fonts(fonts);
        cc.egui_ctx.set_visuals(crate::theme::theme_visuals());
        cc.egui_ctx.set_style(crate::theme::theme_style());
```

- [ ] **Step 2: Style the menu bar with accent bottom border**

Replace the menu bar block in `update()` (lines 180-230) with this styled version:

```rust
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
                });
                // Accent bottom border
                ui.separator();
            });
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check`
Expected: Compilation succeeds.

- [ ] **Step 4: Commit**

```bash
git add src/app.rs
git commit -m "feat: apply theme and style menu bar"
```

---

### Task 3: Browser mode polish — side panel + folder tree + toolbar

**Files:**
- Modify: `src/browser/mod.rs:59-67`
- Modify: `src/browser/tree.rs:66-142`
- Modify: `src/browser/grid.rs:11-63`

- [ ] **Step 1: Style the left side panel in `browser/mod.rs`**

Replace the `egui::SidePanel::left("folder_tree")` block:

```rust
    egui::SidePanel::left("folder_tree")
        .resizable(true)
        .frame(egui::Frame {
            fill: crate::theme::PANEL_BG,
            inner_margin: egui::Margin::symmetric(4, 4),
            ..Default::default()
        })
        .default_width(200.0)
        .min_width(120.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                tree::show_tree(app, ui);
            });
        });
```

- [ ] **Step 2: Refine folder tree visuals in `browser/tree.rs`**

Replace `show_tree()` and `show_node()`:

```rust
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
        corner_radius: egui::Rounding::same(4),
        inner_margin: egui::Margin::symmetric(2, 2),
        ..Default::default()
    }
    .show(ui, |ui| {
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

            ui.label("\u{1F4C1}"); // folder icon
            ui.add_space(4.0);

            let label_color = if is_selected {
                crate::theme::TEXT_PRIMARY
            } else {
                crate::theme::TEXT_SECONDARY
            };
            let label = ui.colored_label(label_color, &node.name)
                .on_hover_cursor(egui::CursorIcon::PointingHand);

            if label.clicked() {
                *click_folder = Some(node.path.clone());
            }
        });
    });
    // On hover outside selection, show hover bg
    if !is_selected && response.response.hovered() {
        ui.painter().rect_filled(
            response.response.rect,
            egui::Rounding::same(4),
            egui::Color32::from_white_alpha(8),
        );
    }

    if expanded {
        for child in &node.children {
            show_node(app, ui, child, depth + 1, click_folder);
        }
    }
}
```

- [ ] **Step 3: Style the browser toolbar in `grid.rs`**

Replace the toolbar block at the top of `show_grid()` (lines 11-63) with this styled version:

```rust
pub fn show_grid(app: &mut App, ui: &mut egui::Ui) {
    // ── Toolbar ────────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.label(crate::theme::styled_icon("\u{25C0}"));
        if ui.button("Back").clicked() {
            if let Some(ref cur) = app.current_folder {
                if let Some(parent) = cur.parent() {
                    app.current_folder = Some(parent.to_path_buf());
                    app.scan_folder();
                }
            }
        }
        if ui.button("Up").clicked() {
            if let Some(ref cur) = app.current_folder {
                if let Some(parent) = cur.parent() {
                    app.current_folder = Some(parent.to_path_buf());
                    app.scan_folder();
                }
            }
        }
        ui.separator();
        if ui
            .selectable_label(app.browser_state.show_list_view, "\u{2630} List")
            .clicked()
        {
            app.browser_state.show_list_view = !app.browser_state.show_list_view;
        }
        let mut sort_changed = false;
        ui.separator();
        egui::ComboBox::new("sort_by", "")
            .selected_text(match app.config.sort_by.as_str() {
                "date" => "Date",
                "size" => "Size",
                _ => "Name",
            })
            .show_ui(ui, |ui| {
                sort_changed |= ui.selectable_value(&mut app.config.sort_by, "name".to_string(), "Name").changed();
                sort_changed |= ui.selectable_value(&mut app.config.sort_by, "date".to_string(), "Date").changed();
                sort_changed |= ui.selectable_value(&mut app.config.sort_by, "size".to_string(), "Size").changed();
            });
        let dir_label = if app.config.sort_descending { "\u{25BC}" } else { "\u{25B2}" };
        if ui.selectable_label(false, dir_label).clicked() {
            app.config.sort_descending = !app.config.sort_descending;
            sort_changed = true;
        }
        if sort_changed {
            app.scan_folder();
        }
        ui.separator();
        if ui.button("\u{21BB} Refresh").clicked() {
            app.scan_folder();
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.colored_label(crate::theme::TEXT_SECONDARY, format!("{} files", app.image_files.len()));
        });
    });

    ui.separator();
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check`
Expected: Compilation succeeds.

- [ ] **Step 5: Commit**

```bash
git add src/browser/mod.rs src/browser/tree.rs src/browser/grid.rs
git commit -m "feat: polish browser mode — side panel, folder tree, toolbar"
```

---

### Task 4: Browser polish — thumbnail grid + list view card styling

**Files:**
- Modify: `src/browser/grid.rs:73-353` (folder label, grid, list view)

- [ ] **Step 1: Style the folder name label + empty state**

Replace the folder name and empty-state block (lines 67-84):

```rust
    let folder_name = app
        .current_folder
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(&folder_name)
            .size(18.0)
            .color(crate::theme::TEXT_PRIMARY)
            .strong(),
    );
    ui.add_space(8.0);

    if app.image_files.is_empty() {
        ui.allocate_space(ui.available_size());
        ui.centered_and_justified(|ui| {
            ui.colored_label(crate::theme::TEXT_SECONDARY, "No images found in this folder.");
        });
        return;
    }
```

- [ ] **Step 2: Replace `show_thumbnail_grid` with card-style thumbnails**

Replace the entire `show_thumbnail_grid` function:

```rust
fn show_thumbnail_grid(app: &mut App, ui: &mut egui::Ui, cols: usize) {
    let paths: Vec<PathBuf> = app.image_files.clone();
    let ctx = ui.ctx().clone();

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let cell_size = egui::Vec2::new(THUMB_SIZE, THUMB_SIZE + LABEL_HEIGHT);
            egui::Grid::new("thumb_grid")
                .spacing([THUMB_PADDING, THUMB_PADDING])
                .min_col_width(THUMB_SIZE)
                .show(ui, |ui| {
                    for (i, path) in paths.iter().enumerate() {
                        if i > 0 && i % cols == 0 {
                            ui.end_row();
                        }

                        let is_selected = app.browser_state.selected_thumb == Some(i);
                        let is_hovered = false; // computed below

                        let (rect, response) = ui.allocate_exact_size(cell_size, egui::Sense::click().hover());
                        let hovered = response.hovered();

                        // Shadow (subtle dark rect offset)
                        if is_selected || hovered {
                            let shadow_offset = egui::Vec2::new(2.0, 2.0);
                            ui.painter().rect_filled(
                                egui::Rect::from_min_size(rect.min + shadow_offset, cell_size),
                                egui::Rounding::same(4),
                                egui::Color32::from_black_alpha(60),
                            );
                        }

                        // Selection glow
                        if is_selected {
                            let glow_rect = rect.expand(3.0);
                            ui.painter().rect_filled(
                                glow_rect,
                                egui::Rounding::same(6),
                                egui::Color32::from_rgba_premultiplied(
                                    0x4a, 0x9e, 0xff, 30,
                                ),
                            );
                        }

                        // Card background
                        let card_bg = if is_selected {
                            crate::theme::SELECTED_BG
                        } else {
                            crate::theme::CARD_BG
                        };
                        let border_color = if is_selected {
                            crate::theme::ACCENT
                        } else if hovered {
                            crate::theme::ACCENT
                        } else {
                            crate::theme::BORDER
                        };
                        let border_width: f32 = if is_selected { 2.0 } else { 1.0 };

                        ui.painter().rect(
                            rect,
                            egui::Rounding::same(4),
                            card_bg,
                            egui::Stroke::new(border_width, border_color),
                        );

                        // Thumbnail image area
                        let thumb_rect = egui::Rect::from_min_size(
                            rect.min,
                            egui::Vec2::new(THUMB_SIZE, THUMB_SIZE),
                        );

                        if let Some(Some(ci)) = app.browser_state.thumbnails.get(path) {
                            let tex = if let Some(t) = app.browser_state.thumb_textures.get(path) {
                                t.clone()
                            } else {
                                let key = format!("thumb_{}", path.to_string_lossy());
                                let t = ctx.load_texture(&key, ci.clone(), egui::TextureOptions::LINEAR);
                                app.browser_state.thumb_textures.insert(path.clone(), t.clone());
                                t
                            };
                            let tex_size = tex.size_vec2();
                            let scale =
                                (THUMB_SIZE / tex_size.x).min(THUMB_SIZE / tex_size.y).min(1.0);
                            let draw_size = tex_size * scale;
                            let offset = egui::Vec2::new(
                                (THUMB_SIZE - draw_size.x) / 2.0,
                                (THUMB_SIZE - draw_size.y) / 2.0,
                            );
                            let image_rect = egui::Rect::from_min_size(
                                thumb_rect.min + offset,
                                draw_size,
                            );
                            ui.painter().image(
                                tex.id(),
                                image_rect,
                                egui::Rect::from_min_max(
                                    egui::pos2(0.0, 0.0),
                                    egui::pos2(1.0, 1.0),
                                ),
                                egui::Color32::WHITE,
                            );
                        } else if app.browser_state.thumbnails.contains_key(path) {
                            ui.painter().text(
                                thumb_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "\u{2716}",
                                egui::FontId::proportional(20.0),
                                crate::theme::DANGER,
                            );
                        } else {
                            ui.painter().text(
                                thumb_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "...",
                                egui::FontId::proportional(20.0),
                                crate::theme::ACCENT,
                            );
                        }

                        // Filename label
                        let name = path
                            .file_stem()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        let label_rect = egui::Rect::from_min_size(
                            rect.min + egui::Vec2::new(4.0, THUMB_SIZE),
                            egui::Vec2::new(THUMB_SIZE - 8.0, LABEL_HEIGHT),
                        );
                        let display_name = if name.len() > 18 {
                            format!("{}…", &name[..17])
                        } else {
                            name
                        };
                        ui.painter().text(
                            label_rect.left_center(),
                            egui::Align2::LEFT_CENTER,
                            &display_name,
                            egui::FontId::proportional(11.0),
                            crate::theme::TEXT_SECONDARY,
                        );

                        // Context menu
                        response.context_menu(|ui| {
                            if ui.button("Open").clicked() {
                                app.switch_to_viewer(i);
                                ui.close_menu();
                            }
                            if ui.button("Delete").clicked() {
                                let _ = crate::browser::files::execute(crate::browser::files::FileOp::Delete { path: path.clone() });
                                app.scan_folder();
                                ui.close_menu();
                            }
                            if ui.button("Copy").clicked() {
                                let _ = crate::browser::files::execute(crate::browser::files::FileOp::Copy { path: path.clone() });
                                app.scan_folder();
                                ui.close_menu();
                            }
                            if ui.button("Open in system viewer").clicked() {
                                let _ = crate::browser::files::execute(crate::browser::files::FileOp::OpenExternal { path: path.clone() });
                                ui.close_menu();
                            }
                            ui.menu_button("Save as", |ui| {
                                let mut save = |fmt: &str, img_fmt: image::ImageFormat| {
                                    if let Ok(img) = image::open(path) {
                                        let new_name = path.with_extension(fmt);
                                        if fmt == "jpeg" {
                                            let mut output = std::fs::File::create(&new_name).ok();
                                            if let Some(ref mut f) = output {
                                                let (w, h) = img.dimensions();
                                                let rgba = img.to_rgba8();
                                                let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(f, app.editor_state.save_jpeg_quality);
                                                enc.encode(&rgba, w, h, image::ExtendedColorType::Rgba8).ok();
                                            }
                                        } else {
                                            img.save_with_format(&new_name, img_fmt).ok();
                                        }
                                        app.scan_folder();
                                    }
                                };
                                if ui.button("PNG").clicked() { save("png", image::ImageFormat::Png); ui.close_menu(); }
                                if ui.button("JPEG").clicked() { save("jpeg", image::ImageFormat::Jpeg); ui.close_menu(); }
                                if ui.button("BMP").clicked() { save("bmp", image::ImageFormat::Bmp); ui.close_menu(); }
                                if ui.button("WEBP").clicked() { save("webp", image::ImageFormat::WebP); ui.close_menu(); }
                            });
                        });

                        // Selection + double-click
                        if response.double_clicked() {
                            app.switch_to_viewer(i);
                            return;
                        }
                        if response.clicked() {
                            app.browser_state.selected_thumb = Some(i);
                        }
                    }
                });
        });
}
```

- [ ] **Step 3: Replace `show_list_view` with styled list view**

Replace the entire `show_list_view` function:

```rust
fn show_list_view(app: &mut App, ui: &mut egui::Ui) {
    let paths: Vec<PathBuf> = app.image_files.clone();

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // Column headers
            ui.horizontal(|ui| {
                ui.allocate_space(egui::Vec2::new(24.0, 0.0)); // icon column
                ui.strong("Name");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.strong("Date");
                    ui.add_space(16.0);
                    ui.strong("Size");
                    ui.add_space(16.0);
                    ui.strong("Dimensions");
                    ui.add_space(16.0);
                });
            });
            ui.separator();

            for (i, path) in paths.iter().enumerate() {
                let is_selected = app.browser_state.selected_thumb == Some(i);
                let row_bg = if is_selected {
                    crate::theme::SELECTED_BG
                } else if i % 2 == 0 {
                    crate::theme::PANEL_BG
                } else {
                    crate::theme::CARD_BG
                };

                let id = ui.next_auto_id();
                let (rect, response) = ui.allocate_exact_size(
                    egui::Vec2::new(ui.available_width(), 24.0),
                    egui::Sense::click(),
                );

                // Row background
                let actual_bg = if response.hovered() && !is_selected {
                    crate::theme::HOVER_BG
                } else {
                    row_bg
                };
                ui.painter().rect_filled(rect, egui::Rounding::same(2), actual_bg);

                // Content inside row
                let inner = egui::Rect::from_min_size(rect.min, egui::Vec2::new(rect.width(), 24.0));
                let mut child_ui = ui.child_ui(inner, *ui.layout());
                child_ui.horizontal(|ui| {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("\u{1F5BC}").size(12.0).color(crate::theme::TEXT_SECONDARY));
                    ui.add_space(4.0);
                    let name = path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    ui.colored_label(
                        if is_selected { crate::theme::TEXT_PRIMARY } else { crate::theme::TEXT_SECONDARY },
                        &name,
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let meta = std::fs::metadata(path).ok();
                        if let Some(ref m) = meta {
                            ui.colored_label(crate::theme::TEXT_SECONDARY, format_size(m.len()));
                            if let Ok(modified) = m.modified() {
                                if let Ok(dt) = modified.duration_since(std::time::UNIX_EPOCH) {
                                    let secs = dt.as_secs();
                                    let days = secs / 86400;
                                    let time = secs % 86400;
                                    let h = time / 3600;
                                    let min = (time % 3600) / 60;
                                    ui.colored_label(crate::theme::TEXT_SECONDARY, format!("{days}d {h:02}:{min:02}"));
                                } else {
                                    ui.colored_label(crate::theme::TEXT_SECONDARY, "-");
                                }
                            } else {
                                ui.colored_label(crate::theme::TEXT_SECONDARY, "-");
                            }
                            ui.colored_label(crate::theme::TEXT_SECONDARY, "-");
                        } else {
                            ui.colored_label(crate::theme::TEXT_SECONDARY, "-");
                            ui.colored_label(crate::theme::TEXT_SECONDARY, "-");
                        }
                    });
                });

                if response.double_clicked() {
                    app.switch_to_viewer(i);
                    return;
                }
                if response.clicked() {
                    app.browser_state.selected_thumb = Some(i);
                }
            }
        });
}
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check`
Expected: Compilation succeeds.

- [ ] **Step 5: Commit**

```bash
git add src/browser/grid.rs
git commit -m "feat: polish thumbnail grid and list view styling"
```

---

### Task 5: Viewer mode visual refinements

**Files:**
- Modify: `src/viewer.rs:37-293` and `src/viewer.rs:296-378`

- [ ] **Step 1: Style viewer toolbar (lines 48-118)**

Replace the viewer toolbar block with styled version:

```rust
    if !is_fullscreen {
        egui::TopBottomPanel::top("viewer_toolbar")
            .frame(egui::Frame {
                fill: crate::theme::PANEL_BG,
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Navigation group
                    if ui.button(egui::RichText::new("\u{2190} Browser").color(crate::theme::ACCENT)).clicked() {
                        app.mode = Mode::Browser;
                        app.viewer_state.image_loaded = false;
                    }
                    ui.separator();
                    if ui.button("\u{25C0} Prev").clicked() {
                        app.prev_image();
                    }
                    if ui.button("\u{25B6} Next").clicked() {
                        app.next_image();
                    }
                    // Zoom group
                    ui.separator();
                    if ui.button("Fit").clicked() {
                        app.viewer_state.zoom = app.viewer_state.fit_zoom;
                        app.viewer_state.pan_offset = Vec2::ZERO;
                    }
                    if ui.button("1:1").clicked() {
                        app.viewer_state.zoom = 1.0;
                        app.viewer_state.pan_offset = Vec2::ZERO;
                    }
                    ui.colored_label(crate::theme::TEXT_SECONDARY, "Zoom:");
                    let mut zoom_pct = (app.viewer_state.zoom * 100.0) as i32;
                    if ui
                        .add(egui::Slider::new(&mut zoom_pct, 10..=3200).text("%"))
                        .changed()
                    {
                        app.viewer_state.zoom = zoom_pct as f32 / 100.0;
                    }
                    // Display group
                    ui.separator();
                    if ui
                        .selectable_label(app.viewer_state.show_info, "Info")
                        .clicked()
                    {
                        app.viewer_state.show_info = !app.viewer_state.show_info;
                    }
                    if ui.selectable_label(app.exif_state.visible, "Exif").clicked() {
                        app.exif_state.visible = !app.exif_state.visible;
                    }
                    if ui.selectable_label(app.editor_state.visible, "Edit").clicked() {
                        app.editor_state.visible = !app.editor_state.visible;
                    }
                    // Fullscreen
                    if ui.button("\u{26F6} FS").clicked() {
                        app.viewer_state.is_fullscreen = true;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
                    }
                    ui.separator();
                    let slideshow_label = if app.viewer_state.is_slideshow {
                        "\u{23F8} Stop"
                    } else {
                        "\u{25B6} Slide"
                    };
                    if ui.button(slideshow_label).clicked() {
                        app.viewer_state.is_slideshow = !app.viewer_state.is_slideshow;
                        app.viewer_state.slideshow_timer = 0.0;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.colored_label(crate::theme::TEXT_SECONDARY, format!(
                            "{}/{}",
                            app.selected_image_index + 1,
                            app.image_files.len()
                        ));
                        if let Some(name) = path.file_name() {
                            ui.colored_label(crate::theme::TEXT_SECONDARY, name.to_string_lossy().to_string());
                        }
                    });
                });
            });
    }
```

- [ ] **Step 2: Style the status bar (lines 172-191)**

Replace the status bar block:

```rust
    egui::TopBottomPanel::bottom("viewer_status")
        .frame(egui::Frame {
            fill: crate::theme::PANEL_BG,
            ..Default::default()
        })
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let z = app.viewer_state.zoom;
                let px = app.viewer_state.pan_offset.x;
                let py = app.viewer_state.pan_offset.y;
                ui.colored_label(crate::theme::TEXT_SECONDARY,
                    format!("Zoom: {:.0}% | Pos: ({px:.0}, {py:.0})", z * 100.0));
                ui.separator();
                if let Ok(meta) = std::fs::metadata(&path) {
                    let sz = meta.len();
                    let size_str = if sz >= 1024 * 1024 {
                        format!("{:.1} MB", sz as f64 / (1024.0 * 1024.0))
                    } else if sz >= 1024 {
                        format!("{:.1} KB", sz as f64 / 1024.0)
                    } else {
                        format!("{sz} B")
                    };
                    ui.colored_label(crate::theme::TEXT_SECONDARY, size_str);
                }
            });
        });
```

- [ ] **Step 3: Add checkerboard alpha background and refined info overlay in `draw_image`**

Replace `draw_image` function (lines 296-378):

```rust
fn draw_image(
    app: &mut App,
    ui: &mut egui::Ui,
    tex: &egui::TextureHandle,
    image_rect: egui::Rect,
    available: Vec2,
    path: &PathBuf,
) {
    let tex_size = tex.size_vec2();
    let zoom = app.viewer_state.zoom;

    let scale = (available.x / tex_size.x).min(available.y / tex_size.y);
    app.viewer_state.fit_zoom = scale;
    let base_size = tex_size * scale;
    let display_size = base_size * zoom;

    let offset = Vec2::new(
        (available.x - display_size.x).max(0.0) / 2.0,
        (available.y - display_size.y).max(0.0) / 2.0,
    );

    let draw_rect = egui::Rect::from_min_size(
        egui::pos2(
            image_rect.min.x + offset.x + app.viewer_state.pan_offset.x,
            image_rect.min.y + offset.y + app.viewer_state.pan_offset.y,
        ),
        display_size,
    );

    // Checkerboard alpha background (draw behind image)
    let checker_size = 8.0;
    let check_colors = [
        egui::Color32::from_rgb(0x33, 0x33, 0x33),
        egui::Color32::from_rgb(0x44, 0x44, 0x44),
    ];
    {
        let mut x = draw_rect.min.x;
        let mut row = 0i32;
        while x < draw_rect.max.x {
            let mut y = draw_rect.min.y;
            let mut col = 0i32;
            while y < draw_rect.max.y {
                let idx = ((row & 1) ^ (col & 1)) as usize;
                let cell = egui::Rect::from_min_size(
                    egui::pos2(x, y),
                    egui::Vec2::new(checker_size, checker_size),
                );
                ui.painter().rect_filled(cell, egui::Rounding::ZERO, check_colors[idx]);
                y += checker_size;
                col += 1;
            }
            x += checker_size;
            row += 1;
        }
    }

    // Inner border around image area
    let border_rect = egui::Rect::from_min_size(image_rect.min, available);
    ui.painter().rect_stroke(
        border_rect,
        egui::Rounding::ZERO,
        egui::Stroke::new(1.0, crate::theme::BORDER),
    );

    // Draw image
    ui.painter().image(
        tex.id(),
        draw_rect,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );

    let (mouse_pos, scroll_delta) = ctx_input(ui.ctx());

    if let Some(pos) = mouse_pos {
        if draw_rect.contains(pos) {
            if scroll_delta.y != 0.0 {
                let old_zoom = app.viewer_state.zoom;
                app.viewer_state.zoom = (app.viewer_state.zoom
                    * (1.0 + scroll_delta.y * 0.001))
                .clamp(0.1, 32.0);
                let ratio = app.viewer_state.zoom / old_zoom;
                let mouse_rel = pos - draw_rect.min;
                app.viewer_state.pan_offset =
                    mouse_rel - (mouse_rel - app.viewer_state.pan_offset) * ratio;
            }
        }
    }

    let drag = ui.interact(
        egui::Rect::from_min_size(image_rect.min, available),
        ui.next_auto_id(),
        egui::Sense::drag(),
    );
    if drag.dragged() {
        app.viewer_state.pan_offset += drag.drag_delta();
    }

    // Styled info overlay
    if app.viewer_state.show_info {
        let info_text = format!(
            "{}x{}\nZoom: {:.0}%\n{}",
            tex_size.x as u32,
            tex_size.y as u32,
            app.viewer_state.zoom * 100.0,
            path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
        );
        let painter = ui.painter();
        let text_pos = egui::pos2(image_rect.min.x + 12.0, image_rect.min.y + 12.0);

        // Semi-transparent background
        let font_id = egui::FontId::monospace(14.0);
        let galley = painter.layout_no_wrap(info_text, font_id, egui::Color32::WHITE);
        let bg_rect = egui::Rect::from_min_size(
            text_pos - egui::Vec2::new(4.0, 4.0),
            galley.size + egui::Vec2::new(8.0, 8.0),
        );
        painter.rect_filled(
            bg_rect,
            egui::Rounding::same(4),
            egui::Color32::from_black_alpha(180),
        );
        painter.galley(text_pos, galley, egui::Color32::WHITE);
    }
}
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check`
Expected: Compilation succeeds.

- [ ] **Step 5: Commit**

```bash
git add src/viewer.rs
git commit -m "feat: polish viewer toolbar, status bar, checkerboard, info overlay"
```

---

### Task 6: Editor panel visual polish

**Files:**
- Modify: `src/editor/mod.rs:59-172`

- [ ] **Step 1: Replace `show()` with styled editor panel**

Replace the entire `show()` function:

```rust
pub fn show(app: &mut App, ctx: &egui::Context) {
    if !app.editor_state.visible {
        return;
    }

    egui::SidePanel::right("editor_panel")
        .resizable(true)
        .frame(egui::Frame {
            fill: crate::theme::PANEL_BG,
            inner_margin: egui::Margin::symmetric(8, 8),
            ..Default::default()
        })
        .default_width(250.0)
        .min_width(200.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Header
                ui.horizontal(|ui| {
                    ui.heading("Edit");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("X").clicked() {
                            app.editor_state.visible = false;
                        }
                    });
                });
                ui.separator();

                // Undo/Redo
                ui.horizontal(|ui| {
                    let can_undo = !app.editor_state.undo_stack.is_empty();
                    if ui.add_enabled(can_undo, egui::Button::new("\u{21A9} Undo")).clicked() {
                        undo(app, ctx);
                    }
                    let can_redo = !app.editor_state.redo_stack.is_empty();
                    if ui.add_enabled(can_redo, egui::Button::new("\u{21AA} Redo")).clicked() {
                        redo(app, ctx);
                    }
                });

                ui.separator();

                // Crop section
                ui.label(egui::RichText::new("Crop").strong().color(crate::theme::ACCENT));
                if ui.selectable_label(app.editor_state.crop_active, "Crop").clicked() {
                    app.editor_state.crop_active = !app.editor_state.crop_active;
                    if !app.editor_state.crop_active {
                        app.editor_state.crop_start = None;
                        app.editor_state.crop_end = None;
                    }
                }
                if app.editor_state.crop_active {
                    if ui.button("Apply Crop").clicked() {
                        apply_crop(app, ctx);
                    }
                    if ui.button("Cancel Crop").clicked() {
                        app.editor_state.crop_active = false;
                        app.editor_state.crop_start = None;
                        app.editor_state.crop_end = None;
                    }
                }

                ui.separator();

                // Transform section
                ui.label(egui::RichText::new("Transform").strong().color(crate::theme::ACCENT));
                if ui.button("Rotate 90\u{00B0} CW").clicked() {
                    apply_op(app, ctx, EditOp::Rotate90Cw);
                }
                if ui.button("Rotate 90\u{00B0} CCW").clicked() {
                    apply_op(app, ctx, EditOp::Rotate90Ccw);
                }
                if ui.button("Rotate 180\u{00B0}").clicked() {
                    apply_op(app, ctx, EditOp::Rotate180);
                }
                ui.horizontal(|ui| {
                    if ui.button("Flip H").clicked() {
                        apply_op(app, ctx, EditOp::FlipHorizontal);
                    }
                    if ui.button("Flip V").clicked() {
                        apply_op(app, ctx, EditOp::FlipVertical);
                    }
                });

                ui.separator();

                // Resize section
                ui.label(egui::RichText::new("Resize").strong().color(crate::theme::ACCENT));
                ui.horizontal(|ui| {
                    ui.colored_label(crate::theme::TEXT_SECONDARY, "W:");
                    let mut w = app.editor_state.resize_width as f32;
                    if ui.add(egui::DragValue::new(&mut w).range(1..=16384)).changed() {
                        app.editor_state.resize_width = w as u32;
                    }
                });
                ui.horizontal(|ui| {
                    ui.colored_label(crate::theme::TEXT_SECONDARY, "H:");
                    let mut h = app.editor_state.resize_height as f32;
                    if ui.add(egui::DragValue::new(&mut h).range(1..=16384)).changed() {
                        app.editor_state.resize_height = h as u32;
                    }
                });
                ui.checkbox(&mut app.editor_state.resize_lock_aspect, "Lock aspect ratio");
                if ui.button("Apply").clicked() {
                    apply_op(app, ctx, EditOp::Resize {
                        width: app.editor_state.resize_width,
                        height: app.editor_state.resize_height,
                    });
                }

                ui.separator();

                // Save As section
                ui.label(egui::RichText::new("Save As").strong().color(crate::theme::ACCENT));
                egui::ComboBox::new("save_format", "")
                    .selected_text(app.editor_state.save_format)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut app.editor_state.save_format, "png", "PNG");
                        ui.selectable_value(&mut app.editor_state.save_format, "jpeg", "JPEG");
                        ui.selectable_value(&mut app.editor_state.save_format, "bmp", "BMP");
                        ui.selectable_value(&mut app.editor_state.save_format, "webp", "WEBP");
                    });
                if app.editor_state.save_format == "jpeg" {
                    ui.add(egui::Slider::new(&mut app.editor_state.save_jpeg_quality, 1..=100).text("Quality"));
                }
                if ui.button("Save As...").clicked() {
                    save_as(app);
                }
            });
        });
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: Compilation succeeds.

- [ ] **Step 3: Commit**

```bash
git add src/editor/mod.rs
git commit -m "feat: polish editor panel with styled sections"
```

---

### Task 7: EXIF panel + batch modal visual polish

**Files:**
- Modify: `src/exif.rs:165-186`
- Modify: `src/batch/mod.rs:209-379`

- [ ] **Step 1: Style EXIF panel with alternating rows**

Replace the `show()` function in `exif.rs`:

```rust
pub fn show(data: &mut ExifData, ctx: &egui::Context) {
    if !data.visible {
        return;
    }

    egui::SidePanel::right("exif_panel")
        .resizable(true)
        .frame(egui::Frame {
            fill: crate::theme::PANEL_BG,
            inner_margin: egui::Margin::symmetric(8, 8),
            ..Default::default()
        })
        .default_width(280.0)
        .min_width(200.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("EXIF Data");
                ui.separator();
                for (i, (label, value)) in data.entries.iter().enumerate() {
                    let row_bg = if i % 2 == 0 {
                        crate::theme::PANEL_BG
                    } else {
                        crate::theme::CARD_BG
                    };
                    let id = ui.next_auto_id();
                    let (rect, _) = ui.allocate_exact_size(
                        egui::Vec2::new(ui.available_width(), 20.0),
                        egui::Sense::hover(),
                    );
                    ui.painter().rect_filled(rect, egui::Rounding::ZERO, row_bg);
                    let mut child_ui = ui.child_ui(rect, *ui.layout());
                    child_ui.horizontal(|ui| {
                        ui.colored_label(crate::theme::ACCENT, format!("{label}:"));
                        ui.colored_label(crate::theme::TEXT_PRIMARY, value);
                    });
                }
            });
        });
}
```

- [ ] **Step 2: Style batch modal with accent buttons and styled tabs**

In `src/batch/mod.rs`, replace the `show()` function. Find the `egui::Window::new("Batch Tool")` block and update mode tabs, file list, and Apply button styling.

Replace the mode tab section inside the window (around lines 222-226):

```rust
            ui.horizontal(|ui| {
                ui.selectable_value(&mut app.batch_state.mode, BatchMode::Convert, "Convert");
                ui.selectable_value(&mut app.batch_state.mode, BatchMode::Rename, "Rename");
                ui.selectable_value(&mut app.batch_state.mode, BatchMode::Resize, "Resize");
            });
```

And replace each Apply button section. For example in the Convert block, replace the Apply button (around line 280):

```rust
                    if ui.add_enabled(!app.batch_state.running, egui::Button::new(
                        egui::RichText::new("Apply").color(crate::theme::ACCENT)
                    )).clicked() {
```

Also replace in the Rename block (around line 313):

```rust
                    if ui.add_enabled(!app.batch_state.running, egui::Button::new(
                        egui::RichText::new("Apply").color(crate::theme::ACCENT)
                    )).clicked() {
```

And in the Resize block (around line 345):

```rust
                    if ui.add_enabled(!app.batch_state.running, egui::Button::new(
                        egui::RichText::new("Apply").color(crate::theme::ACCENT)
                    )).clicked() {
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check`
Expected: Compilation succeeds.

- [ ] **Step 4: Run all tests**

Run: `cargo test`
Expected: All tests pass.

- [ ] **Step 5: Bump version and commit**

Edit `Cargo.toml` to bump patch version (e.g. `0.1.4` → `0.1.5`).

```bash
git add src/exif.rs src/batch/mod.rs Cargo.toml
git commit -m "feat: polish EXIF panel and batch modal"
```

---

### Task 8: Final verification

- [ ] **Step 1: Run cargo check**

Run: `cargo check`
Expected: Clean compile with no warnings.

- [ ] **Step 2: Run cargo test**

Run: `cargo test`
Expected: All tests pass.
