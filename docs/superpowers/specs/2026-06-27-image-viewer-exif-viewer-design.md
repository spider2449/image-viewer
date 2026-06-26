# Image Viewer — EXIF Viewer Design Spec

## Overview
Add a right-side panel in Viewer mode that displays EXIF metadata for the currently open image. Uses the `kamadak-exif` crate to parse EXIF data from JPEG and TIFF files.

## Architecture

### New Dependency
```toml
exif = "1"
```

The `exif` crate reads EXIF data from a file path. It supports JPEG and TIFF. For other formats (PNG, BMP, GIF, WebP), the panel shows "No EXIF data available."

### New Module: `src/exif.rs`
```rust
pub struct ExifData {
    pub visible: bool,
    pub entries: Vec<(String, String)>,
}

impl ExifData {
    pub fn new() -> Self;
    pub fn parse(&mut self, path: &Path);
    pub fn clear(&mut self);
}

pub fn show(app: &mut App, ctx: &egui::Context);
```

**`parse()`** opens the file at `path`, reads EXIF data via `exif::Reader::new().read_from_file()`, extracts commonly useful tags, and populates `entries` as formatted `(label, value)` pairs.

**`clear()`** clears entries (called when no valid image is loaded).

### Tag Mapping
The following EXIF tags are extracted and formatted:

| Tag | EXIF Field | Format |
|-----|-----------|--------|
| Camera Make | `Make` | raw string |
| Camera Model | `Model` | raw string |
| Lens | `LensModel` | raw string |
| Date/Time | `DateTimeOriginal` | formatted `YYYY-MM-DD HH:MM:SS` |
| Image Dimensions | `PixelXDimension` / `PixelYDimension` | `{W} × {H}` |
| Orientation | `Orientation` | human-readable (Normal, Rotated 90 CW, etc.) |
| Exposure Time | `ExposureTime` | `1/{n}` sec or `{n}` sec |
| F-Number | `FNumber` | `F/{n}` |
| ISO | `PhotographicSensitivity` | integer |
| Focal Length | `FocalLength` | `{n} mm` |
| Flash | `Flash` | Fired / Did not fire / Unknown |
| GPS Latitude | `GPSLatitude` | `{deg}° {min}' {sec}" {N/S}` |
| GPS Longitude | `GPSLongitude` | `{deg}° {min}' {sec}" {E/W}` |
| Software | `Software` | raw string |

Tags that don't exist in the file are omitted from the list. The order follows the table above.p

### UI

**Toggle button:** Add "Exif" toggle button in the viewer toolbar, before the "Edit" button.

**Side panel:** `egui::SidePanel::right("exif_panel")`, resizable, default width 280px, min 200px.

**Content:** Scrollable list of label/value rows. Each row is:
```
Label: Value
```
Labels are right-aligned and bold, values are left-aligned. Alternating row backgrounds for readability. Only visible when `ExifData::visible` is true.

### Changes to Existing Files

**`Cargo.toml`** — add `exif = "1"` to dependencies.

**`src/main.rs`** — add `mod exif;`.

**`src/app.rs`** — add `exif_state: exif::ExifData` field.

**`src/viewer.rs`** — add "Exif" toggle button in toolbar; call `exif::show()` after editor panel.

**`src/editor/mod.rs`** — in `load_image()`, also call `app.exif_state.parse(path)`.

### Data Flow
```
User opens image in Viewer
  → editor::State::load_image() called
  → app.exif_state.parse(path)
  → EXIF data parsed, entries populated

User clicks "Exif" in toolbar
  → app.exif_state.visible = true
  → exif::show() renders side panel

User navigates to next/prev image
  → editor::State::load_image() called again
  → app.exif_state.parse(new_path) → entries updated
  → panel refreshes automatically
```

## Error Handling
- If the file can't be opened, `entries` is cleared and a single entry `("Error", "Cannot read file")` is shown.
- If the file has no EXIF data, a single entry `("Info", "No EXIF data found")` is shown.
- If the format doesn't support EXIF (PNG, BMP, GIF, WebP), `parse()` returns early with `("Info", "EXIF not available for this format")`.

## Out of Scope
- EXIF editing / writing
- Thumbnail extraction from EXIF
- Raw file (CR2, NEF, etc.) EXIF support
- EXIF in PNG (proprietary/not standard)
