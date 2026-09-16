# List All Files Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Browser lists every file in the folder; non-images show a generic icon and never enter the Viewer.

**Architecture:** Keep the single `App::image_files` list (now "all files, sorted") and add `is_supported_extension` guards at thumbnail-request, open, navigation, and batch boundaries. Two small pure helpers (`collect_folder_files`, `find_next_supported`) carry the testable logic.

**Tech Stack:** Rust, egui/eframe 0.31, `image` crate 0.25, cargo test.

---

## File structure

- Modify: `src/app.rs` — `scan_folder` filter removal, thumbnail-request guards, `switch_to_viewer`/`select_image`/`next_image`/`prev_image` skip logic, batch call-site filtering. New helpers: `collect_folder_files`, `find_next_supported`.
- Modify: `src/browser/grid.rs` — generic icon for non-images in grid, double-click/Open guards in grid + list, empty-state text. New helper: `can_open_in_viewer`.
- Modify: `src/batch/mod.rs:200` — file list shown in batch window filters to images.
- Modify: `Cargo.toml` — patch bump `0.1.60` -> `0.1.61` (repo convention: bump before each commit).
- Test: existing `cargo test` suites in `app.rs`, `browser/grid.rs`, `format_ext.rs` plus new unit tests below.

---

### Task 1: List all files in `scan_folder`, thumbnails for images only

**Files:**
- Modify: `src/app.rs:127-195` (`scan_folder` + helper)
- Test: `src/app.rs` `mod tests`

**Why:** Current code drops non-images at the source (`src/app.rs:152`).

- [ ] **Step 1: Write the failing test**

Add to `mod tests` in `src/app.rs`:

```rust
#[test]
fn test_collect_folder_files_lists_all_files() {
    let dir = std::env::temp_dir().join("collect_files_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("a.jpg"), b"fake").unwrap();
    std::fs::write(dir.join("b.txt"), b"fake").unwrap();
    std::fs::write(dir.join("noext"), b"fake").unwrap();
    std::fs::create_dir_all(dir.join("subdir")).unwrap();

    let mut files = super::collect_folder_files(&dir);
    files.sort();
    let names: Vec<String> = files
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    assert_eq!(names, vec!["a.jpg", "b.txt", "noext"]);
    let _ = std::fs::remove_dir_all(&dir);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test collect_folder_files`
Expected: FAIL with "cannot find function `collect_folder_files`".

- [ ] **Step 3: Write minimal implementation**

Add above `impl App` in `src/app.rs`:

```rust
fn collect_folder_files(folder: &std::path::Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(folder) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                files.push(path);
            }
        }
    }
    files
}
```

Replace in `scan_folder` (`src/app.rs:148-156`):

```rust
// BEFORE:
let mut files: Vec<PathBuf> = Vec::new();
if let Ok(entries) = std::fs::read_dir(&folder) {
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && crate::format_ext::is_supported_extension(&path) {
            files.push(path);
        }
    }
}
```

```rust
// AFTER:
let mut files: Vec<PathBuf> = collect_folder_files(&folder);
```

Guard thumbnail requests (`src/app.rs:192-194`):

```rust
// BEFORE:
for path in &self.image_files {
    self.thumbnail_cache.request(path.clone(), decode_size);
}
```

```rust
// AFTER:
for path in &self.image_files {
    if crate::format_ext::is_supported_extension(path) {
        self.thumbnail_cache.request(path.clone(), decode_size);
    }
}
```

Guard the re-request loop in `update` (`src/app.rs:308-312`):

```rust
// BEFORE:
for path in &self.image_files {
    if !self.browser_state.thumbnails.contains(path) {
        self.thumbnail_cache.request(path.clone(), decode_size);
    }
}
```

```rust
// AFTER:
for path in &self.image_files {
    if crate::format_ext::is_supported_extension(path)
        && !self.browser_state.thumbnails.contains(path)
    {
        self.thumbnail_cache.request(path.clone(), decode_size);
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test collect_folder_files`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/app.rs
git commit -m "feat(browser): list all files, request thumbnails for images only"
```

---

### Task 2: Viewer navigation skips non-images

**Files:**
- Modify: `src/app.rs:208-294` (`select_image`, `switch_to_viewer`, `next_image`, `prev_image`)
- Test: `src/app.rs` `mod tests`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn test_find_next_supported_skips_non_images() {
    use std::path::PathBuf;
    let files = vec![
        PathBuf::from("a.txt"),
        PathBuf::from("b.jpg"),
        PathBuf::from("c.txt"),
        PathBuf::from("d.png"),
    ];
    assert_eq!(super::find_next_supported(&files, 0, 1), Some(1));
    assert_eq!(super::find_next_supported(&files, 1, 1), Some(3));
    assert_eq!(super::find_next_supported(&files, 3, 1), None);
    assert_eq!(super::find_next_supported(&files, 3, -1), Some(1));
    assert_eq!(super::find_next_supported(&files, 1, -1), None);
    let none = vec![PathBuf::from("a.txt")];
    assert_eq!(super::find_next_supported(&none, 0, 1), None);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test find_next_supported`
Expected: FAIL with "cannot find function `find_next_supported`".

- [ ] **Step 3: Write minimal implementation**

Add next to `collect_folder_files` in `src/app.rs`:

```rust
fn find_next_supported(files: &[PathBuf], from: usize, direction: i32) -> Option<usize> {
    let mut i = from as i64 + direction as i64;
    while i >= 0 && (i as usize) < files.len() {
        if crate::format_ext::is_supported_extension(&files[i as usize]) {
            return Some(i as usize);
        }
        i += direction as i64;
    }
    None
}
```

Wire into navigation (`src/app.rs`):

```rust
// switch_to_viewer: add guard at top
pub fn switch_to_viewer(&mut self, index: usize) {
    if index >= self.image_files.len() {
        return;
    }
    if !crate::format_ext::is_supported_extension(&self.image_files[index]) {
        return;
    }
    self.select_image(index);
    self.mode = Mode::Viewer;
}
```

```rust
// next_image
pub fn next_image(&mut self) {
    if let Some(next) = find_next_supported(&self.image_files, self.selected_image_index, 1) {
        self.select_image(next);
    }
}
```

```rust
// prev_image
pub fn prev_image(&mut self) {
    if let Some(prev) = find_next_supported(&self.image_files, self.selected_image_index, -1) {
        self.select_image(prev);
    }
}
```

Note: `select_image` itself needs no change — every caller now guarantees an
image index (`switch_to_viewer`, `next_image`, `prev_image`).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test find_next_supported`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/app.rs
git commit -m "feat(viewer): navigation skips non-image files"
```

---

### Task 3: Grid shows generic icon for non-images, blocks Viewer entry

**Files:**
- Modify: `src/browser/grid.rs:89-95` (empty state), `src/browser/grid.rs:215-267` (thumb cell), `src/browser/grid.rs:292-296` + `339-347` (Open/double-click)
- Test: `src/browser/grid.rs` `mod tests`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn test_can_open_in_viewer_only_for_images() {
    assert!(super::can_open_in_viewer(std::path::Path::new("a.jpg")));
    assert!(super::can_open_in_viewer(std::path::Path::new("a.PNG")));
    assert!(!super::can_open_in_viewer(std::path::Path::new("a.txt")));
    assert!(!super::can_open_in_viewer(std::path::Path::new("noext")));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test can_open_in_viewer`
Expected: FAIL with "cannot find function `can_open_in_viewer`".

- [ ] **Step 3: Write minimal implementation**

Add helper in `src/browser/grid.rs` (next to `truncate_name`):

```rust
fn can_open_in_viewer(path: &std::path::Path) -> bool {
    crate::format_ext::is_supported_extension(path)
}
```

Empty state (`src/browser/grid.rs:89-95`): change label text only:

```rust
// BEFORE:
ui.colored_label(colors.text_secondary, "No images found in this folder.");
// AFTER:
ui.colored_label(colors.text_secondary, "No files in this folder.");
```

Thumbnail cell (`src/browser/grid.rs:221`): wrap the existing
`if let Some(Some(ci)) ... else if ... else ...` chain so non-images skip it:

```rust
// BEFORE:
if let Some(Some(ci)) = app.browser_state.thumbnails.get(path) {
```

```rust
// AFTER:
if !can_open_in_viewer(path) {
    ui.painter().text(
        thumb_rect.center(),
        egui::Align2::CENTER_CENTER,
        "\u{1F5C0}",
        egui::FontId::proportional(32.0),
        colors.text_secondary,
    );
} else if let Some(Some(ci)) = app.browser_state.thumbnails.get(path) {
```

Context menu Open (`src/browser/grid.rs:292-296`):

```rust
// BEFORE:
if ui.button("Open").clicked() {
    app.switch_to_viewer(i);
    ui.close_menu();
}
```

```rust
// AFTER:
if ui.button("Open").clicked() {
    if can_open_in_viewer(path) {
        app.switch_to_viewer(i);
    }
    ui.close_menu();
}
```

Click/double-click (`src/browser/grid.rs:339-347`):

```rust
// BEFORE:
if response.double_clicked() {
    app.switch_to_viewer(i);
    return;
}
```

```rust
// AFTER:
if response.double_clicked() {
    if can_open_in_viewer(path) {
        app.switch_to_viewer(i);
    }
    return;
}
```

Single click selection stays unchanged (all files selectable for
rename/delete/copy).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test can_open_in_viewer`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/browser/grid.rs
git commit -m "feat(browser): generic icon for non-images, block viewer entry"
```

---

### Task 4: List view double-click guard + batch lists images only

**Files:**
- Modify: `src/browser/grid.rs:571-574`, `src/app.rs:375-391`, `src/batch/mod.rs:200`
- Test: none new (covered by `can_open_in_viewer` test); run full suite

- [ ] **Step 1: Guard list-view double-click**

`src/browser/grid.rs:571-574`:

```rust
// BEFORE:
if response.double_clicked() {
    app.switch_to_viewer(i);
    return;
}
```

```rust
// AFTER:
if response.double_clicked() {
    if super::can_open_in_viewer(path) {
        app.switch_to_viewer(i);
    }
    return;
}
```

Wait — `show_list_view` is in the same module as the helper, so call it
directly as `can_open_in_viewer(path)` (no `super::` prefix). Use:

```rust
if response.double_clicked() {
    if can_open_in_viewer(path) {
        app.switch_to_viewer(i);
    }
    return;
}
```

- [ ] **Step 2: Filter batch entry points to images**

`src/app.rs` Tools menu (three call sites). Before each
`self.batch_state.open(&self.image_files)` build a filtered vec:

```rust
// BEFORE (x3: Convert, Rename, Resize):
self.batch_state.open(&self.image_files);
```

```rust
// AFTER (x3):
let images: Vec<std::path::PathBuf> = self
    .image_files
    .iter()
    .filter(|p| crate::format_ext::is_supported_extension(p))
    .cloned()
    .collect();
self.batch_state.open(&images);
```

`src/batch/mod.rs:200` — the window re-reads the full list every frame, so
filter there too:

```rust
// BEFORE:
let files = app.image_files.clone();
// AFTER:
let files: Vec<PathBuf> = app
    .image_files
    .iter()
    .filter(|p| crate::format_ext::is_supported_extension(p))
    .cloned()
    .collect();
```

- [ ] **Step 3: Run checks**

Run: `cargo check`
Expected: no errors.

Run: `cargo test`
Expected: all tests pass (existing 11+ plus 3 new).

- [ ] **Step 4: Commit**

```bash
git add src/browser/grid.rs src/app.rs src/batch/mod.rs
git commit -m "feat(batch): operate on images only; list view guards viewer entry"
```

---

### Task 5: Version bump + final verification

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Bump patch version**

```toml
# BEFORE:
version = "0.1.60"
# AFTER:
version = "0.1.61"
```

- [ ] **Step 2: Final verification**

Run: `cargo check`
Expected: clean build, no warnings introduced.

Run: `cargo test`
Expected: all pass.

Manual: open a folder mixing `.jpg`/`.txt`/extensionless files; confirm all
listed, txt shows `\u{1F5C0}` icon, double-click txt is a no-op, arrow keys
skip txt, Batch Convert lists only images.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "chore: bump to 0.1.61"
```

---

## Self-review

1. **Spec coverage:** scan-all-files -> Task 1; generic icon -> Task 3; no-op double-click -> Tasks 3+4; viewer skip -> Task 2; batch images-only -> Task 4; empty-state text -> Task 3. All covered.
2. **Placeholder scan:** no TBD/TODO; every code step shows exact before/after; commands include expected output.
3. **Type consistency:** helpers use `&Path`/`&[PathBuf]` consistently; `find_next_supported(files, from, direction)` signature matches test and call sites; `can_open_in_viewer` called without prefix in grid.rs (same module).
