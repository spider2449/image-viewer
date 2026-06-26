# Image Viewer — Phase 3 Design Spec: Menu Bar + Batch Operations

## Overview
Add a top menu bar to both Browser and Viewer modes, and implement batch operations (convert, rename, resize) as a modal tool accessible from the menu.

## Architecture

### Menu Bar
A `egui::TopBottomPanel::top("menu_bar")` rendered before mode-specific panels in `app::update`. Shared across Browser and Viewer modes.

**Menu structure:**
- **File** → Open Folder, Separator, Refresh, Exit
- **View** → Sort by Name / Date / Size (radio), Toggle sort direction, Separator, Toggle Grid/List view
- **Tools** → Batch Convert, Batch Rename, Batch Resize

Only active in Browser mode: if app is in Viewer mode, the View and Tools items degrade gracefully (View items still work for the folder context; Tools open the batch tool for the current folder).

### Batch Tool (`src/batch/`)
New module with two files:
- `src/batch/mod.rs` — `State` struct + `show()` panel (modal window)
- `src/batch/operations.rs` — batch apply functions

**State:**
```rust
pub struct State {
    pub visible: bool,
    pub mode: BatchMode,
    pub checked: HashSet<PathBuf>,      // files selected for operation
    pub select_all: bool,
    // Convert params
    pub convert_format: &'static str,
    pub jpeg_quality: u8,
    // Rename params
    pub rename_pattern: String,
    pub rename_preview: Vec<(PathBuf, PathBuf)>, // (old, new)
    // Resize params
    pub resize_width: u32,
    pub resize_height: u32,
    pub resize_lock_aspect: bool,
    // Status
    pub running: bool,
    pub progress_current: usize,
    pub progress_total: usize,
    pub log: Vec<String>,
}

pub enum BatchMode {
    Convert,
    Rename,
    Resize,
}
```

**UI:** Opens as a modal `egui::Window` (centered, non-resizable, ~600x500). Top has mode tabs (Convert | Rename | Resize). Below that, a scrollable list of files from `app.image_files` with checkboxes and a Select All toggle. Bottom section shows operation-specific controls (format dropdown, rename pattern field, resize width/height). Bottom has an Apply button and a collapsible log area.

### Batch Operations (`batch/operations.rs`)
```rust
pub fn batch_convert(
    files: &[PathBuf],
    format: &str,
    jpeg_quality: u8,
    progress: &ProgressSender,  // channel to report progress
) -> Result<(), Vec<String>>;

pub fn batch_rename(
    files: &[PathBuf],
    pattern: &str,
    progress: &ProgressSender,
) -> Result<(), Vec<String>>;

pub fn batch_resize(
    files: &[PathBuf],
    width: u32,
    height: u32,
    progress: &ProgressSender,
) -> Result<(), Vec<String>>;
```

Each function iterates files sequentially, reports progress via channel to the UI, and returns either success or a list of errors encountered. The UI displays errors inline in the log area.

**Convert:** Reads the image, re-encodes with the target format, writes alongside the original with new extension (`image.png` → `image.jpeg`). Uses `JpegEncoder::new_with_quality` for JPEG.

**Rename:** Pattern supports `{n}` (zero-padded sequence number) and `{name}` (original file stem). Example: `vacation_{n}` → `vacation_001.png`, `vacation_002.jpg`. Files are renamed in the same folder, preserving their extensions.

**Resize:** Reads each image, resizes using `image::imageops::FilterType::Lanczos3`, overwrites the original file (with undo via a `.bak` approach? No — keep it simple, warn the user).

**Progress reporting:** Use `std::sync::mpsc::Sender` passed to the operation, polled via `try_recv` in the UI's update loop (same pattern as `thumbnail_cache`).

### Changes to Existing Files

**`src/main.rs`** — add `mod batch;`

**`src/app.rs`** — add fields:
- `batch_state: batch::State`
- menu bar panel in `update()` before mode dispatch
- batch modal window call in `update()`

**`src/browser/grid.rs`** — no changes (list of files already available via `app.image_files`).

## Data Flow
```
User clicks Tools > Batch Convert
  → app.batch_state.visible = true
  → app.batch_state.checked = all image_files (select_all)
  → User adjusts checks, picks format, clicks Apply
  → spawn batch on a std::thread, send progress over mpsc channel
  → UI polls channel, updates progress + log
  → on completion: scan_folder() to refresh, show summary in log
```

## Out of Scope
- Batch rename undo (no backup mechanism)
- Subfolder recursion for batch ops (current folder only)
- Drag-to-reorder files in batch list
- Batch watermark / overlay
- EXIF preservation during convert/resize
