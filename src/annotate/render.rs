// Annotation rendering: egui overlay painting + CPU burn.

use super::shapes::{Shape, Style};
use eframe::egui;

// Two wing tips of an arrowhead: `dir` is the unit flight direction.
pub fn arrow_head_points(tip: egui::Pos2, dir: egui::Vec2, size: f32) -> [egui::Pos2; 2] {
    let base = tip - dir * size;
    let perp = egui::vec2(-dir.y, dir.x) * size * 0.5;
    [base + perp, base - perp]
}

fn stroke(style: Style, zoom: f32) -> egui::Stroke {
    egui::Stroke::new((style.width * zoom).max(1.0), style.effective_color())
}

// Paint all shapes; `to_screen` maps image px -> screen px.
pub fn paint_shapes(
    painter: &egui::Painter,
    shapes: &[Shape],
    selected: Option<usize>,
    zoom: f32,
    to_screen: impl Fn(egui::Pos2) -> egui::Pos2,
) {
    for (i, shape) in shapes.iter().enumerate() {
        paint_shape(painter, shape, zoom, &to_screen);
        if selected == Some(i) {
            let b = shape.bounding_rect();
            painter.rect_stroke(
                egui::Rect::from_min_max(to_screen(b.min), to_screen(b.max)),
                egui::CornerRadius::ZERO,
                egui::Stroke::new(1.5_f32, egui::Color32::WHITE),
                egui::StrokeKind::Outside,
            );
        }
    }
}

fn paint_shape(
    painter: &egui::Painter,
    shape: &Shape,
    zoom: f32,
    to_screen: &impl Fn(egui::Pos2) -> egui::Pos2,
) {
    match shape {
        Shape::Line { a, b, style } => {
            painter.line_segment([to_screen(*a), to_screen(*b)], stroke(*style, zoom));
        }
        Shape::Arrow { a, b, style } => {
            let (sa, sb) = (to_screen(*a), to_screen(*b));
            painter.line_segment([sa, sb], stroke(*style, zoom));
            let delta = *b - *a;
            let dir = if delta.length_sq() > 1e-6 {
                delta.normalized()
            } else {
                egui::vec2(1.0, 0.0)
            };
            let size = (style.width * 4.0 + 6.0) * zoom;
            let [w1, w2] = arrow_head_points(sb, dir, size);
            painter.line_segment([sb, w1], stroke(*style, zoom));
            painter.line_segment([sb, w2], stroke(*style, zoom));
        }
        Shape::Pencil { points, style } => {
            let pts: Vec<egui::Pos2> = points.iter().map(|p| to_screen(*p)).collect();
            painter.add(egui::Shape::line(pts, stroke(*style, zoom)));
        }
        Shape::Rect { rect, style } => {
            let r = egui::Rect::from_min_max(to_screen(rect.min), to_screen(rect.max));
            if style.filled {
                painter.rect_filled(r, egui::CornerRadius::ZERO, style.effective_color());
            } else {
                painter.rect_stroke(
                    r,
                    egui::CornerRadius::ZERO,
                    stroke(*style, zoom),
                    egui::StrokeKind::Inside,
                );
            }
        }
        Shape::Ellipse { rect, style } => {
            let r = egui::Rect::from_min_max(to_screen(rect.min), to_screen(rect.max));
            if style.filled {
                painter.add(egui::Shape::ellipse_filled(
                    r.center(),
                    r.size() / 2.0,
                    style.effective_color(),
                ));
            } else {
                painter.add(egui::Shape::ellipse_stroke(
                    r.center(),
                    r.size() / 2.0,
                    stroke(*style, zoom),
                ));
            }
        }
        Shape::Highlight { rect, style } => {
            let r = egui::Rect::from_min_max(to_screen(rect.min), to_screen(rect.max));
            let mut hl = *style;
            hl.opacity = (hl.opacity * 0.4_f32).clamp(0.05_f32, 0.6_f32);
            painter.rect_filled(r, egui::CornerRadius::ZERO, hl.effective_color());
        }
        Shape::Text { at, text, style } => {
            painter.text(
                to_screen(*at),
                egui::Align2::LEFT_TOP,
                text,
                egui::FontId::proportional((style.font_size * zoom).max(4.0)),
                style.effective_color(),
            );
        }
        Shape::BlurRegion { rect, .. } => {
            let r = egui::Rect::from_min_max(to_screen(rect.min), to_screen(rect.max));
            painter.rect_filled(
                r,
                egui::CornerRadius::ZERO,
                egui::Color32::from_black_alpha(80),
            );
            painter.rect_stroke(
                r,
                egui::CornerRadius::ZERO,
                egui::Stroke::new(1.0_f32, egui::Color32::WHITE),
                egui::StrokeKind::Outside,
            );
        }
    }
}

// Paint the in-progress drag as a preview shape.
pub fn paint_draft(
    painter: &egui::Painter,
    draft: &super::Draft,
    style: Style,
    zoom: f32,
    to_screen: impl Fn(egui::Pos2) -> egui::Pos2,
) {
    let preview = match draft.tool {
        super::Tool::Select => return,
        super::Tool::Line => Shape::Line {
            a: draft.start,
            b: draft.current,
            style,
        },
        super::Tool::Arrow => Shape::Arrow {
            a: draft.start,
            b: draft.current,
            style,
        },
        super::Tool::Rect => Shape::Rect {
            rect: super::shapes::ImgRect::from_points(draft.start, draft.current),
            style,
        },
        super::Tool::Ellipse => Shape::Ellipse {
            rect: super::shapes::ImgRect::from_points(draft.start, draft.current),
            style,
        },
        super::Tool::Pencil => Shape::Pencil {
            points: draft.points.clone(),
            style,
        },
        super::Tool::Highlight => Shape::Highlight {
            rect: super::shapes::ImgRect::from_points(draft.start, draft.current),
            style,
        },
        super::Tool::BlurRegion => Shape::BlurRegion {
            rect: super::shapes::ImgRect::from_points(draft.start, draft.current),
            kind: super::shapes::BlurKind::Blur,
        },
        // Text previews via the panel input, not drag.
        super::Tool::Text => return,
    };
    paint_shape(painter, &preview, zoom, &to_screen);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrowhead_wings_sit_behind_tip() {
        let wings = arrow_head_points(egui::pos2(10.0, 0.0), egui::vec2(1.0, 0.0), 6.0);
        assert_eq!(wings, [egui::pos2(4.0, 3.0), egui::pos2(4.0, -3.0)]);
    }
}
