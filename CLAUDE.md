# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

Single-crate Rust project (egui/eframe desktop app). No workspace, no CI.

## Commands

```bash
cargo check            # only verification that matters day-to-day
cargo test              # unit tests (editor::operations, browser::grid, theme)
cargo test <name>       # run a single test by (partial) name
cargo build --release   # produces single binary at target/release/image-viewer[.exe]
```

## Versioning

Bump the patch version in `Cargo.toml` before each commit. No tooling — manual convention.

## Architecture

- **egui/eframe 0.31** — immediate mode GUI, single window. `ctx.request_repaint()` is called every frame in `app::update` — required for egui polling, not a bug.
- **`image` crate 0.25** — decoding; features limited to png, jpeg, bmp, gif, tiff, webp.
- **Modes:** `Browser` (folder tree left + thumbnail grid center) ↔ `Viewer` (image display + optional right-side editor panel).
- **Thumbnail cache:** background `std::thread` workers + LRU via `lru` crate. `Arc<Mutex<Receiver>>` pattern with `try_recv` + sleep to avoid blocking. Requests all images on scan; `update` re-requests any missing thumbnail each frame.
- **Disk cache** (`disk_cache.rs`): persists thumbnails/derived data to disk alongside the in-memory LRU.
- **Config:** JSON at `<exe dir>/cache/config.json` (portable, next to the binary).
- **Fonts:** CJK auto-detected (`msyh.ttc`, `simsun.ttc`, `PingFang.ttc`, `NotoSansCJK`).
- **Theming** (`theme.rs`): `Theme::Dark`/`Theme::Light` enum, `palette(theme) -> ThemeColors`, `apply_theme(ctx, theme)`. `Context::set_style`/`set_visuals` only affect whichever theme slot `ctx.theme()` resolves to — `apply_theme` pins the preference with `ctx.set_theme()` and targets the slot explicitly via `set_style_of`/`set_visuals_of`. All UI code should read colors via `App::theme_colors()` rather than hardcoding constants.

## File layout

```
src/
  main.rs             — entry, hides console on release
  app.rs              — App struct (all state), mode switching, scan_folder, theme_colors()
  config.rs           — Config load/save (serde_json)
  theme.rs            — Theme enum, ThemeColors palette, egui Visuals/Style builders
  viewer.rs           — image display, zoom/pan, slideshow
  image_loader.rs     — decode → egui ColorImage/TextureHandle
  thumbnail_cache.rs  — async thumbnail requests
  disk_cache.rs       — on-disk thumbnail/derived-data cache
  font_loader.rs      — cross-platform CJK font loading
  exif.rs             — EXIF metadata parsing and display
  format_ext.rs       — file extension/format helpers
  browser/
    mod.rs            — side panel (tree) + central panel (grid)
    tree.rs           — folder tree widget (max depth 2, 50 dirs per node)
    grid.rs           — thumbnail grid + list view
    files.rs          — file ops (rename, delete, copy, open) — unused in UI
  editor/
    mod.rs            — right panel: undo/redo, crop, rotate/flip, resize, save as
    operations.rs     — EditOp enum + apply() dispatch
  batch/
    mod.rs            — batch processing UI and orchestration
    operations.rs     — batch operation definitions
```

## Quirks & gotchas

- **"Fit" zoom** correctly computed and stored in `viewer_state.fit_zoom` during `draw_image`. Previously set zoom to 1.0 (same as "1:1") — don't regress this.
- **JPEG quality** requires `image::codecs::jpeg::JpegEncoder::new_with_quality` — `save_with_format` does not support quality.
- **Crop UI** exists in editor panel but viewer has no crop-drag interaction. `crop_start`/`crop_end` must be set by viewer mouse handling before `Apply Crop` works.
- **Textures cleared on `scan_folder`** — `app.textures.clear()` runs before re-scanning a directory. Still grows unboundedly within a single folder if user opens every image.
- **Editor's `save_as`** writes alongside source file with new extension (no file dialog).
- **Drive roots hardcoded** to `C:\`, `D:\`, `E:\` on Windows (`browser/mod.rs`).
- **Viewer status bar** reads `std::fs::metadata` each frame — can be slow over network drives.
