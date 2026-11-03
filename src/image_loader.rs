use egui::{ColorImage, TextureFilter, TextureHandle, TextureOptions};
use image::{DynamicImage, GenericImageView, ImageFormat, RgbaImage};
use lru::LruCache;
use std::path::Path;
use std::sync::OnceLock;

const SRGB_LUT_SIZE: usize = 1 << 14;

// Magnified texels stay sharp: linear filtering would blend fine detail such as
// one-pixel lines into each other. Shrinking still uses linear filtering.
pub const VIEWER_TEXTURE_OPTIONS: TextureOptions = TextureOptions {
    magnification: TextureFilter::Nearest,
    ..TextureOptions::LINEAR
};

fn decode_raw(bytes: &[u8]) -> image::ImageResult<DynamicImage> {
    image::load_from_memory(bytes)
        .or_else(|_| image::load_from_memory_with_format(bytes, ImageFormat::Png))
        .or_else(|_| image::load_from_memory_with_format(bytes, ImageFormat::Jpeg))
}

fn linear_to_srgb(value: f32) -> f32 {
    if value <= 0.003_130_8 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
}

// A table keeps full-resolution conversion cheap: decoding runs on the UI thread.
fn encode_srgb8(value: f32) -> u8 {
    static LUT: OnceLock<Vec<u8>> = OnceLock::new();
    let lut = LUT.get_or_init(|| {
        (0..SRGB_LUT_SIZE)
            .map(|i| {
                let linear = i as f32 / (SRGB_LUT_SIZE - 1) as f32;
                (linear_to_srgb(linear) * 255.0 + 0.5) as u8
            })
            .collect()
    });
    // NaN saturates to index 0 and +inf clamps to the top entry.
    lut[(value.clamp(0.0, 1.0) * (SRGB_LUT_SIZE - 1) as f32 + 0.5) as usize]
}

// EXR stores scene-linear, premultiplied values. Values above 1.0 clip to white.
fn exr_to_srgb8(img: &DynamicImage) -> DynamicImage {
    let source = img.to_rgba32f();
    let (width, height) = source.dimensions();
    let mut output = RgbaImage::new(width, height);
    for (src, dst) in source.pixels().zip(output.pixels_mut()) {
        let [r, g, b, a] = src.0;
        let unpremultiply = if a > 0.0 { 1.0 / a } else { 1.0 };
        dst.0 = [
            encode_srgb8(r * unpremultiply),
            encode_srgb8(g * unpremultiply),
            encode_srgb8(b * unpremultiply),
            (a.clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
        ];
    }
    DynamicImage::ImageRgba8(output)
}

fn to_display_image(img: DynamicImage, bytes: &[u8]) -> DynamicImage {
    if matches!(image::guess_format(bytes), Ok(ImageFormat::OpenExr)) {
        exr_to_srgb8(&img)
    } else {
        img
    }
}

// Every code path that shows, edits or converts a file decodes through here,
// so EXR files look the same in the viewer, editor, thumbnails and exports.
pub fn open_image(path: &Path) -> Result<DynamicImage, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Failed to read: {e}"))?;
    let img = decode_raw(&bytes).map_err(|e| format!("Failed to decode: {e}"))?;
    Ok(to_display_image(img, &bytes))
}

pub fn decode_to_colorimage(path: &Path) -> Result<(ColorImage, u32, u32, u8), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Failed to read: {e}"))?;
    let img = decode_raw(&bytes).map_err(|e| format!("Failed to decode: {e}"))?;
    let (w, h) = img.dimensions();
    let bit_depth = match img {
        DynamicImage::ImageLuma8(_) => 8,
        DynamicImage::ImageLumaA8(_) => 8,
        DynamicImage::ImageRgb8(_) => 24,
        DynamicImage::ImageRgba8(_) => 32,
        DynamicImage::ImageLuma16(_) => 16,
        DynamicImage::ImageLumaA16(_) => 16,
        DynamicImage::ImageRgb16(_) => 48,
        DynamicImage::ImageRgba16(_) => 64,
        DynamicImage::ImageRgb32F(_) => 96,
        DynamicImage::ImageRgba32F(_) => 128,
        _ => 24,
    };
    let rgba = to_display_image(img, &bytes).to_rgba8();
    let ci = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
    Ok((ci, w, h, bit_depth))
}

pub fn load_to_texture(
    cc: &egui::Context,
    textures: &mut LruCache<String, TextureHandle>,
    path: &Path,
) -> Result<(TextureHandle, u32, u32, u8), String> {
    let key = path.to_string_lossy().to_string();
    let ci = decode_to_colorimage(path)?;
    let tex = cc.load_texture(&key, ci.0, VIEWER_TEXTURE_OPTIONS);
    textures.put(key.clone(), tex.clone());
    Ok((tex, ci.1, ci.2, ci.3))
}

pub fn load_thumbnail(
    path: &Path,
    max_size: u32,
) -> Result<(ColorImage, u32, u32), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Failed to read: {e}"))?;
    let img = decode_raw(&bytes).map_err(|e| format!("Failed to decode thumb: {e}"))?;
    let img = to_display_image(img, &bytes);
    let (w, h) = img.dimensions();
    let scale = (max_size as f32 / w.max(h) as f32).min(1.0);
    let new_w = (w as f32 * scale) as u32;
    let new_h = (h as f32 * scale) as u32;
    let thumb = img.thumbnail(new_w.max(1), new_h.max(1));
    let (actual_w, actual_h) = thumb.dimensions();
    let rgba = thumb.to_rgba8();
    let ci = ColorImage::from_rgba_unmultiplied([actual_w as usize, actual_h as usize], &rgba);
    Ok((ci, w, h))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, Rgb32FImage, Rgba, Rgba32FImage};

    fn converted(pixel: [f32; 4]) -> [u8; 4] {
        let img = DynamicImage::ImageRgba32F(Rgba32FImage::from_pixel(1, 1, Rgba(pixel)));
        exr_to_srgb8(&img).to_rgba8().get_pixel(0, 0).0
    }

    #[test]
    fn test_encode_srgb8_matches_transfer_function() {
        assert_eq!(encode_srgb8(0.0), 0);
        assert_eq!(encode_srgb8(1.0), 255);
        assert_eq!(encode_srgb8(0.5), 188);
        for i in 0..=1000 {
            let linear = i as f32 / 1000.0;
            let exact = linear_to_srgb(linear) * 255.0;
            let diff = (encode_srgb8(linear) as f32 - exact).abs();
            assert!(diff <= 1.0, "linear {linear}: table is off by {diff}");
        }
    }

    #[test]
    fn test_exr_to_srgb8_handles_out_of_range_values() {
        assert_eq!(
            converted([4.0, f32::INFINITY, f32::NAN, 1.0]),
            [255, 255, 0, 255]
        );
        assert_eq!(converted([-1.0, 0.0, 0.0, 1.0]), [0, 0, 0, 255]);
    }

    #[test]
    fn test_exr_to_srgb8_unpremultiplies_before_encoding() {
        // Linear 0.25 at 50% alpha is a straight linear 0.5.
        assert_eq!(converted([0.25, 0.25, 0.25, 0.5]), [188, 188, 188, 128]);
        assert_eq!(converted([0.0, 0.0, 0.0, 0.0]), [0, 0, 0, 0]);
    }

    #[test]
    fn test_exr_file_decodes_to_srgb_on_every_path() {
        let dir = std::env::temp_dir().join("image_loader_test_exr");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("gray.exr");
        DynamicImage::ImageRgb32F(Rgb32FImage::from_pixel(4, 4, Rgb([0.5, 0.5, 0.5])))
            .save_with_format(&path, ImageFormat::OpenExr)
            .unwrap();

        let expected = egui::Color32::from_rgb(188, 188, 188);
        let (viewer, _, _, bit_depth) = decode_to_colorimage(&path).unwrap();
        assert_eq!(viewer.pixels[0], expected);
        assert_eq!(bit_depth, 96);
        assert_eq!(load_thumbnail(&path, 2).unwrap().0.pixels[0], expected);
        assert_eq!(
            open_image(&path).unwrap().to_rgba8().get_pixel(0, 0).0,
            [188, 188, 188, 255]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_non_exr_bytes_are_left_untouched() {
        let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(1, 1, Rgba([128, 64, 32, 255])));
        let result = to_display_image(img.clone(), b"\x89PNG\r\n\x1a\n");
        assert_eq!(result.to_rgba8(), img.to_rgba8());
    }
}
