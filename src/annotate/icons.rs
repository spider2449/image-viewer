// Small vector icons for the annotation tool buttons, drawn with the egui painter
// so they follow the theme and need no font glyphs or image assets.

use super::Tool;
use eframe::egui;

// A square toggle button showing the tool's icon; `label` becomes the tooltip.
pub fn tool_button(ui: &mut egui::Ui, tool: Tool, selected: bool, label: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(32.0, 28.0), egui::Sense::click());
    let visuals = ui.visuals();
    let bg = if selected {
        visuals.selection.bg_fill
    } else if response.hovered() {
        visuals.widgets.hovered.bg_fill
    } else {
        visuals.widgets.inactive.bg_fill
    };
    let ink = if selected {
        visuals.selection.stroke.color
    } else {
        visuals.text_color()
    };
    let painter = ui.painter();
    painter.rect_filled(rect, egui::CornerRadius::same(4), bg);
    draw_icon(painter, rect.shrink2(egui::vec2(8.0, 6.0)), tool, ink);
    response.on_hover_text(label)
}

fn draw_icon(p: &egui::Painter, r: egui::Rect, tool: Tool, ink: egui::Color32) {
    let st = egui::Stroke::new(1.6_f32, ink);
    let at = |fx: f32, fy: f32| egui::pos2(r.min.x + r.width() * fx, r.min.y + r.height() * fy);
    match tool {
        Tool::Select => {
            p.add(egui::Shape::convex_polygon(
                vec![at(0.1, 0.0), at(0.1, 0.95), at(0.4, 0.7), at(1.0, 0.75)],
                ink,
                egui::Stroke::NONE,
            ));
        }
        Tool::Pencil => {
            let pts: Vec<egui::Pos2> = (0..=12)
                .map(|i| {
                    let t = i as f32 / 12.0;
                    at(t, 0.5 + 0.35 * (t * std::f32::consts::TAU).sin())
                })
                .collect();
            p.add(egui::Shape::line(pts, st));
        }
        Tool::Line => {
            p.line_segment([at(0.0, 1.0), at(1.0, 0.0)], st);
        }
        Tool::Arrow => {
            p.line_segment([at(0.0, 1.0), at(0.85, 0.15)], st);
            p.add(egui::Shape::convex_polygon(
                vec![at(1.0, 0.0), at(0.55, 0.12), at(0.88, 0.45)],
                ink,
                egui::Stroke::NONE,
            ));
        }
        Tool::Rect => {
            p.rect_stroke(r.shrink(1.0), egui::CornerRadius::ZERO, st, egui::StrokeKind::Middle);
        }
        Tool::Ellipse => {
            p.add(egui::Shape::ellipse_stroke(
                r.center(),
                egui::vec2(r.width() / 2.0 - 1.0, r.height() / 2.0 - 1.0),
                st,
            ));
        }
        Tool::Highlight => {
            p.rect_filled(
                r.shrink2(egui::vec2(0.0, 2.0)),
                egui::CornerRadius::ZERO,
                egui::Color32::from_rgba_unmultiplied(255, 225, 0, 200),
            );
            p.line_segment([at(0.1, 0.5), at(0.9, 0.5)], egui::Stroke::new(1.2_f32, ink));
        }
        Tool::Text => {
            p.text(
                r.center(),
                egui::Align2::CENTER_CENTER,
                "T",
                egui::FontId::proportional(r.height() + 2.0),
                ink,
            );
        }
        Tool::Callout => {
            let body = egui::Rect::from_min_max(at(0.0, 0.0), at(1.0, 0.7));
            p.rect_stroke(body, egui::CornerRadius::same(3), st, egui::StrokeKind::Middle);
            p.add(egui::Shape::convex_polygon(
                vec![at(0.2, 0.68), at(0.5, 0.68), at(0.15, 1.0)],
                ink,
                egui::Stroke::NONE,
            ));
        }
        Tool::Step => {
            p.circle_stroke(r.center(), r.height() / 2.0, st);
            p.text(
                r.center(),
                egui::Align2::CENTER_CENTER,
                "1",
                egui::FontId::proportional(r.height() * 0.7),
                ink,
            );
        }
        Tool::BlurRegion => {
            let cell = egui::vec2(r.width() / 3.0, r.height() / 3.0);
            for row in 0..3 {
                for col in 0..3 {
                    if (row + col) % 2 == 0 {
                        let min = r.min + egui::vec2(cell.x * col as f32, cell.y * row as f32);
                        p.rect_filled(
                            egui::Rect::from_min_size(min, cell),
                            egui::CornerRadius::ZERO,
                            ink,
                        );
                    }
                }
            }
        }
    }
}
