use crate::app::{App, Mode};
use crate::image_loader;
use eframe::egui::{self, emath::GuiRounding, Color32, Vec2};
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
                    let pixels_per_point = ui.ctx().pixels_per_point();
                    if ui.button("1:1").clicked() {
                        app.viewer_state.zoom = one_to_one_zoom(pixels_per_point);
                        app.viewer_state.zoom_mode = ZoomMode::Manual;
                        app.viewer_state.pan_offset = Vec2::ZERO;
                    }
                    ui.colored_label(colors.text_secondary, "Zoom:");
                    let mut zoom_pct =
                        zoom_percent(app.viewer_state.zoom, pixels_per_point).round() as i32;
                    if ui
                        .add(egui::Slider::new(&mut zoom_pct, 10..=3200).text("%"))
                        .changed()
                    {
                        app.viewer_state.zoom =
                            zoom_pct as f32 / 100.0 * one_to_one_zoom(pixels_per_point);
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
                ui.colored_label(colors.text_secondary, path.to_string_lossy().to_string());
                ui.separator();
                let z = app.viewer_state.zoom;
                let px = app.viewer_state.pan_offset.x;
                let py = app.viewer_state.pan_offset.y;
                ui.colored_label(
                    colors.text_secondary,
                    format!(
                        "Zoom: {:.0}% | Pos: ({px:.0}, {py:.0})",
                        zoom_percent(z, ui.ctx().pixels_per_point())
                    ),
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
                    ui.separator();
                }
                ui.colored_label(
                    colors.text_secondary,
                    format!("{}/{}", app.selected_image_index + 1, app.image_files.len()),
                );
            });
        });

    // Side panels must claim their space before the central panel; otherwise the
    // image area extends underneath them and receives their pointer input.
    crate::editor::show(app, ctx);
    crate::exif::show(&mut app.exif_state, ctx, &colors);

    egui::CentralPanel::default().show(ctx, |ui| {
        let available = ui.available_size();
        let image_rect = ui.max_rect();

        // Handle keyboard (not while a text field such as the annotation text has focus)
        let typing = ctx.wants_keyboard_input();
        ctx.input(|i| {
            if typing {
                return;
            }
            for event in &i.events {
                if let egui::Event::Key {
                    key,
                    pressed: true,
                    repeat,
                    modifiers,
                    ..
                } = event
                {
                    let ann = &mut app.editor_state.annotate;
                    if ann.active && modifiers.command {
                        match key {
                            egui::Key::A => ann.select_all(),
                            egui::Key::D => ann.duplicate_selected(),
                            _ => {}
                        }
                    }
                    // With objects selected the arrows nudge them instead of
                    // changing image. A held key is one undo step.
                    if ann.active && !ann.selected.is_empty() {
                        if let Some(delta) = nudge_delta(*key, modifiers.shift) {
                            ann.nudge_selected(delta, !*repeat);
                            continue;
                        }
                    }
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
                        egui::Key::Delete if app.editor_state.annotate.active => {
                            app.editor_state.annotate.delete_selected();
                        }
                        egui::Key::Escape if app.editor_state.annotate.active => {
                            // Esc backs out of draw mode one level at a time.
                            let ann = &mut app.editor_state.annotate;
                            if ann.draft.is_some() {
                                ann.draft = None;
                            } else if ann.marquee.is_some() {
                                ann.marquee = None;
                            } else if !ann.selected.is_empty() {
                                ann.select(None);
                            } else {
                                ann.active = false;
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
                            app.viewer_state.zoom = one_to_one_zoom(ui.ctx().pixels_per_point());
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
                    colors.danger,
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
    app.viewer_state.fit_zoom =
        snap_fit_zoom(calculate_fit_zoom(tex_size, available), ui.ctx().pixels_per_point());
    if app.viewer_state.zoom_mode == ZoomMode::Fit {
        app.viewer_state.zoom = app.viewer_state.fit_zoom;
    }
    let display_size = display_size(tex_size, app.viewer_state.zoom);

    let offset = Vec2::new(
        (available.x - display_size.x).max(0.0) / 2.0,
        (available.y - display_size.y).max(0.0) / 2.0,
    );

    // Snap to physical pixels: a fractional origin shifts every sampled texel,
    // which would blur a true 1:1 view.
    let draw_rect = egui::Rect::from_min_size(
        egui::pos2(
            image_rect.min.x + offset.x + app.viewer_state.pan_offset.x,
            image_rect.min.y + offset.y + app.viewer_state.pan_offset.y,
        )
        .round_to_pixels(ui.ctx().pixels_per_point()),
        display_size,
    );

    // Dark background behind image area (replaces cell-by-cell checkerboard).
    // The border below provides visual separation from the image.
    ui.painter().rect_filled(
        draw_rect,
        egui::CornerRadius::ZERO,
        colors.bg_dark,
    );

    // Inner border around image area
    let border_rect = egui::Rect::from_min_size(image_rect.min, available);
    ui.painter().rect_stroke(
        border_rect,
        egui::CornerRadius::ZERO,
        egui::Stroke::new(1.0_f32, colors.border),
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

    handle_annotate_clipboard(app, ui.ctx(), draw_rect, tex_size, mouse_pos);

    if let Some(pos) = mouse_pos {
        if ui.rect_contains_pointer(draw_rect) {
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
    let annot_active = app.editor_state.annotate.active;
    let zoom = app.viewer_state.zoom;
    let drag = ui.interact(
        egui::Rect::from_min_size(image_rect.min, available),
        egui::Id::new("viewer_image_drag"),
        egui::Sense::click_and_drag(),
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
        } else if drag.clicked() && app.editor_state.auto_region.enabled {
            if let Some(pos) = mouse_pos {
                let img_pos = screen_to_image_pos(pos, draw_rect, tex_size);
                // A miss keeps whatever crop rectangle is already there.
                let params = app.config.auto_region;
                if let Some(r) = crate::editor::auto_region_at(&mut app.editor_state, &params, img_pos) {
                    app.editor_state.crop_start = Some(r.min);
                    app.editor_state.crop_end = Some(r.max);
                }
            }
        }
    } else if annot_active {
        handle_annotate_drag(app, &drag, draw_rect, tex_size, mouse_pos, zoom);
        if drag.hovered() && app.editor_state.annotate.tool != crate::annotate::Tool::Select {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        }
        // Middle-button drag pans the image while the left button draws.
        if drag.hovered() {
            let delta = ui.ctx().input(|i| {
                if i.pointer.middle_down() {
                    Some(i.pointer.delta())
                } else {
                    None
                }
            });
            if let Some(d) = delta {
                app.viewer_state.pan_offset += d;
            }
        }
        // Remember which curve anchor (if any) the right click landed on.
        if drag.secondary_clicked() {
            let tolerance = 8.0 / zoom.max(1e-3);
            let ann = &mut app.editor_state.annotate;
            // Right-clicking an unselected object targets the menu at that object.
            let hit = mouse_pos.and_then(|m| {
                let pos = screen_to_image_pos(m, draw_rect, tex_size);
                ann.shapes.iter().rposition(|s| s.hit_test(pos, tolerance))
            });
            if let Some(i) = hit {
                ann.grab(i, false);
            }
            ann.menu_anchor = mouse_pos.and_then(|m| {
                let pos = screen_to_image_pos(m, draw_rect, tex_size);
                ann.single_selected().and_then(|i| ann.shapes.get(i)).and_then(|s| match s {
                    crate::annotate::Shape::Pencil { .. } => s
                        .handles()
                        .iter()
                        .position(|h| (*h - pos).length() <= tolerance),
                    _ => None,
                })
            });
        }
        // Right-click action menu in draw mode.
        drag.context_menu(|ui| {
            let ctx = ui.ctx().clone();
            let has_shapes = !app.editor_state.annotate.shapes.is_empty();
            let has_selection = !app.editor_state.annotate.selected.is_empty();
            if ui
                .add_enabled(has_shapes, egui::Button::new("Apply"))
                .on_hover_text("Burn annotations into the image as one undo step")
                .clicked()
            {
                crate::editor::apply_annotations(app, &ctx);
                ui.close_menu();
            }
            if ui
                .button("Save")
                .on_hover_text("Burn annotations in and overwrite the opened file")
                .clicked()
            {
                crate::editor::request_save(app);
                ui.close_menu();
            }
            let ann = &mut app.editor_state.annotate;
            ui.horizontal(|ui| {
                if ui.add_enabled(ann.can_undo(), egui::Button::new("Undo")).clicked() {
                    ann.undo();
                }
                if ui.add_enabled(ann.can_redo(), egui::Button::new("Redo")).clicked() {
                    ann.redo();
                }
            });
            ui.separator();
            if let Some(anchor) = ann.menu_anchor {
                if ui
                    .add_enabled(
                        ann.can_delete_anchor(anchor),
                        egui::Button::new("Delete Point"),
                    )
                    .on_hover_text("Remove this curve point (a curve keeps at least 2)")
                    .clicked()
                {
                    ann.delete_anchor(anchor);
                    ann.menu_anchor = None;
                    ui.close_menu();
                }
            }
            if ui
                .add_enabled(has_selection, egui::Button::new("Cut").shortcut_text("Ctrl+X"))
                .clicked()
            {
                crate::annotate::cut_to_clipboard(ann, &ctx);
                ui.close_menu();
            }
            if ui
                .add_enabled(has_selection, egui::Button::new("Copy").shortcut_text("Ctrl+C"))
                .clicked()
            {
                crate::annotate::copy_to_clipboard(ann, &ctx);
                ui.close_menu();
            }
            if ui
                .add_enabled(ann.can_paste(), egui::Button::new("Paste").shortcut_text("Ctrl+V"))
                .clicked()
            {
                ann.paste(tex_size);
                ui.close_menu();
            }
            if ui
                .add_enabled(has_shapes, egui::Button::new("Select All").shortcut_text("Ctrl+A"))
                .clicked()
            {
                ann.select_all();
                ui.close_menu();
            }
            if ui
                .add_enabled(has_selection, egui::Button::new("Duplicate").shortcut_text("Ctrl+D"))
                .clicked()
            {
                ann.duplicate_selected();
                ui.close_menu();
            }
            if ui
                .add_enabled(has_selection, egui::Button::new("Bring Forward"))
                .clicked()
            {
                ann.reorder_selected(true);
                ui.close_menu();
            }
            if ui
                .add_enabled(has_selection, egui::Button::new("Send Backward"))
                .clicked()
            {
                ann.reorder_selected(false);
                ui.close_menu();
            }
            if ui
                .add_enabled(has_selection, egui::Button::new("Delete Selected"))
                .clicked()
            {
                ann.delete_selected();
                ui.close_menu();
            }
            if ui.add_enabled(has_shapes, egui::Button::new("Clear")).clicked() {
                ann.clear_objects();
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Exit Draw Mode").clicked() {
                app.editor_state.annotate.active = false;
                app.editor_state.annotate.draft = None;
                app.editor_state.annotate.selected.clear();
                ui.close_menu();
            }
        });
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
                egui::Stroke::new(1.5_f32, colors.accent),
                egui::StrokeKind::Inside,
            );
        }
    }

    // Annotation overlay (FastStone-style vector objects).
    {
        let ann = &app.editor_state.annotate;
        if ann.active || !ann.shapes.is_empty() {
            let to_screen = |p: egui::Pos2| image_to_screen_pos(p, draw_rect, tex_size);
            crate::annotate::render::paint_shapes(
                ui.painter(),
                &ann.shapes,
                &ann.selected,
                app.viewer_state.zoom,
                to_screen,
            );
            if let Some((a, b)) = ann.marquee {
                crate::annotate::render::paint_marquee(
                    ui.painter(),
                    egui::Rect::from_two_pos(to_screen(a), to_screen(b)),
                );
            }
            if let Some(d) = &ann.draft {
                crate::annotate::render::paint_draft(
                    ui.painter(),
                    d,
                    ann.style,
                    app.viewer_state.zoom,
                    to_screen,
                );
            }
        }
    }

    // Styled info overlay
    if app.viewer_state.show_info {
        let info_text = format!(
            "{}x{}\nZoom: {:.0}%\n{}",
            tex_size.x as u32,
            tex_size.y as u32,
            zoom_percent(app.viewer_state.zoom, ui.ctx().pixels_per_point()),
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

// Distance one arrow key press moves the selection, in image px.
fn nudge_delta(key: egui::Key, shift: bool) -> Option<Vec2> {
    let step = if shift { 10.0 } else { 1.0 };
    match key {
        egui::Key::ArrowLeft => Some(Vec2::new(-step, 0.0)),
        egui::Key::ArrowRight => Some(Vec2::new(step, 0.0)),
        egui::Key::ArrowUp => Some(Vec2::new(0.0, -step)),
        egui::Key::ArrowDown => Some(Vec2::new(0.0, step)),
        _ => None,
    }
}

// Ctrl+C/X/V reach egui as Copy/Cut/Paste events instead of Key events.
fn handle_annotate_clipboard(
    app: &mut App,
    ctx: &egui::Context,
    draw_rect: egui::Rect,
    tex_size: Vec2,
    mouse_pos: Option<egui::Pos2>,
) {
    let ann = &mut app.editor_state.annotate;
    if !ann.active || ctx.wants_keyboard_input() {
        return;
    }
    // Collected first: `copy_text` must not run while `ctx.input` holds its lock.
    let events: Vec<egui::Event> = ctx.input(|i| {
        i.events
            .iter()
            .filter(|e| matches!(e, egui::Event::Copy | egui::Event::Cut | egui::Event::Paste(_)))
            .cloned()
            .collect()
    });
    for event in events {
        match event {
            egui::Event::Copy => crate::annotate::copy_to_clipboard(ann, ctx),
            egui::Event::Cut => crate::annotate::cut_to_clipboard(ann, ctx),
            egui::Event::Paste(text) if text == crate::annotate::CLIPBOARD_MARKER => {
                ann.paste(tex_size);
            }
            egui::Event::Paste(text) => {
                let at = mouse_pos
                    .filter(|p| draw_rect.contains(*p))
                    .map_or((tex_size / 2.0).to_pos2(), |p| {
                        screen_to_image_pos(p, draw_rect, tex_size)
                    });
                ann.paste_text(&text, at, tex_size);
            }
            _ => {}
        }
    }
}

fn screen_delta_to_image(delta: egui::Vec2, zoom: f32) -> egui::Vec2 {
    delta / zoom.max(1e-3)
}

fn handle_annotate_drag(
    app: &mut App,
    drag: &egui::Response,
    draw_rect: egui::Rect,
    tex_size: Vec2,
    mouse_pos: Option<egui::Pos2>,
    zoom: f32,
) {
    use crate::annotate::{commit_draft, place_stamp, Draft, DragMode, Tool};
    // Only the left button draws; middle pans and right opens the menu.
    let primary = drag.ctx.input(|i| {
        i.pointer.button_down(egui::PointerButton::Primary)
            || i.pointer.button_released(egui::PointerButton::Primary)
    });
    if !primary {
        return;
    }
    // A drag only registers after the pointer moved a few px, so anchor it to the press.
    let press_pos = drag
        .ctx
        .input(|i| i.pointer.press_origin())
        .or(mouse_pos)
        .map(|p| screen_to_image_pos(p, draw_rect, tex_size));
    let hover_pos = mouse_pos.map(|p| screen_to_image_pos(p, draw_rect, tex_size));
    let tolerance = 8.0 / zoom.max(1e-3);
    let additive = drag.ctx.input(|i| i.modifiers.shift);
    let ann = &mut app.editor_state.annotate;
    // The selected object's resize handles work with every tool, so a freshly drawn
    // line can be adjusted without switching to Select first.
    if drag.drag_started() {
        ann.drag_snapped = false;
        ann.drag_mode = None;
        if let Some(pos) = press_pos {
            let handle = ann.single_selected().and_then(|i| ann.shapes.get(i)).and_then(|s| {
                s.handles()
                    .iter()
                    .position(|h| (*h - pos).length() <= tolerance)
            });
            if let Some(id) = handle {
                ann.drag_mode = Some(DragMode::Handle(id));
            } else if drag.ctx.input(|i| i.modifiers.ctrl) {
                // Ctrl+drag grabs and moves the object under the pointer with any tool.
                let hit = ann.shapes.iter().rposition(|s| s.hit_test(pos, tolerance));
                if let Some(i) = hit {
                    ann.grab(i, additive);
                    ann.drag_mode = Some(DragMode::Move);
                }
            }
            if ann.drag_mode.is_some() {
                ann.draft = None;
            }
        }
    }
    if let Some(mode) = ann.drag_mode {
        if drag.dragged() {
            let delta = screen_delta_to_image(drag.drag_delta(), zoom);
            let changes = !ann.selected.is_empty()
                && match mode {
                    DragMode::Move => delta != Vec2::ZERO,
                    DragMode::Handle(_) => hover_pos.is_some(),
                };
            if changes {
                // One undo step per gesture, taken right before the first change.
                if !ann.drag_snapped {
                    ann.snapshot();
                    ann.drag_snapped = true;
                }
                match (mode, hover_pos, ann.single_selected()) {
                    (DragMode::Move, _, _) => ann.translate_selected(delta),
                    (DragMode::Handle(id), Some(pos), Some(i)) => {
                        ann.shapes[i].drag_handle(id, pos)
                    }
                    _ => {}
                }
            }
        }
        if drag.drag_stopped() {
            ann.drag_mode = None;
        }
        return;
    }
    if ann.marquee.is_some() {
        if drag.dragged() {
            if let (Some((_, current)), Some(pos)) = (ann.marquee.as_mut(), hover_pos) {
                *current = pos;
            }
        }
        if drag.drag_stopped() {
            ann.finish_marquee();
        }
        return;
    }
    // Plain click on an existing object selects it, whatever drawing tool is active.
    if drag.clicked() && ann.tool != Tool::Select && !ann.tool.is_stamp() {
        if let Some(pos) = hover_pos {
            let hit = ann.shapes.iter().rposition(|s| s.hit_test(pos, tolerance));
            // Objects win over detection, so existing ones stay clickable.
            if hit.is_none()
                && !additive
                && ann.tool.takes_region()
                && app.editor_state.auto_region.enabled
            {
                let params = app.config.auto_region;
                let region = crate::editor::auto_region_at(&mut app.editor_state, &params, pos);
                let ann = &mut app.editor_state.annotate;
                ann.select(None);
                if let Some(r) = region {
                    ann.draft = Some(Draft {
                        tool: ann.tool,
                        start: r.min,
                        current: r.max,
                        points: Vec::new(),
                    });
                    commit_draft(ann);
                }
                return;
            }
            ann.click_select(hit, additive);
        }
        return;
    }
    if ann.tool == Tool::Select {
        // Handles were already tried above; a press here grabs the object body.
        if drag.drag_started() {
            if let Some(pos) = press_pos {
                let hit = ann.shapes.iter().rposition(|s| s.hit_test(pos, tolerance));
                if let Some(i) = hit {
                    ann.grab(i, additive);
                    ann.drag_mode = Some(DragMode::Move);
                } else {
                    // Empty canvas: box-select. Shift keeps the current selection.
                    if !additive {
                        ann.select(None);
                    }
                    ann.marquee = Some((pos, pos));
                }
            }
        }
        if drag.clicked() {
            let hit = hover_pos
                .and_then(|pos| ann.shapes.iter().rposition(|s| s.hit_test(pos, tolerance)));
            ann.click_select(hit, additive);
        }
        return;
    }
    if ann.tool.is_stamp() && drag.clicked() {
        if let Some(pos) = hover_pos {
            place_stamp(ann, pos);
        }
        return;
    }
    if drag.drag_started() {
        if let Some(img_pos) = press_pos {
            ann.draft = Some(Draft {
                tool: ann.tool,
                start: img_pos,
                current: img_pos,
                points: vec![img_pos],
            });
        }
    } else if drag.dragged() {
        if let (Some(d), Some(pos)) = (ann.draft.as_mut(), mouse_pos) {
            let img_pos = screen_to_image_pos(pos, draw_rect, tex_size);
            d.current = img_pos;
            if d.tool == Tool::Pencil {
                let far_enough = d
                    .points
                    .last()
                    .map_or(true, |last| (*last - img_pos).length() >= 1.0);
                if far_enough {
                    d.points.push(img_pos);
                }
            }
        }
    }
    if drag.drag_stopped() {
        commit_draft(ann);
    }
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

// `zoom` is points per texel, because egui lays out in points. The user-facing
// percentage is physical pixels per texel, so 100% is exact at any display scale.
fn one_to_one_zoom(pixels_per_point: f32) -> f32 {
    1.0 / pixels_per_point
}

fn zoom_percent(zoom: f32, pixels_per_point: f32) -> f32 {
    zoom * pixels_per_point * 100.0
}

// Upscaling by a whole number of physical pixels keeps every texel the same size;
// with nearest filtering a scale like 109% would double some rows and cause moire.
fn snap_fit_zoom(zoom: f32, pixels_per_point: f32) -> f32 {
    let physical_scale = zoom * pixels_per_point;
    if physical_scale > 1.0 {
        (physical_scale + 1e-4).floor() / pixels_per_point
    } else {
        zoom
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_delta_converts_by_zoom() {
        assert_eq!(
            screen_delta_to_image(egui::vec2(10.0, -6.0), 2.0),
            egui::vec2(5.0, -3.0)
        );
    }

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
    fn test_one_to_one_maps_each_texel_to_one_physical_pixel() {
        for pixels_per_point in [1.0, 1.25, 1.5, 2.0] {
            let zoom = one_to_one_zoom(pixels_per_point);
            let physical = display_size(Vec2::new(800.0, 800.0), zoom) * pixels_per_point;
            assert!((physical.x - 800.0).abs() < 0.001);
            assert!((zoom_percent(zoom, pixels_per_point) - 100.0).abs() < 0.001);
        }
    }

    #[test]
    fn test_fit_snaps_upscales_to_whole_physical_pixels() {
        let physical = |zoom: f32, ppp: f32| snap_fit_zoom(zoom, ppp) * ppp;
        assert!((physical(1.0, 1.25) - 1.0).abs() < 0.001); // 125% -> 100%
        assert!((physical(1.6, 1.25) - 2.0).abs() < 0.001); // exactly 200%
        assert!((physical(2.3, 1.0) - 2.0).abs() < 0.001);
        // Shrinking is left alone.
        assert_eq!(snap_fit_zoom(0.25, 1.25), 0.25);
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
