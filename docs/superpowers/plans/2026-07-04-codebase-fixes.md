# Codebase Review Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix critical and important issues identified in the codebase review: config I/O thrash, thread leak, unbounded texture growth, UI-thread metadata calls, idle CPU usage, and format/extension inconsistency.

**Architecture:** Incremental fixes to existing modules. Each task is independently testable and committed separately. No new modules introduced unless a fix requires it (Task 6 adds a small `format_ext` helper module for unified format→extension mapping).

**Tech Stack:** Rust 2021, eframe/egui 0.31, image 0.25, lru 0.12, std::sync::mpsc, std::thread

## Global Constraints

- All existing 26 tests must continue to pass after every task.
- `cargo check` must be clean (no errors; warnings acceptable only if pre-existing).
- Follow existing code style: no new comments unless explicitly requested, 4-space indentation, `crate::` imports.
- Bump `Cargo.toml` patch version before each commit (per AGENTS.md convention).
- Do not change the egui version (0.31) or add new external dependencies.
- Preserve the immediate-mode GUI pattern: do not introduce async/await on the UI thread.

---

### Task 1: Fix config save thrash on every pointer release (Critical C1)

**Files:**
- Modify: `src/browser/grid.rs:401-404`
- Test: manual (GUI behavior; no new unit test — existing tests must still pass)

**Interfaces:**
- Consumes: `app.config.save()` from `src/config.rs`
- Produces: config only saved when a column resize actually occurred

- [ ] **Step 1: Read the current list-view header code**

Run: `Read src/browser/grid.rs` offset 360-410

Confirm the current code at lines 402-404:
```rust
if ui.input(|i| i.pointer.any_released()) {
    app.config.save();
}
```

- [ ] **Step 2: Locate the `drag_handle` helper and the size_changed tracking**

Run: `Grep "fn drag_handle"` in `src/browser/grid.rs`

Note: `show_grid` already tracks `let mut size_changed = false;` at line 10 and sets it `true` when thumb_size slider changes (line 42). However, list view column resizes happen in `show_list_view`, a separate function, so `size_changed` from `show_grid` is not in scope there.

- [ ] **Step 3: Replace the unconditional save with a resize-specific save**

Edit `src/browser/grid.rs`. Replace these three lines (402-404):

```rust
if ui.input(|i| i.pointer.any_released()) {
    app.config.save();
}
```

with:

```rust
if ui.input(|i| i.pointer.any_released()) {
    if app.config.column_widths.name != 200.0
        || app.config.column_widths.size != 80.0
        || app.config.column_widths.date != 150.0
    {
        app.config.save();
    }
}
```

Wait — that compares against hardcoded defaults, which won't catch re-saves. Better: track a local `saved_widths` snapshot.

Replace with this approach instead. First, add a local snapshot at the top of `show_list_view` (right after the function signature — find it via `Grep "fn show_list_view"`), and save only when widths differ from the snapshot:

At the top of `show_list_view`, after the opening `{`, add:
```rust
let saved_widths = app.config.column_widths.clone();
```

Then replace lines 402-404 with:
```rust
if ui.input(|i| i.pointer.any_released()) {
    if app.config.column_widths.name != saved_widths.name
        || app.config.column_widths.size != saved_widths.size
        || app.config.column_widths.date != saved_widths.date
    {
        app.config.save();
    }
}
```

- [ ] **Step 4: Verify cargo check passes**

Run: `cargo check`
Expected: clean (no new errors/warnings)

- [ ] **Step 5: Verify tests still pass**

Run: `cargo test`
Expected: 26 tests pass

- [ ] **Step 6: Bump version and commit**

Edit `Cargo.toml`: change `version = "0.1.16"` to `version = "0.1.17"`.

```bash
git add src/browser/grid.rs Cargo.toml
git commit -m "fix: save config only when column widths actually change

Previously config.save() fired on every pointer release in list view,
causing synchronous disk I/O on every mouse-up anywhere in the UI."
```

---

### Task 2: Fix thread leak in scan_folder by adding shutdown token (Critical C2)

**Files:**
- Modify: `src/thumbnail_cache.rs:33-175`
- Modify: `src/app.rs:96`
- Test: `src/thumbnail_cache.rs` (add unit test for shutdown)

**Interfaces:**
- Consumes: `std::sync::mpsc`, `std::thread`, `std::sync::Arc`
- Produces: `ThumbnailCache` with a `shutdown: Arc<AtomicBool>` field; threads check the flag and exit within ~10ms

- [ ] **Step 1: Read the current ThumbnailCache struct and spawn logic**

Run: `Read src/thumbnail_cache.rs` (full file, 207 lines)

Confirm:
- Struct at lines 33-39 has no shutdown field
- Store thread spawn at lines 59-78 loops on `Disconnected`
- Decoder threads spawn at lines 82-166, each loops on `Disconnected` with 10ms sleep

- [ ] **Step 2: Add shutdown field to ThumbnailCache struct**

Edit `src/thumbnail_cache.rs`. Add `use std::sync::atomic::{AtomicBool, Ordering};` to the imports at the top (after line 7).

Then modify the struct (lines 33-39) to add a `shutdown` field:

```rust
pub struct ThumbnailCache {
    cache: Arc<Mutex<LruCache<PathBuf, (ColorImage, u32, u32)>>>,
    pending: Arc<Mutex<Vec<PathBuf>>>,
    sender: Sender<ThumbnailRequest>,
    receiver: Receiver<ThumbnailResult>,
    disk_cache: Option<Arc<DiskCache>>,
    shutdown: Arc<AtomicBool>,
}
```

- [ ] **Step 3: Create the shutdown flag in `new()` and pass clones to worker threads**

Edit `ThumbnailCache::new`. After line 50 (`let pending: Arc<Mutex<Vec<PathBuf>>> = ...`), add:

```rust
let shutdown = Arc::new(AtomicBool::new(false));
```

Modify the store thread spawn (lines 59-78). Pass `shutdown.clone()` into the closure and change the empty-branch to also check shutdown:

Replace the store thread block (lines 59-78) with:

```rust
if let Some(ref dc) = disk_cache {
    let dc = dc.clone();
    let store_rx = Arc::new(Mutex::new(store_rx));
    let sd = shutdown.clone();
    thread::spawn(move || {
        loop {
            if sd.load(Ordering::Relaxed) {
                break;
            }
            let req = {
                let rx = store_rx.lock().unwrap();
                match rx.try_recv() {
                    Ok(r) => Some(r),
                    Err(TryRecvError::Empty) => None,
                    Err(TryRecvError::Disconnected) => break,
                }
            };
            match req {
                Some(req) => {
                    dc.store(&req.path, req.max_size, &req.image);
                }
                None => {
                    thread::sleep(Duration::from_millis(10));
                }
            }
        }
    });
}
```

Modify the decoder thread spawn (lines 82-166). Pass `shutdown.clone()` into each worker closure. Add `let sd = shutdown.clone();` inside the loop body before `thread::spawn`, and add a shutdown check at the top of the inner `loop`:

Replace the decoder loop opening (lines 89-98) with:

```rust
thread::spawn(move || {
    loop {
        if sd.load(Ordering::Relaxed) {
            break;
        }
        let req = {
            let rx = req_rx.lock().unwrap();
            match rx.try_recv() {
                Ok(r) => Some(r),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => break,
            }
        };
```

And add `let sd = shutdown.clone();` right after `let store_tx = store_tx.clone();` (line 87), inside the `for _ in 0..worker_count.max(1)` loop.

- [ ] **Step 4: Add a Drop impl that sets the shutdown flag and drops the sender**

At the end of the `impl ThumbnailCache` block (after the closing brace of `clear_disk_cache`, currently line 206), add:

```rust
impl Drop for ThumbnailCache {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
}
```

The `sender` field is dropped automatically when the struct is dropped, which triggers `Disconnected` in workers as a backup path. The shutdown flag lets them exit within ~10ms instead of waiting on the next `try_recv`.

- [ ] **Step 5: Add a `clear()` method that recreates the cache contents without recreating the whole struct**

Add this method inside `impl ThumbnailCache` (before the `Drop` impl):

```rust
pub fn clear(&self) {
    self.cache.lock().unwrap().clear();
    self.pending.lock().unwrap().clear();
}
```

- [ ] **Step 6: Update `app.rs::scan_folder` to call `clear()` instead of recreating the cache**

Edit `src/app.rs`. Replace lines 93-100:

```rust
pub fn scan_folder(&mut self) {
    // Drop old thumbnail cache (channels close → old worker threads exit on Disconnected)
    // and create a fresh one so new folder gets dedicated threads with no stale requests.
    self.thumbnail_cache = ThumbnailCache::new(512, 4, Some(self.disk_cache.clone()));
    self.textures.clear();
    self.browser_state.thumbnails.clear();
    self.browser_state.thumb_textures.clear();
    self.browser_state.tree_nodes.clear(); // rebuild tree on next frame
```

with:

```rust
pub fn scan_folder(&mut self) {
    self.thumbnail_cache.clear();
    self.textures.clear();
    self.browser_state.thumbnails.clear();
    self.browser_state.thumb_textures.clear();
    self.browser_state.tree_nodes.clear();
```

- [ ] **Step 7: Add a unit test for shutdown**

Edit `src/thumbnail_cache.rs`. In the file, add a `#[cfg(test)]` mod at the end (after the `Drop` impl):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shutdown_flag_set_on_drop() {
        let cache = ThumbnailCache::new(16, 1, None);
        let sd = cache.shutdown.clone();
        drop(cache);
        assert!(sd.load(Ordering::Relaxed), "shutdown flag must be true after drop");
    }

    #[test]
    fn test_clear_empties_cache_and_pending() {
        let cache = ThumbnailCache::new(16, 1, None);
        cache.request(PathBuf::from("test.png"), 100);
        {
            let p = cache.pending.lock().unwrap();
            assert_eq!(p.len(), 1);
        }
        cache.clear();
        {
            let p = cache.pending.lock().unwrap();
            assert!(p.is_empty(), "pending must be empty after clear");
        }
        {
            let c = cache.cache.lock().unwrap();
            assert!(c.is_empty(), "cache must be empty after clear");
        }
    }
}
```

- [ ] **Step 8: Verify cargo check passes**

Run: `cargo check`
Expected: clean

- [ ] **Step 9: Verify tests pass**

Run: `cargo test`
Expected: 28 tests pass (26 existing + 2 new)

- [ ] **Step 10: Bump version and commit**

Edit `Cargo.toml`: `version = "0.1.17"` → `version = "0.1.18"`.

```bash
git add src/thumbnail_cache.rs src/app.rs Cargo.toml
git commit -m "fix: stop thread leak in scan_folder via shutdown token

ThumbnailCache now uses an AtomicBool shutdown flag and a clear() method
instead of being recreated on every scan_folder call. Drop sets the flag
so worker threads exit within ~10ms. Old approach leaked threads because
Disconnected-based exit is not guaranteed to be prompt."
```

---

### Task 3: Cap viewer texture memory via LRU (Important I3)

**Files:**
- Modify: `src/app.rs:18-34` (App struct), `src/app.rs:37-84` (App::new)
- Modify: `src/viewer.rs:291-303` (texture load site)
- Modify: `src/editor/mod.rs:190-257` (apply_op/undo/redo texture insert sites)
- Test: `src/app.rs` (add unit test for eviction)

**Interfaces:**
- Consumes: `lru::LruCache`, `egui::TextureHandle`
- Produces: `app.textures` becomes `LruCache<String, TextureHandle>` with capacity 8; accessing the current image's texture each frame keeps it hot

- [ ] **Step 1: Change the `textures` field type in App**

Edit `src/app.rs`. Add `use lru::LruCache;` and `use std::num::NonZeroUsize;` to the imports (near line 10).

Change the struct field (line 28):
```rust
pub textures: HashMap<String, egui::TextureHandle>,
```
to:
```rust
pub textures: LruCache<String, egui::TextureHandle>,
```

- [ ] **Step 2: Initialize the LRU in App::new**

Edit `src/app.rs:67`. Change:
```rust
textures: HashMap::new(),
```
to:
```rust
textures: LruCache::new(NonZeroUsize::new(8).unwrap()),
```

- [ ] **Step 3: Update scan_folder's `self.textures.clear()`**

`LruCache::clear()` exists and has the same signature. No change needed to line 97 — verify it compiles.

- [ ] **Step 4: Update `image_loader::load_to_texture`**

Edit `src/image_loader.rs:31-41`. Change the signature to accept `LruCache`:

```rust
use lru::LruCache;

pub fn load_to_texture(
    cc: &egui::Context,
    textures: &mut LruCache<String, TextureHandle>,
    path: &Path,
) -> Result<(TextureHandle, u32, u32, u8), String> {
    let key = path.to_string_lossy().to_string();
    let ci = decode_to_colorimage(path)?;
    let tex = cc.load_texture(&key, ci.0, TextureOptions::LINEAR);
    textures.put(key.clone(), tex.clone());
    Ok((tex, ci.1, ci.2, ci.3))
}
```

(Change `textures.insert(...)` to `textures.put(...)` since LruCache uses `put`.)

- [ ] **Step 5: Update viewer.rs texture lookup**

Edit `src/viewer.rs:291-293`. Change:

```rust
let tex_key = path.to_string_lossy().to_string();
if !app.textures.contains_key(&tex_key) {
    if let Err(e) = image_loader::load_to_texture(ctx, &mut app.textures, &path) {
```

to:

```rust
let tex_key = path.to_string_lossy().to_string();
if !app.textures.contains(&tex_key) {
    if let Err(e) = image_loader::load_to_texture(ctx, &mut app.textures, &path) {
```

(`LruCache` uses `contains` rather than `contains_key`.)

Also find the line where the existing texture is fetched for drawing. Run: `Grep "app.textures.get" src/viewer.rs`

For any `app.textures.get(&tex_key)` call, change to `app.textures.get(&tex_key).cloned()` (LruCache::get returns `Option<&V>` and also marks it as recently-used — which is exactly what we want for keeping the current image hot). Since `TextureHandle` is `Clone`, `.cloned()` works.

If the code instead uses `app.textures.get_mut`, switch to `app.textures.get(&tex_key).cloned()`.

- [ ] **Step 6: Update editor/mod.rs texture insert sites**

Edit `src/editor/mod.rs`. In `apply_op` (line 215), `undo` (line 234), and `redo` (line 254), change:

```rust
app.textures.insert(tex_key, tex);
```

to:

```rust
app.textures.put(tex_key, tex);
```

(Three replacements; use replaceAll or do them one at a time.)

- [ ] **Step 7: Search for any other `app.textures` accesses**

Run: `Grep "app.textures"` across `src/`

For each match, check the call:
- `.clear()` → fine (LruCache has `clear()`)
- `.insert(k, v)` → change to `.put(k, v)`
- `.contains_key(k)` → change to `.contains(k)`
- `.get(k)` → change to `.get(k).cloned()` (note: LruCache::get takes `&self`-equivalent Q and returns `Option<&V>`; for the draw site we need `.cloned()`)

- [ ] **Step 8: Add a unit test for LRU eviction**

Edit `src/app.rs`. Add at the end of the file (after the `impl eframe::App for App` block):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_textures_lru_evicts_old_entries() {
        let mut cache: LruCache<String, i32> = LruCache::new(NonZeroUsize::new(2).unwrap());
        cache.put("a".to_string(), 1);
        cache.put("b".to_string(), 2);
        cache.put("c".to_string(), 3);
        assert!(!cache.contains("a"), "a should be evicted");
        assert!(cache.contains("b"));
        assert!(cache.contains("c"));
        assert_eq!(cache.len(), 2);
    }
}
```

(Note: this tests the LruCache behavior directly; we can't construct `TextureHandle` in a unit test without an egui context, so we verify the eviction logic on the underlying container type.)

- [ ] **Step 9: Verify cargo check passes**

Run: `cargo check`
Expected: clean

- [ ] **Step 10: Verify tests pass**

Run: `cargo test`
Expected: 29 tests pass (28 + 1 new)

- [ ] **Step 11: Bump version and commit**

Edit `Cargo.toml`: `version = "0.1.18"` → `version = "0.1.19"`.

```bash
git add src/app.rs src/image_loader.rs src/viewer.rs src/editor/mod.rs Cargo.toml
git commit -m "fix: bound viewer texture memory with LRU cache (capacity 8)

Previously app.textures was an unbounded HashMap<String, TextureHandle>;
opening every image in a large folder accumulated GPU textures until OOM.
Now an LruCache<String, TextureHandle> with capacity 8 evicts the
least-recently-used texture, and accessing the current image each frame
keeps it hot."
```

---

### Task 4: Cache file metadata during scan_folder (Important I2)

**Files:**
- Modify: `src/app.rs:93-154` (scan_folder)
- Test: no new unit test (filesystem-dependent); existing tests must pass

**Interfaces:**
- Consumes: `std::fs::metadata`, `std::time::SystemTime`
- Produces: `image_files` is sorted using metadata read once during scan, not in the sort comparator

- [ ] **Step 1: Read the current scan_folder sort block**

Confirm lines 122-145 call `std::fs::metadata(a)` and `std::fs::metadata(b)` inside the sort comparator, which means N log N metadata calls for N files.

- [ ] **Step 2: Restructure scan_folder to collect metadata once**

Edit `src/app.rs`. Replace the scan_folder body from line 105 (`let mut files = Vec::new();`) through line 147 (`self.image_files = files;`) with:

```rust
let mut files: Vec<PathBuf> = Vec::new();
if let Ok(entries) = std::fs::read_dir(&folder) {
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                let ext = ext.to_string_lossy().to_lowercase();
                match ext.as_str() {
                    "png" | "jpg" | "jpeg" | "bmp" | "gif" | "tiff" | "tif" | "webp" => {
                        files.push(path);
                    }
                    _ => {}
                }
            }
        }
    }
}

let sort_desc = self.config.sort_descending;
match self.config.sort_by.as_str() {
    "date" => {
        let mtimes: Vec<Option<std::time::SystemTime>> = files
            .iter()
            .map(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok())
            .collect();
        let mut indices: Vec<usize> = (0..files.len()).collect();
        indices.sort_by(|&a, &b| mtimes[a].cmp(&mtimes[b]));
        let sorted: Vec<PathBuf> = indices.iter().map(|&i| files[i].clone()).collect();
        files = sorted;
    }
    "size" => {
        let sizes: Vec<u64> = files
            .iter()
            .map(|p| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0))
            .collect();
        let mut indices: Vec<usize> = (0..files.len()).collect();
        indices.sort_by(|&a, &b| sizes[a].cmp(&sizes[b]));
        let sorted: Vec<PathBuf> = indices.iter().map(|&i| files[i].clone()).collect();
        files = sorted;
    }
    _ => {
        files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    }
}
if sort_desc {
    files.reverse();
}

self.image_files = files;
```

(Pre-collecting metadata into a Vec turns N log N syscalls into N syscalls, a substantial speedup for large folders. The index-sort pattern avoids needing to pair `(PathBuf, metadata)` in a struct.)

- [ ] **Step 3: Verify cargo check passes**

Run: `cargo check`
Expected: clean

- [ ] **Step 4: Verify tests pass**

Run: `cargo test`
Expected: 29 tests pass

- [ ] **Step 5: Bump version and commit**

Edit `Cargo.toml`: `version = "0.1.19"` → `version = "0.1.20"`.

```bash
git add src/app.rs Cargo.toml
git commit -m "perf: cache file metadata once during scan_folder sort

Previously the sort comparator called std::fs::metadata on every
comparison, causing N log N blocking syscalls on the UI thread. For
1000+ files this froze the UI for seconds, especially on network
drives. Now metadata is read N times into a Vec and the sort operates
on indices."
```

---

### Task 5: Stop idle CPU from unconditional request_repaint (Important I6)

**Files:**
- Modify: `src/app.rs:195-358` (update method)
- Test: manual (existing tests must pass; behavior not unit-testable)

**Interfaces:**
- Consumes: `app.viewer_state.is_slideshow`, `app.thumbnail_cache.pending` length
- Produces: `ctx.request_repaint()` only called when there is actual work to do

- [ ] **Step 1: Read the current unconditional request_repaint**

Confirm line 357: `ctx.request_repaint();` at the end of `update`.

- [ ] **Step 2: Replace with conditional repaint**

Edit `src/app.rs`. Replace line 357:

```rust
ctx.request_repaint();
```

with:

```rust
let slideshow_active = matches!(self.mode, Mode::Viewer) && self.viewer_state.is_slideshow;
let pending_thumbs = !self.image_files.is_empty() && {
            let p = self.thumbnail_cache.pending.lock().unwrap();
            !p.is_empty()
        };
if slideshow_active || pending_thumbs {
    ctx.request_repaint();
}
```

Rationale:
- Slideshow needs continuous repaints to advance the timer.
- Pending thumbnail requests mean we need to poll for results next frame.
- Otherwise, egui's default behavior is to sleep until input — saving CPU when idle.

- [ ] **Step 3: Verify cargo check passes**

Run: `cargo check`
Expected: clean

- [ ] **Step 4: Verify tests pass**

Run: `cargo test`
Expected: 29 tests pass

- [ ] **Step 5: Bump version and commit**

Edit `Cargo.toml`: `version = "0.1.20"` → `version = "0.1.21"`.

```bash
git add src/app.rs Cargo.toml
git commit -m "perf: only request_repaint when slideshow or thumbnails pending

Previously ctx.request_repaint() was called every frame unconditionally,
forcing egui to repaint even when idle and causing constant CPU usage.
Now repaints are requested only during slideshow playback or while
thumbnail requests are in flight, letting egui sleep otherwise."
```

---

### Task 6: Unify format→extension mapping (Important C4 + I7)

**Files:**
- Create: `src/format_ext.rs` (new small module)
- Modify: `src/main.rs` (add `mod format_ext;`)
- Modify: `src/batch/operations.rs:15` (use helper)
- Modify: `src/editor/mod.rs` (use helper for save_as)
- Modify: `src/browser/grid.rs` (use helper for save-as in batch dialog if present)
- Test: `src/format_ext.rs` (unit tests)

**Interfaces:**
- Consumes: `image::ImageFormat`
- Produces:
  - `pub fn format_to_extension(format: &str) -> &str` — returns canonical file extension (e.g. "jpeg" → "jpg", "png" → "png")
  - `pub fn extension_to_image_format(ext: &str) -> Option<image::ImageFormat>` — case-insensitive
  - `pub fn is_supported_extension(ext: &str) -> bool` — case-insensitive, matches scan_folder list

- [ ] **Step 1: Create the new module file**

Create `src/format_ext.rs`:

```rust
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
```

- [ ] **Step 2: Register the module in main.rs**

Edit `src/main.rs`. Add `mod format_ext;` alongside the other module declarations. Run `Grep "^mod " src/main.rs` to find the existing list, then insert `mod format_ext;` in alphabetical order.

- [ ] **Step 3: Use the helper in scan_folder**

Edit `src/app.rs`. Replace the extension match block (lines 110-118):

```rust
if let Some(ext) = path.extension() {
    let ext = ext.to_string_lossy().to_lowercase();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "bmp" | "gif" | "tiff" | "tif" | "webp" => {
            files.push(path);
        }
        _ => {}
    }
}
```

with:

```rust
if crate::format_ext::is_supported_extension(&path) {
    files.push(path);
}
```

- [ ] **Step 4: Use the helper in batch_convert**

Edit `src/batch/operations.rs:15`. Replace:

```rust
let new_ext = if format == "jpeg" { "jpg" } else { format };
```

with:

```rust
let new_ext = crate::format_ext::format_to_extension(format);
```

- [ ] **Step 5: Search for other format/extension switches and unify**

Run: `Grep "ImageFormat::|with_extension|\"jpeg\"|\"jpg\"" src/`

Locate `src/editor/mod.rs` save_as (search `Grep "with_extension" src/editor/mod.rs`). If it uses `with_extension("jpeg")`, change to:

```rust
let ext = crate::format_ext::format_to_extension(&app.editor_state.save_format);
let save_path = path.with_extension(ext);
```

(Only make this change if a `with_extension` call exists in `editor/mod.rs`; if the current code already uses a different pattern, adapt accordingly without introducing new logic.)

- [ ] **Step 6: Verify cargo check passes**

Run: `cargo check`
Expected: clean

- [ ] **Step 7: Verify tests pass**

Run: `cargo test`
Expected: 33 tests pass (29 + 4 new — 3 unit tests in format_ext plus one assertion dealing with case-insensitive matching across all 3)

Actually, count carefully: tests added in format_ext.rs are `test_format_to_extension`, `test_extension_to_image_format_case_insensitive`, `test_is_supported_extension` = 3 new tests. Expected total: 32.

- [ ] **Step 8: Bump version and commit**

Edit `Cargo.toml`: `version = "0.1.21"` → `version = "0.1.22"`.

```bash
git add src/format_ext.rs src/main.rs src/app.rs src/batch/operations.rs src/editor/mod.rs Cargo.toml
git commit -m "refactor: unify format→extension mapping in new format_ext module

Previously the jpeg→jpg mapping and extension whitelist were duplicated
in scan_folder, batch_convert, and editor save_as. New format_ext
module provides a single source of truth: format_to_extension,
extension_to_image_format (case-insensitive), and is_supported_extension.
Also fixes the case-sensitivity bug where IMAGE.PNG was skipped."
```

---

### Task 7: Remove accidental tree clone in show_tree (Minor I9)

**Files:**
- Modify: `src/browser/tree.rs:87`

**Interfaces:**
- None (internal cleanup only)

- [ ] **Step 1: Confirm the line**

Run: `Read src/browser/tree.rs` offset 84-94

Confirm line 87: `for node in &app.browser_state.tree_nodes.clone() {`

- [ ] **Step 2: Remove the .clone()**

Edit `src/browser/tree.rs:87`. Replace:

```rust
for node in &app.browser_state.tree_nodes.clone() {
```

with:

```rust
for node in &app.browser_state.tree_nodes {
```

- [ ] **Step 3: Verify cargo check passes**

Run: `cargo check`
Expected: clean (the clone was unnecessary; iterating by reference is sufficient for `show_node`, which takes `&TreeNode`)

- [ ] **Step 4: Verify tests pass**

Run: `cargo test`
Expected: 32 tests pass

- [ ] **Step 5: Bump version and commit**

Edit `Cargo.toml`: `version = "0.1.22"` → `version = "0.1.23"`.

```bash
git add src/browser/tree.rs Cargo.toml
git commit -m "perf: remove unnecessary tree_nodes clone in show_tree

Cloning the Vec<TreeNode> (which contains nested Vecs) on every frame
just to iterate it by reference was wasteful. Iterating &Vec directly
is sufficient and avoids the allocation."
```

---

### Task 8: De-duplicate editor texture upload helper (Important I8)

**Files:**
- Modify: `src/editor/mod.rs:190-257` (apply_op, undo, redo)
- Test: existing tests must pass; refactoring only, no behavior change

**Interfaces:**
- Produces: private `fn upload_texture(app: &mut App, ctx: &egui::Context, img: &image::DynamicImage)`

- [ ] **Step 1: Read the three duplicated blocks in editor/mod.rs**

Run: `Read src/editor/mod.rs` offset 190-257

Confirm the three blocks (apply_op 211-216, undo 230-234, redo 250-254) each:
1. Get `path` from `app.image_files.get(app.selected_image_index)`
2. Compute `tex_key = path.to_string_lossy().to_string()`
3. Convert `img.to_rgba8()` to `ColorImage`
4. `ctx.load_texture(...)` and `app.textures.put(...)` (Task 3 changed `insert` to `put`)

- [ ] **Step 2: Add the helper function**

Edit `src/editor/mod.rs`. Add this function before `apply_op` (around line 190):

```rust
fn upload_texture(app: &mut App, ctx: &egui::Context, img: &image::DynamicImage) {
    let path = match app.image_files.get(app.selected_image_index) {
        Some(p) => p.clone(),
        None => return,
    };
    let tex_key = path.to_string_lossy().to_string();
    let (w, h) = img.dimensions();
    let rgba = img.to_rgba8();
    let ci = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
    let tex = ctx.load_texture(&tex_key, ci, egui::TextureOptions::LINEAR);
    app.textures.put(tex_key, tex);
}
```

- [ ] **Step 3: Replace apply_op's block with a call**

Edit `src/editor/mod.rs`. In `apply_op`, replace lines 207-216:

```rust
let path = match app.image_files.get(app.selected_image_index) {
    Some(p) => p.clone(),
    None => return,
};
let tex_key = path.to_string_lossy().to_string();
let rgba = result.to_rgba8();
let ci = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
let tex = ctx.load_texture(&tex_key, ci, egui::TextureOptions::LINEAR);
app.textures.put(tex_key, tex);
app.editor_state.current_image = Some(result);
```

with:

```rust
upload_texture(app, ctx, &result);
app.editor_state.current_image = Some(result);
```

- [ ] **Step 4: Replace undo's block with a call**

Edit `src/editor/mod.rs`. In `undo`, replace lines 226-235:

```rust
let path = match app.image_files.get(app.selected_image_index) {
    Some(p) => p.clone(),
    None => return,
};
let tex_key = path.to_string_lossy().to_string();
let rgba = prev_img.to_rgba8();
let ci = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
let tex = ctx.load_texture(&tex_key, ci, egui::TextureOptions::LINEAR);
app.textures.put(tex_key, tex);
app.editor_state.current_image = Some(prev_img);
```

with:

```rust
upload_texture(app, ctx, &prev_img);
app.editor_state.current_image = Some(prev_img);
```

- [ ] **Step 5: Replace redo's block with a call**

Edit `src/editor/mod.rs`. In `redo`, replace the analogous block (lines 246-255) with:

```rust
upload_texture(app, ctx, &next_img);
app.editor_state.current_image = Some(next_img);
```

- [ ] **Step 6: Verify cargo check passes**

Run: `cargo check`
Expected: clean (some `w`/`h` locals in the surrounding code may now be unused — check for warnings; if so, remove the unused `let (w, h) = ...dimensions();` lines that were only used for the texture upload, but keep the ones used for `resize_width`/`resize_height`)

- [ ] **Step 7: Verify tests pass**

Run: `cargo test`
Expected: 32 tests pass

- [ ] **Step 8: Bump version and commit**

Edit `Cargo.toml`: `version = "0.1.23"` → `version = "0.1.24"`.

```bash
git add src/editor/mod.rs Cargo.toml
git commit -m "refactor: extract upload_texture helper in editor

apply_op/undo/redo each duplicated ~10 lines for converting a
DynamicImage to a ColorImage, loading it as a texture, and inserting
into app.textures. Now a single upload_texture helper handles all three.
Reduces duplication and ensures consistent texture key behavior."
```

---

### Task 9: De-duplicate viewer keyboard nav and fix prev-image flicker (Important I10)

**Files:**
- Modify: `src/viewer.rs:217-232`

**Interfaces:**
- Consumes: `app.prev_image()`, `app.next_image()` from `src/app.rs`
- Produces: keyboard handler calls existing methods; no manual index fiddling

- [ ] **Step 1: Confirm the duplicated code**

Run: `Read src/viewer.rs` offset 213-232

Confirm:
- Line 217-223: ArrowLeft manually does `saturating_sub(1)` and sets `image_loaded = false` — bug: when index is 0, `saturating_sub` returns 0 but still resets `image_loaded = false`, causing a redundant texture reload next frame.
- Line 225-232: ArrowRight manually increments index — duplicates `next_image()` logic in `app.rs:167-176`.

- [ ] **Step 2: Replace both handlers with calls to app methods**

Edit `src/viewer.rs`. Replace lines 217-232:

```rust
egui::Key::ArrowLeft => {
    if !app.viewer_state.is_slideshow {
        let prev =
            app.selected_image_index.saturating_sub(1);
        app.selected_image_index = prev;
        app.viewer_state.image_loaded = false;
    }
}
egui::Key::ArrowRight => {
    if !app.viewer_state.is_slideshow
        && app.selected_image_index + 1 < app.image_files.len()
    {
        app.selected_image_index += 1;
        app.viewer_state.image_loaded = false;
    }
}
```

with:

```rust
egui::Key::ArrowLeft => {
    if !app.viewer_state.is_slideshow {
        app.prev_image();
    }
}
egui::Key::ArrowRight => {
    if !app.viewer_state.is_slideshow {
        app.next_image();
    }
}
```

`prev_image()` already checks `if self.selected_image_index > 0` (app.rs:179) before mutating, which fixes the index-0 flicker. `next_image()` already bounds-checks (app.rs:168). Both set `viewer_state.image_loaded = false` and refresh editor/exif state — which the inline code was *not* doing, so this is also a behavior fix.

- [ ] **Step 3: Verify cargo check passes**

Run: `cargo check`
Expected: clean

- [ ] **Step 4: Verify tests pass**

Run: `cargo test`
Expected: 32 tests pass

- [ ] **Step 5: Bump version and commit**

Edit `Cargo.toml`: `version = "0.1.24"` → `version = "0.1.25"`.

```bash
git add src/viewer.rs Cargo.toml
git commit -m "fix: viewer arrow keys call prev_image/next_image instead of inline logic

Inline ArrowLeft handler used saturating_sub which reset image_loaded
even when the index didn't change, causing a redundant flicker at index 0.
Inline ArrowRight handler skipped the editor_state.load_image and
exif_state.parse refreshes that next_image() performs. Both now call
the existing App methods, which handle bounds checks and state refresh."
```

---

### Task 10: Remove duplicate thumbnail_cache.poll() in show_grid (Important I12)

**Files:**
- Modify: `src/browser/grid.rs:99-101`

**Interfaces:**
- None (internal cleanup)

- [ ] **Step 1: Confirm the duplicate poll**

Run: `Grep "thumbnail_cache.poll" src/`

Should find two matches:
- `src/app.rs:196` — in `App::update` (the authoritative one)
- `src/browser/grid.rs:99` — in `show_grid` (the duplicate)

With mpsc's `try_recv`, only one consumer receives each message. So when `App::update` polls first (which it does, since `update` runs before `show_grid`), `show_grid` will rarely get anything — but the double-poll is wasteful and confusing.

- [ ] **Step 2: Remove the duplicate poll from show_grid**

Edit `src/browser/grid.rs`. Delete lines 99-101:

```rust
while let Some(result) = app.thumbnail_cache.poll() {
    app.browser_state.thumbnails.insert(result.path, result.image);
}
```

(The poll in `App::update` at app.rs:196-198 handles this; `show_grid` reads from `app.browser_state.thumbnails` which `App::update` populates.)

- [ ] **Step 3: Verify cargo check passes**

Run: `cargo check`
Expected: clean

- [ ] **Step 4: Verify tests pass**

Run: `cargo test`
Expected: 32 tests pass

- [ ] **Step 5: Bump version and commit**

Edit `Cargo.toml`: `version = "0.1.25"` → `version = "0.1.26"`.

```bash
git add src/browser/grid.rs Cargo.toml
git commit -m "refactor: remove duplicate thumbnail_cache.poll in show_grid

App::update already polls thumbnail_cache and populates
browser_state.thumbnails each frame; the second poll in show_grid was
both redundant (mpsc delivers each message once) and confusing."
```

---

## Self-Review Checklist

After completing all 10 tasks, run this checklist:

- [ ] **All 32 tests pass**: `cargo test` exits 0
- [ ] **cargo check clean**: no new errors or warnings introduced
- [ ] **git log shows 10 commits**, each with a clear message and version bump
- [ ] **No new external dependencies** added to Cargo.toml (only `lru` was already present)
- [ ] **All issues from code review addressed**:
  - C1 (config save thrash) → Task 1
  - C2 (thread leak) → Task 2
  - C3 (texture overwrite) → subsumed by Task 3 (LRU eviction + `contains` check means stale textures are replaced)
  - C4 (jpg/jpeg inconsistency) → Task 6
  - I2 (UI-thread metadata) → Task 4
  - I3 (unbounded textures) → Task 3
  - I6 (idle CPU) → Task 5
  - I7 (PNG/BMP hardcoded) → subsumed by Task 6 (extension_to_image_format handles all formats in one place)
  - I8 (editor DRY) → Task 8
  - I9 (tree clone) → Task 7
  - I10 (keyboard nav) → Task 9
  - I12 (duplicate poll) → Task 10
- [ ] **Issues deliberately deferred** (with rationale):
  - I4 (DefaultHasher stability): would require adding a new dependency (xxhash/blake3); deferred per "no new dependencies" constraint. Disk cache rebuild is acceptable.
  - I5 (mtime fallback): requires content-hash fallback, complex; deferred.
  - I11 (slideshow end feedback): UX polish, not a bug; deferred.
  - M4 (batch_resize in-place overwrite): behavior change requiring UI confirmation dialog; deferred to future task.
  - M5 (browser/files.rs dead code): requires UI wiring decision (remove vs implement); deferred.
  - M6/M1 (config path): behavior change; currently portable (exe-adjacent) which some users prefer; deferred until spec decision.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-07-04-codebase-fixes.md`. Two execution options:

**1. Subagent-Driven (recommended)** — Dispatch a fresh subagent per task, review between tasks, fast iteration. Best for catching issues early and keeping context clean. Use `superpowers:subagent-driven-development` skill.

**2. Inline Execution** — Execute tasks in this session using `superpowers:executing-plans`, batch execution with checkpoints. Best if you want to watch progress live.

Which approach?
