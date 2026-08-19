use image::{imageops, DynamicImage, RgbaImage};

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

#[cfg(test)]
mod tests {
    use super::*;
    use image::{GenericImageView, Rgba};

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
