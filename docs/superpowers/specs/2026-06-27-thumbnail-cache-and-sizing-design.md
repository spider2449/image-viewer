# Thumbnail Cache & Sizing — Design Spec

## Overview

Two related improvements to the thumbnail system:
1. **Configurable thumbnail size** — slider + Ctrl+scroll in grid toolbar
2. **Disk-backed thumbnail cache** — persisted thumbnails survive restarts

---

## 1. Configurable Thumbnail Size

### Config changes (`config.rs`)

```rust
pub thumb_size: f32,            // display size in px, default 140.0
```

Derived at runtime (not stored):
```
thumb_decode_max_size = max(ceil(thumb_size * 1.5), 200) as u32
```

### UI — grid toolbar (`grid.rs`)

Add slider between "List" toggle and "Refresh" button:
```
[Grid  ──────────●────── ]  140px
```
- Range: 60..=400
- Display shows current px value (e.g. "140px")
- Ctrl+scroll on the grid area changes by ±10px, clamped

### Reflow behavior

- `THUMB_SIZE` constant → `app.config.thumb_size`
- When `thumb_size` increases such that `ceil(thumb_size * 1.5) > current_decode_size`:
  - Clear `browser_state.thumbnails` and `browser_state.thumb_textures`
  - Re-request all thumbnails with new `thumb_decode_max_size`
- When `thumb_size` decreases: no re-request needed (existing textures are downscaled by egui)

---

## 2. Disk-Backed Thumbnail Cache

### New file: `src/disk_cache.rs`

Public interface:
```rust
pub struct DiskCache { dir: PathBuf }

impl DiskCache {
    pub fn new(base_dir: PathBuf) -> Self;
    pub fn lookup(&self, path: &Path, max_size: u32) -> Option<ColorImage>;
    pub fn store(&self, path: &Path, max_size: u32, image: &ColorImage);
    pub fn clear_all(&self);
}
```

### Cache directory

`<exe_dir>/cache/thumbnails/` — alongside existing `config.json`.

### Filename scheme

`{hash}.{mtime}.{size}.jpg`

- `hash`: first 16 hex chars of SHA-256 of the canonical source path
- `mtime`: source file's `modified()` as unix seconds (`u64`)
- `size`: `max_size` used for encoding

Example: `a3f2c1b4e87d09f6.1700000000.200.jpg`

### Lookup flow (called from worker threads)

```
1. Compute hash = sha256(path_to_string)[..16]
2. Read source_mtime = path.metadata().modified().unix_timestamp()
3. Cache path = dir / "{hash}.{source_mtime}.{max_size}.jpg"
4. If cache_path.exists():
     if source_path.exists():  // guard against deleted source
       load JPEG → decode to ColorImage → return
     else:
       delete cache_path  // orphan cleanup
5. Fall through to decode from source (existing path)
```

### Store flow

```
1. After decode from source, encode ColorImage as JPEG
2. Delete any existing files matching `{hash}.*.*.jpg` in cache dir (old sizes / stale mtimes)
3. Write to cache_path using JpegEncoderWithQuality (quality 85)
```

### Integration with ThumbnailCache

- `ThumbnailCache::new()` takes an optional `DiskCache`
- Worker's loop: after receiving a request, call `disk_cache.lookup()` first → if hit, skip decode
- After successful decode, call `disk_cache.store()` in background

### Auto-cleanup

- **On lookup:** if source file is gone → delete cache file
- **Menu item:** "File → Clear thumbnail cache" calls `DiskCache::clear_all()`

---

## 3. Additional In-Memory Improvements

- **Re-request guard:** Track which paths have been dispatched to workers; skip re-request in `update()` if already pending (the existing `pending` Vec already does this partially — ensure it also covers the re-request loop)
- **Texture eviction:** On `scan_folder`, also clear `browser_state.thumb_textures` (currently only `app.textures` is cleared)

---

## Files Modified

| File | Changes |
|------|---------|
| `Cargo.toml` | Add `sha2` dependency for hashing |
| `config.rs` | Add `thumb_size` field |
| `thumbnail_cache.rs` | Integrate `DiskCache` in worker loop, add `DiskCache` reference |
| `disk_cache.rs` | **New file** — disk cache logic |
| `browser/grid.rs` | Replace `THUMB_SIZE` constant with config, add slider + Ctrl+scroll |
| `browser/mod.rs` | Update `State::new()` if needed |
| `app.rs` | Pass decode size derived from config, clear thumb_textures on scan |
| `image_loader.rs` | Add `load_thumbnail_to_jpeg_bytes()` for disk storage |
