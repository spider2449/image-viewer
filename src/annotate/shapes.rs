use eframe::egui;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImgRect {
    pub min: egui::Pos2,
    pub max: egui::Pos2,
}

impl ImgRect {
    // Build from any two corners; result is always normalized.
    pub fn from_points(a: egui::Pos2, b: egui::Pos2) -> Self {
        Self {
            min: egui::pos2(a.x.min(b.x), a.y.min(b.y)),
            max: egui::pos2(a.x.max(b.x), a.y.max(b.y)),
        }
    }

    pub fn normalize(mut self) -> Self {
        let (lox, hix) = (
            self.min.x.min(self.max.x),
            self.min.x.max(self.max.x),
        );
        let (loy, hiy) = (
            self.min.y.min(self.max.y),
            self.min.y.max(self.max.y),
        );
        self.min = egui::pos2(lox, loy);
        self.max = egui::pos2(hix, hiy);
        self
    }

    // Clamp inside a (width, height) image; keeps at least 1px where possible.
    pub fn clamp(mut self, w: f32, h: f32) -> Self {
        self.min.x = self.min.x.clamp(0.0, w);
        self.min.y = self.min.y.clamp(0.0, h);
        self.max.x = self.max.x.clamp(0.0, w);
        self.max.y = self.max.y.clamp(0.0, h);
        self.normalize()
    }

    pub fn width(self) -> f32 {
        (self.max.x - self.min.x).max(0.0)
    }

    pub fn height(self) -> f32 {
        (self.max.y - self.min.y).max(0.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    pub color: egui::Color32,
    pub width: f32,     // stroke width in image px
    pub filled: bool,   // rect / ellipse fill
    pub opacity: f32,   // 0.0..=1.0
    pub font_size: f32, // image px
}

impl Default for Style {
    fn default() -> Self {
        Self {
            color: egui::Color32::RED,
            width: 3.0,
            filled: false,
            opacity: 1.0,
            font_size: 24.0,
        }
    }
}

impl Style {
    // Color with opacity applied to the alpha channel.
    pub fn effective_color(self) -> egui::Color32 {
        let a = (self.color.a() as f32 * self.opacity.clamp(0.0, 1.0)) as u8;
        egui::Color32::from_rgba_premultiplied(self.color.r(), self.color.g(), self.color.b(), a)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlurKind {
    Blur,
    Mosaic,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    Line {
        a: egui::Pos2,
        b: egui::Pos2,
        style: Style,
    },
    Arrow {
        a: egui::Pos2,
        b: egui::Pos2,
        style: Style,
    },
    Pencil {
        points: Vec<egui::Pos2>,
        style: Style,
    },
    Rect {
        rect: ImgRect,
        style: Style,
    },
    Ellipse {
        rect: ImgRect,
        style: Style,
    },
    Highlight {
        rect: ImgRect,
        style: Style,
    },
    Text {
        at: egui::Pos2,
        text: String,
        style: Style,
    },
    BlurRegion {
        rect: ImgRect,
        kind: BlurKind,
    },
}

// Approximate text bounds for hit-testing (monospace estimate).
pub fn text_bounds(at: egui::Pos2, text: &str, font_size: f32) -> ImgRect {
    let lines = text.lines().count().max(1) as f32;
    let widest = text
        .lines()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0) as f32;
    ImgRect {
        min: at,
        max: egui::pos2(
            at.x + widest * font_size * 0.6,
            at.y + lines * font_size * 1.2,
        ),
    }
}

fn dist_point_segment(p: egui::Pos2, a: egui::Pos2, b: egui::Pos2) -> f32 {
    let ab = b - a;
    let denom = ab.length_sq().max(1e-6);
    let t = ((p - a).dot(ab) / denom).clamp(0.0, 1.0);
    (a + ab * t - p).length()
}

impl Shape {
    pub fn bounding_rect(&self) -> ImgRect {
        match self {
            Shape::Line { a, b, .. } | Shape::Arrow { a, b, .. } => {
                ImgRect::from_points(*a, *b)
            }
            // Invariant: Pencil always holds 2+ points (see draft_ready).
            Shape::Pencil { points, .. } => {
                let mut r = ImgRect::from_points(points[0], points[0]);
                for p in &points[1..] {
                    r.min.x = r.min.x.min(p.x);
                    r.min.y = r.min.y.min(p.y);
                    r.max.x = r.max.x.max(p.x);
                    r.max.y = r.max.y.max(p.y);
                }
                r
            }
            Shape::Rect { rect, .. }
            | Shape::Ellipse { rect, .. }
            | Shape::Highlight { rect, .. }
            | Shape::BlurRegion { rect, .. } => *rect,
            Shape::Text { at, text, style } => text_bounds(*at, text, style.font_size),
        }
    }

    pub fn translate(&mut self, delta: egui::Vec2) {
        let mv = |p: &mut egui::Pos2| *p += delta;
        match self {
            Shape::Line { a, b, .. } | Shape::Arrow { a, b, .. } => {
                mv(a);
                mv(b);
            }
            Shape::Pencil { points, .. } => points.iter_mut().for_each(mv),
            Shape::Rect { rect, .. }
            | Shape::Ellipse { rect, .. }
            | Shape::Highlight { rect, .. }
            | Shape::BlurRegion { rect, .. } => {
                rect.min += delta;
                rect.max += delta;
            }
            Shape::Text { at, .. } => mv(at),
        }
    }

    // Selection hit test in image-pixel space; tolerance is in image px.
    pub fn hit_test(&self, pos: egui::Pos2, tolerance: f32) -> bool {
        match self {
            Shape::Line { a, b, .. } | Shape::Arrow { a, b, .. } => {
                dist_point_segment(pos, *a, *b) <= tolerance
            }
            Shape::Pencil { points, .. } => points
                .windows(2)
                .any(|w| dist_point_segment(pos, w[0], w[1]) <= tolerance),
            Shape::Rect { rect, .. }
            | Shape::Highlight { rect, .. }
            | Shape::BlurRegion { rect, .. } => {
                pos.x >= rect.min.x - tolerance
                    && pos.x <= rect.max.x + tolerance
                    && pos.y >= rect.min.y - tolerance
                    && pos.y <= rect.max.y + tolerance
            }
            Shape::Ellipse { rect, .. } => {
                let c = egui::pos2(
                    (rect.min.x + rect.max.x) / 2.0,
                    (rect.min.y + rect.max.y) / 2.0,
                );
                let rx = (rect.width() / 2.0 + tolerance).max(1e-3);
                let ry = (rect.height() / 2.0 + tolerance).max(1e-3);
                let dx = (pos.x - c.x) / rx;
                let dy = (pos.y - c.y) / ry;
                dx * dx + dy * dy <= 1.0
            }
            Shape::Text { at, text, style } => {
                let b = text_bounds(*at, text, style.font_size);
                pos.x >= b.min.x - tolerance
                    && pos.x <= b.max.x + tolerance
                    && pos.y >= b.min.y - tolerance
                    && pos.y <= b.max.y + tolerance
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_from_reversed_points_normalizes() {
        let r = ImgRect::from_points(egui::pos2(50.0, 50.0), egui::pos2(10.0, 20.0));
        assert_eq!(r.min, egui::pos2(10.0, 20.0));
        assert_eq!(r.max, egui::pos2(50.0, 50.0));
    }

    #[test]
    fn clamp_keeps_rect_inside_image() {
        let r = ImgRect {
            min: egui::pos2(-5.0, 10.0),
            max: egui::pos2(500.0, 60.0),
        };
        let c = r.clamp(100.0, 80.0);
        assert_eq!(
            (c.min.x, c.min.y, c.max.x, c.max.y),
            (0.0, 10.0, 100.0, 60.0)
        );
    }

    #[test]
    fn line_hit_test_uses_tolerance() {
        let s = Shape::Line {
            a: egui::pos2(0.0, 0.0),
            b: egui::pos2(100.0, 0.0),
            style: Style::default(),
        };
        assert!(s.hit_test(egui::pos2(50.0, 3.0), 5.0));
        assert!(!s.hit_test(egui::pos2(50.0, 30.0), 5.0));
    }

    #[test]
    fn translate_moves_rect_shape() {
        let mut s = Shape::Rect {
            rect: ImgRect {
                min: egui::pos2(10.0, 10.0),
                max: egui::pos2(20.0, 20.0),
            },
            style: Style::default(),
        };
        s.translate(egui::vec2(5.0, -5.0));
        assert_eq!(s.bounding_rect().min, egui::pos2(15.0, 5.0));
    }
}
