use crate::app::App;
use eframe::egui::{self, Color32, TextureOptions, Vec2, Frame, Margin, Stroke, CornerRadius};
use image::GenericImageView;
use std::path::PathBuf;

const THUMB_SIZE: f32 = 140.0;
const THUMB_PADDING: f32 = 8.0;
const LABEL_HEIGHT: f32 = 30.0;

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

    let folder_name = app
        .current_folder
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    ui.label(
        egui::RichText::new(&folder_name)
            .size(16.0)
            .color(Color32::from_rgb(200, 200, 200)),
    );
    ui.add_space(4.0);

    if app.image_files.is_empty() {
        ui.allocate_space(ui.available_size());
        ui.centered_and_justified(|ui| ui.label("No images found in this folder."));
        return;
    }

    while let Some(result) = app.thumbnail_cache.poll() {
        app.browser_state.thumbnails.insert(result.path, result.image);
    }

    let available_width = ui.available_width();
    let cols = ((available_width - THUMB_PADDING) / (THUMB_SIZE + THUMB_PADDING))
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
        .show(ui, |ui| {
            let cell_size = Vec2::new(THUMB_SIZE, THUMB_SIZE + LABEL_HEIGHT);
            egui::Grid::new("thumb_grid")
                .spacing([THUMB_PADDING, THUMB_PADDING])
                .min_col_width(THUMB_SIZE)
                .show(ui, |ui| {
                    for (i, path) in paths.iter().enumerate() {
                        if i > 0 && i % cols == 0 {
                            ui.end_row();
                        }

                        let is_selected = app.browser_state.selected_thumb == Some(i);
                        let frame = Frame {
                            fill: if is_selected {
                                Color32::from_rgb(40, 80, 140)
                            } else {
                                Color32::from_rgb(30, 30, 30)
                            },
                            corner_radius: CornerRadius::same(4),
                            stroke: if is_selected {
                                Stroke::new(1.0, Color32::from_rgb(80, 160, 255))
                            } else {
                                Stroke::new(1.0, Color32::from_rgb(50, 50, 50))
                            },
                            inner_margin: Margin::symmetric(2, 2),
                            ..Default::default()
                        };

                        let mut thumb_clicked = false;
                        let mut thumb_selected = false;

                        frame.show(ui, |ui| {
                            ui.set_min_size(cell_size);
                            let (rect, response) =
                                ui.allocate_exact_size(cell_size, egui::Sense::click());

                            let thumb_rect =
                                egui::Rect::from_min_size(rect.min, Vec2::new(THUMB_SIZE, THUMB_SIZE));

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
                                    (THUMB_SIZE / tex_size.x).min(THUMB_SIZE / tex_size.y).min(1.0);
                                let draw_size = tex_size * scale;
                                let offset = Vec2::new(
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
                                    Color32::WHITE,
                                );
                            } else if app.browser_state.thumbnails.contains_key(path) {
                                ui.painter().text(
                                    thumb_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "\u{2716}",
                                    egui::FontId::proportional(20.0),
                                    Color32::GRAY,
                                );
                            } else {
                                ui.painter().text(
                                    thumb_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "...",
                                    egui::FontId::proportional(20.0),
                                    Color32::GRAY,
                                );
                            }

                            let name = path
                                .file_stem()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default();
                            let label_rect = egui::Rect::from_min_size(
                                rect.min + Vec2::new(0.0, THUMB_SIZE),
                                Vec2::new(THUMB_SIZE, LABEL_HEIGHT),
                            );
                            ui.painter().text(
                                label_rect.left_center() + Vec2::new(2.0, 0.0),
                                egui::Align2::LEFT_CENTER,
                                &name,
                                egui::FontId::proportional(11.0),
                                Color32::LIGHT_GRAY,
                            );

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

                            if response.double_clicked() {
                                thumb_clicked = true;
                            }
                            if response.clicked() {
                                thumb_selected = true;
                            }
                        });

                        if thumb_clicked {
                            app.switch_to_viewer(i);
                            return;
                        }
                        if thumb_selected {
                            app.browser_state.selected_thumb = Some(i);
                        }
                    }
                });
        });
}

fn show_list_view(app: &mut App, ui: &mut egui::Ui) {
    let paths: Vec<PathBuf> = app.image_files.clone();

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("list_grid")
                .striped(true)
                .spacing([8.0, 2.0])
                .min_col_width(80.0)
                .show(ui, |ui| {
                    ui.strong("Name");
                    ui.strong("Dimensions");
                    ui.strong("Size");
                    ui.strong("Date");
                    ui.end_row();

                    for (i, path) in paths.iter().enumerate() {
                        let is_selected = app.browser_state.selected_thumb == Some(i);
                        let label = egui::RichText::new(
                            path.file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default(),
                        )
                        .color(if is_selected {
                            Color32::WHITE
                        } else {
                            Color32::LIGHT_GRAY
                        });

                        if ui.selectable_label(is_selected, label).clicked() {
                            app.browser_state.selected_thumb = Some(i);
                        }
                        if ui
                            .selectable_label(is_selected, "-")
                            .double_clicked()
                        {
                            app.switch_to_viewer(i);
                            return;
                        }

                        let meta = std::fs::metadata(path).ok();
                        if let Some(ref m) = meta {
                            ui.label(format_size(m.len()));
                            if let Ok(modified) = m.modified() {
                                if let Ok(dt) = modified.duration_since(std::time::UNIX_EPOCH) {
                                    let secs = dt.as_secs();
                                    let days = secs / 86400;
                                    let time = secs % 86400;
                                    let h = time / 3600;
                                    let min = (time % 3600) / 60;
                                    ui.label(format!("{days}d {h:02}:{min:02}"));
                                } else {
                                    ui.label("-");
                                }
                            } else {
                                ui.label("-");
                            }
                        } else {
                            ui.label("-");
                            ui.label("-");
                        }
                        ui.end_row();
                    }
                });
        });
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
