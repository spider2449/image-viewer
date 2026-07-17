# Code Review Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the bugs, performance issues, and cleanups found in the 2026-07-17 codebase review: silent JPEG-save failure, CJK filename panic, broken last-folder restore, dead lock-aspect checkboxes, unsafe delete, busy-polling thumbnail workers, unbounded texture map, blocking editor decode, unreadable date column, doc drift, and dead code.

**Architecture:** Small targeted fixes within the existing module layout. One new shared helper (`format_ext::save_image`) deduplicates the three image-save code paths. The thumbnail cache switches from sleep-polling `try_recv` to blocking `recv` with channel-disconnect shutdown. No new modules.

**Tech Stack:** Rust, egui/eframe 0.31, image 0.25, lru 0.12. New dependency: `trash = "5"` (recycle-bin delete). Removed dependency: `dirs-next`.

## Global Constraints

- ALL code, comments, names in English only.
- Bump the patch version in `Cargo.toml` before each commit (manual convention, e.g. 0.1.38 → 0.1.39 for Task 1, 0.1.40 for Task 2, …).
- Verification commands: `cargo check` and `cargo test`. Run `cargo test` after every implementation step; the full suite must pass before each commit.
- Commit messages end with: `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`

---

### Task 1: Fix CJK filename truncation panic in thumbnail grid

`&name[..17]` in `grid.rs` slices by bytes and panics when byte 17 is not a UTF-8 char boundary (CJK filenames are a supported use case — the app ships a CJK font loader).

**Files:**
- Modify: `src/browser/grid.rs:268-272` (truncation site) and its `tests` module
- Test: `src/browser/grid.rs` (inline `#[cfg(test)]`)

**Interfaces:**
- Produces: `fn truncate_name(name: &str, max_chars: usize) -> String` (private to `grid.rs`)

- [ ] **Step 1: Write the failing tests**

Add to the existing `mod tests` in `src/browser/grid.rs`:

```rust
use super::truncate_name;

#[test]
fn test_truncate_name_short_unchanged() {
    assert_eq!(truncate_name("short.png", 18), "short.png");
}

#[test]
fn test_truncate_name_long_ascii() {
    let name = "a_very_long_filename_indeed";
    let t = truncate_name(name, 18);
    assert_eq!(t.chars().count(), 18); // 17 chars + ellipsis
    assert!(t.ends_with('…'));
}

#[test]
fn test_truncate_name_cjk_no_panic() {
    // 20 CJK chars = 60 bytes; byte-slicing at 17 would panic
    let name = "测试文件名称非常长的图片文件示例一二三四";
    let t = truncate_name(name, 18);
    assert!(t.ends_with('…'));
    assert_eq!(t.chars().count(), 18);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test truncate_name`
Expected: compile error "cannot find function `truncate_name`"

- [ ] **Step 3: Implement `truncate_name` and use it**

Add near the bottom of `src/browser/grid.rs` (next to `format_size`):

```rust
fn truncate_name(name: &str, max_chars: usize) -> String {
    if name.chars().count() > max_chars {
        let truncated: String = name.chars().take(max_chars - 1).collect();
        format!("{truncated}…")
    } else {
        name.to_string()
    }
}
```

Replace the truncation in `show_thumbnail_grid` (currently around line 268):

```rust
// OLD:
let display_name = if name.len() > 18 {
    format!("{}…", &name[..17])
} else {
    name
};
// NEW:
let display_name = truncate_name(&name, 18);
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test truncate_name`
Expected: 3 tests PASS. Then `cargo test` — all pass.

- [ ] **Step 5: Bump version and commit**

```bash
# In Cargo.toml: version = "0.1.39"
git add src/browser/grid.rs Cargo.toml
git commit -m "fix: char-safe filename truncation in thumbnail grid (CJK panic)

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 2: Shared save helper — fix silent RGBA→JPEG save failures

The grid context menu (`grid.rs:310`) and editor `save_as` (`editor/mod.rs:291`) encode `Rgba8` into `JpegEncoder`, which errors (JPEG has no alpha) and the error is swallowed by `.ok()` — the user gets no file and no message. `batch_convert` does it correctly with `to_rgb8()`. Extract one helper and use it in all three places. Also fixes the extension inconsistency (grid wrote `.jpeg`, others `.jpg`).

**Files:**
- Modify: `src/format_ext.rs` (add helper + tests)
- Modify: `src/editor/mod.rs:280-297` (`save_as`)
- Modify: `src/browser/grid.rs:301-323` (context-menu "Save as")
- Modify: `src/batch/operations.rs:17-32` (`batch_convert` save arm)

**Interfaces:**
- Produces: `pub fn save_image(img: &image::DynamicImage, path: &std::path::Path, format: &str, jpeg_quality: u8) -> Result<(), String>` in `src/format_ext.rs`. `format` is one of `"png" | "jpeg" | "bmp" | "webp"`; the caller chooses `path` (callers derive the extension via existing `format_to_extension`).

- [ ] **Step 1: Write the failing test**

Add to `mod tests` in `src/format_ext.rs`:

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test save_image`
Expected: compile error "cannot find function `save_image`"

- [ ] **Step 3: Implement `save_image`**

In `src/format_ext.rs`, add imports and the function:

```rust
use image::{DynamicImage, GenericImageView};

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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test save_image`
Expected: 2 tests PASS.

- [ ] **Step 5: Use the helper in all three call sites**

`src/editor/mod.rs` — replace the body of `save_as` from `let save_format = ...` down to the `if result.is_some()` block with:

```rust
    let save_format = app.editor_state.save_format;
    let new_ext = crate::format_ext::format_to_extension(save_format);
    let new_name = path.with_extension(new_ext);
    match crate::format_ext::save_image(&img, &new_name, save_format, app.editor_state.save_jpeg_quality) {
        Ok(()) => app.scan_folder(),
        Err(e) => eprintln!("Save failed: {e}"),
    }
```

`src/browser/grid.rs` — replace the `save` closure in the context menu with:

```rust
    ui.menu_button("Save as", |ui| {
        let mut save = |fmt: &str| {
            if let Ok(img) = image::open(path) {
                let new_name = path.with_extension(crate::format_ext::format_to_extension(fmt));
                if let Err(e) = crate::format_ext::save_image(&img, &new_name, fmt, app.editor_state.save_jpeg_quality) {
                    eprintln!("Save failed: {e}");
                } else {
                    app.scan_folder();
                }
            }
        };
        if ui.button("PNG").clicked() { save("png"); ui.close_menu(); }
        if ui.button("JPEG").clicked() { save("jpeg"); ui.close_menu(); }
        if ui.button("BMP").clicked() { save("bmp"); ui.close_menu(); }
        if ui.button("WEBP").clicked() { save("webp"); ui.close_menu(); }
    });
```

(The old closure's second parameter `img_fmt: image::ImageFormat` is gone; the `use image::GenericImageView;` import in grid.rs may become unused — remove it if `cargo check` warns.)

`src/batch/operations.rs` — replace the whole `let result = match format { ... }` block in `batch_convert` with:

```rust
        let result = crate::format_ext::save_image(&img, &new_name, format, jpeg_quality);
```

(Remove the now-unused `use image::GenericImageView;` at the top if `cargo check` warns; `batch_resize` still needs it — check before removing.)

- [ ] **Step 6: Verify**

Run: `cargo check` then `cargo test`
Expected: no warnings about the changed files, all tests pass (including existing `test_batch_convert_png_to_jpeg`).

- [ ] **Step 7: Bump version and commit**

```bash
# In Cargo.toml: version = "0.1.40"
git add src/format_ext.rs src/editor/mod.rs src/browser/grid.rs src/batch/operations.rs Cargo.toml
git commit -m "fix: RGBA-to-JPEG saves silently failed; unify saving via format_ext::save_image

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 3: Persist `last_folder` so startup restore works

`config.last_folder` is read at startup (`app.rs:76`) but never written anywhere, so the restore feature never fires.

**Files:**
- Modify: `src/app.rs` (`scan_folder`)

**Interfaces:**
- Consumes: `Config::save()` (existing).
- Produces: no new API — `scan_folder` now records and saves `last_folder`.

- [ ] **Step 1: Record the folder in `scan_folder`**

In `src/app.rs`, in `scan_folder`, right after `folder` is resolved (after the `let folder = match &self.current_folder { ... };` block), add:

```rust
        let folder_str = folder.to_string_lossy().to_string();
        if self.config.last_folder.as_deref() != Some(folder_str.as_str()) {
            self.config.last_folder = Some(folder_str);
            self.config.save();
        }
```

- [ ] **Step 2: Verify**

Run: `cargo check` then `cargo test`
Expected: clean. Manual check (optional): run the app, open a folder, confirm `cache/config.json` next to the binary now contains `"last_folder"` with that path.

- [ ] **Step 3: Bump version and commit**

```bash
# In Cargo.toml: version = "0.1.41"
git add src/app.rs Cargo.toml
git commit -m "fix: persist last_folder so startup folder restore actually works

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 4: Wire up the "Lock aspect ratio" checkboxes

Both `editor_state.resize_lock_aspect` and `batch_state.resize_lock_aspect` render as checkboxes but are never read — `resize_exact` always distorts.

**Files:**
- Modify: `src/batch/operations.rs` (`batch_resize` signature + logic + tests)
- Modify: `src/batch/mod.rs:222` (call site)
- Modify: `src/editor/mod.rs:149-162` (W/H drag values)

**Interfaces:**
- Produces: `pub fn batch_resize(files: &[PathBuf], width: u32, height: u32, lock_aspect: bool) -> Result<(), Vec<String>>` — when `lock_aspect` is true, images are fit within `width`×`height` preserving aspect (`DynamicImage::resize`); when false, exact distorting resize as before.

- [ ] **Step 1: Write the failing test**

Add to `mod tests` in `src/batch/operations.rs`:

```rust
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
```

Also update the two existing `batch_resize` tests to pass `false` as the new fourth argument.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test batch_resize`
Expected: compile error (wrong number of arguments) — that counts as the failing state; the new behavior doesn't exist yet.

- [ ] **Step 3: Implement**

In `src/batch/operations.rs`, change the signature and the resize line:

```rust
pub fn batch_resize(
    files: &[PathBuf],
    width: u32,
    height: u32,
    lock_aspect: bool,
) -> Result<(), Vec<String>> {
```

```rust
        // OLD:
        let resized = img.resize_exact(width, height, image::imageops::FilterType::Lanczos3);
        // NEW:
        let resized = if lock_aspect {
            img.resize(width, height, image::imageops::FilterType::Lanczos3)
        } else {
            img.resize_exact(width, height, image::imageops::FilterType::Lanczos3)
        };
```

In `src/batch/mod.rs`, update the call site inside the `BatchMode::Resize` arm:

```rust
                        let result = operations::batch_resize(&selected, w, h, app.batch_state.resize_lock_aspect);
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test batch_resize`
Expected: 3 tests PASS.

- [ ] **Step 5: Wire the editor checkbox (couple W and H drags)**

In `src/editor/mod.rs`, replace the two `ui.horizontal` blocks for W and H in the Resize section with:

```rust
                let aspect = app.editor_state.current_image.as_ref().map(|img| {
                    let (iw, ih) = img.dimensions();
                    (iw.max(1), ih.max(1))
                });
                ui.horizontal(|ui| {
                    ui.colored_label(colors.text_secondary, "W:");
                    let mut w = app.editor_state.resize_width as f32;
                    if ui.add(egui::DragValue::new(&mut w).range(1..=16384)).changed() {
                        app.editor_state.resize_width = w as u32;
                        if app.editor_state.resize_lock_aspect {
                            if let Some((iw, ih)) = aspect {
                                app.editor_state.resize_height =
                                    ((w * ih as f32 / iw as f32).round() as u32).max(1);
                            }
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.colored_label(colors.text_secondary, "H:");
                    let mut h = app.editor_state.resize_height as f32;
                    if ui.add(egui::DragValue::new(&mut h).range(1..=16384)).changed() {
                        app.editor_state.resize_height = h as u32;
                        if app.editor_state.resize_lock_aspect {
                            if let Some((iw, ih)) = aspect {
                                app.editor_state.resize_width =
                                    ((h * iw as f32 / ih as f32).round() as u32).max(1);
                            }
                        }
                    }
                });
```

(The Apply button keeps using `EditOp::Resize` with `resize_exact` — the coupling above already guarantees the locked ratio.)

- [ ] **Step 6: Verify**

Run: `cargo check` then `cargo test`
Expected: all pass, no warnings.

- [ ] **Step 7: Bump version and commit**

```bash
# In Cargo.toml: version = "0.1.42"
git add src/batch/operations.rs src/batch/mod.rs src/editor/mod.rs Cargo.toml
git commit -m "fix: honor Lock aspect ratio in editor resize and batch resize

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 5: Delete to recycle bin instead of permanent removal

Right-click → Delete calls `fs::remove_file` with no confirmation. Route it through the recycle bin via the `trash` crate so it's recoverable.

**Files:**
- Modify: `Cargo.toml` (add `trash = "5"`)
- Modify: `src/browser/files.rs` (`FileOp::Delete` arm + test)

**Interfaces:**
- Consumes/Produces: `files::execute(FileOp::Delete { path })` signature unchanged; behavior now moves the file to the OS recycle bin.

- [ ] **Step 1: Add the dependency**

In `Cargo.toml` `[dependencies]`, add:

```toml
trash = "5"
```

Run: `cargo check`
Expected: compiles (fetches trash crate).

- [ ] **Step 2: Write the failing test**

Add to `src/browser/files.rs` (create the tests module — there isn't one yet):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delete_moves_file_out_of_place() {
        let dir = std::env::temp_dir().join("files_test_delete");
        let _ = std::fs::create_dir_all(&dir);
        let target = dir.join("delete_me.png");
        std::fs::write(&target, b"data").unwrap();

        let result = execute(FileOp::Delete { path: target.clone() });
        assert!(result.is_ok(), "{result:?}");
        assert!(!target.exists(), "file must no longer exist at original path");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
```

- [ ] **Step 3: Run test to verify current behavior**

Run: `cargo test test_delete_moves_file_out_of_place`
Expected: PASSES already (remove_file also removes it) — this test pins the contract; the change in Step 4 is behavioral (recycle bin), which the test still satisfies. Note this deviation from strict red-green: the safety property (recoverability) isn't unit-testable portably.

- [ ] **Step 4: Switch to trash**

In `src/browser/files.rs`, replace the Delete arm:

```rust
        FileOp::Delete { path } => {
            trash::delete(&path).map_err(|e| format!("Delete failed: {e}"))
        }
```

- [ ] **Step 5: Verify**

Run: `cargo test test_delete_moves_file_out_of_place` then `cargo test`
Expected: PASS. Manual check (optional): delete an image via the context menu in the running app and confirm it appears in the Windows Recycle Bin.

- [ ] **Step 6: Bump version and commit**

```bash
# In Cargo.toml: version = "0.1.43"
git add Cargo.toml Cargo.lock src/browser/files.rs
git commit -m "fix: delete files to recycle bin via trash crate

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 6: Thumbnail cache — blocking recv + HashSet pending

Worker and store threads currently spin on `try_recv()` + 10 ms sleep (constant idle CPU), and `pending` is a `Vec` with O(n) scans hit every frame for every file. Switch to blocking `recv()` (threads exit when the channel disconnects on drop — the `shutdown` AtomicBool and `Drop` impl go away) and make `pending` a `HashSet`.

**Files:**
- Modify: `src/thumbnail_cache.rs` (whole worker setup, `pending` type, tests)

**Interfaces:**
- Consumes/Produces: public API unchanged (`new`, `request`, `poll`, `has_pending`, `clear`, `clear_disk_cache`, `get_cached`).
- Note: the `Ordering`/`AtomicBool` imports, the `shutdown` field, the `Drop` impl, and the `test_shutdown_flag_set_on_drop` test are all removed.

- [ ] **Step 1: Update tests first**

In `src/thumbnail_cache.rs` `mod tests`: delete `test_shutdown_flag_set_on_drop` (the flag no longer exists; thread exit now follows from channel disconnect, which can't be observed portably from a unit test). Keep `test_clear_empties_cache_and_pending` as-is — it must still compile and pass after the `HashSet` change. Add:

```rust
    #[test]
    fn test_request_dedupes_pending() {
        let cache = ThumbnailCache::new(16, 1, None);
        cache.request(PathBuf::from("test.png"), 100);
        cache.request(PathBuf::from("test.png"), 100);
        assert_eq!(cache.pending.lock().unwrap().len(), 1);
    }
```

- [ ] **Step 2: Rework the cache**

Replace `src/thumbnail_cache.rs` structure as follows.

Imports — remove `AtomicBool`/`Ordering` and `TryRecvError`, add `HashSet`:

```rust
use std::collections::HashSet;
use std::sync::mpsc::{self, Receiver, Sender};
```

Struct — drop `shutdown`, change `pending`:

```rust
pub struct ThumbnailCache {
    cache: Arc<Mutex<LruCache<PathBuf, (ColorImage, u32, u32)>>>,
    pending: Arc<Mutex<HashSet<PathBuf>>>,
    sender: Sender<ThumbnailRequest>,
    receiver: Receiver<ThumbnailResult>,
    disk_cache: Option<Arc<DiskCache>>,
}
```

Store thread — blocking recv, exits on disconnect (all decoder-held `store_tx` clones dropped):

```rust
        if let Some(ref dc) = disk_cache {
            let dc = dc.clone();
            let store_rx = Arc::new(Mutex::new(store_rx));
            thread::spawn(move || loop {
                let req = {
                    let rx = store_rx.lock().unwrap();
                    rx.recv()
                };
                match req {
                    Ok(req) => dc.store(&req.path, req.max_size, &req.image),
                    Err(_) => break, // channel disconnected: all senders gone
                }
            });
        }
```

Decoder threads — same pattern; the body inside `Ok(req) =>` is byte-for-byte the current `Some(req)` branch (disk-cache lookup, decode, store enqueue, `res_tx.send`):

```rust
        for _ in 0..worker_count.max(1) {
            let cache_clone = cache.clone();
            let res_tx = res_tx.clone();
            let req_rx = req_rx.clone();
            let dc = disk_cache.clone();
            let store_tx = store_tx.clone();

            thread::spawn(move || loop {
                let req = {
                    let rx = req_rx.lock().unwrap();
                    rx.recv()
                };
                let Ok(req) = req else { break };
                // ... existing decode/lookup/store/send body, unchanged ...
            });
        }
```

After the loop, drop the original `store_tx` so the store thread's disconnect works once decoders exit:

```rust
        drop(store_tx);
```

`pending` construction and methods:

```rust
        let pending: Arc<Mutex<HashSet<PathBuf>>> = Arc::new(Mutex::new(HashSet::new()));
```

```rust
    pub fn request(&self, path: PathBuf, max_size: u32) {
        {
            let mut p = self.pending.lock().unwrap();
            if !p.insert(path.clone()) {
                return; // already pending
            }
        }
        self.sender.send(ThumbnailRequest { path, max_size }).ok();
    }

    pub fn poll(&self) -> Option<ThumbnailResult> {
        let result = self.receiver.try_recv().ok();
        if let Some(ref r) = result {
            self.pending.lock().unwrap().remove(&r.path);
        }
        result
    }
```

Delete the `impl Drop for ThumbnailCache` block and remove the two `sd.load(...)` checks and both `thread::sleep(Duration::from_millis(10))` idle arms. `Instant`/`Duration` imports stay (still used by `load_time`); drop `Duration` only if `cargo check` says it's unused.

Threads now exit because dropping `ThumbnailCache` drops `sender`, disconnecting `req_rx` → decoders break → their `store_tx` clones drop → store thread breaks.

- [ ] **Step 3: Verify**

Run: `cargo check` then `cargo test`
Expected: no warnings in `thumbnail_cache.rs`; `test_request_dedupes_pending` and `test_clear_empties_cache_and_pending` pass. Manual check (optional): open a large folder, confirm thumbnails still stream in, and idle CPU of the process is ~0% afterward.

- [ ] **Step 4: Bump version and commit**

```bash
# In Cargo.toml: version = "0.1.44"
git add src/thumbnail_cache.rs Cargo.toml
git commit -m "perf: blocking recv in thumbnail workers, HashSet pending dedup

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 7: Lazy editor image load — stop double-decoding on every navigation

`editor_state.load_image` synchronously decodes the full image on every `switch_to_viewer` / `next_image` / `prev_image`, even when the editor panel is closed — on top of the viewer's own decode. Only load when the panel is visible, and load on-demand when it's opened.

**Files:**
- Modify: `src/app.rs` (`switch_to_viewer`, `next_image`, `prev_image`)
- Modify: `src/viewer.rs:99-101` (Edit toggle)

**Interfaces:**
- Consumes: `editor::State::load_image(&PathBuf)` (existing).
- Produces: no new API. Contract: `editor_state.current_image` is `Some` only while the editor panel is (or was) visible for the current image; all editor actions already guard on `current_image` being `Some`.

- [ ] **Step 1: Gate loading in app.rs**

In `src/app.rs`, in each of `switch_to_viewer`, `next_image`, and `prev_image`, replace:

```rust
            if let Some(p) = self.image_files.get(/* index expr as in each fn */) {
                self.editor_state.load_image(p);
                self.exif_state.parse(p);
            }
```

with:

```rust
            if let Some(p) = self.image_files.get(/* same index expr */) {
                if self.editor_state.visible {
                    self.editor_state.load_image(p);
                } else {
                    self.editor_state.current_image = None;
                }
                self.exif_state.parse(p);
            }
```

(Setting `current_image = None` also implicitly clears stale undo/redo relevance; `load_image` clears the stacks when the panel is next opened.)

- [ ] **Step 2: Load on toggle in viewer.rs**

In `src/viewer.rs`, replace the Edit toggle:

```rust
                    if ui.selectable_label(app.editor_state.visible, "Edit").clicked() {
                        app.editor_state.visible = !app.editor_state.visible;
                        if app.editor_state.visible && app.editor_state.current_image.is_none() {
                            app.editor_state.load_image(&path);
                        }
                    }
```

- [ ] **Step 3: Verify**

Run: `cargo check` then `cargo test`
Expected: clean. Manual check: open viewer with editor closed, navigate with arrows (should feel snappier on large images); open Edit panel and confirm rotate/undo still work; navigate with the panel open and confirm edits target the new image.

- [ ] **Step 4: Bump version and commit**

```bash
# In Cargo.toml: version = "0.1.45"
git add src/app.rs src/viewer.rs Cargo.toml
git commit -m "perf: load editor image lazily, only when editor panel is visible

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 8: Bound `thumb_textures` with an LRU

`browser_state.thumb_textures` is an unbounded `HashMap` of GPU textures — it grows to one texture per file in the folder until the next `scan_folder`. Cap it with the `lru` crate (already a dependency).

**Files:**
- Modify: `src/browser/mod.rs` (State field + init)
- Modify: `src/browser/grid.rs` (get/insert usage)
- Modify: `src/app.rs:102,225` (`.clear()` call sites — API compatible, no change expected, but verify)

**Interfaces:**
- Produces: `pub thumb_textures: lru::LruCache<PathBuf, egui::TextureHandle>` with capacity 512. `LruCache::clear`, `get` (takes `&mut self`), `put` replace `HashMap::clear/get/insert`.

- [ ] **Step 1: Change the field**

In `src/browser/mod.rs`:

```rust
use lru::LruCache;
use std::num::NonZeroUsize;
```

```rust
    pub thumb_textures: LruCache<PathBuf, egui::TextureHandle>,
```

```rust
            thumb_textures: LruCache::new(NonZeroUsize::new(512).unwrap()),
```

- [ ] **Step 2: Update usages**

In `src/browser/grid.rs` `show_thumbnail_grid`, the texture lookup becomes:

```rust
                            let tex = if let Some(t) = app.browser_state.thumb_textures.get(path) {
                                t.clone()
                            } else {
                                let key = format!("thumb_{}", path.to_string_lossy());
                                let t = ctx.load_texture(&key, ci.clone(), TextureOptions::LINEAR);
                                app.browser_state.thumb_textures.put(path.clone(), t.clone());
                                t
                            };
```

(`get` on `LruCache` needs `&mut` — `app` is already `&mut App`, so this compiles; `insert` → `put`.)

The `.clear()` calls in `src/app.rs` (`scan_folder`, "Clear thumbnail cache" menu) and `src/browser/grid.rs:119` (size change) keep the same name on `LruCache` — no edits needed, but confirm with `cargo check`.

- [ ] **Step 3: Verify**

Run: `cargo check` then `cargo test`
Expected: clean build, tests pass.

- [ ] **Step 4: Bump version and commit**

```bash
# In Cargo.toml: version = "0.1.46"
git add src/browser/mod.rs src/browser/grid.rs Cargo.toml
git commit -m "perf: bound thumbnail GPU textures with a 512-entry LRU

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 9: Readable date column in list view

The list view renders modification time as "20649d 13:42" (days since Unix epoch). Format a real UTC date instead using the standard civil-from-days algorithm (no new dependency).

**Files:**
- Modify: `src/browser/grid.rs` (date cell + new helpers + tests)

**Interfaces:**
- Produces: `fn format_timestamp(secs_since_epoch: u64) -> String` returning `"YYYY-MM-DD HH:MM"` (UTC), private to `grid.rs`.

- [ ] **Step 1: Write the failing tests**

Add to `mod tests` in `src/browser/grid.rs`:

```rust
use super::format_timestamp;

#[test]
fn test_format_timestamp_epoch() {
    assert_eq!(format_timestamp(0), "1970-01-01 00:00");
}

#[test]
fn test_format_timestamp_known_date() {
    // 2026-07-17 12:34:56 UTC
    assert_eq!(format_timestamp(1_784_291_696), "2026-07-17 12:34");
}
```

(Verify the second constant before committing: `1_784_291_696 = days(1970-01-01..2026-07-17) * 86400 + 12*3600 + 34*60 + 56`. If the assertion disagrees with an independent check — e.g. PowerShell `[DateTimeOffset]::FromUnixTimeSeconds(1784291696).UtcDateTime` — fix the constant, not the algorithm.)

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test format_timestamp`
Expected: compile error "cannot find function `format_timestamp`"

- [ ] **Step 3: Implement**

Add to `src/browser/grid.rs` (near `format_size`):

```rust
// Civil-from-days algorithm (Howard Hinnant), UTC.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn format_timestamp(secs_since_epoch: u64) -> String {
    let days = (secs_since_epoch / 86_400) as i64;
    let time = secs_since_epoch % 86_400;
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02}", time / 3600, (time % 3600) / 60)
}
```

In `show_list_view`, replace the date cell computation:

```rust
                let date_str = meta.as_ref()
                    .and_then(|m| m.modified().ok())
                    .and_then(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|dt| format_timestamp(dt.as_secs()))
                    .unwrap_or_else(|| "-".to_string());
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test format_timestamp`
Expected: 2 tests PASS. Then `cargo test` — all pass.

- [ ] **Step 5: Bump version and commit**

```bash
# In Cargo.toml: version = "0.1.47"
git add src/browser/grid.rs Cargo.toml
git commit -m "fix: list view date column shows real YYYY-MM-DD HH:MM instead of epoch days

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

### Task 10: Cleanup — dead code, duplicate button, doc drift, unused dependency

**Files:**
- Modify: `src/viewer.rs` (remove `image_loaded`)
- Modify: `src/app.rs` (remove `image_loaded` writes)
- Modify: `src/browser/grid.rs` (remove duplicate Back button, simplify `col_widths`)
- Modify: `CLAUDE.md`, `AGENTS.md`, `README.md` (config path)
- Modify: `Cargo.toml` (remove `dirs-next`)

**Interfaces:** none — deletions only.

- [ ] **Step 1: Remove write-only `image_loaded`**

Delete the field `pub image_loaded: bool` and its `image_loaded: false` initializer from `viewer::State`, and delete every `... .image_loaded = false;` statement (`src/viewer.rs:61,161,167,281` and `src/app.rs:171,182`). It is never read anywhere.

- [ ] **Step 2: Remove the duplicate toolbar button**

In `src/browser/grid.rs` `show_grid`, the "Back" and "Up" buttons are identical (both go to parent). Delete the entire `if ui.button("Back")` block, keep "Up".

- [ ] **Step 3: Simplify `col_widths`**

In `src/browser/grid.rs`:

```rust
fn col_widths(cw: &crate::config::ColumnWidths, available: f32, icon_w: f32, min_w: f32, gap: f32) -> ColumnWidthSet {
    let name = cw.name.max(min_w);
    let size = cw.size.max(min_w);
    let fixed = icon_w + name + gap + size + gap;
    let date = cw.date.max(min_w).max(available - fixed);
    ColumnWidthSet { name, size, date }
}
```

- [ ] **Step 4: Fix doc drift and drop dirs-next**

- `Cargo.toml`: delete the `dirs-next = "2"` line (grep confirms no `dirs_next` usage in `src/`).
- `CLAUDE.md` line "**Config:** JSON at `dirs_next::config_dir()/image-viewer/config.json`." → "**Config:** JSON at `<exe dir>/cache/config.json` (portable, next to the binary)."
- `AGENTS.md` line 22: same replacement.
- `README.md` line 56: "Settings are stored as JSON in a `cache/` folder next to the executable (portable install)."

- [ ] **Step 5: Verify**

Run: `cargo check` then `cargo test`
Expected: no `dead_code` warnings for the removed items, all tests pass.

- [ ] **Step 6: Bump version and commit**

```bash
# In Cargo.toml: version = "0.1.48"
git add src/viewer.rs src/app.rs src/browser/grid.rs Cargo.toml Cargo.lock CLAUDE.md AGENTS.md README.md
git commit -m "chore: remove dead image_loaded state, duplicate Back button, dirs-next dep; fix config-path docs

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>"
```

---

## Deferred (documented, not in this plan)

- Off-UI-thread decode of the full-size viewer image (larger async refactor; revisit if navigation still stutters after Task 7).
- Confirmation dialog for batch resize overwriting originals (behavioral/UX decision for the user; recycle-bin delete in Task 5 covers the worst data-loss path).
- Disk-cache `delete_old_entries` full-directory scan per write (only matters at tens of thousands of cached thumbnails).
- Save-on-change for all config fields (Task 3 covers `last_folder`; sort/theme already save at their call sites or on exit).
