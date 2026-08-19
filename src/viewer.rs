use crate::app::{App, Mode};
use crate::image_loader;
use eframe::egui::{self, Color32, Vec2};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZoomMode {
    Fit,
    Manual,
}

#[allow(dead_code)]
pub struct State {
    pub zoom: f32,
    pub fit_zoom: f32,
    pub zoom_mode: ZoomMode,
    pub pan_offset: Vec2,
    pub is_fullscreen: bool,
    pub show_info: bool,
    pub show_cursor_color: bool,
    pub is_slideshow: bool,
    pub slideshow_timer: f64,
    pub load_error: Option<String>,
}

impl State {
    pub fn new() -> Self {
        Self {
            zoom: 1.0,
            fit_zoom: 1.0,
            zoom_mode: ZoomMode::Fit,
            pan_offset: Vec2::ZERO,
            is_fullscreen: false,
            show_info: false,
            show_cursor_color: false,
            is_slideshow: false,
            slideshow_timer: 0.0,
            load_error: None,
        }
    }
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    let path = match app.image_files.get(app.selected_image_index) {
        Some(p) => p.clone(),
        None => {
            app.mode = Mode::Browser;
            return;
        }
    };

    let is_fullscreen = app.viewer_state.is_fullscreen;
    let colors = app.theme_colors();

    if !is_fullscreen {
        egui::TopBottomPanel::top("viewer_toolbar")
            .frame(egui::Frame {
                fill: colors.panel_bg,
                ..Default::default()
            })
            .show(ctx, |ui| {
                let colors = colors;
                ui.horizontal(|ui| {
                    // Navigation group
                    if ui
                        .button(egui::RichText::new("\u{2190} Browser").color(colors.accent))
                        .clicked()
                    {
                        app.request_browser();
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
                        app.viewer_state.zoom_mode = ZoomMode::Fit;
                        app.viewer_state.pan_offset = Vec2::ZERO;
                    }
                    if ui.button("1:1").clicked() {
                        app.viewer_state.zoom = 1.0;
                        app.viewer_state.zoom_mode = ZoomMode::Manual;
                        app.viewer_state.pan_offset = Vec2::ZERO;
                    }
                    ui.colored_label(colors.text_secondary, "Zoom:");
                    let mut zoom_pct = (app.viewer_state.zoom * 100.0) as i32;
                    if ui
                        .add(egui::Slider::new(&mut zoom_pct, 10..=3200).text("%"))
                        .changed()
                    {
                        app.viewer_state.zoom = zoom_pct as f32 / 100.0;
                        app.viewer_state.zoom_mode = ZoomMode::Manual;
                    }
                    // Display group
                    ui.separator();
                    if ui
                        .selectable_label(app.viewer_state.show_info, "Info")
                        .clicked()
                    {
                        app.viewer_state.show_info = !app.viewer_state.show_info;
                    }
                    if ui
                        .selectable_label(app.exif_state.visible, "Exif")
                        .clicked()
                    {
                        app.exif_state.visible = !app.exif_state.visible;
                    }
                    if ui
                        .selectable_label(app.editor_state.visible, "Edit")
                        .clicked()
                    {
                        if app.editor_state.visible {
                            crate::editor::discard_adjustment_preview(app);
                            app.editor_state.visible = false;
                        } else {
                            app.editor_state.visible = true;
                        }
                        if app.editor_state.visible && app.editor_state.current_image.is_none() {
                            app.editor_state.load_image(&path);
                        }
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
                        ui.colored_label(
                            colors.text_secondary,
                            format!("{}/{}", app.selected_image_index + 1, app.image_files.len()),
                        );
                        if let Some(name) = path.file_name() {
                            ui.colored_label(
                                colors.text_secondary,
                                name.to_string_lossy().to_string(),
                            );
                        }
                    });
                });
            });
    }

    if is_fullscreen {
        let mouse_pos = ctx.input(|i| i.pointer.hover_pos());
        let show_overlay = mouse_pos.is_some_and(|pos| pos.y < 40.0);

        if show_overlay {
            let overlay_id = egui::Id::new("fullscreen_overlay");
            egui::Area::new(overlay_id)
                .fixed_pos(egui::pos2(0.0, 0.0))
                .show(ctx, |ui| {
                    let vp = ui.ctx().input(|i| i.viewport().inner_rect);
                    let vp_width = vp.map(|r| r.max.x).unwrap_or(800.0);
                    let painter = ui.painter();
                    painter.rect_filled(
                        egui::Rect::from_min_size(egui::pos2(0.0, 0.0), Vec2::new(vp_width, 40.0)),
                        egui::CornerRadius::ZERO,
                        Color32::from_black_alpha(128),
                    );
                    ui.horizontal(|ui| {
                        if ui.button("\u{2190} Browser").clicked() {
                            app.request_browser();
                            app.viewer_state.is_fullscreen = false;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                        }
                        if ui.button("\u{25C0}").clicked() {
                            app.prev_image();
                        }
                        if ui.button("\u{25B6}").clicked() {
                            app.next_image();
                        }
                        if ui.button("Fit").clicked() {
                            app.viewer_state.zoom_mode = ZoomMode::Fit;
                            app.viewer_state.pan_offset = Vec2::ZERO;
                        }
                        if ui.button("ESC").clicked() {
                            app.viewer_state.is_fullscreen = false;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                        }
                    });
                });
        }
    }

    egui::TopBottomPanel::bottom("viewer_status")
        .frame(egui::Frame {
            fill: colors.panel_bg,
            ..Default::default()
        })
        .show(ctx, |ui| {
            let colors = colors;
            ui.horizontal(|ui| {
                let z = app.viewer_state.zoom;
                let px = app.viewer_state.pan_offset.x;
                let py = app.viewer_state.pan_offset.y;
                ui.colored_label(
                    colors.text_secondary,
                    format!("Zoom: {:.0}% | Pos: ({px:.0}, {py:.0})", z * 100.0),
                );
                ui.separator();
                if let Some(meta) = app.file_cache.get_or_fetch(&path) {
                    let sz = meta.len;
                    let size_str = if sz >= 1024 * 1024 {
                        format!("{:.1} MB", sz as f64 / (1024.0 * 1024.0))
                    } else if sz >= 1024 {
                        format!("{:.1} KB", sz as f64 / 1024.0)
                    } else {
                        format!("{sz} B")
                    };
                    ui.colored_label(colors.text_secondary, size_str);
                }
            });
        });

    egui::CentralPanel::default().show(ctx, |ui| {
        let available = ui.available_size();
        let image_rect = ui.max_rect();

        // Handle keyboard
        ctx.input(|i| {
            for event in &i.events {
                if let egui::Event::Key {
                    key, pressed: true, ..
                } = event
                {
                    match key {
                        egui::Key::ArrowLeft => {
                            if !app.viewer_state.is_slideshow {
                                app.prev_image();
                            }
                        }
                        egui::Key::ArrowRight => {
                            if !app.viewer_state.is_slideshow {
                                app.next_image();
                            }
                        }
                        egui::Key::Escape => {
                            if app.viewer_state.is_fullscreen {
                                app.viewer_state.is_fullscreen = false;
                                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                            } else {
                                app.request_browser();
                            }
                        }
                        egui::Key::F11 => {
                            app.viewer_state.is_fullscreen = !app.viewer_state.is_fullscreen;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(
                                app.viewer_state.is_fullscreen,
                            ));
                        }
                        egui::Key::I => {
                            app.viewer_state.show_info = !app.viewer_state.show_info;
                        }
                        egui::Key::F => {
                            app.viewer_state.zoom_mode = ZoomMode::Fit;
                            app.viewer_state.pan_offset = Vec2::ZERO;
                        }
                        egui::Key::Num1 => {
                            app.viewer_state.zoom = 1.0;
                            app.viewer_state.zoom_mode = ZoomMode::Manual;
                            app.viewer_state.pan_offset = Vec2::ZERO;
                        }
                        egui::Key::Space => {
                            app.viewer_state.is_slideshow = !app.viewer_state.is_slideshow;
                            app.viewer_state.slideshow_timer = 0.0;
                        }
                        egui::Key::F5 => {
                            app.viewer_state.is_slideshow = !app.viewer_state.is_slideshow;
                            app.viewer_state.slideshow_timer = 0.0;
                        }
                        _ => {}
                    }
                }
            }
        });

        if app.viewer_state.is_slideshow {
            let delta = ctx.input(|i| i.unstable_dt) as f64;
            app.viewer_state.slideshow_timer += delta;
            if app.viewer_state.slideshow_timer >= app.config.slideshow_interval_secs as f64 {
                app.viewer_state.slideshow_timer = 0.0;
                if app.selected_image_index + 1 < app.image_files.len() {
                    app.next_image();
                } else {
                    app.viewer_state.is_slideshow = false;
                }
            }
        }

        let tex_key = path.to_string_lossy().to_string();
        if !app.textures.contains(&tex_key) {
            if let Err(e) = image_loader::load_to_texture(ctx, &mut app.textures, &path) {
                ui.painter().text(
                    image_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("Error: {e}"),
                    egui::FontId::proportional(18.0),
                    Color32::RED,
                );
                return;
            }
        }
        let tex = app.textures.get(&tex_key).cloned();
        if let Some(tex) = tex {
            draw_image(app, ui, &tex, image_rect, available, &path);
        }
    });
}

fn draw_image(
    app: &mut App,
    ui: &mut egui::Ui,
    tex: &egui::TextureHandle,
    image_rect: egui::Rect,
    available: Vec2,
    path: &PathBuf,
) {
    let colors = app.theme_colors();
    let tex_size = tex.size_vec2();
    app.viewer_state.fit_zoom = calculate_fit_zoom(tex_size, available);
    if app.viewer_state.zoom_mode == ZoomMode::Fit {
        app.viewer_state.zoom = app.viewer_state.fit_zoom;
    }
    let display_size = display_size(tex_size, app.viewer_state.zoom);

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

    // Dark background behind image area (replaces cell-by-cell checkerboard).
    // The border below provides visual separation from the image.
    {
        let bg_rect = egui::Rect::from_min_size(
            egui::pos2(
                image_rect.min.x + offset.x + app.viewer_state.pan_offset.x,
                image_rect.min.y + offset.y + app.viewer_state.pan_offset.y,
            ),
            display_size,
        );
        ui.painter().rect_filled(
            bg_rect,
            egui::CornerRadius::ZERO,
            egui::Color32::from_rgb(0x1a, 0x1a, 0x1a),
        );
    }

    // Inner border around image area
    let border_rect = egui::Rect::from_min_size(image_rect.min, available);
    ui.painter().rect_stroke(
        border_rect,
        egui::CornerRadius::ZERO,
        egui::Stroke::new(1.0, colors.border),
        egui::StrokeKind::Inside,
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
                app.viewer_state.zoom =
                    (app.viewer_state.zoom * (1.0 + scroll_delta.y * 0.001)).clamp(0.1, 32.0);
                app.viewer_state.zoom_mode = ZoomMode::Manual;
                let ratio = app.viewer_state.zoom / old_zoom;
                let mouse_rel = pos - draw_rect.min;
                app.viewer_state.pan_offset =
                    mouse_rel - (mouse_rel - app.viewer_state.pan_offset) * ratio;
            }
        }
    }

    let crop_active = app.editor_state.crop_active;
    let drag = ui.interact(
        egui::Rect::from_min_size(image_rect.min, available),
        egui::Id::new("viewer_image_drag"),
        egui::Sense::drag(),
    );
    if crop_active {
        if drag.drag_started() {
            if let Some(pos) = mouse_pos {
                let img_pos = screen_to_image_pos(pos, draw_rect, tex_size);
                app.editor_state.crop_start = Some(img_pos);
                app.editor_state.crop_end = Some(img_pos);
            }
        } else if drag.dragged() {
            if let Some(pos) = mouse_pos {
                app.editor_state.crop_end = Some(screen_to_image_pos(pos, draw_rect, tex_size));
            }
        }
    } else if drag.dragged() {
        app.viewer_state.pan_offset += drag.drag_delta();
    }

    // Crop selection overlay
    if crop_active {
        if let (Some(start), Some(end)) = (app.editor_state.crop_start, app.editor_state.crop_end) {
            let sel_min = image_to_screen_pos(
                egui::pos2(start.x.min(end.x), start.y.min(end.y)),
                draw_rect,
                tex_size,
            );
            let sel_max = image_to_screen_pos(
                egui::pos2(start.x.max(end.x), start.y.max(end.y)),
                draw_rect,
                tex_size,
            );
            let sel_rect = egui::Rect::from_min_max(sel_min, sel_max);
            let painter = ui.painter();
            painter.rect_filled(
                draw_rect,
                egui::CornerRadius::ZERO,
                egui::Color32::from_black_alpha(120),
            );
            ui.painter().image(
                tex.id(),
                sel_rect,
                egui::Rect::from_min_max(
                    egui::pos2(
                        start.x.min(end.x) / tex_size.x,
                        start.y.min(end.y) / tex_size.y,
                    ),
                    egui::pos2(
                        start.x.max(end.x) / tex_size.x,
                        start.y.max(end.y) / tex_size.y,
                    ),
                ),
                egui::Color32::WHITE,
            );
            painter.rect_stroke(
                sel_rect,
                egui::CornerRadius::ZERO,
                egui::Stroke::new(1.5, egui::Color32::from_rgb(0xFF, 0xD5, 0x00)),
                egui::StrokeKind::Inside,
            );
        }
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
            galley.size() + egui::Vec2::new(8.0, 8.0),
        );
        painter.rect_filled(
            bg_rect,
            egui::CornerRadius::same(4),
            egui::Color32::from_black_alpha(180),
        );
        painter.galley(text_pos, galley, egui::Color32::WHITE);
    }
}

fn ctx_input(ctx: &egui::Context) -> (Option<egui::Pos2>, Vec2) {
    ctx.input(|i| (i.pointer.hover_pos(), i.raw_scroll_delta))
}

fn screen_to_image_pos(pos: egui::Pos2, draw_rect: egui::Rect, tex_size: Vec2) -> egui::Pos2 {
    let rel = (pos - draw_rect.min) / draw_rect.size() * tex_size;
    egui::pos2(rel.x.clamp(0.0, tex_size.x), rel.y.clamp(0.0, tex_size.y))
}

fn image_to_screen_pos(pos: egui::Pos2, draw_rect: egui::Rect, tex_size: Vec2) -> egui::Pos2 {
    draw_rect.min + Vec2::new(pos.x / tex_size.x, pos.y / tex_size.y) * draw_rect.size()
}

fn calculate_fit_zoom(texture_size: Vec2, available: Vec2) -> f32 {
    if texture_size.x <= 0.0 || texture_size.y <= 0.0 {
        return 1.0;
    }
    (available.x / texture_size.x).min(available.y / texture_size.y)
}

fn display_size(texture_size: Vec2, zoom: f32) -> Vec2 {
    texture_size * zoom
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_large_image_fit_uses_absolute_scale() {
        let image = Vec2::new(4000.0, 3000.0);
        let area = Vec2::new(1000.0, 750.0);
        let zoom = calculate_fit_zoom(image, area);
        assert_eq!(zoom, 0.25);
        assert_eq!(display_size(image, zoom), area);
    }

    #[test]
    fn test_one_to_one_uses_source_pixel_dimensions() {
        assert_eq!(
            display_size(Vec2::new(4000.0, 3000.0), 1.0),
            Vec2::new(4000.0, 3000.0)
        );
    }

    #[test]
    fn test_fit_upscales_small_images_to_fill_available_area() {
        assert_eq!(
            calculate_fit_zoom(Vec2::new(400.0, 300.0), Vec2::new(800.0, 600.0)),
            2.0
        );
    }

    #[test]
    fn test_viewport_resize_only_changes_fit_mode_zoom() {
        let image = Vec2::new(4000.0, 3000.0);
        let mut state = State::new();
        state.fit_zoom = calculate_fit_zoom(image, Vec2::new(1000.0, 750.0));
        if state.zoom_mode == ZoomMode::Fit {
            state.zoom = state.fit_zoom;
        }
        assert_eq!(state.zoom, 0.25);

        state.zoom_mode = ZoomMode::Manual;
        state.zoom = 1.5;
        state.fit_zoom = calculate_fit_zoom(image, Vec2::new(2000.0, 1500.0));
        if state.zoom_mode == ZoomMode::Fit {
            state.zoom = state.fit_zoom;
        }
        assert_eq!(state.fit_zoom, 0.5);
        assert_eq!(state.zoom, 1.5);
    }
}
