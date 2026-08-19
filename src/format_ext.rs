use image::{DynamicImage, GenericImageView, ImageFormat};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static SAVE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub fn format_to_extension(format: &str) -> &'static str {
    match format {
        "jpeg" => "jpg",
        "png" => "png",
        "bmp" => "bmp",
        "webp" => "webp",
        "gif" => "gif",
        "tiff" => "tif",
        "exr" => "exr",
        _ => "",
    }
}

pub fn extension_to_image_format(ext: &str) -> Option<ImageFormat> {
    match ext.to_lowercase().as_str() {
        "png" => Some(ImageFormat::Png),
        "jpg" | "jpeg" => Some(ImageFormat::Jpeg),
        "bmp" => Some(ImageFormat::Bmp),
        "gif" => Some(ImageFormat::Gif),
        "tif" | "tiff" => Some(ImageFormat::Tiff),
        "webp" => Some(ImageFormat::WebP),
        "exr" => Some(ImageFormat::OpenExr),
        _ => None,
    }
}

pub fn is_supported_extension(path: &Path) -> bool {
    path.extension()
        .map(|e| extension_to_image_format(&e.to_string_lossy()).is_some())
        .unwrap_or(false)
}

fn to_8bit_dynamic(img: &DynamicImage) -> DynamicImage {
    match img {
        DynamicImage::ImageRgb32F(_) | DynamicImage::ImageRgba32F(_) => {
            DynamicImage::ImageRgba8(img.to_rgba8())
        }
        other => other.clone(),
    }
}

fn unique_sidecar_path(path: &Path, role: &str) -> Result<PathBuf, String> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("image");
    for _ in 0..100 {
        let sequence = SAVE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let candidate = parent.join(format!(
            ".{name}.{role}.{}.{}",
            std::process::id(),
            sequence
        ));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(format!(
        "Could not allocate a temporary path for {}",
        path.display()
    ))
}

fn encode_image(
    img: &DynamicImage,
    path: &Path,
    format: &str,
    jpeg_quality: u8,
) -> Result<(), String> {
    let img = to_8bit_dynamic(img);
    match format {
        "jpeg" => {
            let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
            let (w, h) = img.dimensions();
            let rgb = img.to_rgb8();
            let mut encoder =
                image::codecs::jpeg::JpegEncoder::new_with_quality(file, jpeg_quality);
            encoder
                .encode(&rgb, w, h, image::ExtendedColorType::Rgb8)
                .map_err(|e| e.to_string())
        }
        "png" => img
            .save_with_format(path, ImageFormat::Png)
            .map_err(|e| e.to_string()),
        "bmp" => img
            .save_with_format(path, ImageFormat::Bmp)
            .map_err(|e| e.to_string()),
        "webp" => img
            .save_with_format(path, ImageFormat::WebP)
            .map_err(|e| e.to_string()),
        _ => Err(format!("Unknown format: {format}")),
    }
}

fn replace_completed_file<F>(
    temporary: &Path,
    destination: &Path,
    backup: &Path,
    mut rename: F,
) -> Result<(), String>
where
    F: FnMut(&Path, &Path) -> io::Result<()>,
{
    if !destination.exists() {
        return rename(temporary, destination).map_err(|e| e.to_string());
    }

    rename(destination, backup).map_err(|e| e.to_string())?;
    if let Err(replace_error) = rename(temporary, destination) {
        return match rename(backup, destination) {
            Ok(()) => Err(format!("Replacement failed: {replace_error}")),
            Err(restore_error) => Err(format!(
                "Replacement failed: {replace_error}; restoring the original also failed: {restore_error}"
            )),
        };
    }
    fs::remove_file(backup).map_err(|e| format!("Saved image but could not remove backup: {e}"))
}

pub fn save_image(
    img: &DynamicImage,
    path: &Path,
    format: &str,
    jpeg_quality: u8,
) -> Result<(), String> {
    let temporary = unique_sidecar_path(path, "tmp")?;
    let backup = unique_sidecar_path(path, "bak")?;

    if let Err(error) = encode_image(img, &temporary, format, jpeg_quality) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }

    let result = replace_completed_file(&temporary, path, &backup, |from, to| fs::rename(from, to));
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_to_extension() {
        assert_eq!(format_to_extension("jpeg"), "jpg");
        assert_eq!(format_to_extension("png"), "png");
        assert_eq!(format_to_extension("unknown"), "");
    }

    #[test]
    fn test_extension_to_image_format_case_insensitive() {
        assert_eq!(extension_to_image_format("PNG"), Some(ImageFormat::Png));
        assert_eq!(extension_to_image_format("Jpg"), Some(ImageFormat::Jpeg));
        assert_eq!(extension_to_image_format("JPEG"), Some(ImageFormat::Jpeg));
        assert_eq!(extension_to_image_format("EXR"), Some(ImageFormat::OpenExr));
        assert_eq!(extension_to_image_format("xyz"), None);
    }

    #[test]
    fn test_is_supported_extension() {
        assert!(is_supported_extension(std::path::Path::new("a.PNG")));
        assert!(is_supported_extension(std::path::Path::new("a.Jpg")));
        assert!(is_supported_extension(std::path::Path::new("a.tiff")));
        assert!(is_supported_extension(std::path::Path::new("a.exr")));
        assert!(is_supported_extension(std::path::Path::new("A.EXR")));
        assert!(!is_supported_extension(std::path::Path::new("a.txt")));
        assert!(!is_supported_extension(std::path::Path::new("noext")));
    }

    #[test]
    fn test_save_image_rgba_as_jpeg_succeeds() {
        let dir = std::env::temp_dir().join("save_image_test_jpeg");
        let _ = std::fs::create_dir_all(&dir);
        let dst = dir.join("out.jpg");
        let img = image::DynamicImage::new_rgba8(10, 10); // RGBA source
        let result = save_image(&img, &dst, "jpeg", 90);
        assert!(result.is_ok(), "rgba->jpeg must succeed: {result:?}");
        assert!(dst.exists());
        let decoded = image::open(&dst).unwrap();
        assert_eq!(decoded.dimensions(), (10, 10));
        assert_eq!(decoded.color(), image::ColorType::Rgb8);
        let _ = std::fs::remove_dir_all(&dir);
    }


    #[test]
    fn test_jpeg_quality_changes_encoded_output() {
        let dir = std::env::temp_dir().join("save_image_test_jpeg_quality");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let dst = dir.join("out.jpg");
        let image = image::DynamicImage::ImageRgba8(image::RgbaImage::from_fn(64, 64, |x, y| {
            image::Rgba([x as u8 * 3, y as u8 * 3, (x ^ y) as u8 * 3, 128])
        }));
        save_image(&image, &dst, "jpeg", 10).unwrap();
        let low_quality = std::fs::read(&dst).unwrap();
        save_image(&image, &dst, "jpeg", 95).unwrap();
        let high_quality = std::fs::read(&dst).unwrap();
        assert_ne!(low_quality, high_quality);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_save_image_unknown_format_errors() {
        let dir = std::env::temp_dir().join("save_image_test_failed_encode");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let dst = dir.join("x.zzz");
        std::fs::write(&dst, b"original bytes").unwrap();
        let img = image::DynamicImage::new_rgba8(2, 2);
        assert!(save_image(&img, &dst, "zzz", 90).is_err());
        assert_eq!(std::fs::read(&dst).unwrap(), b"original bytes");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_save_image_rgb32f_as_png_succeeds() {
        let dir = std::env::temp_dir().join("save_image_test_rgb32f_png");
        let _ = std::fs::create_dir_all(&dir);
        let dst = dir.join("out.png");
        let img = image::DynamicImage::ImageRgb32F(image::Rgb32FImage::new(8, 8));
        let result = save_image(&img, &dst, "png", 90);
        assert!(result.is_ok(), "rgb32f->png must succeed: {result:?}");
        let len = std::fs::metadata(&dst).map(|m| m.len()).unwrap_or(0);
        assert!(len > 0, "png file must not be empty, got {len} bytes");
        assert!(image::open(&dst).is_ok(), "saved png must be decodable");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_successful_replacement_cleans_sidecars() {
        let dir = std::env::temp_dir().join("save_image_test_replace");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let dst = dir.join("out.png");
        std::fs::write(&dst, b"old").unwrap();
        let img = image::DynamicImage::new_rgba8(7, 9);

        save_image(&img, &dst, "png", 90).unwrap();

        assert_eq!(image::open(&dst).unwrap().dimensions(), (7, 9));
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_replacement_failure_restores_original() {
        let dir = std::env::temp_dir().join("save_image_test_restore");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let dst = dir.join("out.png");
        let temporary = dir.join("temporary");
        let backup = dir.join("backup");
        std::fs::write(&dst, b"original").unwrap();
        std::fs::write(&temporary, b"replacement").unwrap();
        let mut calls = 0;

        let result = replace_completed_file(&temporary, &dst, &backup, |from, to| {
            calls += 1;
            if calls == 2 {
                Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "simulated failure",
                ))
            } else {
                fs::rename(from, to)
            }
        });

        assert!(result.is_err());
        assert_eq!(std::fs::read(&dst).unwrap(), b"original");
        assert!(!backup.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
