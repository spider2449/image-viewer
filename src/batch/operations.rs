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
                let rgb = img.to_rgb8();
                let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(file, jpeg_quality);
                encoder.encode(&rgb, w, h, image::ExtendedColorType::Rgb8).map_err(|e| e.to_string())
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
                let rgb = resized.to_rgb8();
                let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(file, 90);
                encoder.encode(&rgb, w, h, image::ExtendedColorType::Rgb8).map_err(|e| e.to_string())
            }
            _ => resized.save(path).map_err(|e| e.to_string()),
        };
        if let Err(e) = save_result {
            errors.push(format!("{}: {e}", path.display()));
        }
    }
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_batch_convert_png_to_jpeg() {
        let dir = std::env::temp_dir().join("batch_test_convert");
        let _ = std::fs::create_dir_all(&dir);
        let src = dir.join("test.png");
        let img = image::DynamicImage::new_rgba8(10, 10);
        img.save(&src).unwrap();

        let result = batch_convert(&[src.clone()], "jpeg", 90);
        assert!(result.is_ok());

        let dst = src.with_extension("jpg");
        assert!(dst.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_batch_convert_invalid_file() {
        let result = batch_convert(&[PathBuf::from("nonexistent.png")], "png", 90);
        assert!(result.is_err());
    }

    #[test]
    fn test_batch_rename_sequence() {
        let dir = std::env::temp_dir().join("batch_test_rename");
        let _ = std::fs::create_dir_all(&dir);
        let files: Vec<PathBuf> = (1..=3).map(|i| {
            let p = dir.join(format!("img{i}.png"));
            image::DynamicImage::new_rgba8(10, 10).save(&p).unwrap();
            p
        }).collect();

        let result = batch_rename(&files, "photo_{n}");
        assert!(result.is_ok());

        for i in 0..3 {
            let renamed = dir.join(format!("photo_{:03}.png", i + 1));
            assert!(renamed.exists(), "missing {renamed:?}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_batch_rename_name_pattern() {
        let dir = std::env::temp_dir().join("batch_test_rename2");
        let _ = std::fs::create_dir_all(&dir);
        let src = dir.join("vacation.png");
        image::DynamicImage::new_rgba8(10, 10).save(&src).unwrap();

        let result = batch_rename(&[src.clone()], "{name}_edited");
        assert!(result.is_ok());

        let renamed = dir.join("vacation_edited.png");
        assert!(renamed.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_batch_resize() {
        let dir = std::env::temp_dir().join("batch_test_resize");
        let _ = std::fs::create_dir_all(&dir);
        let src = dir.join("test.png");
        image::DynamicImage::new_rgba8(100, 200).save(&src).unwrap();

        let result = batch_resize(&[src.clone()], 50, 100);
        assert!(result.is_ok());

        let img = image::open(&src).unwrap();
        assert_eq!(img.dimensions(), (50, 100));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_batch_resize_jpeg() {
        let dir = std::env::temp_dir().join("batch_test_resize_jpg");
        let _ = std::fs::create_dir_all(&dir);
        let src = dir.join("test.jpg");
        image::DynamicImage::new_rgba8(100, 200).save(&src).unwrap();

        let result = batch_resize(&[src.clone()], 25, 50);
        assert!(result.is_ok());

        let img = image::open(&src).unwrap();
        assert_eq!(img.dimensions(), (25, 50));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
