use image::{DynamicImage, GenericImageView, ImageFormat};
use std::path::Path;

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

pub fn save_image(
    img: &DynamicImage,
    path: &Path,
    format: &str,
    jpeg_quality: u8,
) -> Result<(), String> {
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
        "png" => img.save_with_format(path, ImageFormat::Png).map_err(|e| e.to_string()),
        "bmp" => img.save_with_format(path, ImageFormat::Bmp).map_err(|e| e.to_string()),
        "webp" => img.save_with_format(path, ImageFormat::WebP).map_err(|e| e.to_string()),
        _ => Err(format!("Unknown format: {format}")),
    }
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
        assert!(image::open(&dst).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_save_image_unknown_format_errors() {
        let img = image::DynamicImage::new_rgba8(2, 2);
        assert!(save_image(&img, std::path::Path::new("x.zzz"), "zzz", 90).is_err());
    }
}
