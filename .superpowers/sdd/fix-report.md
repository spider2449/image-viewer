# Fix Report: Resizable List-View Columns Code Review

## What Was Fixed

### 1. Double `std::fs::metadata()` call per row (Critical)
**File:** `src/browser/grid.rs:453-484`
**Fix:** Extracted a single `meta = std::fs::metadata(path).ok()` variable, then used `meta.as_ref()` for both size and date formatting. Eliminates redundant filesystem calls.

### 2. No save-on-drag after column resize (Important)
**File:** `src/browser/grid.rs:389`
**Fix:** Added `if ui.input(|i| i.pointer.any_released()) { app.config.save(); }` after the column header section. This persists column widths whenever the pointer is released (including after a drag completes), preventing data loss on crash.

### 3. Horizontal overflow on narrow viewports (Important)
**File:** `src/browser/grid.rs:315-317`
**Fix:** 
- Wrapped the vertical `ScrollArea` inside a horizontal `ScrollArea` so the list view can scroll horizontally when columns overflow narrow windows.
- Updated `col_widths()` with an explicit `else if` branch for the overflow case, making it clear that `date` is never shrunk below `MIN_W`.

## What Was Tested

- `cargo check` — compiles with only pre-existing dead_code warning (unrelated `ExifData::clear`)
- `cargo test` — all 21 tests pass (4 grid format_size tests + 11 editor/batch operation tests + 6 exif tests)

## Files Changed

- `src/browser/grid.rs` — all four fixes
- `Cargo.toml` — version bump 0.1.5 → 0.1.6

## Remaining Concerns

- The `unused` `ExifData::clear` method warning is pre-existing and unrelated.
- `col_widths()` is called twice per frame (once for header, once per row) — not a regression from this change.
