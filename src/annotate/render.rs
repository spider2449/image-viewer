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

use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
use imageproc::drawing::{
    draw_filled_ellipse_mut, draw_filled_rect_mut, draw_hollow_ellipse_mut,
    draw_hollow_rect_mut, draw_polygon_mut,
};
use imageproc::rect::Rect as IRect;

// Standard-alpha blend of src over dst (both RGBA).
fn blend(dst: Rgba<u8>, src: Rgba<u8>) -> Rgba<u8> {
    let a = src[3] as f32 / 255.0;
    Rgba([
        (src[0] as f32 * a + dst[0] as f32 * (1.0 - a)) as u8,
        (src[1] as f32 * a + dst[1] as f32 * (1.0 - a)) as u8,
        (src[2] as f32 * a + dst[2] as f32 * (1.0 - a)) as u8,
        dst[3].max(src[3]),
    ])
}

fn to_rgba(c: egui::Color32) -> Rgba<u8> {
    Rgba([c.r(), c.g(), c.b(), c.a()])
}

fn clamp_rect(rect: super::shapes::ImgRect, w: u32, h: u32) -> Option<IRect> {
    let c = rect.clamp(w as f32, h as f32);
    if c.width() < 1.0 || c.height() < 1.0 {
        return None;
    }
    Some(
        IRect::at(c.min.x as i32, c.min.y as i32)
            .of_size(c.width() as u32, c.height() as u32),
    )
}

// Single-shape fast path reused for Line/Arrow-shaft/Pencil segments.
fn burn_one(img: &mut RgbaImage, shape: &super::shapes::Shape) {
    use super::shapes::Shape;
    let Shape::Line { a, b, style } = shape else {
        return;
    };
    let color = to_rgba(style.effective_color());
    let dist = (b.x - a.x).hypot(b.y - a.y);
    let steps = (dist / 1.0).ceil().max(1.0) as i32;
    let r = (style.width / 2.0).max(0.5) as i32;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        draw_filled_circle_blend(
            img,
            (
                (a.x + (b.x - a.x) * t) as i32,
                (a.y + (b.y - a.y) * t) as i32,
            ),
            r,
            color,
        );
    }
}

// Disc stamp with per-pixel blending; skips out-of-bounds pixels.
fn draw_filled_circle_blend(img: &mut RgbaImage, center: (i32, i32), radius: i32, color: Rgba<u8>) {
    let (w, h) = (img.width() as i32, img.height() as i32);
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dy * dy > radius * radius {
                continue;
            }
            let (x, y) = (center.0 + dx, center.1 + dy);
            if x < 0 || y < 0 || x >= w || y >= h {
                continue;
            }
            let px = img.get_pixel_mut(x as u32, y as u32);
            *px = blend(*px, color);
        }
    }
}

// Blend a solid color over every pixel inside `r`.
fn blend_rect(img: &mut RgbaImage, r: IRect, color: Rgba<u8>) {
    let (w, h) = (img.width() as i32, img.height() as i32);
    for y in r.top()..r.bottom() {
        for x in r.left()..r.right() {
            if x < 0 || y < 0 || x >= w || y >= h {
                continue;
            }
            let px = img.get_pixel_mut(x as u32, y as u32);
            *px = blend(*px, color);
        }
    }
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

// Rasterize shapes onto a copy of `base`. Never panics on geometry.
pub fn burn(base: &DynamicImage, shapes: &[super::shapes::Shape]) -> DynamicImage {
    use super::shapes::Shape;
    if shapes.is_empty() {
        return base.clone();
    }
    let (w, h) = base.dimensions();
    let mut img: RgbaImage = base.to_rgba8();
    for shape in shapes {
        match shape {
            Shape::Line { .. } => {
                burn_one(&mut img, shape);
            }
            Shape::Arrow { a, b, style } => {
                // Shaft reuses the Line path; head is a filled triangle.
                let shaft = Shape::Line {
                    a: *a,
                    b: *b,
                    style: *style,
                };
                burn_one(&mut img, &shaft);
                let len = (b.x - a.x).hypot(b.y - a.y).max(1e-6);
                let dir = egui::vec2((b.x - a.x) / len, (b.y - a.y) / len);
                let size = style.width * 4.0 + 6.0;
                let [w1, w2] = arrow_head_points(*b, dir, size);
                let tri = vec![
                    imageproc::point::Point::new(b.x as i32, b.y as i32),
                    imageproc::point::Point::new(w1.x as i32, w1.y as i32),
                    imageproc::point::Point::new(w2.x as i32, w2.y as i32),
                ];
                draw_polygon_mut(&mut img, &tri, to_rgba(style.effective_color()));
            }
            Shape::Pencil { points, style } => {
                for win in points.windows(2) {
                    let seg = Shape::Line {
                        a: win[0],
                        b: win[1],
                        style: *style,
                    };
                    burn_one(&mut img, &seg);
                }
            }
            Shape::Rect { rect, style } => {
                let Some(r) = clamp_rect(*rect, w, h) else {
                    continue;
                };
                let color = to_rgba(style.effective_color());
                if style.filled {
                    draw_filled_rect_mut(&mut img, r, color);
                } else {
                    draw_hollow_rect_mut(&mut img, r, color);
                }
            }
            Shape::Ellipse { rect, style } => {
                let Some(r) = clamp_rect(*rect, w, h) else {
                    continue;
                };
                let center = (
                    r.left() + r.width() as i32 / 2,
                    r.top() + r.height() as i32 / 2,
                );
                let (rx, ry) = (r.width() as i32 / 2, r.height() as i32 / 2);
                let color = to_rgba(style.effective_color());
                if style.filled {
                    draw_filled_ellipse_mut(&mut img, center, rx, ry, color);
                } else {
                    draw_hollow_ellipse_mut(&mut img, center, rx, ry, color);
                }
            }
            Shape::Highlight { rect, style } => {
                // Translucent fill regardless of the filled flag.
                let Some(r) = clamp_rect(*rect, w, h) else {
                    continue;
                };
                let mut hl = *style;
                hl.opacity = (hl.opacity * 0.4_f32).clamp(0.05_f32, 0.6_f32);
                blend_rect(&mut img, r, to_rgba(hl.effective_color()));
            }
            Shape::Text { .. } => {
                // Text rendering lands in Task 6 (needs the burn font);
                // until then Text shapes are skipped so every commit compiles.
            }
            Shape::BlurRegion { rect, kind } => {
                let Some(r) = clamp_rect(*rect, w, h) else {
                    continue;
                };
                burn_region(&mut img, r, *kind);
            }
        }
    }
    DynamicImage::ImageRgba8(img)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotate::shapes::{BlurKind, ImgRect};
    use image::{DynamicImage, Rgba, RgbaImage};

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

    #[test]
    fn arrowhead_wings_sit_behind_tip() {
        let wings = arrow_head_points(egui::pos2(10.0, 0.0), egui::vec2(1.0, 0.0), 6.0);
        assert_eq!(wings, [egui::pos2(4.0, 3.0), egui::pos2(4.0, -3.0)]);
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
}
