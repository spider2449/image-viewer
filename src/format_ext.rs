use image::ImageFormat;
use std::path::Path;

pub fn format_to_extension(format: &str) -> &'static str {
    match format {
        "jpeg" => "jpg",
        "png" => "png",
        "bmp" => "bmp",
        "webp" => "webp",
        "gif" => "gif",
        "tiff" => "tif",
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
        _ => None,
    }
}

pub fn is_supported_extension(path: &Path) -> bool {
    path.extension()
        .map(|e| extension_to_image_format(&e.to_string_lossy()).is_some())
        .unwrap_or(false)
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
        assert_eq!(extension_to_image_format("xyz"), None);
    }

    #[test]
    fn test_is_supported_extension() {
        assert!(is_supported_extension(std::path::Path::new("a.PNG")));
        assert!(is_supported_extension(std::path::Path::new("a.Jpg")));
        assert!(is_supported_extension(std::path::Path::new("a.tiff")));
        assert!(!is_supported_extension(std::path::Path::new("a.txt")));
        assert!(!is_supported_extension(std::path::Path::new("noext")));
    }
}
