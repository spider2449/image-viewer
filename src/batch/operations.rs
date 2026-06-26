use image::GenericImageView;
use std::path::PathBuf;

pub fn batch_convert(
    files: &[PathBuf],
    format: &str,
    jpeg_quality: u8,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    for path in files {
        let img = match image::open(path) {
            Ok(i) => i,
            Err(e) => { errors.push(format!("{}: {e}", path.display())); continue; }
        };
        let new_ext = if format == "jpeg" { "jpg" } else { format };
        let new_name = path.with_extension(new_ext);
        let result = match format {
            "jpeg" => {
                let file = match std::fs::File::create(&new_name) {
                    Ok(f) => f,
                    Err(e) => { errors.push(format!("{}: {e}", new_name.display())); continue; }
                };
                let (w, h) = img.dimensions();
                let rgba = img.to_rgba8();
                let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(file, jpeg_quality);
                encoder.encode(&rgba, w, h, image::ExtendedColorType::Rgba8).map_err(|e| e.to_string())
            }
            "png" => img.save_with_format(&new_name, image::ImageFormat::Png).map_err(|e| e.to_string()),
            "bmp" => img.save_with_format(&new_name, image::ImageFormat::Bmp).map_err(|e| e.to_string()),
            "webp" => img.save_with_format(&new_name, image::ImageFormat::WebP).map_err(|e| e.to_string()),
            _ => Err(format!("Unknown format: {format}")),
        };
        if let Err(e) = result {
            errors.push(format!("{}: {e}", path.display()));
        }
    }
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

pub fn batch_rename(
    files: &[PathBuf],
    pattern: &str,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    for (i, path) in files.iter().enumerate() {
        let stem = path.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let ext = path.extension()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let new_stem = pattern
            .replace("{n}", &format!("{:03}", i + 1))
            .replace("{name}", &stem);
        let new_name = path.with_file_name(format!("{}.{}", new_stem, ext));
        if new_name == *path {
            continue;
        }
        if new_name.exists() {
            errors.push(format!("{} already exists", new_name.display()));
            continue;
        }
        if std::fs::rename(path, &new_name).is_err() {
            errors.push(format!("Failed to rename {}", path.display()));
        }
    }
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

pub fn batch_resize(
    files: &[PathBuf],
    width: u32,
    height: u32,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    for path in files {
        let img = match image::open(path) {
            Ok(i) => i,
            Err(e) => { errors.push(format!("{}: {e}", path.display())); continue; }
        };
        let resized = img.resize_exact(width, height, image::imageops::FilterType::Lanczos3);
        let ext = path.extension()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let save_result = match ext.as_str() {
            "jpg" | "jpeg" => {
                let file = match std::fs::File::create(path) {
                    Ok(f) => f,
                    Err(e) => { errors.push(format!("{}: {e}", path.display())); continue; }
                };
                let (w, h) = resized.dimensions();
                let rgba = resized.to_rgba8();
                let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(file, 90);
                encoder.encode(&rgba, w, h, image::ExtendedColorType::Rgba8).map_err(|e| e.to_string())
            }
            _ => resized.save(path).map_err(|e| e.to_string()),
        };
        if let Err(e) = save_result {
            errors.push(format!("{}: {e}", path.display()));
        }
    }
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}
