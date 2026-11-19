use crate::icons::Icon;
use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Section {
    Adjust,
    Geometry,
    Selection,
    Annotate,
    Export,
}

impl State {
    fn pause_tools(&mut self) {
        self.crop_active = false;
        self.crop_drag = None;
        self.wand.active = false;
        self.annotate.active = false;
        self.annotate.draft = None;
        self.annotate.drag_mode = None;
        self.annotate.marquee = None;
    }
}

pub(super) fn show(app: &mut App, ctx: &egui::Context, colors: crate::theme::ThemeColors) -> bool {
    let mut preview_changed = false;
    egui::SidePanel::right("editor_panel")
        .resizable(true)
        .default_width(320.0)
        .min_width(280.0)
        .frame(egui::Frame {
            fill: colors.panel_bg,
            inner_margin: egui::Margin::symmetric(12, 10),
            ..Default::default()
        })
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Edit image");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Close").on_hover_text("Keep edits and return to viewing").clicked() {
                        close(app, ctx);
                    }
                });
            });
            if let Some(image) = &app.editor_state.current_image {
                ui.small(format!("{} × {} px", image.width(), image.height()));
            }
            ui.horizontal_wrapped(|ui| {
                let state = &app.editor_state;
                let objects = state.annotate.can_undo() || state.annotate.can_redo();
                let can_undo = (objects && state.annotate.can_undo())
                    || !state.adjustments.is_neutral() || state.can_undo();
                let can_redo = (objects && state.annotate.can_redo())
                    || (state.adjustments.is_neutral() && state.can_redo());
                let changed = state.has_unsaved_changes();
                if ui.add_enabled(can_undo, Icon::Undo.button(ui, "Undo")).clicked() {
                    history_action(app, ctx, false);
                }
                if ui.add_enabled(can_redo, Icon::Redo.button(ui, "Redo")).clicked() {
                    history_action(app, ctx, true);
                }
                ui.menu_button("More", |ui| {
                    if ui.add_enabled(changed, egui::Button::new("Revert all edits")).clicked() {
                        revert_all(app, ctx);
                        ui.close_menu();
                    }
                    if ui.button("Paste image from clipboard").clicked() {
                        paste_from_clipboard(app, ctx);
                        ui.close_menu();
                    }
                });
            });
            ui.separator();
            let previous = app.editor_state.section;
            ui.horizontal_wrapped(|ui| {
                for (section, icon, label) in [
                    (Section::Adjust, Icon::Adjust, "Adjust"),
                    (Section::Geometry, Icon::Crop, "Geometry"),
                    (Section::Selection, Icon::Wand, "Selection"),
                    (Section::Annotate, Icon::Pencil, "Annotate"),
                    (Section::Export, Icon::Save, "Export"),
                ] {
                    if ui.add(icon.button(ui, label).selected(app.editor_state.section == section)).clicked() {
                        app.editor_state.section = section;
                    }
                }
            });
            if previous != app.editor_state.section {
                app.editor_state.pause_tools();
            }
            ui.separator();

            // Keep output and pending work reachable while the inspector scrolls.
            egui::TopBottomPanel::bottom("editor_actions")
                .frame(egui::Frame::NONE)
                .show_inside(ui, |ui| {
                    ui.separator();
                    if let Some(error) = &app.editor_state.save_error {
                        ui.colored_label(ui.visuals().error_fg_color, error);
                    }
                    if !app.editor_state.adjustments.is_neutral() {
                        ui.label("Color preview · not applied yet");
                        ui.horizontal(|ui| {
                            if ui.button("Apply colors").clicked() && app.editor_state.commit_adjustments() {
                                sync_editor_image(app, ctx);
                            }
                            if ui.button("Cancel preview").clicked() {
                                reset_adjustment_preview(app, ctx);
                            }
                        });
                    }
                    let state = &app.editor_state;
                    ui.small(format!("{} edit(s) · {} annotation object(s){}",
                        state.history_index, state.annotate.shapes.len(),
                        if state.has_unsaved_changes() { " · unsaved" } else { " · saved" }));
                    ui.horizontal(|ui| {
                        if ui.button("Save").on_hover_text("Save changes to the opened file").clicked() {
                            request_save(app);
                        }
                        if ui.button("Export…").clicked() {
                            app.editor_state.pause_tools();
                            app.editor_state.section = Section::Export;
                        }
                    });
                });

            egui::ScrollArea::vertical()
                .id_salt(("editor_inspector", app.editor_state.section as u8))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add_enabled_ui(app.editor_state.current_image.is_some(), |ui| {
                        match app.editor_state.section {
                            Section::Adjust => {
                                ui.heading("Light & color");
                                ui.small("Preview live. Apply to keep, or cancel to reset.");
                                selection_scope_hint(app, ui);
                                preview_changed |= adjustment_controls(ui, &mut app.editor_state.adjustments);
                                ui.separator();
                                ui.heading("Effects");
                                ui.small("Effects apply immediately and can be undone.");
                                effects(app, ctx, ui);
                            }
                            Section::Geometry => {
                                ui.heading("Crop & orientation");
                                transform_controls(app, ctx, ui);
                                ui.separator();
                                ui.heading("Image size");
                                resize_controls(app, ctx, ui, colors.text_secondary);
                            }
                            Section::Selection => {
                                ui.heading("Select a region");
                                ui.small("Select pixels here, then adjust their color or add an annotation.");
                                wand_controls(app, ctx, ui);
                            }
                            Section::Annotate => {
                                ui.heading("Draw & annotate");
                                ui.small("Objects stay editable until applied or saved.");
                                annotate_controls(app, ctx, ui);
                            }
                            Section::Export => {
                                ui.heading("Save & export");
                                ui.small("Saving includes color adjustments and annotation objects. Apply a pending crop first.");
                                ui.add_space(8.0);
                                save_controls(app, ctx, ui);
                            }
                        }
                    });
                });
        });
    preview_changed
}

fn effects(app: &mut App, ctx: &egui::Context, ui: &mut egui::Ui) {
    ui.horizontal_wrapped(|ui| {
        for (label, op) in [
            ("Auto contrast", EditOp::AutoContrast),
            ("Grayscale", EditOp::Grayscale),
            ("Sepia", EditOp::Sepia),
            ("Invert", EditOp::Invert),
        ] {
            if ui.button(label).clicked() {
                apply_op(app, ctx, op);
            }
        }
    });
    ui.collapsing("Blur & sharpen", |ui| {
        ui.add(egui::Slider::new(&mut app.editor_state.blur_sigma, 0.1..=20.0).text("Blur radius"));
        if ui.button("Apply blur").clicked() {
            apply_op(app, ctx, EditOp::Blur(app.editor_state.blur_sigma));
        }
        ui.add(
            egui::Slider::new(&mut app.editor_state.sharpen_sigma, 0.1..=10.0)
                .text("Sharpen radius"),
        );
        ui.add(
            egui::Slider::new(&mut app.editor_state.sharpen_threshold, 0..=50).text("Threshold"),
        );
        if ui.button("Apply sharpen").clicked() {
            apply_op(
                app,
                ctx,
                EditOp::Sharpen {
                    sigma: app.editor_state.sharpen_sigma,
                    threshold: app.editor_state.sharpen_threshold,
                },
            );
        }
    });
}

pub(super) fn close(app: &mut App, ctx: &egui::Context) {
    if app.editor_state.commit_adjustments() {
        sync_editor_image(app, ctx);
    }
    app.editor_state.pause_tools();
    app.editor_state.visible = false;
}
