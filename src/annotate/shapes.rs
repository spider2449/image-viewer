use crate::selection::PixelMask;
use eframe::egui;
use std::sync::Arc;

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

    // Inclusive, so zero-height bounds of a horizontal line still count.
    pub fn intersects(self, other: ImgRect) -> bool {
        self.min.x <= other.max.x
            && other.min.x <= self.max.x
            && self.min.y <= other.max.y
            && other.min.y <= self.max.y
    }

    pub fn union(self, other: ImgRect) -> ImgRect {
        ImgRect {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    pub fn height(self) -> f32 {
        (self.max.y - self.min.y).max(0.0)
    }

    pub fn center(self) -> egui::Pos2 {
        egui::pos2(
            (self.min.x + self.max.x) / 2.0,
            (self.min.y + self.max.y) / 2.0,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    pub color: egui::Color32,
    pub width: f32,     // stroke width in image px
    pub filled: bool,   // rect / ellipse fill
    pub opacity: f32,   // 0.0..=1.0
    pub font_size: f32, // image px
    pub dashed: bool,   // dashed strokes (line, arrow, rect, ellipse, pencil)
    pub shadow: bool,   // soft drop shadow behind the object
    pub fill_color: egui::Color32, // rect / ellipse interior when `filled`
    pub outline: bool,  // rect / ellipse border drawn with `color`
    // Rotation in radians about the shape center (rect / ellipse only). This is
    // per-object: the panel style always keeps 0 so new shapes start unrotated.
    pub angle: f32,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            color: egui::Color32::RED,
            width: 3.0,
            filled: false,
            opacity: 1.0,
            font_size: 24.0,
            dashed: false,
            shadow: false,
            fill_color: egui::Color32::from_rgb(255, 235, 59),
            outline: true,
            angle: 0.0,
        }
    }
}

pub fn rotate_about(p: egui::Pos2, center: egui::Pos2, angle: f32) -> egui::Pos2 {
    let (s, c) = angle.sin_cos();
    let d = p - center;
    center + egui::vec2(d.x * c - d.y * s, d.x * s + d.y * c)
}

// Distance from the top-center resize handle to the rotate handle, in image px.
pub const ROTATE_HANDLE_OFFSET: f32 = 28.0;

impl Style {
    // Copy only the fields that differ between `before` and `after`, so editing one
    // property on a multi-selection leaves each object's other properties alone.
    // Angle is per-object and never comes from the panel.
    pub fn apply_changes(&mut self, before: &Style, after: &Style) {
        macro_rules! merge {
            ($($field:ident),*) => {
                $(if before.$field != after.$field {
                    self.$field = after.$field;
                })*
            };
        }
        merge!(color, width, filled, opacity, font_size, dashed, shadow, fill_color, outline);
    }

    // Interior color with opacity applied.
    pub fn effective_fill(self) -> egui::Color32 {
        let f = self.fill_color;
        let a = (f.a() as f32 * self.opacity.clamp(0.0, 1.0)) as u8;
        egui::Color32::from_rgba_unmultiplied(f.r(), f.g(), f.b(), a)
    }

    // Color with opacity applied to the alpha channel.
    pub fn effective_color(self) -> egui::Color32 {
        let a = (self.color.a() as f32 * self.opacity.clamp(0.0, 1.0)) as u8;
        egui::Color32::from_rgba_unmultiplied(self.color.r(), self.color.g(), self.color.b(), a)
    }

    // Black or white, whichever reads better on top of `color`.
    pub fn on_color(self) -> egui::Color32 {
        let c = self.color;
        let lum = 0.299 * c.r() as f32 + 0.587 * c.g() as f32 + 0.114 * c.b() as f32;
        if lum > 150.0 {
            egui::Color32::BLACK
        } else {
            egui::Color32::WHITE
        }
    }

    // Drop-shadow offset in image px.
    pub fn shadow_offset(self) -> egui::Vec2 {
        let d = (self.width.max(self.font_size * 0.15) * 0.6).clamp(2.0, 12.0);
        egui::vec2(d, d)
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
    // `mask` limits the effect to the selected pixels; it stretches with `rect`.
    Highlight {
        rect: ImgRect,
        style: Style,
        mask: Option<Arc<PixelMask>>,
    },
    Text {
        at: egui::Pos2,
        text: String,
        style: Style,
    },
    BlurRegion {
        rect: ImgRect,
        kind: BlurKind,
        mask: Option<Arc<PixelMask>>,
    },
    // Speech bubble: box anchored at `at` with a user-resizable `size`, plus a tail tip.
    Callout {
        at: egui::Pos2,
        size: egui::Vec2,
        text: String,
        tail: egui::Pos2,
        style: Style,
    },
    // Numbered circle stamp centered at `at`.
    Step {
        at: egui::Pos2,
        number: u32,
        style: Style,
    },
}

// Screen-space radius of a resize handle, in points.
pub const HANDLE_RADIUS_PX: f32 = 5.0;

// Radius of a step-number stamp in image px.
pub fn step_radius(font_size: f32) -> f32 {
    font_size * 0.8
}

// Padding between a callout's box edge and its text, in image px.
pub fn callout_padding(font_size: f32) -> f32 {
    font_size * 0.5
}

// Width of one line in image px: real glyph advances when the burn font is
// available, else an estimate (ASCII ~0.6em, CJK ~1em per glyph).
pub fn line_width(l: &str, font_size: f32) -> f32 {
    super::render::measure_line(l, font_size).unwrap_or_else(|| {
        l.chars()
            .map(|c| if c.is_ascii() { 0.6 } else { 1.0 })
            .sum::<f32>()
            * font_size
    })
}

// Greedy character wrap so no line is wider than `max_w`; explicit newlines are kept.
pub fn wrap_text(text: &str, max_w: f32, font_size: f32) -> String {
    let mut out = String::new();
    for (i, line) in text.lines().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let mut cur = String::new();
        for ch in line.chars() {
            cur.push(ch);
            if cur.chars().count() > 1 && line_width(&cur, font_size) > max_w {
                cur.pop();
                out.push_str(&cur);
                out.push('\n');
                cur.clear();
                cur.push(ch);
            }
        }
        out.push_str(&cur);
    }
    out
}

// Smallest callout box: room for one glyph plus padding.
pub fn callout_min_size(font_size: f32) -> egui::Vec2 {
    egui::Vec2::splat(font_size + callout_padding(font_size) * 2.0)
}

// Callout box from its anchor and size.
pub fn callout_box(at: egui::Pos2, size: egui::Vec2) -> ImgRect {
    ImgRect {
        min: at,
        max: at + size,
    }
}

// Callout text wrapped to the width inside its box.
pub fn callout_wrapped(text: &str, size: egui::Vec2, font_size: f32) -> String {
    wrap_text(text, size.x - callout_padding(font_size) * 2.0, font_size)
}

// Wrapped lines of a callout plus the y offset (from the box top) of the first line,
// so the text block is vertically centered. Lines are horizontally centered by the
// caller, which knows how wide its own font renders them.
pub fn callout_text_lines(text: &str, size: egui::Vec2, font_size: f32) -> (Vec<String>, f32) {
    let wrapped = callout_wrapped(text, size, font_size);
    let lines: Vec<String> = wrapped.lines().map(str::to_string).collect();
    let block_h = lines.len() as f32 * font_size * 1.2;
    (lines, ((size.y - block_h) / 2.0).max(0.0))
}

// Approximate text bounds.
pub fn text_bounds(at: egui::Pos2, text: &str, font_size: f32) -> ImgRect {
    let lines = text.lines().count().max(1) as f32;
    let widest = text
        .lines()
        .map(|l| line_width(l, font_size))
        .fold(0.0_f32, f32::max);
    ImgRect {
        min: at,
        max: egui::pos2(at.x + widest, at.y + lines * font_size * 1.2),
    }
}

// Default callout size: text bounds plus padding on every side.
pub fn callout_fit_size(text: &str, font_size: f32) -> egui::Vec2 {
    let pad = callout_padding(font_size);
    let t = text_bounds(egui::Pos2::ZERO, text, font_size);
    egui::vec2(t.max.x + pad * 2.0, t.max.y + pad * 2.0)
}

// Speech-bubble tail outline: the base corners from `callout_tail_polygon` joined to
// the tip by inward-curved sides. Star-shaped around the base midpoint, so a fan from
// that point (first element is the pivot) fills it. Painting and burning both use it.
pub fn callout_tail_outline(rect: ImgRect, tail: egui::Pos2) -> Option<Vec<egui::Pos2>> {
    const STEPS: usize = 8;
    let [p, q, tip] = callout_tail_polygon(rect, tail)?;
    let base_mid = p + (q - p) * 0.5;
    let axis_mid = base_mid + (tip - base_mid) * 0.5;
    // Pull each side's control point toward the tail axis for a concave, tapering curve.
    let side = |from: egui::Pos2| {
        let ctrl = from + (tip - from) * 0.5;
        let ctrl = ctrl + (axis_mid - ctrl) * 0.55;
        (1..=STEPS).map(move |i| {
            let t = i as f32 / STEPS as f32;
            let u = 1.0 - t;
            egui::pos2(
                u * u * from.x + 2.0 * u * t * ctrl.x + t * t * tip.x,
                u * u * from.y + 2.0 * u * t * ctrl.y + t * t * tip.y,
            )
        })
    };
    let mut out = vec![base_mid, p];
    out.extend(side(p));
    let back: Vec<egui::Pos2> = side(q).collect();
    // Walk the q side from the tip back down to q.
    out.extend(back.iter().rev().skip(1));
    out.push(q);
    Some(out)
}

// Triangle joining the callout box to its tail tip; None when the tip is inside the box.
pub fn callout_tail_polygon(rect: ImgRect, tail: egui::Pos2) -> Option<[egui::Pos2; 3]> {
    let (w, h) = (rect.width().max(1.0), rect.height().max(1.0));
    let c = rect.center();
    let inside = tail.x >= rect.min.x
        && tail.x <= rect.max.x
        && tail.y >= rect.min.y
        && tail.y <= rect.max.y;
    if inside {
        return None;
    }
    let half = (w.min(h) * 0.12).clamp(3.0, 18.0);
    // The base slides from the nearest side's midpoint toward where the center->tip
    // ray crosses that side, in proportion to the tip's offset from the center. A tip
    // straight out from a side gives the midpoint; a near-diagonal tip leans toward
    // the corner (half-way, kept clear of the rounded corners).
    const LEAN: f32 = 0.5;
    let margin = (half + w.min(h) * 0.15).min(w.min(h) / 2.0);
    let (dx, dy) = (tail.x - c.x, tail.y - c.y);
    if dx.abs() / (w / 2.0) > dy.abs() / (h / 2.0) {
        let x = if dx > 0.0 { rect.max.x } else { rect.min.x };
        let hit_y = c.y + dy * (w / 2.0) / dx.abs();
        let y = (c.y + (hit_y - c.y) * LEAN).clamp(rect.min.y + margin, rect.max.y - margin);
        Some([egui::pos2(x, y - half), egui::pos2(x, y + half), tail])
    } else {
        let y = if dy > 0.0 { rect.max.y } else { rect.min.y };
        let hit_x = c.x + dx * (h / 2.0) / dy.abs();
        let x = (c.x + (hit_x - c.x) * LEAN).clamp(rect.min.x + margin, rect.max.x - margin);
        Some([egui::pos2(x - half, y), egui::pos2(x + half, y), tail])
    }
}

// Move the edges named by handle `id` (0..8, see `Shape::handles`) to `to`;
// edges never cross each other.
fn resize_rect(rect: &mut ImgRect, id: usize, to: egui::Pos2) {
    resize_rect_min(rect, id, to, egui::Vec2::splat(1.0));
}

// Like `resize_rect`, but the box never gets smaller than `min`.
fn resize_rect_min(rect: &mut ImgRect, id: usize, to: egui::Pos2, min: egui::Vec2) {
    if matches!(id, 0 | 6 | 7) {
        rect.min.x = to.x.min(rect.max.x - min.x);
    }
    if matches!(id, 2 | 3 | 4) {
        rect.max.x = to.x.max(rect.min.x + min.x);
    }
    if matches!(id, 0 | 1 | 2) {
        rect.min.y = to.y.min(rect.max.y - min.y);
    }
    if matches!(id, 4 | 5 | 6) {
        rect.max.y = to.y.max(rect.min.y + min.y);
    }
}

pub fn dist_point_segment(p: egui::Pos2, a: egui::Pos2, b: egui::Pos2) -> f32 {
    let ab = b - a;
    let denom = ab.length_sq().max(1e-6);
    let t = ((p - a).dot(ab) / denom).clamp(0.0, 1.0);
    (a + ab * t - p).length()
}

// Ramer-Douglas-Peucker: keep the fewest points that stay within `epsilon` of the
// original polyline. Endpoints are always kept.
pub fn simplify_rdp(points: &[egui::Pos2], epsilon: f32) -> Vec<egui::Pos2> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let (first, last) = (points[0], points[points.len() - 1]);
    let (idx, dist) = points[1..points.len() - 1]
        .iter()
        .enumerate()
        .map(|(i, p)| (i + 1, dist_point_segment(*p, first, last)))
        .fold((0, 0.0_f32), |best, c| if c.1 > best.1 { c } else { best });
    if dist <= epsilon {
        return vec![first, last];
    }
    let mut left = simplify_rdp(&points[..=idx], epsilon);
    let right = simplify_rdp(&points[idx..], epsilon);
    left.pop();
    left.extend(right);
    left
}

// Samples a Catmull-Rom spline passing through every anchor. Used for painting,
// burning, hit-testing and bounds so all four agree. Fewer than 3 anchors is a
// straight segment.
pub fn spline_points(anchors: &[egui::Pos2]) -> Vec<egui::Pos2> {
    const STEPS: usize = 12;
    if anchors.len() < 3 {
        return anchors.to_vec();
    }
    let n = anchors.len();
    let mut out = Vec::with_capacity((n - 1) * STEPS + 1);
    out.push(anchors[0]);
    for i in 0..n - 1 {
        let p0 = anchors[i.saturating_sub(1)];
        let (p1, p2) = (anchors[i], anchors[i + 1]);
        let p3 = anchors[(i + 2).min(n - 1)];
        for s in 1..=STEPS {
            let t = s as f32 / STEPS as f32;
            let (t2, t3) = (t * t, t * t * t);
            let f = |a: f32, b: f32, c: f32, d: f32| {
                0.5 * (2.0 * b + (c - a) * t + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2
                    + (3.0 * b - a - 3.0 * c + d) * t3)
            };
            out.push(egui::pos2(f(p0.x, p1.x, p2.x, p3.x), f(p0.y, p1.y, p2.y, p3.y)));
        }
    }
    out
}

// The 8 resize handles of a rect: TL, T, TR, R, BR, B, BL, L.
fn rect_handles(rect: ImgRect) -> Vec<egui::Pos2> {
    let (l, r, t, b) = (rect.min.x, rect.max.x, rect.min.y, rect.max.y);
    let c = rect.center();
    vec![
        egui::pos2(l, t),
        egui::pos2(c.x, t),
        egui::pos2(r, t),
        egui::pos2(r, c.y),
        egui::pos2(r, b),
        egui::pos2(c.x, b),
        egui::pos2(l, b),
        egui::pos2(l, c.y),
    ]
}

fn rect_contains(r: ImgRect, pos: egui::Pos2, tolerance: f32) -> bool {
    pos.x >= r.min.x - tolerance
        && pos.x <= r.max.x + tolerance
        && pos.y >= r.min.y - tolerance
        && pos.y <= r.max.y + tolerance
}

impl Shape {
    pub fn bounding_rect(&self) -> ImgRect {
        match self {
            Shape::Line { a, b, .. } | Shape::Arrow { a, b, .. } => {
                ImgRect::from_points(*a, *b)
            }
            // Invariant: Pencil always holds 2+ points (see draft_ready).
            Shape::Pencil { points, .. } => {
                // Bounds of the sampled spline, which can overshoot the anchors.
                let curve = spline_points(points);
                let mut r = ImgRect::from_points(curve[0], curve[0]);
                for p in &curve[1..] {
                    r.min.x = r.min.x.min(p.x);
                    r.min.y = r.min.y.min(p.y);
                    r.max.x = r.max.x.max(p.x);
                    r.max.y = r.max.y.max(p.y);
                }
                r
            }
            Shape::Rect { rect, style } | Shape::Ellipse { rect, style } if style.angle != 0.0 => {
                // Axis-aligned box around the rotated rectangle.
                let c = rect.center();
                let corners = [
                    rect.min,
                    egui::pos2(rect.max.x, rect.min.y),
                    rect.max,
                    egui::pos2(rect.min.x, rect.max.y),
                ]
                .map(|p| rotate_about(p, c, style.angle));
                let mut r = ImgRect::from_points(corners[0], corners[0]);
                for p in &corners[1..] {
                    r.min = egui::pos2(r.min.x.min(p.x), r.min.y.min(p.y));
                    r.max = egui::pos2(r.max.x.max(p.x), r.max.y.max(p.y));
                }
                r
            }
            Shape::Rect { rect, .. }
            | Shape::Ellipse { rect, .. }
            | Shape::Highlight { rect, .. }
            | Shape::BlurRegion { rect, .. } => *rect,
            Shape::Text { at, text, style } => text_bounds(*at, text, style.font_size),
            Shape::Callout { at, size, tail, .. } => {
                let r = callout_box(*at, *size);
                ImgRect {
                    min: egui::pos2(r.min.x.min(tail.x), r.min.y.min(tail.y)),
                    max: egui::pos2(r.max.x.max(tail.x), r.max.y.max(tail.y)),
                }
            }
            Shape::Step { at, style, .. } => {
                let r = step_radius(style.font_size);
                ImgRect {
                    min: egui::pos2(at.x - r, at.y - r),
                    max: egui::pos2(at.x + r, at.y + r),
                }
            }
        }
    }

    pub fn style(&self) -> Option<&Style> {
        match self {
            Shape::Line { style, .. }
            | Shape::Arrow { style, .. }
            | Shape::Pencil { style, .. }
            | Shape::Rect { style, .. }
            | Shape::Ellipse { style, .. }
            | Shape::Highlight { style, .. }
            | Shape::Text { style, .. }
            | Shape::Callout { style, .. }
            | Shape::Step { style, .. } => Some(style),
            Shape::BlurRegion { .. } => None,
        }
    }

    pub fn style_mut(&mut self) -> Option<&mut Style> {
        match self {
            Shape::Line { style, .. }
            | Shape::Arrow { style, .. }
            | Shape::Pencil { style, .. }
            | Shape::Rect { style, .. }
            | Shape::Ellipse { style, .. }
            | Shape::Highlight { style, .. }
            | Shape::Text { style, .. }
            | Shape::Callout { style, .. }
            | Shape::Step { style, .. } => Some(style),
            Shape::BlurRegion { .. } => None,
        }
    }

    // Editable text of Text / Callout shapes.
    pub fn text_mut(&mut self) -> Option<&mut String> {
        match self {
            Shape::Text { text, .. } | Shape::Callout { text, .. } => Some(text),
            _ => None,
        }
    }

    // Control points for resizing, indexed by handle id.
    // Rect-like shapes: 0..8 = TL, T, TR, R, BR, B, BL, L. Line/Arrow: 0 = start,
    // 1 = end. Callout: 0..8 = box edges as for rects, 8 = tail tip.
    pub fn handles(&self) -> Vec<egui::Pos2> {
        match self {
            Shape::Line { a, b, .. } | Shape::Arrow { a, b, .. } => vec![*a, *b],
            Shape::Rect { rect, style } | Shape::Ellipse { rect, style } => {
                let (l, r, t, b) = (rect.min.x, rect.max.x, rect.min.y, rect.max.y);
                let c = rect.center();
                let mut pts = vec![
                    egui::pos2(l, t),
                    egui::pos2(c.x, t),
                    egui::pos2(r, t),
                    egui::pos2(r, c.y),
                    egui::pos2(r, b),
                    egui::pos2(c.x, b),
                    egui::pos2(l, b),
                    egui::pos2(l, c.y),
                    // 8 = rotate handle, above the top edge.
                    egui::pos2(c.x, t - ROTATE_HANDLE_OFFSET),
                ];
                for p in &mut pts {
                    *p = rotate_about(*p, c, style.angle);
                }
                pts
            }
            Shape::Highlight { rect, .. } | Shape::BlurRegion { rect, .. } => {
                rect_handles(*rect)
            }
            // One handle per curve anchor.
            Shape::Pencil { points, .. } => points.clone(),
            Shape::Callout { at, size, tail, .. } => {
                let mut pts = rect_handles(callout_box(*at, *size));
                // 8 = tail tip.
                pts.push(*tail);
                pts
            }
            Shape::Text { .. } | Shape::Step { .. } => vec![],
        }
    }

    // Move one handle to `to` (image px). Rect edges never cross each other.
    pub fn drag_handle(&mut self, id: usize, to: egui::Pos2) {
        match self {
            Shape::Line { a, b, .. } | Shape::Arrow { a, b, .. } => match id {
                0 => *a = to,
                1 => *b = to,
                _ => {}
            },
            Shape::Rect { rect, style } | Shape::Ellipse { rect, style } => {
                let c = rect.center();
                if id == 8 {
                    let mut angle = (to.y - c.y).atan2(to.x - c.x) + std::f32::consts::FRAC_PI_2;
                    // Snap to the nearest right angle when within ~3 degrees.
                    let quarter = std::f32::consts::FRAC_PI_2;
                    let nearest = (angle / quarter).round() * quarter;
                    if (angle - nearest).abs() < 0.05 {
                        angle = nearest;
                    }
                    style.angle = angle;
                    return;
                }
                // Resize in the unrotated frame, then re-center so the opposite
                // edge stays where it was on screen.
                let local = rotate_about(to, c, -style.angle);
                resize_rect(rect, id, local);
                let new_local_center = rect.center();
                let new_world_center = rotate_about(new_local_center, c, style.angle);
                let shift = new_world_center - new_local_center;
                rect.min += shift;
                rect.max += shift;
            }
            Shape::Highlight { rect, .. } | Shape::BlurRegion { rect, .. } => {
                resize_rect(rect, id, to);
            }
            Shape::Callout {
                at,
                size,
                tail,
                style,
                ..
            } => {
                if id == 8 {
                    *tail = to;
                } else {
                    let old = *size;
                    let mut r = callout_box(*at, *size);
                    resize_rect_min(&mut r, id, to, callout_min_size(style.font_size));
                    *at = r.min;
                    *size = egui::vec2(r.width(), r.height());
                    // Corner drags scale the text with the box; edge drags only reflow it.
                    if matches!(id, 0 | 2 | 4 | 6) && old.x > 1e-3 && old.y > 1e-3 {
                        let k = (size.x / old.x * size.y / old.y).sqrt();
                        style.font_size = (style.font_size * k).clamp(6.0, 400.0);
                    }
                }
            }
            Shape::Pencil { points, .. } => {
                if let Some(p) = points.get_mut(id) {
                    *p = to;
                }
            }
            Shape::Text { .. } | Shape::Step { .. } => {}
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
            Shape::Text { at, .. } | Shape::Step { at, .. } => mv(at),
            Shape::Callout { at, tail, .. } => {
                mv(at);
                mv(tail);
            }
        }
    }

    // Selection hit test in image-pixel space; tolerance is in image px.
    pub fn hit_test(&self, pos: egui::Pos2, tolerance: f32) -> bool {
        match self {
            Shape::Line { a, b, .. } | Shape::Arrow { a, b, .. } => {
                dist_point_segment(pos, *a, *b) <= tolerance
            }
            Shape::Pencil { points, .. } => spline_points(points)
                .windows(2)
                .any(|w| dist_point_segment(pos, w[0], w[1]) <= tolerance),
            Shape::Rect { rect, style } => {
                let p = rotate_about(pos, rect.center(), -style.angle);
                rect_contains(*rect, p, tolerance)
            }
            Shape::Highlight { rect, .. } | Shape::BlurRegion { rect, .. } => {
                rect_contains(*rect, pos, tolerance)
            }
            Shape::Ellipse { rect, style } => {
                let c = rect.center();
                let pos = rotate_about(pos, c, -style.angle);
                let rx = (rect.width() / 2.0 + tolerance).max(1e-3);
                let ry = (rect.height() / 2.0 + tolerance).max(1e-3);
                let dx = (pos.x - c.x) / rx;
                let dy = (pos.y - c.y) / ry;
                dx * dx + dy * dy <= 1.0
            }
            Shape::Text { at, text, style } => {
                rect_contains(text_bounds(*at, text, style.font_size), pos, tolerance)
            }
            Shape::Callout { at, size, tail, .. } => {
                let b = callout_box(*at, *size);
                rect_contains(b, pos, tolerance)
                    || callout_tail_polygon(b, *tail).is_some_and(|[p, q, tip]| {
                        dist_point_segment(pos, p, tip) <= tolerance
                            || dist_point_segment(pos, q, tip) <= tolerance
                            || dist_point_segment(pos, p, q) <= tolerance
                    })
            }
            Shape::Step { at, style, .. } => {
                (pos - *at).length() <= step_radius(style.font_size) + tolerance
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

    #[test]
    fn rect_handle_drag_resizes_and_never_flips() {
        let mut s = Shape::Rect {
            rect: ImgRect {
                min: egui::pos2(10.0, 10.0),
                max: egui::pos2(50.0, 50.0),
            },
            style: Style::default(),
        };
        s.drag_handle(4, egui::pos2(80.0, 90.0));
        assert_eq!(s.bounding_rect().max, egui::pos2(80.0, 90.0));
        s.drag_handle(0, egui::pos2(500.0, 500.0));
        let r = s.bounding_rect();
        assert!(r.min.x < r.max.x && r.min.y < r.max.y);
    }

    #[test]
    fn line_handles_move_endpoints() {
        let mut s = Shape::Line {
            a: egui::pos2(0.0, 0.0),
            b: egui::pos2(10.0, 10.0),
            style: Style::default(),
        };
        assert_eq!(s.handles().len(), 2);
        s.drag_handle(1, egui::pos2(30.0, 5.0));
        assert_eq!(s.handles()[1], egui::pos2(30.0, 5.0));
    }

    #[test]
    fn callout_tail_exists_only_outside_box() {
        let r = ImgRect {
            min: egui::pos2(0.0, 0.0),
            max: egui::pos2(100.0, 40.0),
        };
        assert!(callout_tail_polygon(r, egui::pos2(50.0, 20.0)).is_none());
        let poly = callout_tail_polygon(r, egui::pos2(30.0, 90.0)).unwrap();
        assert_eq!(poly[2], egui::pos2(30.0, 90.0));
        assert_eq!(poly[0].y, 40.0); // base sits on the bottom edge
    }

    #[test]
    fn step_hit_test_uses_radius() {
        let s = Shape::Step {
            at: egui::pos2(50.0, 50.0),
            number: 1,
            style: Style::default(),
        };
        assert!(s.hit_test(egui::pos2(60.0, 50.0), 2.0));
        assert!(!s.hit_test(egui::pos2(120.0, 50.0), 2.0));
    }
}
