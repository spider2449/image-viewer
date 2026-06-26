# Image Viewer — Phase 1 Design Spec

## Overview
A cross-platform image viewer inspired by FastStone Image Viewer, built with Rust and egui/eframe. Phase 1 focuses on the core dual-mode experience: a file browser mode and a full image viewer mode with navigation, zoom/pan, slideshow, and fullscreen.

## Architecture

### Module Structure
```
src/
  main.rs          — entry point, eframe window setup
  app.rs           — top-level App, mode switching (Browser ↔ Viewer)
  browser/
    mod.rs         — browser mode root, panel layout
    tree.rs        — folder tree widget
    grid.rs        — thumbnail grid widget
    files.rs       — file operations (rename, delete, copy, open)
  viewer.rs        — viewer mode: image display, zoom, pan, fullscreen
  image_loader.rs  — decode images via `image` crate, manage egui textures
  thumbnail_cache.rs — async thumbnail generation + LRU cache
  config.rs        — user settings persistence (JSON)
```

### Key Dependencies
- `eframe` / `egui` — GUI framework
- `image` — image decoding (png, jpeg, gif, bmp, webp, tiff, etc.)
- `walkdir` — recursive directory traversal
- `serde` / `serde_json` — config serialization
- `rfd` — native file dialogs
- `egui_extras` — table support for list view
- `dirs-next` — platform config directory resolution
- `lru` — LRU cache for decoded thumbnails

### Data Flow
```
Filesystem → image_loader decodes image → egui texture
                                    ↓
                        thumbnail_cache: async load → resized → cached
                                    ↓
                        rendered in browser grid or viewer
```

## Phase 1: Dual-mode Core

### Browser Mode
- **Left panel:** Folder tree using `walkdir`, expandable nodes, current folder highlighted, auto-refresh on FS changes.
- **Center:** Thumbnail grid — columns auto-fit to window width, filenames below each thumb, selection highlight.
- **Toolbar:** Folder navigation (up/back), view toggle (grid/list), sort dropdown (name/date/size), refresh button.
- **Status bar:** Image count in current folder, selected file dimensions + size.
- **Interactions:**
  - Click folder → scan and show thumbnails
  - Double-click thumbnail → switch to Viewer mode at that image
  - Right-click → context menu: rename, delete, copy, open in system viewer
  - Sort: name (asc/desc), date modified, file size
  - List view alternative: table with filename, dimensions, size, date

### Viewer Mode
- Image displayed at max-fit on open
- **Zoom:** mouse wheel, Ctrl++/-, toolbar zoom slider (10%–3200%)
- **Pan:** left-click drag when zoomed in, scroll bars
- **View shortcuts:** `F` fit-to-window, `1` actual-size (1:1), `Z` zoom-to-fill
- **Navigation:** ← → arrow keys, mouse wheel scroll through folder, Ctrl+↑/↓ prev/next folder
- **Fullscreen:** F11 toggle, ESC to exit, auto-hide toolbar, semi-transparent overlay controls
- **Info overlay:** `I` to toggle — filename, dimensions (W×H), file size, bit depth, zoom %, current/total
- **Cursor color:** RGB hex + decimal value under cursor shown in status bar
- **Slideshow:** F5 start, pause/resume with P or Space, configurable interval (1–60s), transition (instant/fade)
- **Context menu:** Right-click → save as, copy image, set as wallpaper, print

### Shared Components
- **Toolbar:** Mode-aware — browser toolbar has folder nav + sort; viewer toolbar has zoom controls + slideshow + fullscreen
- **Status bar:** Mode-aware — shows appropriate info for current mode
- **Keyboard shortcuts:** Unified system, mode-dependent action dispatch
- **Drag & drop:** Drag image files/folders into window to open them

### Image Loading Strategy
- `image` crate decodes to `RgbaImage`
- Converted to egui `ColorImage`, uploaded as `egui::TextureHandle`
- Large images (>4K) loaded at reduced resolution first, full resolution on demand
- Format support: PNG, JPEG, BMP, GIF (static), TIFF, WEBP
- Progressive loading for very large images (decode in chunks)
- Thumbnails generated on a background `std::thread` (no async runtime), sent back via channel, cached in an LRU cache

### Config Persistence
- JSON file saved to platform-appropriate config dir (`dirs::config_dir`)
- Settings stored: window geometry, last opened folder, sort preference, slideshow interval, zoom defaults
- Loaded on startup, saved on exit and on setting changes

## Out of Scope (Phase 1)
- Image editing (crop, rotate, resize, color adjustment)
- Batch operations (convert, rename, resize)
- EXIF metadata viewer
- Image comparison
- Screenshot capture
- Color picker
- Multi-page TIFF / PDF support
- Plugin system

## Future Phases (Post-Phase 1)
- **Phase 2:** Basic editing — crop, rotate/flip, resize, undo/redo
- **Phase 3:** Batch operations — batch convert, batch rename, batch resize
- **Phase 4:** EXIF viewer, image comparison, slideshow transitions, multi-page support
