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

use ab_glyph::{FontArc, PxScale};
use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
use imageproc::drawing::{
    draw_filled_ellipse_mut, draw_filled_rect_mut, draw_hollow_ellipse_mut,
    draw_hollow_rect_mut, draw_text_mut,
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
    let steps = (dist / 0.5).ceil().max(1.0) as i32;
    let r = (style.width / 2.0).max(0.5);
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        draw_soft_disc(
            img,
            (a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t),
            r,
            color,
        );
    }
}

// Soft-edge disc stamp with ~1px radial falloff at the rim, so stroked
// paths get antialiased edges. Sub-pixel centers; skips out-of-bounds.
fn draw_soft_disc(img: &mut RgbaImage, center: (f32, f32), radius: f32, color: Rgba<u8>) {
    let (w, h) = (img.width() as i32, img.height() as i32);
    let r = radius.max(0.5);
    let x0 = (center.0 - r - 1.0).floor() as i32;
    let x1 = (center.0 + r + 1.0).ceil() as i32;
    let y0 = (center.1 - r - 1.0).floor() as i32;
    let y1 = (center.1 + r + 1.0).ceil() as i32;
    for y in y0..=y1 {
        for x in x0..=x1 {
            if x < 0 || y < 0 || x >= w || y >= h {
                continue;
            }
            // Distance from the pixel center to the stamp center.
            let d = (x as f32 + 0.5 - center.0).hypot(y as f32 + 0.5 - center.1);
            let coverage = (r + 0.5 - d).clamp(0.0, 1.0);
            if coverage <= 0.0 {
                continue;
            }
            let px = img.get_pixel_mut(x as u32, y as u32);
            let soft = Rgba([
                color[0],
                color[1],
                color[2],
                (color[3] as f32 * coverage) as u8,
            ]);
            *px = blend(*px, soft);
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

fn load_burn_font() -> Option<FontArc> {
    let bytes = crate::font_loader::cjk_font_bytes()?;
    FontArc::try_from_vec(bytes).ok()
}

fn burn_text(img: &mut RgbaImage, at: egui::Pos2, text: &str, style: super::shapes::Style) {
    let Some(font) = load_burn_font() else {
        eprintln!("annotate: no CJK font found, skipping text burn");
        return;
    };
    let scale = PxScale::from(style.font_size.max(4.0));
    let color = to_rgba(style.effective_color());
    let mut y = at.y.max(0.0);
    for line in text.lines() {
        draw_text_mut(
            img,
            color,
            at.x.max(0.0) as i32,
            y as i32,
            scale,
            &font,
            line,
        );
        y += style.font_size * 1.2;
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
                // Shaft reuses the Line path; the head is stroked as two
                // short wing segments so it gets the same antialiasing.
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
                for wing in [w1, w2] {
                    let edge = Shape::Line {
                        a: *b,
                        b: wing,
                        style: *style,
                    };
                    burn_one(&mut img, &edge);
                }
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
            Shape::Text { at, text, style } => {
                burn_text(&mut img, *at, text, *style);
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
        if load_burn_font().is_none() {
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
