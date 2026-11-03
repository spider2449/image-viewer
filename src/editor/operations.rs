use image::{imageops, DynamicImage, RgbaImage};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResizeMode {
    Exact,
    Fit,
    Fill,
}

impl ResizeMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Exact => "Stretch",
            Self::Fit => "Fit inside",
            Self::Fill => "Fill and crop",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResizeFilter {
    Nearest,
    Triangle,
    CatmullRom,
    Gaussian,
    Lanczos3,
}

impl ResizeFilter {
    pub fn label(self) -> &'static str {
        match self {
            Self::Nearest => "Nearest",
            Self::Triangle => "Bilinear",
            Self::CatmullRom => "Bicubic",
            Self::Gaussian => "Gaussian",
            Self::Lanczos3 => "Lanczos3",
        }
    }

    fn image_filter(self) -> imageops::FilterType {
        match self {
            Self::Nearest => imageops::FilterType::Nearest,
            Self::Triangle => imageops::FilterType::Triangle,
            Self::CatmullRom => imageops::FilterType::CatmullRom,
            Self::Gaussian => imageops::FilterType::Gaussian,
            Self::Lanczos3 => imageops::FilterType::Lanczos3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorAdjustments {
    pub exposure: f32,
    pub brightness: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub hue: i32,
    pub temperature: f32,
    pub tint: f32,
    pub gamma: f32,
}

impl Default for ColorAdjustments {
    fn default() -> Self {
        Self {
            exposure: 0.0,
            brightness: 0.0,
            contrast: 0.0,
            saturation: 0.0,
            hue: 0,
            temperature: 0.0,
            tint: 0.0,
            gamma: 1.0,
        }
    }
}

impl ColorAdjustments {
    pub fn is_neutral(self) -> bool {
        self == Self::default()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum EditOp {
    Crop {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
    Rotate180,
    Rotate90Cw,
    Rotate90Ccw,
    FlipHorizontal,
    FlipVertical,
    Resize {
        width: u32,
        height: u32,
        mode: ResizeMode,
        filter: ResizeFilter,
    },
    Adjust(ColorAdjustments),
    AutoContrast,
    Grayscale,
    Sepia,
    Invert,
    Blur(f32),
    Sharpen {
        sigma: f32,
        threshold: i32,
    },
    Replace(Arc<DynamicImage>),
    #[allow(dead_code)]
    NoOp,
}

impl EditOp {
    #[allow(dead_code)]
    pub fn label(&self) -> &str {
        match self {
            EditOp::Crop { .. } => "Crop",
            EditOp::Rotate180 => "Rotate 180\u{00B0}",
            EditOp::Rotate90Cw => "Rotate 90\u{00B0} CW",
            EditOp::Rotate90Ccw => "Rotate 90\u{00B0} CCW",
            EditOp::FlipHorizontal => "Flip H",
            EditOp::FlipVertical => "Flip V",
            EditOp::Resize { .. } => "Resize",
            EditOp::Adjust(_) => "Adjust color",
            EditOp::AutoContrast => "Auto contrast",
            EditOp::Grayscale => "Grayscale",
            EditOp::Sepia => "Sepia",
            EditOp::Invert => "Invert",
            EditOp::Blur(_) => "Blur",
            EditOp::Sharpen { .. } => "Sharpen",
            EditOp::Replace(_) => "Replace image",
            EditOp::NoOp => "",
        }
    }

    pub fn apply(&self, img: &DynamicImage) -> DynamicImage {
        match self {
            EditOp::Crop {
                x,
                y,
                width,
                height,
            } => img.crop_imm(*x, *y, *width, *height),
            EditOp::Rotate180 => img.rotate180(),
            EditOp::Rotate90Cw => img.rotate90(),
            EditOp::Rotate90Ccw => img.rotate270(),
            EditOp::FlipHorizontal => img.fliph(),
            EditOp::FlipVertical => img.flipv(),
            EditOp::Resize {
                width,
                height,
                mode,
                filter,
            } => match mode {
                ResizeMode::Exact => img.resize_exact(*width, *height, filter.image_filter()),
                ResizeMode::Fit => img.resize(*width, *height, filter.image_filter()),
                ResizeMode::Fill => img.resize_to_fill(*width, *height, filter.image_filter()),
            },
            EditOp::Adjust(adjustments) => adjust_color(img, *adjustments),
            EditOp::AutoContrast => auto_contrast(img),
            EditOp::Grayscale => img.grayscale(),
            EditOp::Sepia => sepia(img),
            EditOp::Invert => {
                let mut result = img.clone();
                result.invert();
                result
            }
            EditOp::Blur(sigma) => img.blur((*sigma).max(0.1)),
            EditOp::Sharpen { sigma, threshold } => {
                img.unsharpen((*sigma).max(0.1), (*threshold).max(0))
            }
            EditOp::Replace(image) => image.as_ref().clone(),
            EditOp::NoOp => img.clone(),
        }
    }
}

fn adjust_color(img: &DynamicImage, adjustments: ColorAdjustments) -> DynamicImage {
    if adjustments.is_neutral() {
        return img.clone();
    }

    let mut rgba = img.to_rgba8();
    let exposure = 2.0_f32.powf(adjustments.exposure);
    let brightness = adjustments.brightness / 100.0;
    let contrast = 1.0 + adjustments.contrast / 100.0;
    let saturation = 1.0 + adjustments.saturation / 100.0;
    let temperature = adjustments.temperature / 100.0;
    let tint = adjustments.tint / 100.0;
    let inverse_gamma = 1.0 / adjustments.gamma.clamp(0.1, 3.0);

    for pixel in rgba.pixels_mut() {
        let alpha = pixel[3];
        let mut r = pixel[0] as f32 / 255.0;
        let mut g = pixel[1] as f32 / 255.0;
        let mut b = pixel[2] as f32 / 255.0;

        r = r * exposure + brightness + temperature * 0.12 + tint * 0.04;
        g = g * exposure + brightness - tint * 0.08;
        b = b * exposure + brightness - temperature * 0.12 + tint * 0.04;

        r = (r - 0.5) * contrast + 0.5;
        g = (g - 0.5) * contrast + 0.5;
        b = (b - 0.5) * contrast + 0.5;

        let luminance = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        r = luminance + (r - luminance) * saturation;
        g = luminance + (g - luminance) * saturation;
        b = luminance + (b - luminance) * saturation;

        pixel[0] = to_channel(r.max(0.0).powf(inverse_gamma));
        pixel[1] = to_channel(g.max(0.0).powf(inverse_gamma));
        pixel[2] = to_channel(b.max(0.0).powf(inverse_gamma));
        pixel[3] = alpha;
    }

    let adjusted = DynamicImage::ImageRgba8(rgba);
    if adjustments.hue == 0 {
        adjusted
    } else {
        adjusted.huerotate(adjustments.hue)
    }
}

fn to_channel(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn auto_contrast(img: &DynamicImage) -> DynamicImage {
    let rgba = img.to_rgba8();
    let mut minimum = [u8::MAX; 3];
    let mut maximum = [u8::MIN; 3];
    for pixel in rgba.pixels() {
        for channel in 0..3 {
            minimum[channel] = minimum[channel].min(pixel[channel]);
            maximum[channel] = maximum[channel].max(pixel[channel]);
        }
    }

    let mut output = rgba.clone();
    for pixel in output.pixels_mut() {
        for channel in 0..3 {
            let range = maximum[channel].saturating_sub(minimum[channel]);
            if range > 0 {
                pixel[channel] =
                    (((pixel[channel] - minimum[channel]) as u16 * 255) / range as u16) as u8;
            }
        }
    }
    DynamicImage::ImageRgba8(output)
}

fn sepia(img: &DynamicImage) -> DynamicImage {
    let mut rgba: RgbaImage = img.to_rgba8();
    for pixel in rgba.pixels_mut() {
        let r = pixel[0] as f32;
        let g = pixel[1] as f32;
        let b = pixel[2] as f32;
        pixel[0] = (0.393 * r + 0.769 * g + 0.189 * b).min(255.0) as u8;
        pixel[1] = (0.349 * r + 0.686 * g + 0.168 * b).min(255.0) as u8;
        pixel[2] = (0.272 * r + 0.534 * g + 0.131 * b).min(255.0) as u8;
    }
    DynamicImage::ImageRgba8(rgba)
}

// Tunable Auto Region settings, saved in config.json.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RegionParams {
    // Largest per-channel difference still counted as the same area.
    pub tolerance: u8,
    // Compare each pixel with its neighbour instead of the clicked pixel, so the
    // fill can walk along a soft gradient.
    pub follow_gradient: bool,
    // Let the fill cross diagonal pixel corners (8-way instead of 4-way).
    pub diagonal: bool,
    // Reject a region whose longer side is below this, in image px.
    pub min_size: u32,
    // Grow (positive) or shrink (negative) the result on every side, in image px.
    pub padding: i32,
}

impl Default for RegionParams {
    fn default() -> Self {
        Self {
            tolerance: 32,
            follow_gradient: false,
            diagonal: false,
            min_size: 4,
            padding: 0,
        }
    }
}

// Magic-wand detection: flood-fill the connected pixels similar to the seed and
// return their bounding box as (x, y, width, height). Returns None for a region under
// `min_size`, when the fill spans the entire image (the click landed on an open
// background), or when negative padding leaves nothing.
pub fn detect_region(
    image: &DynamicImage,
    seed: (u32, u32),
    params: &RegionParams,
) -> Option<(u32, u32, u32, u32)> {
    let rgba = image.to_rgba8();
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
    if w == 0 || h == 0 {
        return None;
    }
    let px = rgba.as_raw();
    let start = (seed.1 as usize).min(h - 1) * w + (seed.0 as usize).min(w - 1);
    let similar = |a: usize, b: usize| {
        px[a * 4..a * 4 + 4]
            .iter()
            .zip(&px[b * 4..b * 4 + 4])
            .all(|(a, b)| a.abs_diff(*b) <= params.tolerance)
    };

    let mut visited = vec![false; w * h];
    visited[start] = true;
    let mut stack = vec![start];
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (w, h, 0, 0);
    while let Some(i) = stack.pop() {
        let (x, y) = (i % w, i / w);
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
        let reference = if params.follow_gradient { i } else { start };
        for (dx, dy) in NEIGHBOURS {
            if !params.diagonal && dx != 0 && dy != 0 {
                continue;
            }
            let (Some(nx), Some(ny)) = (x.checked_add_signed(dx), y.checked_add_signed(dy)) else {
                continue;
            };
            if nx >= w || ny >= h {
                continue;
            }
            let n = ny * w + nx;
            if !visited[n] && similar(n, reference) {
                visited[n] = true;
                stack.push(n);
            }
        }
    }

    let (rw, rh) = (max_x - min_x + 1, max_y - min_y + 1);
    if rw.max(rh) < params.min_size as usize || (rw == w && rh == h) {
        return None;
    }
    // Padding is applied last and clamped, so it never leaves the image.
    let pad = params.padding as i64;
    let x0 = (min_x as i64 - pad).max(0);
    let y0 = (min_y as i64 - pad).max(0);
    let x1 = (max_x as i64 + 1 + pad).min(w as i64);
    let y1 = (max_y as i64 + 1 + pad).min(h as i64);
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    Some((x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32))
}

const NEIGHBOURS: [(isize, isize); 8] = [
    (-1, 0),
    (1, 0),
    (0, -1),
    (0, 1),
    (-1, -1),
    (1, -1),
    (-1, 1),
    (1, 1),
];

#[cfg(test)]
mod tests {
    use super::*;
    use image::{GenericImageView, Rgba};

    // 20x20 white image with a black outlined box whose inside is grey.
    fn boxed_image() -> DynamicImage {
        DynamicImage::ImageRgba8(RgbaImage::from_fn(20, 20, |x, y| {
            let inside = (5..15).contains(&x) && (4..12).contains(&y);
            let border = inside && (x == 5 || x == 14 || y == 4 || y == 11);
            Rgba(match (inside, border) {
                (_, true) => [0, 0, 0, 255],
                (true, false) => [120, 120, 120, 255],
                _ => [255, 255, 255, 255],
            })
        }))
    }

    fn with_tolerance(tolerance: u8) -> RegionParams {
        RegionParams {
            tolerance,
            ..RegionParams::default()
        }
    }

    #[test]
    fn detect_region_finds_the_enclosed_area_bounds() {
        let params = with_tolerance(10);
        assert_eq!(detect_region(&boxed_image(), (9, 8), &params), Some((6, 5, 8, 6)));
    }

    #[test]
    fn detect_region_padding_grows_shrinks_and_clamps() {
        let padded = |padding| RegionParams {
            padding,
            ..with_tolerance(10)
        };
        let image = boxed_image();
        assert_eq!(detect_region(&image, (9, 8), &padded(2)), Some((4, 3, 12, 10)));
        assert_eq!(detect_region(&image, (9, 8), &padded(-1)), Some((7, 6, 6, 4)));
        assert_eq!(detect_region(&image, (9, 8), &padded(-10)), None);
        assert_eq!(detect_region(&image, (9, 8), &padded(50)), Some((0, 0, 20, 20)));
    }

    #[test]
    fn detect_region_min_size_rejects_small_areas() {
        let params = |min_size| RegionParams {
            min_size,
            ..with_tolerance(10)
        };
        assert_eq!(detect_region(&boxed_image(), (9, 8), &params(9)), None);
        assert!(detect_region(&boxed_image(), (9, 8), &params(8)).is_some());
    }

    #[test]
    fn detect_region_diagonal_setting_crosses_pixel_corners() {
        let image = DynamicImage::ImageRgba8(RgbaImage::from_fn(20, 20, |x, y| {
            Rgba(if x == y && (5..12).contains(&x) {
                [0, 0, 0, 255]
            } else {
                [255, 255, 255, 255]
            })
        }));
        assert_eq!(detect_region(&image, (5, 5), &with_tolerance(10)), None);
        let diagonal = RegionParams {
            diagonal: true,
            ..with_tolerance(10)
        };
        assert_eq!(detect_region(&image, (5, 5), &diagonal), Some((5, 5, 7, 7)));
    }

    #[test]
    fn detect_region_follow_gradient_walks_a_soft_ramp() {
        let image = DynamicImage::ImageRgba8(RgbaImage::from_fn(20, 20, |x, _| {
            let v = if x < 10 { (x * 10) as u8 } else { 255 };
            Rgba([v, v, v, 255])
        }));
        assert_eq!(detect_region(&image, (0, 0), &with_tolerance(15)), Some((0, 0, 2, 20)));
        let follow = RegionParams {
            follow_gradient: true,
            ..with_tolerance(15)
        };
        assert_eq!(detect_region(&image, (0, 0), &follow), Some((0, 0, 10, 20)));
    }

    #[test]
    fn detect_region_rejects_open_background_and_specks() {
        assert_eq!(detect_region(&boxed_image(), (0, 0), &with_tolerance(10)), None);
        let speck = DynamicImage::ImageRgba8(RgbaImage::from_fn(20, 20, |x, y| {
            Rgba(if (x, y) == (9, 9) {
                [0, 0, 0, 255]
            } else {
                [255, 255, 255, 255]
            })
        }));
        assert_eq!(detect_region(&speck, (9, 9), &with_tolerance(10)), None);
    }

    #[test]
    fn detect_region_tolerance_decides_what_merges() {
        // Inside and border differ by 120: a high tolerance swallows the border.
        let params = with_tolerance(130);
        assert_eq!(detect_region(&boxed_image(), (9, 8), &params), Some((5, 4, 10, 8)));
    }

    #[test]
    fn detect_region_clamps_a_seed_on_the_far_edge() {
        let image = DynamicImage::ImageRgba8(RgbaImage::from_fn(10, 10, |x, _| {
            Rgba(if x >= 6 {
                [0, 0, 0, 255]
            } else {
                [255, 255, 255, 255]
            })
        }));
        assert_eq!(detect_region(&image, (10, 10), &with_tolerance(10)), Some((6, 0, 4, 10)));
    }

    fn sample_image() -> DynamicImage {
        DynamicImage::ImageRgba8(RgbaImage::from_fn(2, 1, |x, _| {
            if x == 0 {
                Rgba([40, 80, 120, 77])
            } else {
                Rgba([140, 180, 220, 155])
            }
        }))
    }

    #[test]
    fn rotations_and_flips_preserve_expected_dimensions() {
        let img = DynamicImage::new_rgba8(10, 20);
        assert_eq!(EditOp::Rotate180.apply(&img).dimensions(), (10, 20));
        assert_eq!(EditOp::Rotate90Cw.apply(&img).dimensions(), (20, 10));
        assert_eq!(EditOp::Rotate90Ccw.apply(&img).dimensions(), (20, 10));
        assert_eq!(EditOp::FlipHorizontal.apply(&img).dimensions(), (10, 20));
        assert_eq!(EditOp::FlipVertical.apply(&img).dimensions(), (10, 20));
    }

    #[test]
    fn resize_modes_produce_expected_dimensions() {
        let img = DynamicImage::new_rgba8(200, 100);
        let resize = |mode| {
            EditOp::Resize {
                width: 100,
                height: 100,
                mode,
                filter: ResizeFilter::Nearest,
            }
            .apply(&img)
        };
        assert_eq!(resize(ResizeMode::Exact).dimensions(), (100, 100));
        assert_eq!(resize(ResizeMode::Fit).dimensions(), (100, 50));
        assert_eq!(resize(ResizeMode::Fill).dimensions(), (100, 100));
    }

    #[test]
    fn neutral_adjustment_preserves_pixels_exactly() {
        let img = sample_image();
        assert_eq!(
            EditOp::Adjust(ColorAdjustments::default())
                .apply(&img)
                .to_rgba8(),
            img.to_rgba8()
        );
    }

    #[test]
    fn adjustment_changes_color_and_preserves_alpha() {
        let img = sample_image();
        let adjusted = EditOp::Adjust(ColorAdjustments {
            exposure: 1.0,
            saturation: -50.0,
            hue: 30,
            temperature: 30.0,
            ..Default::default()
        })
        .apply(&img)
        .to_rgba8();
        assert_ne!(adjusted.get_pixel(0, 0), img.to_rgba8().get_pixel(0, 0));
        assert_eq!(adjusted.get_pixel(0, 0)[3], 77);
        assert_eq!(adjusted.get_pixel(1, 0)[3], 155);
    }

    #[test]
    fn auto_contrast_expands_each_channel() {
        let result = EditOp::AutoContrast.apply(&sample_image()).to_rgba8();
        assert_eq!(&result.get_pixel(0, 0).0[..3], &[0, 0, 0]);
        assert_eq!(&result.get_pixel(1, 0).0[..3], &[255, 255, 255]);
    }

    #[test]
    fn effects_preserve_dimensions_and_alpha() {
        let img = sample_image();
        for op in [EditOp::Grayscale, EditOp::Sepia, EditOp::Invert] {
            let result = op.apply(&img);
            assert_eq!(result.dimensions(), img.dimensions());
            assert_eq!(result.to_rgba8().get_pixel(0, 0)[3], 77);
        }
        for op in [
            EditOp::Blur(1.0),
            EditOp::Sharpen {
                sigma: 1.0,
                threshold: 1,
            },
        ] {
            assert_eq!(op.apply(&img).dimensions(), img.dimensions());
        }
    }

    #[test]
    fn noop_preserves_dimensions() {
        let img = DynamicImage::new_rgba8(10, 20);
        assert_eq!(EditOp::NoOp.apply(&img).dimensions(), (10, 20));
    }
}
