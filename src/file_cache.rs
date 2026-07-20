use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

#[derive(Clone)]
pub struct FileMetadata {
    pub len: u64,
    pub modified: SystemTime,
    pub fetched_at: Instant,
}

pub struct FileCache {
    entries: HashMap<PathBuf, FileMetadata>,
    ttl: Duration,
}

impl FileCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            ttl: Duration::from_secs(1),
        }
    }

    pub fn get_or_fetch(&mut self, path: &PathBuf) -> Option<FileMetadata> {
        let now = Instant::now();
        if let Some(meta) = self.entries.get(path) {
            if now.duration_since(meta.fetched_at) < self.ttl {
                return Some(meta.clone());
            }
        }
        let m = std::fs::metadata(path).ok()?;
        let len = m.len();
        let modified = m.modified().ok()?;
        let meta = FileMetadata {
            len,
            modified,
            fetched_at: now,
        };
        self.entries.insert(path.clone(), meta);
        Some(self.entries.get(path).cloned().unwrap())
    }
}

impl Default for FileCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_cache_returns_cached_value_within_ttl() {
        let mut cache = FileCache::new();
        let path = std::env::temp_dir().join("test_file_cache.txt");
        fs::write(&path, b"hello").unwrap();

        let first = cache.get_or_fetch(&path);
        assert!(first.is_some());
        let first = first.unwrap();
        assert_eq!(first.len, 5);

        // Modify file after first fetch
        fs::write(&path, b"hello world").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));

        let second = cache.get_or_fetch(&path);
        assert_eq!(second.unwrap().len, 5); // Still returns cached value
        fs::remove_file(&path).ok();
    }

    #[test]
    fn test_cache_refreshes_after_ttl() {
        let mut cache = FileCache::new();
        let path = std::env::temp_dir().join("test_file_cache_ttl.txt");
        fs::write(&path, b"hello").unwrap();

        let first = cache.get_or_fetch(&path).unwrap();
        assert_eq!(first.len, 5);

        fs::write(&path, b"hello world").unwrap();
        std::thread::sleep(std::time::Duration::from_secs(2));

        let second = cache.get_or_fetch(&path).unwrap();
        assert_eq!(second.len, 11); // Now returns updated value
        fs::remove_file(&path).ok();
    }
}
