use egui::ColorImage;
use image::GenericImageView;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub struct DiskCache {
    dir: PathBuf,
}

#[allow(dead_code)]
impl DiskCache {
    pub fn new(dir: PathBuf) -> Self {
        fs::create_dir_all(&dir).ok();
        Self { dir }
    }

    fn cache_path(&self, path: &Path, max_size: u32) -> Option<PathBuf> {
        let hash = self.path_hash(path);
        let mtime = Self::get_mtime(path)?;
        Some(self.dir.join(format!("{hash}.{mtime}.{max_size}.jpg")))
    }

    pub fn lookup(&self, path: &Path, max_size: u32) -> Option<ColorImage> {
        let cache_path = match self.cache_path(path, max_size) {
            Some(cp) if cp.exists() => cp,
            _ => {
                // Source may be deleted — clean up orphaned cache entries
                if !path.exists() {
                    self.delete_old_entries(path);
                }
                return None;
            }
        };
        let bytes = fs::read(&cache_path).ok()?;
        let img = image::load_from_memory(&bytes).ok()?;
        let rgba = img.to_rgba8();
        let (w, h) = img.dimensions();
        Some(ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba))
    }

    pub fn store(&self, path: &Path, max_size: u32, image: &ColorImage) {
        self.delete_old_entries(path);
        let Some(cache_path) = self.cache_path(path, max_size) else { return };
        let raw: Vec<u8> = image.pixels.iter().flat_map(|c| c.to_array()).collect();
        let rgba = image::RgbaImage::from_raw(image.size[0] as u32, image.size[1] as u32, raw).unwrap();
        let rgb = image::DynamicImage::ImageRgba8(rgba).to_rgb8();
        let mut buf = Vec::new();
        {
            let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 85);
            enc.encode(&rgb, image.size[0] as u32, image.size[1] as u32, image::ExtendedColorType::Rgb8).ok();
        }
        fs::write(&cache_path, &buf).ok();
    }

    #[allow(dead_code)]
    pub fn clear_all(&self) {
        if let Ok(entries) = fs::read_dir(&self.dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "jpg").unwrap_or(false) {
                    fs::remove_file(&path).ok();
                }
            }
        }
    }

    fn delete_old_entries(&self, path: &Path) {
        let hash = self.path_hash(path);
        let prefix = format!("{hash}.");
        if let Ok(entries) = fs::read_dir(&self.dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with(&prefix) && name_str.ends_with(".jpg") {
                    fs::remove_file(entry.path()).ok();
                }
            }
        }
    }

    pub fn path_hash(&self, path: &Path) -> String {
        let mut hasher = Sha256::new();
        hasher.update(path.to_string_lossy().as_bytes());
        hex_encode(&hasher.finalize()[..8])
    }

    fn get_mtime(path: &Path) -> Option<u64> {
        let meta = fs::metadata(path).ok()?;
        meta.modified().ok()?.duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs())
    }
}

#[allow(dead_code)]
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_hash_deterministic() {
        let cache = DiskCache::new(PathBuf::from("."));
        let path = PathBuf::from("C:\\test\\image.jpg");
        let h1 = cache.path_hash(&path);
        let h2 = cache.path_hash(&path);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_path_hash_different() {
        let cache = DiskCache::new(PathBuf::from("."));
        let h1 = cache.path_hash(&PathBuf::from("C:\\test\\a.jpg"));
        let h2 = cache.path_hash(&PathBuf::from("C:\\test\\b.jpg"));
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_lookup_miss() {
        let dir = std::env::temp_dir().join("thumb_cache_test_miss");
        let _ = fs::remove_dir_all(&dir);
        let cache = DiskCache::new(dir);
        let result = cache.lookup(Path::new("nonexistent.jpg"), 200);
        assert!(result.is_none());
    }

    #[test]
    fn test_store_and_lookup_roundtrip() {
        let dir = std::env::temp_dir().join("thumb_cache_test_roundtrip");
        let _ = fs::remove_dir_all(&dir);
        let cache = DiskCache::new(dir.clone());

        let src = dir.join("test_source.png");
        // Create a minimal valid PNG
        let mut png_data = Vec::new();
        png_data.extend_from_slice(b"\x89PNG\r\n\x1a\n");
        png_data.extend_from_slice(&[0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 2, 0, 0, 0, 144, 119, 83, 222]);
        // CRC for IHDR (1x1 RGB)
        png_data.extend_from_slice(&[0x4d, 0x92, 0x7c, 0xd2]);
        // IDAT chunk: compressed 1 pixel red
        let raw = [0x78, 0x01, 0x62, 0x60, 0x60, 0x60, 0x60, 0x00, 0x00, 0x00, 0x04, 0x00, 0x01];
        png_data.extend_from_slice(&[0, 0, 0, raw.len() as u8, 73, 68, 65, 84]);
        png_data.extend_from_slice(&raw);
        // CRC placeholder — just skip for test
        png_data.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0]);
        fs::write(&src, &png_data).ok();

        // Create a simple 2x2 red ColorImage
        let ci = ColorImage::from_rgba_unmultiplied([2, 2], &[
            255, 0, 0, 255, 255, 0, 0, 255,
            255, 0, 0, 255, 255, 0, 0, 255,
        ]);

        cache.store(&src, 200, &ci);
        let loaded = cache.lookup(&src, 200);
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.size, [2, 2]);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_orphan_cleanup() {
        let dir = std::env::temp_dir().join("thumb_cache_test_orphan");
        let _ = fs::remove_dir_all(&dir);
        let cache = DiskCache::new(dir.clone());

        let src = dir.join("orphan_source.png");
        // Create source file so mtime can be read
        fs::write(&src, b"dummy content").ok();
        let ci = ColorImage::from_rgba_unmultiplied([1, 1], &[255, 0, 0, 255]);
        cache.store(&src, 200, &ci);

        // Capture cache path before deleting source
        let cache_path = cache.cache_path(&src, 200).unwrap();
        // Delete the source, lookup should remove cache
        fs::remove_file(&src).ok();
        let result = cache.lookup(&src, 200);
        assert!(result.is_none());
        // Cache file should be gone
        assert!(!cache_path.exists());

        let _ = fs::remove_dir_all(&dir);
    }
}
