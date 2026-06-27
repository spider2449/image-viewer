use crate::app::App;
use eframe::egui::{self, Color32, TextureOptions, Vec2, Stroke, CornerRadius};
use image::GenericImageView;
use std::path::PathBuf;

const THUMB_PADDING: f32 = 8.0;
const LABEL_HEIGHT: f32 = 30.0;

pub fn show_grid(app: &mut App, ui: &mut egui::Ui) {
    let mut size_changed = false;
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
        ui.separator();
        ui.label("Size:");
        let mut ts = app.config.thumb_size;
        if ui.add(egui::Slider::new(&mut ts, 60.0..=400.0).text("px")).changed() {
            app.config.thumb_size = ts;
            size_changed = true;
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

    while let Some(result) = app.thumbnail_cache.poll() {
        app.browser_state.thumbnails.insert(result.path, result.image);
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

    let available_width = ui.available_width();
    let cols = ((available_width - THUMB_PADDING) / (app.config.thumb_size + THUMB_PADDING))
        .floor()
        .max(1.0) as usize;

    if app.browser_state.show_list_view {
        show_list_view(app, ui);
    } else {
        show_thumbnail_grid(app, ui, cols);
    }
}

fn show_thumbnail_grid(app: &mut App, ui: &mut egui::Ui, cols: usize) {
    let paths: Vec<PathBuf> = app.image_files.clone();
    let ctx = ui.ctx().clone();

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .id_salt("thumb_grid_scroll")
        .show(ui, |ui| {
            let cell_size = Vec2::new(app.config.thumb_size, app.config.thumb_size + LABEL_HEIGHT);
            egui::Grid::new("thumb_grid")
                .spacing([THUMB_PADDING, THUMB_PADDING])
                .min_col_width(app.config.thumb_size)
                .show(ui, |ui| {
                    for (i, path) in paths.iter().enumerate() {
                        if i > 0 && i % cols == 0 {
                            ui.end_row();
                        }

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

                        if let Some(Some(ci)) = app.browser_state.thumbnails.get(path) {
                            let tex = if let Some(t) = app.browser_state.thumb_textures.get(path) {
                                t.clone()
                            } else {
                                let key = format!("thumb_{}", path.to_string_lossy());
                                let t = ctx.load_texture(&key, ci.clone(), TextureOptions::LINEAR);
                                app.browser_state.thumb_textures.insert(path.clone(), t.clone());
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
                            rect.min + Vec2::new(4.0, app.config.thumb_size),
                            Vec2::new(app.config.thumb_size - 8.0, LABEL_HEIGHT),
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

fn show_list_view(app: &mut App, ui: &mut egui::Ui) {
    let paths: Vec<PathBuf> = app.image_files.clone();

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

            ui.painter().rect_filled(header_rect, egui::CornerRadius::same(2), crate::theme::PANEL_BG);

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
                crate::theme::TEXT_PRIMARY,
            );
            x += widths.name;
            x = drag_handle(ui, egui::Id::new("drag_name"), x, header_y, header_h, HANDLE_W, |d| {
                app.config.column_widths.name = (app.config.column_widths.name + d).max(MIN_W);
            });
            x += GAP;

            // Size header + drag handle
            ui.painter().text(
                egui::pos2(x + 4.0, header_y + header_h / 2.0),
                egui::Align2::LEFT_CENTER,
                "Size",
                egui::FontId::proportional(14.0),
                crate::theme::TEXT_PRIMARY,
            );
            x += widths.size;
            x = drag_handle(ui, egui::Id::new("drag_size"), x, header_y, header_h, HANDLE_W, |d| {
                app.config.column_widths.size = (app.config.column_widths.size + d).max(MIN_W);
            });
            x += GAP;

            // Date header (no handle after)
            ui.painter().text(
                egui::pos2(x + 4.0, header_y + header_h / 2.0),
                egui::Align2::LEFT_CENTER,
                "Date",
                egui::FontId::proportional(14.0),
                crate::theme::TEXT_PRIMARY,
            );

            if ui.input(|i| i.pointer.any_released()) {
                app.config.save();
            }

            ui.separator();

            // ── Rows ────────────────────────────────────────
            for (i, path) in paths.iter().enumerate() {
                let is_selected = app.browser_state.selected_thumb == Some(i);
                let row_bg = if is_selected {
                    crate::theme::SELECTED_BG
                } else if i % 2 == 0 {
                    crate::theme::PANEL_BG
                } else {
                    crate::theme::CARD_BG
                };

                let row_h = 24.0;
                let (rect, response) = ui.allocate_exact_size(
                    Vec2::new(available, row_h),
                    egui::Sense::click(),
                );

                let actual_bg = if response.hovered() && !is_selected {
                    crate::theme::HOVER_BG
                } else {
                    row_bg
                };
                ui.painter().rect_filled(rect, egui::CornerRadius::same(2), actual_bg);

                // Row content
                let widths = col_widths(&app.config.column_widths, rect.width(), ICON_W, MIN_W, GAP);
                let mut x = rect.min.x;
                let cy = rect.center().y;

                // Icon
                ui.painter().text(
                    egui::pos2(x + ICON_W / 2.0, cy),
                    egui::Align2::CENTER_CENTER,
                    "\u{1F5BC}",
                    egui::FontId::proportional(12.0),
                    crate::theme::TEXT_SECONDARY,
                );
                x += ICON_W;

                // Name
                let name = path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let name_color = if is_selected { crate::theme::TEXT_PRIMARY } else { crate::theme::TEXT_SECONDARY };
                ui.painter().text(
                    egui::pos2(x + 4.0, cy),
                    egui::Align2::LEFT_CENTER,
                    &name,
                    egui::FontId::proportional(12.0),
                    name_color,
                );
                x += widths.name + GAP;

                // Size
                let meta = std::fs::metadata(path).ok();
                let size_str = meta.as_ref()
                    .map(|m| format_size(m.len()))
                    .unwrap_or_else(|| "-".to_string());
                ui.painter().text(
                    egui::pos2(x + widths.size - 4.0, cy),
                    egui::Align2::RIGHT_CENTER,
                    &size_str,
                    egui::FontId::proportional(12.0),
                    crate::theme::TEXT_SECONDARY,
                );
                x += widths.size + GAP;

                // Date
                let date_str = meta.as_ref()
                    .and_then(|m| m.modified().ok())
                    .and_then(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|dt| {
                        let secs = dt.as_secs();
                        let days = secs / 86400;
                        let time = secs % 86400;
                        let h = time / 3600;
                        let min = (time % 3600) / 60;
                        format!("{days}d {h:02}:{min:02}")
                    })
                    .unwrap_or_else(|| "-".to_string());
                ui.painter().text(
                    egui::pos2(x + widths.date - 4.0, cy),
                    egui::Align2::RIGHT_CENTER,
                    &date_str,
                    egui::FontId::proportional(12.0),
                    crate::theme::TEXT_SECONDARY,
                );

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

fn col_widths(cw: &crate::config::ColumnWidths, available: f32, icon_w: f32, min_w: f32, gap: f32) -> ColumnWidthSet {
    let name = cw.name.max(min_w);
    let size = cw.size.max(min_w);
    let mut date = cw.date.max(min_w);
    let fixed = icon_w + name + gap + size + gap;
    if fixed + date < available {
        date += available - fixed - date;
    }
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
    mut on_drag: impl FnMut(f32),
) -> f32 {
    let handle_rect = egui::Rect::from_min_size(
        egui::pos2(x - handle_w / 2.0, header_y),
        egui::vec2(handle_w, header_h),
    );
    let resp = ui.interact(handle_rect, id, egui::Sense::click_and_drag());

    ui.painter().vline(x, header_y..=(header_y + header_h), egui::Stroke::new(1.0, crate::theme::BORDER));

    if resp.drag_started() || resp.dragged() || resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeColumn);
    }
    if resp.dragged() {
        on_drag(resp.drag_delta().x);
    }

    x
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

#[cfg(test)]
mod tests {
    use super::format_size;

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
