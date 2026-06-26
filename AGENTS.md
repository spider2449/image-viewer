# image-viewer — AGENTS.md

Single-crate Rust project. No workspace, no CI, no tests.

## Commands

```bash
cargo check          # only verification that matters
cargo build --release  # produces single binary
```

## Versioning

Bump the patch version in `Cargo.toml` before each commit. No tooling — manual convention.

## Architecture

- **egui/eframe 0.31** — immediate mode GUI, single window
- **`image` crate 0.25** — decoding; features: png, jpeg, bmp, gif, tiff, webp
- **Modes:** `Browser` (folder tree left + thumbnail grid center) ↔ `Viewer` (image display + optional right-side editor panel)
- **Thumbnail cache:** background `std::thread` workers + LRU via `lru` crate. `Arc<Mutex<Receiver>>` pattern — only one worker blocks on recv at a time.
- **Config:** JSON at `dirs_next::config_dir()/image-viewer/config.json`
- **Fonts:** CJK auto-detected (`msyh.ttc`, `simsun.ttc`, `PingFang.ttc`, `NotoSansCJK`)

## File layout

```
src/
  main.rs          — entry, hides console on release
  app.rs           — App struct (all state), mode switching, scan_folder
  config.rs        — Config load/save (serde_json)
  viewer.rs        — image display, zoom/pan, slideshow
  image_loader.rs  — decode → egui ColorImage/TextureHandle
  thumbnail_cache.rs — async thumbnail requests
  font_loader.rs   — cross-platform CJK font loading
  browser/
    mod.rs         — side panel (tree) + central panel (grid)
    tree.rs        — folder tree widget (max depth 2, 50 dirs per node)
    grid.rs        — thumbnail grid + list view
    files.rs       — file ops (rename, delete, copy, open) — unused in UI
  editor/
    mod.rs         — right panel: undo/redo, crop, rotate/flip, resize, save as
    operations.rs  — EditOp enum + apply() dispatch
```

## Quirks & gotchas

- **No tests exist.** Any test setup must be created from scratch.
- **`walkdir`, `rfd`, `egui_extras`** declared in Cargo.toml but unused in source.
- **"Fit" zoom** (`viewer.rs:61`) is broken — sets zoom to 1.0 instead of computing fit-to-viewport. The correct `scale` is computed at line 302 but never stored.
- **Thumbnail requests capped at 50** (`app.rs:118-123`, `167-175`). Images beyond index 50 never get thumbnails.
- **JPEG quality** requires `image::codecs::jpeg::JpegEncoder::new_with_quality` — `save_with_format` does not support quality.
- **Crop UI** exists in editor panel but viewer has no crop-drag interaction. `crop_start`/`crop_end` must be set by viewer mouse handling before `Apply Crop` works.
- **Textures never evicted** — `app.textures: HashMap<String, TextureHandle>` grows unboundedly as user navigates images.
- **Editor's `save_as`** writes alongside source file with new extension (no file dialog).
- **Drive roots hardcoded** to `C:\`, `D:\`, `E:\` on Windows (`browser/mod.rs:37-43`).
- **`ctx.request_repaint()`** called every frame in `app::update` — required for egui polling.
- **Viewer status bar** reads `std::fs::metadata` each frame — can be slow over network drives.
