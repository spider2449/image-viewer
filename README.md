# Image Viewer

A cross-platform desktop image viewer and editor built with [egui](https://github.com/emilk/egui) and Rust.

## Features

- **Browser mode** — folder tree navigation with thumbnail grid or list view
- **Viewer mode** — zoom, pan, fit-to-window, 1:1, and fullscreen display
- **Slideshow** — automatic playback with configurable interval
- **Editor** — crop, rotate/flip, resize, undo/redo, save as
- **Batch processing** — apply operations to multiple images at once
- **EXIF metadata** — view camera settings and file info
- **Thumbnail cache** — background threaded loading with LRU eviction
- **CJK font detection** — auto-loads system fonts for Chinese, Japanese, Korean

## Supported Formats

PNG, JPEG, BMP, GIF, TIFF, WebP

## Installation

```bash
cargo build --release
```

The binary is at `target/release/image-viewer.exe` (Windows) or `target/release/image-viewer`.

## Usage

- **Browser ↔ Viewer:** Double-click a thumbnail to open, press `Esc` or click Back to return
- **Zoom:** Scroll wheel, `Ctrl+=` / `Ctrl+-`, or buttons for Fit / 1:1
- **Pan:** Drag while zoomed in
- **Slideshow:** Press `F5` or click Play in viewer
- **Editor:** Click Edit in viewer to open the right-side editor panel
- **EXIF:** Press `F2` or click Info in viewer
- **Fullscreen:** Press `F11`

## Controls

| Key | Action |
|-----|--------|
| `←` / `→` or `↑` / `↓` | Previous / next image |
| `Esc` | Back to browser |
| `F5` | Toggle slideshow |
| `F11` | Toggle fullscreen |
| `F2` | Toggle EXIF overlay |
| `Ctrl+E` | Toggle editor panel |
| `Ctrl+=` / `Ctrl+-` | Zoom in / out |
| `Ctrl+0` | Fit to window |
| `Ctrl+1` | 1:1 zoom |
| `Ctrl+S` | Save (editor) |
| `Ctrl+Z` / `Ctrl+Y` | Undo / redo (editor) |

## Configuration

Settings are stored as JSON at the platform config directory (`dirs_next::config_dir()/image-viewer/config.json`).

## Build

```bash
# Check (recommended during development)
cargo check

# Release build
cargo build --release

# Run tests
cargo test
```

## Architecture

```
src/
  main.rs              — entry point, hides console on release
  app.rs               — App state, mode switching, folder scanning
  config.rs            — JSON config load/save
  viewer.rs            — image display, zoom/pan, slideshow
  image_loader.rs      — decode images → egui textures
  thumbnail_cache.rs   — background threaded thumbnail loading
  font_loader.rs       — cross-platform CJK font detection
  exif.rs              — EXIF metadata parsing and display
  browser/
    mod.rs             — folder tree (left) + thumbnail grid (center)
    tree.rs            — folder tree widget
    grid.rs            — thumbnail grid + list view
    files.rs           — file operations (rename, delete, copy, open)
  editor/
    mod.rs             — right panel: undo/redo, crop, rotate, resize, save
    operations.rs      — EditOp enum and apply dispatch
  batch/
    mod.rs             — batch processing UI and orchestration
    operations.rs      — batch operation definitions
```

## License

MIT
