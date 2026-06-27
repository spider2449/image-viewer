# Thumbnail Cache & Sizing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add disk-backed thumbnail persistence and a configurable thumbnail size slider in the grid.

**Architecture:** New `DiskCache` struct in `src/disk_cache.rs` stores thumbnails as JPEG files keyed by path hash + source mtime + decode size. Workers check disk before decoding and store after. Grid uses `config.thumb_size` instead of hardcoded `THUMB_SIZE`.

**Tech Stack:** `sha2` crate for path hashing, existing `image` crate (jpeg feature) for disk encode/decode, egui slider + input handling for size control.

## Global Constraints

- No new crate dependencies beyond `sha2`
- Thumbnail decode size derived from display: `max(ceil(thumb_size * 1.5), 200) as u32`
- Disk cache directory: `<exe_dir>/cache/thumbnails/`
- Cache filename format: `{hash}.{mtime}.{size}.jpg`
- Bump Cargo.toml patch version before each commit

---

### Task 1: Add sha2 dependency + create DiskCache module

**Files:**
- Modify: `Cargo.toml`
- Create: `src/disk_cache.rs`

**Interfaces:**
- Produces: `DiskCache` struct with `new(dir)`, `lookup(path, max_size) -> Option<ColorImage>`, `store(path, max_size, image)`, `clear_all()`, `path_hash(path) -> String`

- [ ] **Step 1: Add sha2 to Cargo.toml**

Edit `Cargo.toml` to add line after `open = "5"`:
```toml
sha2 = "0.10"
```

- [ ] **Step 2: Create src/disk_cache.rs**

```rust
use egui::ColorImage;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub struct DiskCache {
    dir: PathBuf,
}

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
        let cache_path = self.cache_path(path, max_size)?;
        if !cache_path.exists() {
            return None;
        }
        if !path.exists() {
            fs::remove_file(&cache_path).ok();
            return None;
        }
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
        let img = image::RgbaImage::from_raw(image.size[0] as u32, image.size[1] as u32, raw).unwrap();
        let mut buf = Vec::new();
        {
            let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 85);
            enc.encode(&img, image.size[0] as u32, image.size[1] as u32, image::ExtendedColorType::Rgba8).ok();
        }
        fs::write(&cache_path, &buf).ok();
    }

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

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

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
        let ci = ColorImage::from_rgba_unmultiplied([1, 1], &[255, 0, 0, 255]);
        cache.store(&src, 200, &ci);

        // Delete the source, lookup should remove cache
        fs::remove_file(&src).ok();
        let result = cache.lookup(&src, 200);
        assert!(result.is_none());
        // Cache file should be gone
        let cache_path = cache.cache_path(&src, 200).unwrap();
        assert!(!cache_path.exists());

        let _ = fs::remove_dir_all(&dir);
    }
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test disk_cache 2>&1 | tail -20
```
Expected: 5 passed, 0 failed

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml src/disk_cache.rs
git commit -m "feat: add disk-backed thumbnail cache module"
```

---

### Task 2: Add thumb_size to Config + grid slider + Ctrl+scroll

**Files:**
- Modify: `src/config.rs` — add `thumb_size: f32`
- Modify: `src/browser/grid.rs` — replace THUMB_SIZE, add slider and scroll handler

**Interfaces:**
- Consumes: config has new `thumb_size`, `browser_state.show_list_view` exists
- Produces: grid toolbar with slider, Ctrl+scroll on grid area

- [ ] **Step 1: Add thumb_size to Config**

Edit `src/config.rs` — add field after `column_widths`:
```rust
    pub thumb_size: f32,
```

Add to `Default::default()`:
```rust
            thumb_size: 140.0,
```

Add to `Config` struct (replace the struct definition to include the field):
```rust
#[derive(Serialize, Deserialize)]
pub struct Config {
    pub last_folder: Option<String>,
    pub window_pos: Option<[f32; 2]>,
    pub window_size: Option<[f32; 2]>,
    pub sort_by: String,
    pub sort_descending: bool,
    pub slideshow_interval_secs: u32,
    pub zoom_default: f32,
    pub column_widths: ColumnWidths,
    pub thumb_size: f32,
}
```

- [ ] **Step 2: Replace THUMB_SIZE constant + add slider + Ctrl+scroll in grid.rs**

Edit `src/browser/grid.rs`:

Remove the constant:
```rust
const THUMB_SIZE: f32 = 140.0;
```

Add a helper function near the top (after imports):
```rust
fn thumb_display_size(app: &App) -> f32 {
    app.config.thumb_size
}
```

In `show_grid()`, add a slider to the toolbar (after the list view toggle):
```rust
        ui.separator();
        let mut size_changed = false;
        ui.label("Size:");
        let mut ts = app.config.thumb_size;
        if ui.add(egui::Slider::new(&mut ts, 60.0..=400.0).text("px")).changed() {
            app.config.thumb_size = ts;
            size_changed = true;
        }
```

Add Ctrl+scroll handler before the grid rendering in `show_grid()`:
```rust
        // Ctrl+scroll to change thumbnail size
        let (scroll, mods) = ui.input(|i| (i.raw_scroll_delta, i.modifiers));
        if mods.ctrl && scroll.y != 0.0 {
            let step = if scroll.y > 0.0 { 10.0 } else { -10.0 };
            app.config.thumb_size = (app.config.thumb_size + step).clamp(60.0, 400.0);
            size_changed = true;
        }
```

After the scroll/size slider block, handle size change:
```rust
        if size_changed {
            // If decode size increased, need to re-request thumbnails
            let new_decode = (app.config.thumb_size * 1.5).ceil().max(200.0) as u32;
            if new_decode > app.config.thumb_decode_max_size {
                app.config.thumb_decode_max_size = new_decode;
                app.browser_state.thumbnails.clear();
                app.browser_state.thumb_textures.clear();
                for path in &app.image_files {
                    app.thumbnail_cache.request(path.clone(), new_decode);
                }
            }
        }
```

Add `thumb_decode_max_size` field to Config... actually it's derived at runtime, not stored. Let me think about this differently.

The decode size is derived from `thumb_size`. It's not a config field — it's computed in `app.rs` when requesting thumbnails. So:

In `show_grid()`, when size changes, compute the new decode size and compare with what was used before. The used decode size needs to be tracked somewhere. Options:
1. Store it in browser_state
2. Don't track it — just clear and re-request every time (wasteful on downsizing)

Option 1 is better. Add to `browser/mod.rs` State:

```rust
pub struct State {
    // ... existing fields ...
    pub thumb_decode_size: u32,  // track last used decode size
}
```

And in `State::new()`:
```rust
            thumb_decode_size: 0,  // will be set on first request
```

In `show_grid()`, the size change handler:
```rust
        if size_changed {
            let new_decode = ((app.config.thumb_size * 1.5).ceil() as u32).max(200);
            if new_decode > app.browser_state.thumb_decode_size {
                app.browser_state.thumb_decode_size = new_decode;
                app.browser_state.thumbnails.clear();
                app.browser_state.thumb_textures.clear();
                for path in &app.image_files {
                    app.thumbnail_cache.request(path.clone(), new_decode);
                }
            }
        }
```

And when initially requesting thumbnails (in `app.rs`), set `thumb_decode_size`:
```rust
        self.browser_state.thumb_decode_size = decode_size;
```

Now, also replace all uses of `THUMB_SIZE` in `show_thumbnail_grid()` with `app.config.thumb_size`:

In `show_thumbnail_grid()`:
- `let cell_size = Vec2::new(THUMB_SIZE, THUMB_SIZE + LABEL_HEIGHT);` → `app.config.thumb_size`
- `let thumb_rect = ... Vec2::new(THUMB_SIZE, THUMB_SIZE)` → `app.config.thumb_size`
- `THUMB_SIZE / tex_size.x` etc → `app.config.thumb_size`
- `(THUMB_SIZE - draw_size.x) / 2.0` → `app.config.thumb_size`
- `rect.min + Vec2::new(4.0, THUMB_SIZE)` → `app.config.thumb_size`
- `Vec2::new(THUMB_SIZE - 8.0, LABEL_HEIGHT)` → `app.config.thumb_size`

Update `min_col_width`:
```rust
                .min_col_width(app.config.thumb_size)
```

Also update the `cols` calculation in `show_grid()`:
```rust
    let cols = ((available_width - THUMB_PADDING) / (app.config.thumb_size + THUMB_PADDING))
        .floor()
        .max(1.0) as usize;
```

- [ ] **Step 3: Update browser State**

Edit `src/browser/mod.rs` — add `thumb_decode_size` field:
```rust
    pub thumb_decode_size: u32,
```

In `State::new()`:
```rust
            thumb_decode_size: 0,
```

- [ ] **Step 4: Run check**

```bash
cargo check 2>&1
```
Expected: Compilation succeeds

- [ ] **Step 5: Commit**

```bash
git add src/config.rs src/browser/grid.rs src/browser/mod.rs
git commit -m "feat: add configurable thumbnail size with slider + Ctrl+scroll"
```

---

### Task 3: Integrate DiskCache into ThumbnailCache

**Files:**
- Modify: `src/thumbnail_cache.rs` — pass DiskCache to workers, check disk first, store after decode

**Interfaces:**
- Consumes: `DiskCache` from Task 1
- Produces: `ThumbnailCache` with disk-backed workers

- [ ] **Step 1: Update ThumbnailCache struct and new()**

Edit `src/thumbnail_cache.rs`:

Add import:
```rust
use crate::disk_cache::DiskCache;
```

Add `disk_cache` field to struct:
```rust
pub struct ThumbnailCache {
    cache: Arc<Mutex<LruCache<PathBuf, (ColorImage, u32, u32)>>>,
    pending: Arc<Mutex<Vec<PathBuf>>>,
    sender: Sender<ThumbnailRequest>,
    receiver: Receiver<ThumbnailResult>,
    disk_cache: Option<Arc<DiskCache>>,
}
```

Update `new()` to accept optional disk cache:
```rust
    pub fn new(capacity: usize, worker_count: usize, disk_cache: Option<DiskCache>) -> Self {
```

Wrap disk_cache in Arc:
```rust
        let disk_cache = disk_cache.map(Arc::new);
```

Pass to workers via clone:
```rust
        let dc = disk_cache.clone();
```

Update the worker loop to check disk first. The relevant section of the worker loop:
```rust
                    match req {
                        Some(req) => {
                            let start = Instant::now();

                            // Check disk cache first
                            let from_disk = dc.as_ref()
                                .and_then(|d| d.lookup(&req.path, req.max_size));

                            let result = match from_disk {
                                Some(ci) => {
                                    let w = ci.size[0] as u32;
                                    let h = ci.size[1] as u32;
                                    {
                                        let mut c = cache_clone.lock().unwrap();
                                        c.put(req.path.clone(), (ci.clone(), w, h));
                                    }
                                    ThumbnailResult {
                                        path: req.path,
                                        image: Some(ci),
                                        full_width: w,
                                        full_height: h,
                                        load_time: start.elapsed(),
                                    }
                                }
                                None => {
                                    let result = crate::image_loader::load_thumbnail(&req.path, req.max_size);
                                    match result {
                                        Ok((ci, w, h)) => {
                                            // Store to disk
                                            if let Some(ref d) = dc {
                                                d.store(&req.path, req.max_size, &ci);
                                            }
                                            {
                                                let mut c = cache_clone.lock().unwrap();
                                                c.put(req.path.clone(), (ci.clone(), w, h));
                                            }
                                            ThumbnailResult {
                                                path: req.path,
                                                image: Some(ci),
                                                full_width: w,
                                                full_height: h,
                                                load_time: start.elapsed(),
                                            }
                                        }
                                        Err(_) => {
                                            ThumbnailResult {
                                                path: req.path,
                                                image: None,
                                                full_width: 0,
                                                full_height: 0,
                                                load_time: start.elapsed(),
                                            }
                                        }
                                    }
                                }
                            };
                            res_tx.send(result).ok();
                        }
```

Return `disk_cache` in the struct:
```rust
        Self {
            cache,
            pending,
            sender: req_tx,
            receiver: res_rx,
            disk_cache,
        }
```

Add a `clear_disk_cache` method:
```rust
    pub fn clear_disk_cache(&self) {
        if let Some(ref dc) = self.disk_cache {
            dc.clear_all();
        }
    }
```

- [ ] **Step 2: Run tests**

```bash
cargo test 2>&1 | tail -20
```
Expected: All tests pass

- [ ] **Step 3: Commit**

```bash
git add src/thumbnail_cache.rs
git commit -m "feat: integrate disk cache into thumbnail workers"
```

---

### Task 4: Wire up app.rs — use config size, clear textures, re-request on size change

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 1: Update ThumbnailCache creation and request calls**

Edit `src/app.rs`:

Add import:
```rust
use crate::disk_cache::DiskCache;
```

Update ThumbnailCache creation in `App::new()`:
```rust
        let cache_dir = Self::cache_dir();
        let disk_cache = DiskCache::new(cache_dir.join("thumbnails"));
        let thumbnail_cache = ThumbnailCache::new(512, 4, Some(disk_cache));
```

Add cache_dir helper to App:
```rust
    fn cache_dir() -> PathBuf {
        let mut p = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
        p.pop();
        p.push("cache");
        p
    }
```

Actually, I should use the same directory as config. Let me check how Config::path() works in config.rs:

```rust
    fn path() -> PathBuf {
        let mut p = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
        p.pop(); // exe dir
        p.push("cache");
        p.push("config.json");
        p
    }
```

So the cache dir is `<exe_dir>/cache/`. I should use `<exe_dir>/cache/thumbnails/` for the disk cache.

Update `scan_folder()` to compute decode size from config and pass to request:
```rust
        let decode_size = ((self.config.thumb_size * 1.5).ceil() as u32).max(200);
        self.browser_state.thumb_decode_size = decode_size;
        for path in &self.image_files {
            self.thumbnail_cache.request(path.clone(), decode_size);
        }
```

Update the re-request loop in `update()`:
```rust
        let decode_size = ((self.config.thumb_size * 1.5).ceil() as u32).max(200);
        for path in &self.image_files {
            if !self.browser_state.thumbnails.contains_key(path) {
                self.thumbnail_cache.request(path.clone(), decode_size);
            }
        }
```

Clear `browser_state.thumb_textures` in `scan_folder()`:
```rust
        self.textures.clear();
        self.browser_state.thumbnails.clear();
        self.browser_state.thumb_textures.clear();
```

Add "Clear thumbnail cache" to the File menu:
```rust
                        if ui.button("Clear thumbnail cache").clicked() {
                            self.thumbnail_cache.clear_disk_cache();
                            self.browser_state.thumbnails.clear();
                            self.browser_state.thumb_textures.clear();
                            self.scan_folder();
                            ui.close_menu();
                        }
```

- [ ] **Step 2: Remove the `thumb_decode_size` fallback for grid.rs**

Wait, in Task 2, the grid.rs checks `app.browser_state.thumb_decode_size`. That field is set in app.rs now. This should work correctly — when scan_folder runs, it sets `thumb_decode_size` to the current decode size. The grid's size change handler compares the new decode size with the stored one.

- [ ] **Step 3: Run check**

```bash
cargo check 2>&1
```
Expected: Compilation succeeds

- [ ] **Step 4: Run tests**

```bash
cargo test 2>&1 | tail -30
```
Expected: All tests pass (disk_cache tests + existing tests)

- [ ] **Step 5: Bump version and commit**

Edit `Cargo.toml`:
```toml
version = "0.1.8"
```

```bash
git add src/app.rs Cargo.toml
git commit -m "feat: wire up configurable thumbnail size and disk cache"
```

---

### Task 5: Final verification

- [ ] **Step 1: Full build**

```bash
cargo build --release 2>&1
```
Expected: Build succeeds

- [ ] **Step 2: Full test suite**

```bash
cargo test 2>&1
```
Expected: All tests pass
