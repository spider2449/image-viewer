use crate::app::{App, Mode};
use crate::image_loader;
use eframe::egui::{self, Color32, Vec2};
use std::path::PathBuf;

#[allow(dead_code)]
pub struct State {
    pub zoom: f32,
    pub fit_zoom: f32,
    pub pan_offset: Vec2,
    pub is_fullscreen: bool,
    pub show_info: bool,
    pub show_cursor_color: bool,
    pub is_slideshow: bool,
    pub slideshow_timer: f64,
    pub image_loaded: bool,
    pub load_error: Option<String>,
}

impl State {
    pub fn new() -> Self {
        Self {
            zoom: 1.0,
            fit_zoom: 1.0,
            pan_offset: Vec2::ZERO,
            is_fullscreen: false,
            show_info: false,
            show_cursor_color: false,
            is_slideshow: false,
            slideshow_timer: 0.0,
            image_loaded: false,
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
                        egui::Rect::from_min_size(
                            egui::pos2(0.0, 0.0),
                            Vec2::new(vp_width, 40.0),
                        ),
                        egui::CornerRadius::ZERO,
                        Color32::from_black_alpha(128),
                    );
                    ui.horizontal(|ui| {
                        if ui.button("\u{2190} Browser").clicked() {
                            app.mode = Mode::Browser;
                            app.viewer_state.is_fullscreen = false;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                        }
                        if ui.button("\u{25C0}").clicked() {
                            if app.selected_image_index > 0 {
                                app.selected_image_index -= 1;
                                app.viewer_state.image_loaded = false;
                            }
                        }
                        if ui.button("\u{25B6}").clicked() {
                            if app.selected_image_index + 1 < app.image_files.len() {
                                app.selected_image_index += 1;
                                app.viewer_state.image_loaded = false;
                            }
                        }
                        if ui.button("Fit").clicked() {
                            app.viewer_state.zoom = app.viewer_state.fit_zoom;
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

    egui::CentralPanel::default().show(ctx, |ui| {
        let available = ui.available_size();
        let image_rect = ui.max_rect();

        // Handle keyboard
        ctx.input(|i| {
            for event in &i.events {
                if let egui::Event::Key { key, pressed: true, .. } = event {
                    match key {
                        egui::Key::ArrowLeft => {
                            if !app.viewer_state.is_slideshow {
                                let prev =
                                    app.selected_image_index.saturating_sub(1);
                                app.selected_image_index = prev;
                                app.viewer_state.image_loaded = false;
                            }
                        }
                        egui::Key::ArrowRight => {
                            if !app.viewer_state.is_slideshow
                                && app.selected_image_index + 1 < app.image_files.len()
                            {
                                app.selected_image_index += 1;
                                app.viewer_state.image_loaded = false;
                            }
                        }
                        egui::Key::Escape => {
                            if app.viewer_state.is_fullscreen {
                                app.viewer_state.is_fullscreen = false;
                                ctx.send_viewport_cmd(
                                    egui::ViewportCommand::Fullscreen(false),
                                );
                            }
                        }
                        egui::Key::F11 => {
                            app.viewer_state.is_fullscreen =
                                !app.viewer_state.is_fullscreen;
                            ctx.send_viewport_cmd(
                                egui::ViewportCommand::Fullscreen(app.viewer_state.is_fullscreen),
                            );
                        }
                        egui::Key::I => {
                            app.viewer_state.show_info = !app.viewer_state.show_info;
                        }
                        egui::Key::F => {
                            app.viewer_state.zoom = app.viewer_state.fit_zoom;
                            app.viewer_state.pan_offset = Vec2::ZERO;
                        }
                        egui::Key::Num1 => {
                            app.viewer_state.zoom = 1.0;
                            app.viewer_state.pan_offset = Vec2::ZERO;
                        }
                        egui::Key::Space => {
                            app.viewer_state.is_slideshow =
                                !app.viewer_state.is_slideshow;
                            app.viewer_state.slideshow_timer = 0.0;
                        }
                        egui::Key::F5 => {
                            app.viewer_state.is_slideshow =
                                !app.viewer_state.is_slideshow;
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
            if app.viewer_state.slideshow_timer
                >= app.config.slideshow_interval_secs as f64
            {
                app.viewer_state.slideshow_timer = 0.0;
                if app.selected_image_index + 1 < app.image_files.len() {
                    app.selected_image_index += 1;
                    app.viewer_state.image_loaded = false;
                } else {
                    app.viewer_state.is_slideshow = false;
                }
            }
        }

        let tex_key = path.to_string_lossy().to_string();
        if !app.textures.contains_key(&tex_key) {
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
                ui.painter().rect_filled(cell, egui::CornerRadius::ZERO, check_colors[idx]);
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
        egui::CornerRadius::ZERO,
        egui::Stroke::new(1.0, crate::theme::BORDER),
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
