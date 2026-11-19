use super::Tool;
use crate::icons::Icon;
use eframe::egui;

pub fn tool_button(ui: &mut egui::Ui, tool: Tool, selected: bool, label: &str) -> egui::Response {
    let icon = match tool {
        Tool::Select => Icon::Select,
        Tool::Pencil => Icon::Pencil,
        Tool::Line => Icon::Line,
        Tool::Arrow => Icon::Arrow,
        Tool::Rect => Icon::Rect,
        Tool::Ellipse => Icon::Ellipse,
        Tool::Highlight => Icon::Highlight,
        Tool::Text => Icon::Text,
        Tool::Callout => Icon::Callout,
        Tool::Step => Icon::Step,
        Tool::BlurRegion => Icon::BlurRegion,
    };
    ui.add(
        egui::Button::image(icon.image(ui, 20.0))
            .min_size(egui::vec2(32.0, 28.0))
            .selected(selected),
    )
    .on_hover_text(label)
}
