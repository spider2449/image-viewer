use std::path::PathBuf;

#[derive(Clone, Debug)]
pub enum Operation {
    Convert {
        format: String,
        jpeg_quality: u8,
    },
    Rename {
        pattern: String,
    },
    Resize {
        width: u32,
        height: u32,
        lock_aspect: bool,
    },
}

pub fn process_file(operation: &Operation, path: &PathBuf, index: usize) -> Result<(), String> {
    match operation {
        Operation::Convert {
            format,
            jpeg_quality,
        } => convert_file(path, format, *jpeg_quality),
        Operation::Rename { pattern } => rename_file(path, pattern, index),
        Operation::Resize {
            width,
            height,
            lock_aspect,
        } => resize_file(path, *width, *height, *lock_aspect),
    }
}

fn convert_file(path: &PathBuf, format: &str, jpeg_quality: u8) -> Result<(), String> {
    let img = image::open(path).map_err(|e| e.to_string())?;
    let new_ext = crate::format_ext::format_to_extension(format);
    let new_name = path.with_extension(new_ext);
    crate::format_ext::save_image(&img, &new_name, format, jpeg_quality)
}

fn rename_file(path: &PathBuf, pattern: &str, index: usize) -> Result<(), String> {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let ext = path
        .extension()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let new_stem = pattern
        .replace("{n}", &format!("{:03}", index + 1))
        .replace("{name}", &stem);
    let new_name = path.with_file_name(format!("{new_stem}.{ext}"));
    if new_name == *path {
        return Ok(());
    }
    if new_name.exists() {
        return Err(format!("{} already exists", new_name.display()));
    }
    std::fs::rename(path, &new_name)
        .map_err(|e| format!("Failed to rename {}: {e}", path.display()))
}

fn resize_file(path: &PathBuf, width: u32, height: u32, lock_aspect: bool) -> Result<(), String> {
    let img = image::open(path).map_err(|e| e.to_string())?;
    let resized = if lock_aspect {
        img.resize(width, height, image::imageops::FilterType::Lanczos3)
    } else {
        img.resize_exact(width, height, image::imageops::FilterType::Lanczos3)
    };
    let ext = path
        .extension()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let format = match ext.as_str() {
        "jpg" | "jpeg" => "jpeg",
        "png" => "png",
        "bmp" => "bmp",
        "webp" => "webp",
        _ => return Err("Unsupported output format".to_string()),
    };
    crate::format_ext::save_image(&resized, path, format, 90)
}

#[cfg(test)]
pub fn batch_convert(files: &[PathBuf], format: &str, jpeg_quality: u8) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let operation = Operation::Convert {
        format: format.to_string(),
        jpeg_quality,
    };
    for (index, path) in files.iter().enumerate() {
        if let Err(e) = process_file(&operation, path, index) {
            errors.push(format!("{}: {e}", path.display()));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
pub fn batch_rename(files: &[PathBuf], pattern: &str) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let operation = Operation::Rename {
        pattern: pattern.to_string(),
    };
    for (index, path) in files.iter().enumerate() {
        if let Err(e) = process_file(&operation, path, index) {
            errors.push(e);
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
pub fn batch_resize(
    files: &[PathBuf],
    width: u32,
    height: u32,
    lock_aspect: bool,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let operation = Operation::Resize {
        width,
        height,
        lock_aspect,
    };
    for (index, path) in files.iter().enumerate() {
        if let Err(e) = process_file(&operation, path, index) {
            errors.push(format!("{}: {e}", path.display()));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::GenericImageView;
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
        let files: Vec<PathBuf> = (1..=3)
            .map(|i| {
                let p = dir.join(format!("img{i}.png"));
                image::DynamicImage::new_rgba8(10, 10).save(&p).unwrap();
                p
            })
            .collect();

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

        let result = batch_resize(&[src.clone()], 50, 100, false);
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

        let result = batch_resize(&[src.clone()], 25, 50, false);
        assert!(result.is_ok());

        let img = image::open(&src).unwrap();
        assert_eq!(img.dimensions(), (25, 50));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_batch_resize_lock_aspect_fits_within_box() {
        let dir = std::env::temp_dir().join("batch_test_resize_lock");
        let _ = std::fs::create_dir_all(&dir);
        let src = dir.join("test.png");
        image::DynamicImage::new_rgba8(100, 200).save(&src).unwrap();

        // 1:2 image into a 50x50 box with aspect locked -> 25x50
        let result = batch_resize(&[src.clone()], 50, 50, true);
        assert!(result.is_ok());
        let img = image::open(&src).unwrap();
        assert_eq!(img.dimensions(), (25, 50));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
