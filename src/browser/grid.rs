use crate::app::App;
use eframe::egui::{self, Color32, TextureOptions, Vec2, Stroke, CornerRadius};
use std::path::PathBuf;

const THUMB_PADDING: f32 = 8.0;
const LABEL_HEIGHT: f32 = 30.0;

struct GridLayout {
    cols: usize,
    left_pad: f32,
    grid_gap_x: f32,
}

/// Explorer-style grid metrics: full multi-row grids distribute leftover
/// width evenly (centered); a single incomplete row left-aligns with fixed
/// padding instead of spreading items across the whole width.
fn compute_grid_layout(avail: f32, thumb: f32, total: usize, padding: f32) -> GridLayout {
    let cols_by_width = ((avail + padding) / (thumb + padding)).floor().max(1.0) as usize;
    let cols = cols_by_width.min(total.max(1));
    if cols <= 1 {
        let left_pad = ((avail - thumb) / 2.0).max(0.0);
        GridLayout { cols, left_pad, grid_gap_x: padding }
    } else if total < cols_by_width {
        GridLayout { cols, left_pad: 0.0, grid_gap_x: padding }
    } else {
        let grid_gap_x = ((avail - cols as f32 * thumb) / (cols as f32 + 1.0)).max(0.0);
        GridLayout { cols, left_pad: grid_gap_x, grid_gap_x }
    }
}

pub fn show_grid(app: &mut App, ui: &mut egui::Ui) {
    let colors = app.theme_colors();
    let mut size_changed = false;
    // ── Toolbar ────────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.label(crate::theme::styled_icon("\u{25C0}", &colors));
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
        ui.separator();
        ui.label("Size:");
        let mut ts = app.config.thumb_size;
        ui.scope(|ui| {
            ui.spacing_mut().slider_rail_height = 6.0;
            ui.spacing_mut().interact_size.y = 22.0;
            if ui
                .add(egui::Slider::new(&mut ts, 60.0..=400.0).text("px").trailing_fill(true))
                .changed()
            {
                app.config.thumb_size = ts;
                size_changed = true;
            }
        });
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
            ui.colored_label(
                colors.text_secondary,
                format!("{} folders, {} files", app.subfolders.len(), app.image_files.len()),
            );
        });
    });

    ui.separator();

    let folder_path = app
        .current_folder
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(&folder_path)
            .size(18.0)
            .color(colors.text_primary)
            .strong(),
    );
    ui.add_space(8.0);

    if app.image_files.is_empty() && app.subfolders.is_empty() {
        ui.allocate_space(ui.available_size());
        ui.centered_and_justified(|ui| {
            ui.colored_label(colors.text_secondary, "No files in this folder.");
        });
        return;
    }

    let (scroll, mods) = ui.input(|i| (i.raw_scroll_delta, i.modifiers));
    if mods.ctrl && scroll.y != 0.0 {
        let step = if scroll.y > 0.0 { 10.0 } else { -10.0 };
        app.config.thumb_size = (app.config.thumb_size + step).clamp(60.0, 400.0);
        size_changed = true;
    }

    if size_changed {
        let new_decode = ((app.config.thumb_size * 1.5).ceil() as u32).max(200);
        if new_decode > app.browser_state.thumb_decode_size {
            app.browser_state.thumb_decode_size = new_decode;
            app.browser_state.thumbnails.clear();
            app.browser_state.thumb_textures.clear();
            for path in &app.image_files {
                app.thumbnail_cache.request(path.clone(), new_decode);
            }
        }
    }

    if app.browser_state.show_list_view {
        show_list_view(app, ui);
    } else {
        show_thumbnail_grid(app, ui);
    }
}

fn show_thumbnail_grid(app: &mut App, ui: &mut egui::Ui) {
    let paths: Vec<PathBuf> = app.image_files.clone();
    let folders: Vec<PathBuf> = app.subfolders.clone();
    let ctx = ui.ctx().clone();
    let colors = app.theme_colors();

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .id_salt("thumb_grid_scroll")
        .show(ui, |ui| {
            // Recompute every frame so slider / Ctrl+wheel resizing recenters.
            // Explorer-style: multi-row grids auto-spread, single row left-aligns.
            let avail = ui.available_width();
            let thumb = app.config.thumb_size;
            let total = folders.len() + paths.len();
            let layout = compute_grid_layout(avail, thumb, total, THUMB_PADDING);
            let cols = layout.cols;
            let grid_gap_x = layout.grid_gap_x;
            let left_pad = layout.left_pad;

            ui.horizontal_top(|ui| {
                // Remove the default item spacing so left_pad is exact.
                ui.spacing_mut().item_spacing.x = 0.0;
                if left_pad > 0.0 {
                    ui.add_space(left_pad);
                }
                ui.vertical(|ui| {
            let cell_size = Vec2::new(app.config.thumb_size, app.config.thumb_size + LABEL_HEIGHT);
            egui::Grid::new("thumb_grid")
                .spacing([grid_gap_x, THUMB_PADDING])
                .min_col_width(app.config.thumb_size)
                .show(ui, |ui| {
                    let mut pos: usize = 0;
                    // ── Folders on top ───────────────────────
                    for (fj, folder) in folders.iter().enumerate() {
                        if pos > 0 && pos % cols == 0 {
                            ui.end_row();
                        }
                        pos += 1;

                        let is_selected = app.browser_state.selected_folder == Some(fj);
                        let (rect, response) = ui.allocate_exact_size(cell_size, egui::Sense::click());
                        let hovered = response.hovered();

                        if is_selected || hovered {
                            let shadow_offset = Vec2::new(2.0, 2.0);
                            ui.painter().rect_filled(
                                egui::Rect::from_min_size(rect.min + shadow_offset, cell_size),
                                CornerRadius::same(4),
                                Color32::from_black_alpha(60),
                            );
                        }
                        if is_selected {
                            let glow_rect = rect.expand(3.0);
                            ui.painter().rect_filled(
                                glow_rect,
                                CornerRadius::same(6),
                                Color32::from_rgba_premultiplied(0x4a, 0x9e, 0xff, 30),
                            );
                        }
                        let card_bg = if is_selected { colors.selected_bg } else { colors.card_bg };
                        let border_color = if is_selected || hovered { colors.accent } else { colors.border };
                        let border_width: f32 = if is_selected { 2.0 } else { 1.0 };
                        ui.painter().rect(
                            rect,
                            CornerRadius::same(4),
                            card_bg,
                            Stroke::new(border_width, border_color),
                            egui::StrokeKind::Outside,
                        );
                        let thumb_rect = egui::Rect::from_min_size(
                            rect.min,
                            Vec2::new(app.config.thumb_size, app.config.thumb_size),
                        );
                        ui.painter().text(
                            thumb_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            entry_glyph(true, folder),
                            egui::FontId::proportional(40.0),
                            colors.accent,
                        );
                        let name = folder
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        let label_rect = egui::Rect::from_min_size(
                            rect.min + Vec2::new(4.0, app.config.thumb_size),
                            Vec2::new(app.config.thumb_size - 8.0, LABEL_HEIGHT),
                        );
                        ui.painter().text(
                            label_rect.left_center(),
                            egui::Align2::LEFT_CENTER,
                            &truncate_name(&name, 18),
                            egui::FontId::proportional(11.0),
                            colors.text_primary,
                        );
                        if response.double_clicked() {
                            let target = folder.clone();
                            app.current_folder = Some(target);
                            app.scan_folder();
                            return;
                        }
                        if response.clicked() {
                            app.browser_state.selected_folder = Some(fj);
                            app.browser_state.selected_thumb = None;
                        }
                    }
                    for (i, path) in paths.iter().enumerate() {
                        if pos > 0 && pos % cols == 0 {
                            ui.end_row();
                        }
                        pos += 1;

                        let is_selected = app.browser_state.selected_thumb == Some(i);

                        let (rect, response) = ui.allocate_exact_size(cell_size, egui::Sense::click());
                        let hovered = response.hovered();

                        // Shadow (subtle dark rect offset)
                        if is_selected || hovered {
                            let shadow_offset = Vec2::new(2.0, 2.0);
                            ui.painter().rect_filled(
                                egui::Rect::from_min_size(rect.min + shadow_offset, cell_size),
                                CornerRadius::same(4),
                                Color32::from_black_alpha(60),
                            );
                        }

                        // Selection glow
                        if is_selected {
                            let glow_rect = rect.expand(3.0);
                            ui.painter().rect_filled(
                                glow_rect,
                                CornerRadius::same(6),
                                Color32::from_rgba_premultiplied(0x4a, 0x9e, 0xff, 30),
                            );
                        }

                        // Card background
                        let card_bg = if is_selected {
                            colors.selected_bg
                        } else {
                            colors.card_bg
                        };
                        let border_color = if is_selected {
                            colors.accent
                        } else if hovered {
                            colors.accent
                        } else {
                            colors.border
                        };
                        let border_width: f32 = if is_selected { 2.0 } else { 1.0 };

                        ui.painter().rect(
                            rect,
                            CornerRadius::same(4),
                            card_bg,
                            Stroke::new(border_width, border_color),
                            egui::StrokeKind::Outside,
                        );

                        // Thumbnail image area
                        let thumb_rect = egui::Rect::from_min_size(
                            rect.min,
                            Vec2::new(app.config.thumb_size, app.config.thumb_size),
                        );

                        if !can_open_in_viewer(path) {
                            ui.painter().text(
                                thumb_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                entry_glyph(false, path),
                                egui::FontId::proportional(28.0),
                                colors.text_secondary,
                            );
                        } else if let Some(Some(ci)) = app.browser_state.thumbnails.get(path) {
                            let tex = if let Some(t) = app.browser_state.thumb_textures.get(path) {
                                t.clone()
                            } else {
                                let key = format!("thumb_{}", path.to_string_lossy());
                                let t = ctx.load_texture(&key, ci.clone(), TextureOptions::LINEAR);
                                app.browser_state.thumb_textures.put(path.clone(), t.clone());
                                t
                            };
                            let tex_size = tex.size_vec2();
                            let scale =
                                (app.config.thumb_size / tex_size.x).min(app.config.thumb_size / tex_size.y).min(1.0);
                            let draw_size = tex_size * scale;
                            let offset = Vec2::new(
                                (app.config.thumb_size - draw_size.x) / 2.0,
                                (app.config.thumb_size - draw_size.y) / 2.0,
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
                                Color32::WHITE,
                            );
                        } else if app.browser_state.thumbnails.contains(path) {
                            ui.painter().text(
                                thumb_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "\u{2716}",
                                egui::FontId::proportional(20.0),
                                colors.danger,
                            );
                        } else {
                            ui.painter().text(
                                thumb_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                "...",
                                egui::FontId::proportional(20.0),
                                colors.accent,
                            );
                        }

                        // Filename label
                        let name = path
                            .file_stem()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        let label_rect = egui::Rect::from_min_size(
                            rect.min + Vec2::new(4.0, app.config.thumb_size),
                            Vec2::new(app.config.thumb_size - 8.0, LABEL_HEIGHT),
                        );
                        if app.browser_state.rename_target == Some(i) {
                            inline_rename(app, ui, path, label_rect);
                        } else {
                            let display_name = truncate_name(&name, 18);
                            ui.painter().text(
                                label_rect.left_center(),
                                egui::Align2::LEFT_CENTER,
                                &display_name,
                                egui::FontId::proportional(11.0),
                                colors.text_secondary,
                            );
                        }

                        // Context menu
                        response.context_menu(|ui| {
                            if ui.button("Open").clicked() {
                                if can_open_in_viewer(path) {
                                    app.switch_to_viewer(i);
                                }
                                ui.close_menu();
                            }
                            if ui.button("Rename").clicked() {
                                app.browser_state.rename_target = Some(i);
                                app.browser_state.rename_buffer = path
                                    .file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_default();
                                app.browser_state.rename_focus = true;
                                ui.close_menu();
                            }
                            if ui.button("Delete").clicked() {
                                let _ = crate::browser::files::execute(crate::browser::files::FileOp::Delete { path: path.clone() });
                                app.rescan_selecting(None);
                                ui.close_menu();
                            }
                            if ui.button("Copy").clicked() {
                                let _ = crate::browser::files::execute(crate::browser::files::FileOp::Copy { path: path.clone() });
                                app.rescan_selecting(Some(path.clone()));
                                ui.close_menu();
                            }
                            if ui.button("Open in system viewer").clicked() {
                                let _ = crate::browser::files::execute(crate::browser::files::FileOp::OpenExternal { path: path.clone() });
                                ui.close_menu();
                            }
                            ui.menu_button("Save as", |ui| {
                                let mut save = |fmt: &str| {
                                    if let Ok(img) = image::open(path) {
                                        let new_name = path.with_extension(crate::format_ext::format_to_extension(fmt));
                                        if let Err(e) = crate::format_ext::save_image(&img, &new_name, fmt, app.editor_state.save_jpeg_quality) {
                                            eprintln!("Save failed: {e}");
                                        } else {
                                            app.rescan_selecting(Some(path.clone()));
                                        }
                                    }
                                };
                                if ui.button("PNG").clicked() { save("png"); ui.close_menu(); }
                                if ui.button("JPEG").clicked() { save("jpeg"); ui.close_menu(); }
                                if ui.button("BMP").clicked() { save("bmp"); ui.close_menu(); }
                                if ui.button("WEBP").clicked() { save("webp"); ui.close_menu(); }
                            });
                        });

                        // Selection + double-click (suppressed while renaming this cell)
                        if app.browser_state.rename_target != Some(i) {
                            if response.double_clicked() {
                                if can_open_in_viewer(path) {
                                    app.switch_to_viewer(i);
                                }
                                return;
                            }
                            if response.clicked() {
                                app.browser_state.selected_thumb = Some(i);
                                app.browser_state.selected_folder = None;
                            }
                        }
                    }
                });
                });
            });
        });
}

/// Draw the inline rename text field for a thumbnail cell and commit or cancel
/// the edit based on user input. Enter commits, Escape or clicking away cancels.
fn inline_rename(app: &mut App, ui: &mut egui::Ui, path: &PathBuf, rect: egui::Rect) {
    let mut buf = std::mem::take(&mut app.browser_state.rename_buffer);
    let resp = ui.put(
        rect,
        egui::TextEdit::singleline(&mut buf)
            .font(egui::FontId::proportional(11.0))
            .margin(egui::Margin::symmetric(2, 2)),
    );
    app.browser_state.rename_buffer = buf;

    if app.browser_state.rename_focus {
        resp.request_focus();
        app.browser_state.rename_focus = false;
    }

    let enter = resp.lost_focus() && ui.input(|inp| inp.key_pressed(egui::Key::Enter));
    let escape = ui.input(|inp| inp.key_pressed(egui::Key::Escape));
    // lost_focus without Enter (e.g. clicking elsewhere) cancels the edit.
    let cancelled = escape || (resp.lost_focus() && !enter);

    if enter {
        commit_rename(app, path);
        app.browser_state.rename_target = None;
    } else if cancelled {
        app.browser_state.rename_target = None;
    }
}

/// Rename `path` to the full filename (including extension) in the edit buffer.
fn commit_rename(app: &mut App, path: &PathBuf) {
    let new_name = app.browser_state.rename_buffer.trim().to_string();
    if new_name.is_empty() {
        return;
    }
    // No-op if the name is unchanged.
    if path.file_name().map(|n| n.to_string_lossy().to_string()).as_deref() == Some(new_name.as_str()) {
        return;
    }
    let dest = path.with_file_name(&new_name);
    if dest.exists() {
        eprintln!("Rename failed: {} already exists", dest.display());
        return;
    }
    match crate::browser::files::execute(crate::browser::files::FileOp::Rename {
        old: path.clone(),
        new: new_name,
    }) {
        Ok(()) => app.rescan_selecting(Some(dest)),
        Err(e) => eprintln!("{e}"),
    }
}

fn show_list_view(app: &mut App, ui: &mut egui::Ui) {
    let paths: Vec<PathBuf> = app.image_files.clone();
    let folders: Vec<PathBuf> = app.subfolders.clone();
    let saved_widths = app.config.column_widths.clone();
    let colors = app.theme_colors();

    const ICON_W: f32 = 24.0;
    const GAP: f32 = 4.0;
    const MIN_W: f32 = 60.0;
    const HANDLE_W: f32 = 8.0;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .id_salt("list_view_scroll")
        .show(ui, |ui| {
            let available = ui.available_width();

            let widths = col_widths(&app.config.column_widths, available, ICON_W, MIN_W, GAP);

            // ── Column headers ──────────────────────────────
            let header_h = 20.0;
            let (header_rect, _) = ui.allocate_exact_size(
                Vec2::new(available, header_h),
                egui::Sense::hover(),
            );

            ui.painter().rect_filled(header_rect, egui::CornerRadius::same(2), colors.panel_bg);

            let mut x = header_rect.min.x;
            let header_y = header_rect.min.y;

            // Icon header (fixed width spacer)
            x += ICON_W;

            // Name header + drag handle
            ui.painter().text(
                egui::pos2(x + 4.0, header_y + header_h / 2.0),
                egui::Align2::LEFT_CENTER,
                "Name",
                egui::FontId::proportional(14.0),
                colors.text_primary,
            );
            x += widths.name;
            x = drag_handle(ui, egui::Id::new("drag_name"), x, header_y, header_h, HANDLE_W, colors.border, |d| {
                app.config.column_widths.name = (app.config.column_widths.name + d).max(MIN_W);
            });
            x += GAP;

            // Size header + drag handle
            ui.painter().text(
                egui::pos2(x + 4.0, header_y + header_h / 2.0),
                egui::Align2::LEFT_CENTER,
                "Size",
                egui::FontId::proportional(14.0),
                colors.text_primary,
            );
            x += widths.size;
            x = drag_handle(ui, egui::Id::new("drag_size"), x, header_y, header_h, HANDLE_W, colors.border, |d| {
                app.config.column_widths.size = (app.config.column_widths.size + d).max(MIN_W);
            });
            x += GAP;

            // Date header (no handle after)
            ui.painter().text(
                egui::pos2(x + 4.0, header_y + header_h / 2.0),
                egui::Align2::LEFT_CENTER,
                "Date",
                egui::FontId::proportional(14.0),
                colors.text_primary,
            );

            if ui.input(|i| i.pointer.any_released()) {
                if app.config.column_widths.name != saved_widths.name
                    || app.config.column_widths.size != saved_widths.size
                    || app.config.column_widths.date != saved_widths.date
                {
                    app.config.save();
                }
            }

            ui.separator();

            // ── Folder rows (on top) ────────────────────
            for (fj, folder) in folders.iter().enumerate() {
                let is_selected = app.browser_state.selected_folder == Some(fj);
                let row_bg = if is_selected { colors.selected_bg } else { colors.card_bg };
                let row_h = 24.0;
                let (rect, response) =
                    ui.allocate_exact_size(Vec2::new(available, row_h), egui::Sense::click());
                let actual_bg = if response.hovered() && !is_selected {
                    colors.hover_bg
                } else {
                    row_bg
                };
                ui.painter().rect_filled(rect, egui::CornerRadius::same(2), actual_bg);
                let widths = col_widths(&app.config.column_widths, rect.width(), ICON_W, MIN_W, GAP);
                let mut x = rect.min.x;
                let cy = rect.center().y;
                ui.painter().text(
                    egui::pos2(x + ICON_W / 2.0, cy),
                    egui::Align2::CENTER_CENTER,
                    entry_glyph(true, folder),
                    egui::FontId::proportional(12.0),
                    colors.accent,
                );
                x += ICON_W;
                let name = folder
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let name_color = if is_selected { colors.text_primary } else { colors.text_secondary };
                ui.painter().text(
                    egui::pos2(x + 4.0, cy),
                    egui::Align2::LEFT_CENTER,
                    &name,
                    egui::FontId::proportional(12.0),
                    name_color,
                );
                x += widths.name + GAP;
                ui.painter().text(
                    egui::pos2(x + widths.size - 4.0, cy),
                    egui::Align2::RIGHT_CENTER,
                    "-",
                    egui::FontId::proportional(12.0),
                    colors.text_secondary,
                );
                x += widths.size + GAP;
                ui.painter().text(
                    egui::pos2(x + widths.date - 4.0, cy),
                    egui::Align2::RIGHT_CENTER,
                    "-",
                    egui::FontId::proportional(12.0),
                    colors.text_secondary,
                );
                if response.double_clicked() {
                    let target = folder.clone();
                    app.current_folder = Some(target);
                    app.scan_folder();
                    return;
                }
                if response.clicked() {
                    app.browser_state.selected_folder = Some(fj);
                    app.browser_state.selected_thumb = None;
                }
            }

            // ── Rows ────────────────────────────────────────
            for (i, path) in paths.iter().enumerate() {
                let is_selected = app.browser_state.selected_thumb == Some(i);
                let stripe = (i + folders.len()) % 2 == 0;
                let row_bg = if is_selected {
                    colors.selected_bg
                } else if stripe {
                    colors.panel_bg
                } else {
                    colors.card_bg
                };

                let row_h = 24.0;
                let (rect, response) = ui.allocate_exact_size(
                    Vec2::new(available, row_h),
                    egui::Sense::click(),
                );

                let actual_bg = if response.hovered() && !is_selected {
                    colors.hover_bg
                } else {
                    row_bg
                };
                ui.painter().rect_filled(rect, egui::CornerRadius::same(2), actual_bg);

                // Row content
                let widths = col_widths(&app.config.column_widths, rect.width(), ICON_W, MIN_W, GAP);
                let mut x = rect.min.x;
                let cy = rect.center().y;

                // Icon: image vs generic file
                let (glyph, glyph_color) = if can_open_in_viewer(path) {
                    ("\u{1F5BC}", colors.text_secondary)
                } else {
                    (entry_glyph(false, path), colors.text_secondary)
                };
                ui.painter().text(
                    egui::pos2(x + ICON_W / 2.0, cy),
                    egui::Align2::CENTER_CENTER,
                    glyph,
                    egui::FontId::proportional(12.0),
                    glyph_color,
                );
                x += ICON_W;

                // Name
                let name = path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let name_color = if is_selected { colors.text_primary } else { colors.text_secondary };
                ui.painter().text(
                    egui::pos2(x + 4.0, cy),
                    egui::Align2::LEFT_CENTER,
                    &name,
                    egui::FontId::proportional(12.0),
                    name_color,
                );
                x += widths.name + GAP;

                // Size
                let meta = app.file_cache.get_or_fetch(path);
                let size_str = meta.as_ref()
                    .map(|m| format_size(m.len))
                    .unwrap_or_else(|| "-".to_string());
                ui.painter().text(
                    egui::pos2(x + widths.size - 4.0, cy),
                    egui::Align2::RIGHT_CENTER,
                    &size_str,
                    egui::FontId::proportional(12.0),
                    colors.text_secondary,
                );
                x += widths.size + GAP;

                // Date
                let date_str = meta.as_ref()
                    .map(|m| m.modified)
                    .and_then(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|dt| format_timestamp(dt.as_secs()))
                    .unwrap_or_else(|| "-".to_string());
                ui.painter().text(
                    egui::pos2(x + widths.date - 4.0, cy),
                    egui::Align2::RIGHT_CENTER,
                    &date_str,
                    egui::FontId::proportional(12.0),
                    colors.text_secondary,
                );

                if response.double_clicked() {
                    if can_open_in_viewer(path) {
                        app.switch_to_viewer(i);
                    }
                    return;
                }
                if response.clicked() {
                    app.browser_state.selected_thumb = Some(i);
                    app.browser_state.selected_folder = None;
                }
            }
        });
}

fn col_widths(cw: &crate::config::ColumnWidths, available: f32, icon_w: f32, min_w: f32, gap: f32) -> ColumnWidthSet {
    let name = cw.name.max(min_w);
    let size = cw.size.max(min_w);
    let fixed = icon_w + name + gap + size + gap;
    let date = cw.date.max(min_w).max(available - fixed);
    ColumnWidthSet { name, size, date }
}

struct ColumnWidthSet {
    name: f32,
    size: f32,
    date: f32,
}

fn drag_handle(
    ui: &mut egui::Ui,
    id: egui::Id,
    x: f32,
    header_y: f32,
    header_h: f32,
    handle_w: f32,
    border_color: egui::Color32,
    mut on_drag: impl FnMut(f32),
) -> f32 {
    let handle_rect = egui::Rect::from_min_size(
        egui::pos2(x - handle_w / 2.0, header_y),
        egui::vec2(handle_w, header_h),
    );
    let resp = ui.interact(handle_rect, id, egui::Sense::click_and_drag());

    ui.painter().vline(x, header_y..=(header_y + header_h), egui::Stroke::new(1.0_f32, border_color));

    if resp.drag_started() || resp.dragged() || resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeColumn);
    }
    if resp.dragged() {
        on_drag(resp.drag_delta().x);
    }

    x
}

// Civil-from-days algorithm (Howard Hinnant), UTC.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn format_timestamp(secs_since_epoch: u64) -> String {
    let days = (secs_since_epoch / 86_400) as i64;
    let time = secs_since_epoch % 86_400;
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02}", time / 3600, (time % 3600) / 60)
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

fn can_open_in_viewer(path: &std::path::Path) -> bool {
    crate::format_ext::is_supported_extension(path)
}

/// Glyph for a grid/list cell that has no image thumbnail:
/// folders get a folder glyph, non-image files get a document glyph,
/// images return "" (thumbnail is drawn instead).
pub fn entry_glyph(is_dir: bool, path: &std::path::Path) -> &'static str {
    if is_dir {
        "\u{1F4C1}" // 📁 folder — visually distinct from files
    } else if can_open_in_viewer(path) {
        ""
    } else {
        "\u{1F4C4}" // 📄 document — generic file, not a folder
    }
}

fn truncate_name(name: &str, max_chars: usize) -> String {
    if name.chars().count() > max_chars {
        let truncated: String = name.chars().take(max_chars - 1).collect();
        format!("{truncated}…")
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{can_open_in_viewer, compute_grid_layout, entry_glyph, format_size, format_timestamp, truncate_name};

    #[test]
    fn test_single_incomplete_row_left_aligns() {
        // avail fits 4 thumbs but only 2 items: must left-align with fixed padding,
        // not spread centered with a huge gap.
        let l = compute_grid_layout(1000.0, 200.0, 2, 8.0);
        assert_eq!(l.cols, 2);
        assert_eq!(l.grid_gap_x, 8.0);
        assert_eq!(l.left_pad, 0.0);
    }

    #[test]
    fn test_full_rows_keep_centered_spacing() {
        // 10 items, avail fits 4 per row: leftover width distributes evenly.
        let l = compute_grid_layout(1000.0, 200.0, 10, 8.0);
        assert_eq!(l.cols, 4);
        assert!(l.grid_gap_x > 8.0);
        assert_eq!(l.left_pad, l.grid_gap_x);
    }

    #[test]
    fn test_entry_glyph_folder_distinct_from_file() {
        let folder = entry_glyph(true, std::path::Path::new("anything"));
        let doc = entry_glyph(false, std::path::Path::new("a.txt"));
        let image = entry_glyph(false, std::path::Path::new("a.jpg"));
        assert_ne!(folder, doc, "folder and generic file must not share a glyph");
        assert_eq!(image, "", "images render thumbnails, not a glyph");
        assert!(!doc.is_empty());
    }

    #[test]
    fn test_can_open_in_viewer_only_for_images() {
        assert!(can_open_in_viewer(std::path::Path::new("a.jpg")));
        assert!(can_open_in_viewer(std::path::Path::new("a.PNG")));
        assert!(!can_open_in_viewer(std::path::Path::new("a.txt")));
        assert!(!can_open_in_viewer(std::path::Path::new("noext")));
    }

    #[test]
    fn test_format_timestamp_epoch() {
        assert_eq!(format_timestamp(0), "1970-01-01 00:00");
    }

    #[test]
    fn test_format_timestamp_known_date() {
        // 2026-07-17 12:34:56 UTC
        assert_eq!(format_timestamp(1_784_291_696), "2026-07-17 12:34");
    }

    #[test]
    fn test_truncate_name_short_unchanged() {
        assert_eq!(truncate_name("short.png", 18), "short.png");
    }

    #[test]
    fn test_truncate_name_long_ascii() {
        let name = "a_very_long_filename_indeed";
        let t = truncate_name(name, 18);
        assert_eq!(t.chars().count(), 18); // 17 chars + ellipsis
        assert!(t.ends_with('…'));
    }

    #[test]
    fn test_truncate_name_cjk_no_panic() {
        // 20 CJK chars = 60 bytes; byte-slicing at 17 would panic
        let name = "测试文件名称非常长的图片文件示例一二三四";
        let t = truncate_name(name, 18);
        assert!(t.ends_with('…'));
        assert_eq!(t.chars().count(), 18);
    }

    #[test]
    fn test_format_size_bytes() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(1023), "1023 B");
    }

    #[test]
    fn test_format_size_kb() {
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(50 * 1024), "50.0 KB");
        assert_eq!(format_size(1024 * 1024 - 1), "1024.0 KB");
    }

    #[test]
    fn test_format_size_mb() {
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(5 * 1024 * 1024), "5.0 MB");
    }

    #[test]
    fn test_format_size_gb() {
        assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(format_size(2 * 1024 * 1024 * 1024), "2.0 GB");
    }
}
