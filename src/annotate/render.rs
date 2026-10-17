// Annotation rendering: egui overlay painting + CPU burn.

use super::shapes::{
    callout_padding, callout_rect, callout_tail_polygon, dist_point_segment, rotate_about,
    step_radius, ImgRect, Shape, Style, HANDLE_RADIUS_PX,
};
use eframe::egui;

// Two wing tips of an arrowhead: `dir` is the unit flight direction.
pub fn arrow_head_points(tip: egui::Pos2, dir: egui::Vec2, size: f32) -> [egui::Pos2; 2] {
    let base = tip - dir * size;
    let perp = egui::vec2(-dir.y, dir.x) * size * 0.5;
    [base + perp, base - perp]
}

// Where the shaft stops and the filled head triangle [tip, wing, wing] sits.
pub fn arrow_geometry(
    a: egui::Pos2,
    b: egui::Pos2,
    width: f32,
) -> (egui::Pos2, [egui::Pos2; 3]) {
    let delta = b - a;
    let len = delta.length();
    let dir = if len > 1e-6 {
        delta / len
    } else {
        egui::vec2(1.0, 0.0)
    };
    let size = (width * 4.0 + 6.0).min((len * 0.9).max(2.0));
    let [w1, w2] = arrow_head_points(b, dir, size);
    (b - dir * size * 0.6, [b, w1, w2])
}

// Dash and gap lengths in image px for a stroke of `width`.
pub fn dash_pattern(width: f32) -> (f32, f32) {
    ((width * 3.0).max(8.0), (width * 2.0).max(5.0))
}

// Split a polyline into dash segments.
pub fn dash_segments(path: &[egui::Pos2], dash: f32, gap: f32) -> Vec<(egui::Pos2, egui::Pos2)> {
    let mut out = Vec::new();
    let (dash, gap) = (dash.max(0.5), gap.max(0.5));
    let mut drawing = true;
    let mut remaining = dash;
    for w in path.windows(2) {
        let (a, b) = (w[0], w[1]);
        let len = (b - a).length();
        if len < 1e-6 {
            continue;
        }
        let dir = (b - a) / len;
        let mut pos = 0.0;
        while pos < len {
            let step = remaining.min(len - pos);
            if drawing {
                out.push((a + dir * pos, a + dir * (pos + step)));
            }
            pos += step;
            remaining -= step;
            if remaining <= 1e-6 {
                drawing = !drawing;
                remaining = if drawing { dash } else { gap };
            }
        }
    }
    out
}

fn rect_corners(r: ImgRect) -> Vec<egui::Pos2> {
    vec![
        r.min,
        egui::pos2(r.max.x, r.min.y),
        r.max,
        egui::pos2(r.min.x, r.max.y),
    ]
}

// Polyline approximating an ellipse, in image px.
fn ellipse_points(r: ImgRect) -> Vec<egui::Pos2> {
    let c = r.center();
    let (rx, ry) = (r.width() / 2.0, r.height() / 2.0);
    let n = ((std::f32::consts::TAU * rx.max(ry) / 3.0) as usize).clamp(32, 720);
    (0..n)
        .map(|i| {
            let t = i as f32 / n as f32 * std::f32::consts::TAU;
            egui::pos2(c.x + rx * t.cos(), c.y + ry * t.sin())
        })
        .collect()
}

// Closed outline of a rect / ellipse with the style's rotation applied (image px).
fn closed_outline(rect: ImgRect, style: Style, ellipse: bool) -> Vec<egui::Pos2> {
    let mut pts = if ellipse {
        ellipse_points(rect)
    } else {
        rect_corners(rect)
    };
    if style.angle != 0.0 {
        let c = rect.center();
        for p in &mut pts {
            *p = rotate_about(*p, c, style.angle);
        }
    }
    pts
}

// Highlighter fill strength derived from the style opacity.
fn highlight_opacity(style: Style) -> f32 {
    (style.opacity * 0.4_f32).clamp(0.05_f32, 0.6_f32)
}

// Black, translucent copy of `shape` offset like a drop shadow. `offset_scale`
// and `alpha` let the overlay stack several copies into a soft falloff.
fn shadow_of(shape: &Shape, style: Style, offset_scale: f32, alpha: f32) -> Shape {
    let mut sh = shape.clone();
    sh.translate(style.shadow_offset() * offset_scale);
    if let Some(s) = sh.style_mut() {
        s.color = egui::Color32::BLACK;
        s.fill_color = egui::Color32::BLACK;
        s.opacity *= alpha;
        s.shadow = false;
    }
    sh
}

// Overlay approximation of a blurred shadow: three stacked, fainter copies.
const SOFT_SHADOW_LAYERS: [(f32, f32); 3] = [(1.5, 0.12), (1.0, 0.14), (0.5, 0.14)];

// Real glyph width of one line at `font_size`, when a burn font is available.
pub fn measure_line(line: &str, font_size: f32) -> Option<f32> {
    let font = burn_font()?;
    Some(text_size(PxScale::from(font_size.max(4.0)), font, line).0 as f32)
}

fn casts_shadow(shape: &Shape) -> Option<Style> {
    match shape {
        Shape::Highlight { .. } | Shape::BlurRegion { .. } => None,
        _ => shape.style().copied().filter(|s| s.shadow),
    }
}

fn stroke(style: Style, zoom: f32) -> egui::Stroke {
    egui::Stroke::new((style.width * zoom).max(1.0), style.effective_color())
}

// Stroke a screen-space path, dashed when the style asks for it.
fn stroke_path(
    painter: &egui::Painter,
    pts: Vec<egui::Pos2>,
    style: Style,
    zoom: f32,
    closed: bool,
) {
    let st = stroke(style, zoom);
    if style.dashed {
        let (dash, gap) = dash_pattern(style.width);
        let mut path = pts;
        if closed {
            if let Some(first) = path.first().copied() {
                path.push(first);
            }
        }
        painter.extend(egui::Shape::dashed_line(&path, st, dash * zoom, gap * zoom));
    } else if closed {
        painter.add(egui::Shape::closed_line(pts, st));
    } else {
        painter.add(egui::Shape::line(pts, st));
    }
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
        paint_with_shadow(painter, shape, zoom, &to_screen);
        if selected == Some(i) {
            paint_selection(painter, shape, &to_screen);
        }
    }
}

fn paint_with_shadow(
    painter: &egui::Painter,
    shape: &Shape,
    zoom: f32,
    to_screen: &impl Fn(egui::Pos2) -> egui::Pos2,
) {
    if let Some(style) = casts_shadow(shape) {
        for (scale, alpha) in SOFT_SHADOW_LAYERS {
            paint_shape(painter, &shadow_of(shape, style, scale, alpha), zoom, to_screen, true);
        }
    }
    paint_shape(painter, shape, zoom, to_screen, false);
}

fn paint_selection(
    painter: &egui::Painter,
    shape: &Shape,
    to_screen: &impl Fn(egui::Pos2) -> egui::Pos2,
) {
    let b = shape.bounding_rect();
    let outline = egui::Rect::from_min_max(to_screen(b.min), to_screen(b.max));
    painter.rect_stroke(
        outline,
        egui::CornerRadius::ZERO,
        egui::Stroke::new(3.0_f32, egui::Color32::from_black_alpha(140)),
        egui::StrokeKind::Outside,
    );
    painter.rect_stroke(
        outline,
        egui::CornerRadius::ZERO,
        egui::Stroke::new(1.5_f32, egui::Color32::WHITE),
        egui::StrokeKind::Outside,
    );
    let handles = shape.handles();
    let rotatable = matches!(shape, Shape::Rect { .. } | Shape::Ellipse { .. });
    for (i, h) in handles.iter().enumerate() {
        let c = to_screen(*h);
        if rotatable && i == 8 {
            // Rotate handle: round knob joined to the top-center edge handle.
            painter.line_segment(
                [to_screen(handles[1]), c],
                egui::Stroke::new(1.0_f32, egui::Color32::WHITE),
            );
            painter.circle_filled(c, HANDLE_RADIUS_PX, egui::Color32::from_rgb(40, 120, 240));
            painter.circle_stroke(c, HANDLE_RADIUS_PX, egui::Stroke::new(1.0_f32, egui::Color32::WHITE));
            continue;
        }
        let r = egui::Rect::from_center_size(c, egui::Vec2::splat(HANDLE_RADIUS_PX * 2.0));
        painter.rect_filled(r, egui::CornerRadius::same(1), egui::Color32::WHITE);
        painter.rect_stroke(
            r,
            egui::CornerRadius::same(1),
            egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(30, 30, 30)),
            egui::StrokeKind::Outside,
        );
    }
}

// `silhouette` paints only the filled outline (used for shadows): no text glyphs
// on callouts or step stamps.
fn paint_shape(
    painter: &egui::Painter,
    shape: &Shape,
    zoom: f32,
    to_screen: &impl Fn(egui::Pos2) -> egui::Pos2,
    silhouette: bool,
) {
    match shape {
        Shape::Line { a, b, style } => {
            stroke_path(painter, vec![to_screen(*a), to_screen(*b)], *style, zoom, false);
        }
        Shape::Arrow { a, b, style } => {
            let (shaft_end, head) = arrow_geometry(*a, *b, style.width);
            stroke_path(
                painter,
                vec![to_screen(*a), to_screen(shaft_end)],
                *style,
                zoom,
                false,
            );
            painter.add(egui::Shape::convex_polygon(
                head.iter().map(|p| to_screen(*p)).collect(),
                style.effective_color(),
                egui::Stroke::NONE,
            ));
        }
        Shape::Pencil { points, style } => {
            let pts: Vec<egui::Pos2> = points.iter().map(|p| to_screen(*p)).collect();
            stroke_path(painter, pts, *style, zoom, false);
        }
        Shape::Rect { rect, style } | Shape::Ellipse { rect, style } => {
            let is_ellipse = matches!(shape, Shape::Ellipse { .. });
            let r = egui::Rect::from_min_max(to_screen(rect.min), to_screen(rect.max));
            let rotated = style.angle != 0.0;
            let outline_pts = || -> Vec<egui::Pos2> {
                closed_outline(*rect, *style, is_ellipse)
                    .into_iter()
                    .map(&to_screen)
                    .collect()
            };
            if style.filled {
                let fill = style.effective_fill();
                if rotated {
                    painter.add(egui::Shape::convex_polygon(
                        outline_pts(),
                        fill,
                        egui::Stroke::NONE,
                    ));
                } else if is_ellipse {
                    painter.add(egui::Shape::ellipse_filled(r.center(), r.size() / 2.0, fill));
                } else {
                    painter.rect_filled(r, egui::CornerRadius::ZERO, fill);
                }
            }
            if style.outline {
                if rotated || style.dashed {
                    stroke_path(painter, outline_pts(), *style, zoom, true);
                } else if is_ellipse {
                    painter.add(egui::Shape::ellipse_stroke(
                        r.center(),
                        r.size() / 2.0,
                        stroke(*style, zoom),
                    ));
                } else {
                    painter.rect_stroke(
                        r,
                        egui::CornerRadius::ZERO,
                        stroke(*style, zoom),
                        egui::StrokeKind::Middle,
                    );
                }
            }
        }
        Shape::Highlight { rect, style } => {
            let r = egui::Rect::from_min_max(to_screen(rect.min), to_screen(rect.max));
            let mut hl = *style;
            hl.opacity = highlight_opacity(*style);
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
        Shape::Callout {
            at,
            text,
            tail,
            style,
        } => {
            let rect = callout_rect(*at, text, style.font_size);
            let screen_rect = egui::Rect::from_min_max(to_screen(rect.min), to_screen(rect.max));
            let radius = (style.font_size * 0.4 * zoom).clamp(0.0, 60.0) as u8;
            let fill = style.effective_color();
            painter.rect_filled(screen_rect, egui::CornerRadius::same(radius), fill);
            if let Some(poly) = callout_tail_polygon(rect, *tail) {
                painter.add(egui::Shape::convex_polygon(
                    poly.iter().map(|p| to_screen(*p)).collect(),
                    fill,
                    egui::Stroke::NONE,
                ));
            }
            if !silhouette {
                let pad = callout_padding(style.font_size);
                painter.text(
                    to_screen(egui::pos2(at.x + pad, at.y + pad)),
                    egui::Align2::LEFT_TOP,
                    text,
                    egui::FontId::proportional((style.font_size * zoom).max(4.0)),
                    style.on_color(),
                );
            }
        }
        Shape::Step { at, number, style } => {
            let r = step_radius(style.font_size) * zoom;
            painter.circle_filled(to_screen(*at), r, style.effective_color());
            if !silhouette {
                painter.text(
                    to_screen(*at),
                    egui::Align2::CENTER_CENTER,
                    number.to_string(),
                    egui::FontId::proportional((style.font_size * zoom).max(4.0)),
                    style.on_color(),
                );
            }
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
            rect: ImgRect::from_points(draft.start, draft.current),
            style,
        },
        super::Tool::Ellipse => Shape::Ellipse {
            rect: ImgRect::from_points(draft.start, draft.current),
            style,
        },
        super::Tool::Pencil => Shape::Pencil {
            points: draft.points.clone(),
            style,
        },
        super::Tool::Highlight => Shape::Highlight {
            rect: ImgRect::from_points(draft.start, draft.current),
            style,
        },
        super::Tool::BlurRegion => Shape::BlurRegion {
            rect: ImgRect::from_points(draft.start, draft.current),
            kind: super::shapes::BlurKind::Blur,
        },
        // Stamps are dropped on release; nothing to preview while dragging.
        super::Tool::Text | super::Tool::Callout | super::Tool::Step => return,
    };
    paint_with_shadow(painter, &preview, zoom, &to_screen);
}

// ---------------------------------------------------------------------------
// CPU burn
// ---------------------------------------------------------------------------

use ab_glyph::{FontArc, PxScale};
use image::{DynamicImage, GrayImage, Luma, Rgba, RgbaImage};
use imageproc::drawing::{draw_text_mut, text_size};
use imageproc::rect::Rect as IRect;
use std::sync::OnceLock;

// Standard-alpha blend of src over dst (both RGBA).
fn blend(dst: Rgba<u8>, src: Rgba<u8>) -> Rgba<u8> {
    let a = src[3] as f32 / 255.0;
    Rgba([
        (src[0] as f32 * a + dst[0] as f32 * (1.0 - a)).round() as u8,
        (src[1] as f32 * a + dst[1] as f32 * (1.0 - a)).round() as u8,
        (src[2] as f32 * a + dst[2] as f32 * (1.0 - a)).round() as u8,
        dst[3].max(src[3]),
    ])
}

// Style color with opacity folded into alpha, as plain RGBA.
fn style_rgba(style: Style) -> Rgba<u8> {
    let c = style.color;
    Rgba([
        c.r(),
        c.g(),
        c.b(),
        (c.a() as f32 * style.opacity.clamp(0.0, 1.0)).round() as u8,
    ])
}

fn plain_rgba(c: egui::Color32) -> Rgba<u8> {
    Rgba([c.r(), c.g(), c.b(), c.a()])
}

// Per-pixel coverage (0..=1) of one object, limited to its padded bounding box.
// Every primitive max-combines, so overlapping strokes never double-blend when the
// mask is composited once with the object's opacity.
struct Mask {
    x0: i32,
    y0: i32,
    w: usize,
    h: usize,
    data: Vec<f32>,
}

impl Mask {
    fn new(bounds: ImgRect, pad: f32, img_w: u32, img_h: u32) -> Option<Mask> {
        let x0 = ((bounds.min.x - pad).floor() as i32).max(0);
        let y0 = ((bounds.min.y - pad).floor() as i32).max(0);
        let x1 = ((bounds.max.x + pad).ceil() as i32).min(img_w as i32);
        let y1 = ((bounds.max.y + pad).ceil() as i32).min(img_h as i32);
        if x1 <= x0 || y1 <= y0 {
            return None;
        }
        let (w, h) = ((x1 - x0) as usize, (y1 - y0) as usize);
        Some(Mask {
            x0,
            y0,
            w,
            h,
            data: vec![0.0; w * h],
        })
    }

    fn put(&mut self, x: i32, y: i32, c: f32) {
        let (lx, ly) = (x - self.x0, y - self.y0);
        if lx < 0 || ly < 0 || lx as usize >= self.w || ly as usize >= self.h {
            return;
        }
        let cell = &mut self.data[ly as usize * self.w + lx as usize];
        if c > *cell {
            *cell = c;
        }
    }

    // Visit every mask pixel inside the float box [min, max] (image px).
    fn for_box(&mut self, min: egui::Pos2, max: egui::Pos2, mut f: impl FnMut(&mut Mask, i32, i32)) {
        let lo_x = (min.x.floor() as i32).max(self.x0);
        let lo_y = (min.y.floor() as i32).max(self.y0);
        let hi_x = (max.x.ceil() as i32).min(self.x0 + self.w as i32 - 1);
        let hi_y = (max.y.ceil() as i32).min(self.y0 + self.h as i32 - 1);
        for y in lo_y..=hi_y {
            for x in lo_x..=hi_x {
                f(self, x, y);
            }
        }
    }

    // Round-capped line segment of radius `r`.
    fn segment(&mut self, a: egui::Pos2, b: egui::Pos2, r: f32) {
        let r = r.max(0.5);
        let min = egui::pos2(a.x.min(b.x) - r - 1.0, a.y.min(b.y) - r - 1.0);
        let max = egui::pos2(a.x.max(b.x) + r + 1.0, a.y.max(b.y) + r + 1.0);
        self.for_box(min, max, |m, x, y| {
            let p = egui::pos2(x as f32 + 0.5, y as f32 + 0.5);
            let c = (r + 0.5 - dist_point_segment(p, a, b)).clamp(0.0, 1.0);
            if c > 0.0 {
                m.put(x, y, c);
            }
        });
    }

    fn stroke_path(&mut self, pts: &[egui::Pos2], width: f32, dashed: bool, closed: bool) {
        let r = width / 2.0;
        let mut path = pts.to_vec();
        if closed {
            if let Some(first) = pts.first() {
                path.push(*first);
            }
        }
        if dashed {
            let (dash, gap) = dash_pattern(width);
            for (a, b) in dash_segments(&path, dash, gap) {
                // Round caps add `r` at both ends; pull the ends in so the dash keeps
                // its nominal length and the gaps match the on-screen butt caps.
                let len = (b - a).length();
                let inset = r.min(len / 2.0);
                let dir = if len > 1e-6 { (b - a) / len } else { egui::Vec2::ZERO };
                self.segment(a + dir * inset, b - dir * inset, r);
            }
        } else {
            for w in path.windows(2) {
                self.segment(w[0], w[1], r);
            }
        }
    }

    fn fill_rect(&mut self, r: ImgRect) {
        self.for_box(r.min, r.max, |m, x, y| {
            let cx = ((x as f32 + 1.0).min(r.max.x) - (x as f32).max(r.min.x)).clamp(0.0, 1.0);
            let cy = ((y as f32 + 1.0).min(r.max.y) - (y as f32).max(r.min.y)).clamp(0.0, 1.0);
            if cx * cy > 0.0 {
                m.put(x, y, cx * cy);
            }
        });
    }

    fn fill_round_rect(&mut self, r: ImgRect, radius: f32) {
        let half = egui::vec2(r.width() / 2.0, r.height() / 2.0);
        let rad = radius.min(half.x).min(half.y).max(0.0);
        let c = r.center();
        self.for_box(r.min - egui::vec2(1.0, 1.0), r.max + egui::vec2(1.0, 1.0), |m, x, y| {
            let p = egui::vec2((x as f32 + 0.5 - c.x).abs(), (y as f32 + 0.5 - c.y).abs());
            let q = egui::vec2(p.x - (half.x - rad), p.y - (half.y - rad));
            let outside = egui::vec2(q.x.max(0.0), q.y.max(0.0)).length();
            let d = outside + q.x.max(q.y).min(0.0) - rad;
            let cov = (0.5 - d).clamp(0.0, 1.0);
            if cov > 0.0 {
                m.put(x, y, cov);
            }
        });
    }

    fn fill_ellipse(&mut self, r: ImgRect) {
        let c = r.center();
        let (rx, ry) = ((r.width() / 2.0).max(0.5), (r.height() / 2.0).max(0.5));
        let scale = rx.min(ry);
        self.for_box(r.min - egui::vec2(1.0, 1.0), r.max + egui::vec2(1.0, 1.0), |m, x, y| {
            let dx = (x as f32 + 0.5 - c.x) / rx;
            let dy = (y as f32 + 0.5 - c.y) / ry;
            let d = ((dx * dx + dy * dy).sqrt() - 1.0) * scale;
            let cov = (0.5 - d).clamp(0.0, 1.0);
            if cov > 0.0 {
                m.put(x, y, cov);
            }
        });
    }

    // Even-odd polygon fill with 3x3 supersampling.
    fn fill_polygon(&mut self, pts: &[egui::Pos2]) {
        if pts.len() < 3 {
            return;
        }
        let min = pts.iter().fold(pts[0], |m, p| egui::pos2(m.x.min(p.x), m.y.min(p.y)));
        let max = pts.iter().fold(pts[0], |m, p| egui::pos2(m.x.max(p.x), m.y.max(p.y)));
        let inside = |px: f32, py: f32| {
            let mut hit = false;
            let mut j = pts.len() - 1;
            for i in 0..pts.len() {
                let (a, b) = (pts[i], pts[j]);
                if (a.y > py) != (b.y > py) && px < (b.x - a.x) * (py - a.y) / (b.y - a.y) + a.x {
                    hit = !hit;
                }
                j = i;
            }
            hit
        };
        self.for_box(min, max, |m, x, y| {
            let mut n = 0;
            for sy in 0..3 {
                for sx in 0..3 {
                    if inside(x as f32 + (sx as f32 + 0.5) / 3.0, y as f32 + (sy as f32 + 0.5) / 3.0) {
                        n += 1;
                    }
                }
            }
            if n > 0 {
                m.put(x, y, n as f32 / 9.0);
            }
        });
    }

    fn composite(&self, img: &mut RgbaImage, color: Rgba<u8>) {
        for ly in 0..self.h {
            for lx in 0..self.w {
                let cov = self.data[ly * self.w + lx];
                if cov <= 0.0 {
                    continue;
                }
                let a = (color[3] as f32 * cov).round() as u8;
                if a == 0 {
                    continue;
                }
                let px = img.get_pixel_mut((self.x0 + lx as i32) as u32, (self.y0 + ly as i32) as u32);
                *px = blend(*px, Rgba([color[0], color[1], color[2], a]));
            }
        }
    }
}

fn clamp_rect(rect: ImgRect, w: u32, h: u32) -> Option<IRect> {
    let c = rect.clamp(w as f32, h as f32);
    if c.width() < 1.0 || c.height() < 1.0 {
        return None;
    }
    Some(
        IRect::at(c.min.x as i32, c.min.y as i32)
            .of_size(c.width() as u32, c.height() as u32),
    )
}

// Destructive region effect: Blur softens, Mosaic pixelates.
fn burn_region(img: &mut RgbaImage, r: IRect, kind: super::shapes::BlurKind) {
    use super::shapes::BlurKind;
    let (x, y, w, h) = (
        r.left().max(0) as u32,
        r.top().max(0) as u32,
        r.width(),
        r.height(),
    );
    if w == 0 || h == 0 {
        return;
    }
    let sub = image::imageops::crop_imm(img, x, y, w, h).to_image();
    let out = match kind {
        BlurKind::Blur => image::imageops::blur(&sub, 8.0),
        BlurKind::Mosaic => {
            let tiny_w = (w / 12).max(1);
            let tiny_h = (h / 12).max(1);
            let tiny = image::imageops::resize(
                &sub,
                tiny_w,
                tiny_h,
                image::imageops::FilterType::Nearest,
            );
            image::imageops::resize(&tiny, w, h, image::imageops::FilterType::Nearest)
        }
    };
    // The rect is pre-clamped inside the image, so the paste is in bounds.
    for (px, py, pixel) in out.enumerate_pixels() {
        img.put_pixel(x + px, y + py, *pixel);
    }
}

fn burn_font() -> Option<&'static FontArc> {
    static FONT: OnceLock<Option<FontArc>> = OnceLock::new();
    FONT.get_or_init(|| {
        let bytes = crate::font_loader::cjk_font_bytes()?;
        FontArc::try_from_vec(bytes).ok()
    })
    .as_ref()
}

// Rasterize `text` (multi-line) into a coverage image. Returns None without a font.
fn text_coverage(text: &str, font_size: f32) -> Option<GrayImage> {
    let font = burn_font()?;
    let scale = PxScale::from(font_size.max(4.0));
    let line_h = font_size * 1.2;
    let lines: Vec<&str> = text.lines().collect();
    let widest = lines
        .iter()
        .map(|l| text_size(scale, font, l).0)
        .max()
        .unwrap_or(0);
    let w = widest + 2;
    let h = (lines.len() as f32 * line_h).ceil() as u32 + 2;
    if widest == 0 || w > 16384 || h > 16384 {
        return None;
    }
    let mut gray = GrayImage::new(w, h);
    for (i, line) in lines.iter().enumerate() {
        draw_text_mut(
            &mut gray,
            Luma([255]),
            0,
            (i as f32 * line_h) as i32,
            scale,
            font,
            line,
        );
    }
    Some(gray)
}

// Blend `color` through a coverage image placed with its top-left at (x, y).
fn composite_gray(img: &mut RgbaImage, gray: &GrayImage, x: i32, y: i32, color: Rgba<u8>) {
    let (iw, ih) = (img.width() as i32, img.height() as i32);
    for (gx, gy, g) in gray.enumerate_pixels() {
        let (px, py) = (x + gx as i32, y + gy as i32);
        if g[0] == 0 || px < 0 || py < 0 || px >= iw || py >= ih {
            continue;
        }
        let a = (color[3] as f32 * g[0] as f32 / 255.0).round() as u8;
        let dst = img.get_pixel_mut(px as u32, py as u32);
        *dst = blend(*dst, Rgba([color[0], color[1], color[2], a]));
    }
}

fn burn_text(img: &mut RgbaImage, at: egui::Pos2, text: &str, font_size: f32, color: Rgba<u8>) {
    match text_coverage(text, font_size) {
        Some(gray) => composite_gray(img, &gray, at.x.round() as i32, at.y.round() as i32, color),
        None => eprintln!("annotate: no CJK font found, skipping text burn"),
    }
}

// Rasterize one object. `silhouette` skips glyphs on callouts and step stamps so
// their shadow is a plain shape.
fn burn_shape(img: &mut RgbaImage, shape: &Shape, silhouette: bool) {
    let (w, h) = (img.width(), img.height());
    match shape {
        Shape::Line { a, b, style } => {
            let pad = style.width / 2.0 + 2.0;
            if let Some(mut m) = Mask::new(shape.bounding_rect(), pad, w, h) {
                m.stroke_path(&[*a, *b], style.width, style.dashed, false);
                m.composite(img, style_rgba(*style));
            }
        }
        Shape::Arrow { a, b, style } => {
            let (shaft_end, head) = arrow_geometry(*a, *b, style.width);
            let pad = style.width * 4.0 + 8.0;
            if let Some(mut m) = Mask::new(shape.bounding_rect(), pad, w, h) {
                m.stroke_path(&[*a, shaft_end], style.width, style.dashed, false);
                m.fill_polygon(&head);
                m.composite(img, style_rgba(*style));
            }
        }
        Shape::Pencil { points, style } => {
            let pad = style.width / 2.0 + 2.0;
            if let Some(mut m) = Mask::new(shape.bounding_rect(), pad, w, h) {
                m.stroke_path(points, style.width, style.dashed, false);
                m.composite(img, style_rgba(*style));
            }
        }
        Shape::Rect { rect, style } | Shape::Ellipse { rect, style } => {
            let is_ellipse = matches!(shape, Shape::Ellipse { .. });
            let pad = style.width / 2.0 + 2.0;
            let outline = closed_outline(*rect, *style, is_ellipse);
            if style.filled {
                if let Some(mut m) = Mask::new(shape.bounding_rect(), 2.0, w, h) {
                    if style.angle != 0.0 {
                        m.fill_polygon(&outline);
                    } else if is_ellipse {
                        m.fill_ellipse(*rect);
                    } else {
                        m.fill_rect(*rect);
                    }
                    let mut fill = *style;
                    fill.color = style.fill_color;
                    m.composite(img, style_rgba(fill));
                }
            }
            if style.outline {
                if let Some(mut m) = Mask::new(shape.bounding_rect(), pad, w, h) {
                    m.stroke_path(&outline, style.width, style.dashed, true);
                    m.composite(img, style_rgba(*style));
                }
            }
        }
        Shape::Highlight { rect, style } => {
            // Translucent fill regardless of the filled flag.
            if let Some(mut m) = Mask::new(*rect, 1.0, w, h) {
                m.fill_rect(*rect);
                let mut hl = *style;
                hl.opacity = highlight_opacity(*style);
                m.composite(img, style_rgba(hl));
            }
        }
        Shape::Text { at, text, style } => {
            burn_text(img, *at, text, style.font_size, style_rgba(*style));
        }
        Shape::Callout {
            at,
            text,
            tail,
            style,
        } => {
            let rect = callout_rect(*at, text, style.font_size);
            if let Some(mut m) = Mask::new(shape.bounding_rect(), 2.0, w, h) {
                m.fill_round_rect(rect, style.font_size * 0.4);
                if let Some(poly) = callout_tail_polygon(rect, *tail) {
                    m.fill_polygon(&poly);
                }
                m.composite(img, style_rgba(*style));
            }
            if !silhouette {
                let pad = callout_padding(style.font_size);
                burn_text(
                    img,
                    egui::pos2(at.x + pad, at.y + pad),
                    text,
                    style.font_size,
                    plain_rgba(style.on_color()),
                );
            }
        }
        Shape::Step { at, number, style } => {
            if let Some(mut m) = Mask::new(shape.bounding_rect(), 2.0, w, h) {
                m.fill_ellipse(shape.bounding_rect());
                m.composite(img, style_rgba(*style));
            }
            if !silhouette {
                if let Some(gray) = text_coverage(&number.to_string(), style.font_size) {
                    let x = at.x - gray.width() as f32 / 2.0;
                    let y = at.y - style.font_size * 0.6;
                    composite_gray(
                        img,
                        &gray,
                        x.round() as i32,
                        y.round() as i32,
                        plain_rgba(style.on_color()),
                    );
                }
            }
        }
        Shape::BlurRegion { rect, kind } => {
            if let Some(r) = clamp_rect(*rect, w, h) {
                burn_region(img, r, *kind);
            }
        }
    }
}

// Rasterize shapes onto a copy of `base`. Never panics on geometry.
pub fn burn(base: &DynamicImage, shapes: &[Shape]) -> DynamicImage {
    if shapes.is_empty() {
        return base.clone();
    }
    let mut img: RgbaImage = base.to_rgba8();
    for shape in shapes {
        if let Some(style) = casts_shadow(shape) {
            burn_soft_shadow(&mut img, &shadow_of(shape, style, 1.0, 0.4), style);
        }
        burn_shape(&mut img, shape, false);
    }
    DynamicImage::ImageRgba8(img)
}

// Render an already-offset black shadow copy on a scratch layer, blur that layer
// and blend it under the object.
fn burn_soft_shadow(img: &mut RgbaImage, shadow: &Shape, style: Style) {
    let sigma = (style.shadow_offset().x * 0.8).max(1.5);
    let pad = sigma * 3.0 + style.width * 4.0 + 8.0;
    let b = shadow.bounding_rect();
    let x0 = ((b.min.x - pad).floor() as i32).max(0);
    let y0 = ((b.min.y - pad).floor() as i32).max(0);
    let x1 = ((b.max.x + pad).ceil() as i32).min(img.width() as i32);
    let y1 = ((b.max.y + pad).ceil() as i32).min(img.height() as i32);
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    let mut layer = RgbaImage::new((x1 - x0) as u32, (y1 - y0) as u32);
    let mut local = shadow.clone();
    local.translate(egui::vec2(-(x0 as f32), -(y0 as f32)));
    burn_shape(&mut layer, &local, true);
    let layer = image::imageops::blur(&layer, sigma);
    for (lx, ly, p) in layer.enumerate_pixels() {
        if p[3] == 0 {
            continue;
        }
        let dst = img.get_pixel_mut(x0 as u32 + lx, y0 as u32 + ly);
        *dst = blend(*dst, Rgba([0, 0, 0, p[3]]));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotate::shapes::BlurKind;
    use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};

    fn white_image(w: u32, h: u32) -> DynamicImage {
        DynamicImage::ImageRgba8(RgbaImage::from_pixel(w, h, Rgba([255, 255, 255, 255])))
    }

    fn checker_image(w: u32, h: u32) -> DynamicImage {
        DynamicImage::ImageRgba8(RgbaImage::from_fn(w, h, |x, y| {
            if (x / 8 + y / 8) % 2 == 0 {
                Rgba([0, 0, 0, 255])
            } else {
                Rgba([255, 255, 255, 255])
            }
        }))
    }

    fn red_style() -> Style {
        Style {
            color: egui::Color32::RED,
            ..Style::default()
        }
    }

    fn changed_pixels(a: &DynamicImage, b: &DynamicImage) -> usize {
        a.to_rgba8()
            .pixels()
            .zip(b.to_rgba8().pixels())
            .filter(|(p, q)| p != q)
            .count()
    }

    #[test]
    fn arrowhead_wings_sit_behind_tip() {
        let wings = arrow_head_points(egui::pos2(10.0, 0.0), egui::vec2(1.0, 0.0), 6.0);
        assert_eq!(wings, [egui::pos2(4.0, 3.0), egui::pos2(4.0, -3.0)]);
    }

    #[test]
    fn dash_segments_leave_gaps() {
        let path = [egui::pos2(0.0, 0.0), egui::pos2(100.0, 0.0)];
        let segs = dash_segments(&path, 10.0, 10.0);
        assert_eq!(segs.len(), 5);
        assert_eq!(segs[0], (egui::pos2(0.0, 0.0), egui::pos2(10.0, 0.0)));
        assert_eq!(segs[1].0, egui::pos2(20.0, 0.0));
    }

    #[test]
    fn burn_empty_preserves_pixels_exactly() {
        let img = white_image(100, 80);
        assert_eq!(burn(&img, &[]).to_rgba8(), img.to_rgba8());
    }

    #[test]
    fn burn_line_changes_pixels_and_keeps_size() {
        let img = white_image(100, 80);
        let shape = Shape::Line {
            a: egui::pos2(10.0, 10.0),
            b: egui::pos2(90.0, 10.0),
            style: red_style(),
        };
        let out = burn(&img, &[shape]);
        assert_eq!(out.dimensions(), (100, 80));
        assert_ne!(out.to_rgba8(), img.to_rgba8());
    }

    #[test]
    fn burn_clamps_out_of_bounds_rect() {
        let img = white_image(100, 80);
        let shape = Shape::Rect {
            rect: ImgRect {
                min: egui::pos2(-50.0, -50.0),
                max: egui::pos2(500.0, 500.0),
            },
            style: red_style(),
        };
        let out = burn(&img, &[shape]); // must not panic
        assert_eq!(out.dimensions(), (100, 80));
    }

    #[test]
    fn burn_rect_outline_honors_width() {
        let img = white_image(100, 100);
        let shape = Shape::Rect {
            rect: ImgRect {
                min: egui::pos2(20.0, 20.0),
                max: egui::pos2(80.0, 80.0),
            },
            style: Style {
                width: 8.0,
                ..red_style()
            },
        };
        let out = burn(&img, &[shape]).to_rgba8();
        // 3px inside the left edge is still part of an 8px stroke.
        assert_eq!(out.get_pixel(23, 50).0, [255, 0, 0, 255]);
        // Center stays untouched.
        assert_eq!(out.get_pixel(50, 50).0, [255, 255, 255, 255]);
    }

    #[test]
    fn burn_dashed_line_has_gaps() {
        let img = white_image(200, 20);
        let solid = Shape::Line {
            a: egui::pos2(5.0, 10.0),
            b: egui::pos2(195.0, 10.0),
            style: red_style(),
        };
        let dashed = Shape::Line {
            a: egui::pos2(5.0, 10.0),
            b: egui::pos2(195.0, 10.0),
            style: Style {
                dashed: true,
                ..red_style()
            },
        };
        let solid_px = changed_pixels(&img, &burn(&img, &[solid]));
        let dashed_px = changed_pixels(&img, &burn(&img, &[dashed]));
        assert!(dashed_px < solid_px * 3 / 4, "{dashed_px} vs {solid_px}");
        assert!(dashed_px > 0);
    }

    #[test]
    fn burn_overlapping_stroke_does_not_double_blend() {
        let img = white_image(100, 100);
        // A "V" whose two arms overlap near the apex.
        let shape = Shape::Pencil {
            points: vec![
                egui::pos2(20.0, 20.0),
                egui::pos2(50.0, 60.0),
                egui::pos2(51.0, 20.0),
                egui::pos2(50.0, 60.0),
            ],
            style: Style {
                width: 10.0,
                opacity: 0.5,
                ..red_style()
            },
        };
        let out = burn(&img, &[shape]).to_rgba8();
        let darkest_green = out.pixels().map(|p| p[1]).min().unwrap();
        // One 50% blend of red over white leaves green near 128, never ~64.
        assert!(darkest_green >= 120, "darkest green was {darkest_green}");
    }

    #[test]
    fn burn_shadow_darkens_offset_area() {
        let img = white_image(120, 120);
        let rect = ImgRect {
            min: egui::pos2(30.0, 30.0),
            max: egui::pos2(70.0, 70.0),
        };
        let plain = Shape::Rect {
            rect,
            style: Style {
                filled: true,
                ..red_style()
            },
        };
        let shadowed = Shape::Rect {
            rect,
            style: Style {
                filled: true,
                shadow: true,
                ..red_style()
            },
        };
        let a = burn(&img, &[plain]);
        let b = burn(&img, &[shadowed]);
        assert!(changed_pixels(&a, &b) > 20);
    }

    #[test]
    fn burn_ellipse_filled_and_outline_differ() {
        let img = white_image(100, 100);
        let rect = ImgRect {
            min: egui::pos2(10.0, 10.0),
            max: egui::pos2(90.0, 90.0),
        };
        let filled = burn(
            &img,
            &[Shape::Ellipse {
                rect,
                style: Style {
                    filled: true,
                    fill_color: egui::Color32::RED,
                    ..red_style()
                },
            }],
        )
        .to_rgba8();
        assert_eq!(filled.get_pixel(50, 50).0, [255, 0, 0, 255]);
        let hollow = burn(
            &img,
            &[Shape::Ellipse {
                rect,
                style: red_style(),
            }],
        )
        .to_rgba8();
        assert_eq!(hollow.get_pixel(50, 50).0, [255, 255, 255, 255]);
    }

    #[test]
    fn burn_fill_and_border_use_separate_colors() {
        let img = white_image(100, 100);
        let shape = Shape::Rect {
            rect: ImgRect {
                min: egui::pos2(20.0, 20.0),
                max: egui::pos2(80.0, 80.0),
            },
            style: Style {
                filled: true,
                fill_color: egui::Color32::from_rgb(0, 0, 255),
                width: 4.0,
                ..red_style()
            },
        };
        let out = burn(&img, &[shape]).to_rgba8();
        assert_eq!(out.get_pixel(50, 50).0, [0, 0, 255, 255]); // interior
        assert_eq!(out.get_pixel(20, 50).0, [255, 0, 0, 255]); // border
    }

    #[test]
    fn burn_borderless_fill_skips_outline() {
        let img = white_image(100, 100);
        let shape = Shape::Rect {
            rect: ImgRect {
                min: egui::pos2(20.0, 20.0),
                max: egui::pos2(80.0, 80.0),
            },
            style: Style {
                filled: true,
                outline: false,
                fill_color: egui::Color32::from_rgb(0, 0, 255),
                ..red_style()
            },
        };
        let out = burn(&img, &[shape]).to_rgba8();
        assert_eq!(out.get_pixel(21, 50).0, [0, 0, 255, 255]);
    }

    #[test]
    fn burn_rotated_rect_moves_pixels_off_axis() {
        let img = white_image(200, 200);
        let rect = ImgRect {
            min: egui::pos2(50.0, 90.0),
            max: egui::pos2(150.0, 110.0),
        };
        let shape = Shape::Rect {
            rect,
            style: Style {
                filled: true,
                outline: false,
                fill_color: egui::Color32::RED,
                angle: std::f32::consts::FRAC_PI_2,
                ..red_style()
            },
        };
        let out = burn(&img, &[shape]).to_rgba8();
        // Rotated a quarter turn, the bar is vertical around the center (100, 100).
        assert_eq!(out.get_pixel(100, 60).0, [255, 0, 0, 255]);
        assert_eq!(out.get_pixel(60, 100).0, [255, 255, 255, 255]);
    }

    #[test]
    fn burn_shadow_is_blurred_not_hard_edged() {
        let img = white_image(160, 160);
        let shape = Shape::Rect {
            rect: ImgRect {
                min: egui::pos2(40.0, 40.0),
                max: egui::pos2(90.0, 90.0),
            },
            style: Style {
                filled: true,
                outline: false,
                shadow: true,
                width: 10.0,
                ..red_style()
            },
        };
        let out = burn(&img, &[shape]).to_rgba8();
        // Walk right along the shadow strip below the object: a soft edge yields
        // many distinct gray levels, a hard-edged shadow only a couple.
        let mut levels = std::collections::BTreeSet::new();
        for x in 80..120 {
            levels.insert(out.get_pixel(x, 95).0[0]);
        }
        assert!(levels.len() > 4, "only {} gray levels", levels.len());
    }

    #[test]
    fn burn_callout_draws_box_and_tail() {
        let img = white_image(300, 200);
        let shape = Shape::Callout {
            at: egui::pos2(20.0, 20.0),
            text: "Hi".to_string(),
            tail: egui::pos2(40.0, 150.0),
            style: Style {
                color: egui::Color32::from_rgb(255, 220, 0),
                ..Style::default()
            },
        };
        let out = burn(&img, &[shape]).to_rgba8();
        assert_ne!(out.get_pixel(25, 25).0, [255, 255, 255, 255]); // box
        assert_ne!(out.get_pixel(40, 140).0, [255, 255, 255, 255]); // tail
    }

    #[test]
    fn burn_step_draws_filled_circle() {
        let img = white_image(100, 100);
        let shape = Shape::Step {
            at: egui::pos2(50.0, 50.0),
            number: 3,
            style: red_style(),
        };
        let out = burn(&img, &[shape]).to_rgba8();
        assert_ne!(out.get_pixel(50 + 17, 50).0, [255, 255, 255, 255]);
        assert_eq!(out.get_pixel(50 + 40, 50).0, [255, 255, 255, 255]);
    }

    #[test]
    fn burn_blur_region_keeps_size_and_softens_checker() {
        let img = checker_image(64, 64);
        let shape = Shape::BlurRegion {
            rect: ImgRect {
                min: egui::pos2(0.0, 0.0),
                max: egui::pos2(64.0, 64.0),
            },
            kind: BlurKind::Blur,
        };
        let out = burn(&img, &[shape]);
        assert_eq!(out.dimensions(), (64, 64));
        assert_ne!(out.to_rgba8(), img.to_rgba8());
    }

    #[test]
    fn burn_diagonal_line_is_antialiased() {
        let img = white_image(120, 100);
        let shape = Shape::Line {
            a: egui::pos2(10.0, 10.0),
            b: egui::pos2(110.0, 90.0),
            style: Style {
                width: 5.0,
                ..red_style()
            },
        };
        let out = burn(&img, &[shape]).to_rgba8();
        // Fringe = blended edge pixels: neither background-white nor solid line-red.
        let fringe = out
            .pixels()
            .filter(|p| {
                let is_white = p[0] > 240 && p[1] > 240 && p[2] > 240;
                let is_solid_red = p[0] > 200 && p[1] < 100 && p[2] < 100;
                !is_white && !is_solid_red
            })
            .count();
        assert!(
            fringe > 50,
            "expected antialiased fringe along diagonal, found {fringe}"
        );
    }

    #[test]
    fn burn_text_never_panics_and_keeps_size() {
        let img = white_image(200, 100);
        let shape = Shape::Text {
            at: egui::pos2(10.0, 10.0),
            text: "Hi 測試".to_string(),
            style: Style {
                font_size: 24.0,
                ..red_style()
            },
        };
        let out = burn(&img, &[shape]); // must not panic with or without a system font
        assert_eq!(out.dimensions(), (200, 100));
    }

    #[test]
    fn burn_text_renders_when_font_available() {
        if burn_font().is_none() {
            eprintln!("SKIP: no CJK font on this machine");
            return;
        }
        let img = white_image(200, 100);
        let shape = Shape::Text {
            at: egui::pos2(10.0, 10.0),
            text: "Hi".to_string(),
            style: Style {
                font_size: 32.0,
                ..red_style()
            },
        };
        assert_ne!(burn(&img, &[shape]).to_rgba8(), img.to_rgba8());
    }
}
