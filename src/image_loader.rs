use egui::{ColorImage, TextureHandle, TextureOptions};
use image::{DynamicImage, GenericImageView};
use std::collections::HashMap;
use std::path::Path;

pub fn decode_to_colorimage(path: &Path) -> Result<(ColorImage, u32, u32, u8), String> {
    let img = image::open(path).map_err(|e| format!("Failed to decode: {e}"))?;
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
    let rgba = img.to_rgba8();
    let ci = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
    Ok((ci, w, h, bit_depth))
}

pub fn load_to_texture(
    cc: &egui::Context,
    textures: &mut HashMap<String, TextureHandle>,
    path: &Path,
) -> Result<(TextureHandle, u32, u32, u8), String> {
    let key = path.to_string_lossy().to_string();
    let ci = decode_to_colorimage(path)?;
    let tex = cc.load_texture(&key, ci.0, TextureOptions::LINEAR);
    textures.insert(key.clone(), tex.clone());
    Ok((tex, ci.1, ci.2, ci.3))
}

pub fn load_thumbnail(
    path: &Path,
    max_size: u32,
) -> Result<(ColorImage, u32, u32), String> {
    let img = image::open(path).map_err(|e| format!("Failed to decode thumb: {e}"))?;
    let (w, h) = img.dimensions();
    let scale = (max_size as f32 / w.max(h) as f32).min(1.0);
    let new_w = (w as f32 * scale) as u32;
    let new_h = (h as f32 * scale) as u32;
    let thumb = img.resize_exact(new_w.max(1), new_h.max(1), image::imageops::FilterType::Lanczos3);
    let rgba = thumb.to_rgba8();
    let ci = ColorImage::from_rgba_unmultiplied([new_w.max(1) as usize, new_h.max(1) as usize], &rgba);
    Ok((ci, w, h))
}
