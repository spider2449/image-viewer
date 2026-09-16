# List All Files Design

Date: 2026-09-16
Approach: A — single list with boundary guards (approved)

## Problem

`App::scan_folder` (`src/app.rs:152`) filters directory entries with
`crate::format_ext::is_supported_extension`, so folders containing no images
(or a mix of images and other files) show "No images found in this folder."
Non-image files are invisible, which looks broken when the user expects a
file browser.

## Goal

Always list every file in the current folder. Non-image files show a generic
file icon and never enter the Viewer. No filter toggle (user decision).

## Non-goals

- No "images only" toggle or config option.
- No preview/thumbnail generation for non-images (no text/PDF/video preview).
- No rename of `App::image_files` in this change (keeps diff small; it now
  means "all files in folder, sorted").

## Changes

### 1. `src/app.rs` — `scan_folder`

- Collect every `path.is_file()` entry; drop the `is_supported_extension`
  gate.
- Only call `thumbnail_cache.request` for paths where
  `is_supported_extension` is true.
- Same guard in `update` re-request loop (`src/app.rs:308`): skip
  non-images so the worker queue is never filled with undecodable files.
- `resolve_startup_image` unchanged: startup path still requires a
  supported image.

### 2. `src/browser/grid.rs` — `show_thumbnail_grid`

- For non-image paths: skip `thumbnails`/`thumb_textures` lookup entirely;
  paint a generic file glyph (e.g. `\u{1F5C0}`) centered in the thumb rect
  with `colors.text_secondary`, plus the existing filename label.
- Context menu "Open" and double-click: guard with
  `is_supported_extension`; non-images are a no-op (no mode switch, no
  toast). Rename/Delete/Copy/Open-external/Save-as
  keep working for all files, except "Save as" convert stays image-only
  (guard with `image::open` result as today).
- Empty-state text changes from "No images found in this folder." to
  "No files in this folder." (only shown when the folder is truly empty).
- File count label `"{n} files"` now counts all files (no code change,
  just semantics).

### 3. `src/browser/grid.rs` — `show_list_view`

- Rows already generic; non-images render with the same icon/name/size/date.
- Double-click guard identical to grid: only images call `switch_to_viewer`.

### 4. `src/app.rs` — Viewer navigation

- `switch_to_viewer(index)`: return early when the target is not a
  supported image.
- `select_image(index)`: same guard (defensive; callers should already
  be image-only).
- `next_image` / `prev_image`: scan forward/backward for the next supported
  image; stay put when none exists. This keeps slideshow and arrow keys
  working when images and non-images are interleaved.

### 5. `src/batch/mod.rs` — batch entry points

- `batch_state.open(&self.image_files)` call sites (Convert/Rename/Resize,
  `src/app.rs:378-390`) filter to supported images before opening, so txt
  files are never offered for conversion.

## Edge cases

- Folder with zero images, some other files: grid shows file icons, Viewer
  unreachable, no error.
- Truly empty folder: "No files in this folder."
- All thumbnails pending logic (`has_pending`) only tracks image requests,
  so non-images never force a repaint loop.
- Sorting (name/date/size) applies to all files unchanged.

## Verification

- `cargo check`
- `cargo test`
- Manual: open a folder mixing `.txt`/`.exe`/`.jpg`; confirm all files
  listed, icons shown, double-click txt does nothing, arrows skip txt,
  batch convert lists only images.
