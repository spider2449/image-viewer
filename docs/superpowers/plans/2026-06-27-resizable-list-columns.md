# Resizable List-View Columns Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add draggable column-width controls to the browser list view with persistence to `./cache/config.json`.

**Architecture:** Column widths stored in `Config` (on `App`), modified by drag handles in the list view header row, persisted on drag-end via existing `Config::save()`. No new dependencies.

**Tech Stack:** Rust, egui 0.31, serde_json

## Global Constraints

- `egui = "0.31"` — no egui_extras or other UI deps
- `serde = "1"` with `derive` feature — existing
- Config path: executable-relative `./cache/config.json`
- Min column width: 60px for text columns, 24px fixed for icon

---

### Task 1: Add ColumnWidths struct and change config path

**Files:**
- Modify: `src/config.rs`

**Interfaces:**
- Consumes: existing `Config` struct, `serde::{Serialize, Deserialize}`
- Produces: `ColumnWidths` struct with `Serialize + Deserialize + Default`, `Config` gains `column_widths: ColumnWidths` field, config file location changes to `./cache/config.json`

- [ ] **Step 1: Edit `src/config.rs` — add `ColumnWidths` struct after `Config`**

Insert after the `Config` struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnWidths {
    pub name: f32,
    pub dimensions: f32,
    pub size: f32,
    pub date: f32,
}

impl Default for ColumnWidths {
    fn default() -> Self {
        Self {
            name: 200.0,
            dimensions: 100.0,
            size: 80.0,
            date: 150.0,
        }
    }
}
```

- [ ] **Step 2: Edit `src/config.rs` — add `column_widths` to `Config`**

Add field to the `Config` struct:
```rust
pub struct Config {
    pub last_folder: Option<String>,
    pub window_pos: Option<[f32; 2]>,
    pub window_size: Option<[f32; 2]>,
    pub sort_by: String,
    pub sort_descending: bool,
    pub slideshow_interval_secs: u32,
    pub zoom_default: f32,
    pub column_widths: ColumnWidths,
}
```

Add `column_widths: ColumnWidths::default(),` to `Config::default()`.

- [ ] **Step 3: Edit `src/config.rs` — change `Config::path()` to `./cache/config.json`**

Replace the `path()` function:

```rust
fn path() -> PathBuf {
    let mut p = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    p.pop(); // exe dir
    p.push("cache");
    p.push("config.json");
    p
}
```

- [ ] **Step 4: Run `cargo check` to verify compilation**

Run: `cargo check` (in workspace root `F:\coding\rustPrj\image-viewer`)
Expected: clean compile

- [ ] **Step 5: Commit**

```bash
git add src/config.rs
git commit -m "feat: add ColumnWidths and change config path to ./cache/"
```

---

### Task 2: Rewrite list view with resizable columns

**Files:**
- Modify: `src/browser/grid.rs`

**Interfaces:**
- Consumes: `app.config.column_widths: ColumnWidths` on `App`, `app.image_files: Vec<PathBuf>`, existing row interaction pattern
- Produces: `show_list_view()` with draggable column dividers in header, fixed-width column row layout

- [ ] **Step 1: Replace `show_list_view` function body**

Replace the entire `show_list_view` function (lines 307–406) with:

```rust
fn show_list_view(app: &mut App, ui: &mut egui::Ui) {
    let paths: Vec<PathBuf> = app.image_files.clone();

    const ICON_W: f32 = 24.0;
    const GAP: f32 = 4.0;
    const MIN_W: f32 = 60.0;
    const HANDLE_W: f32 = 8.0;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let available = ui.available_width();

            let widths = col_widths(&app.config.column_widths, available, ICON_W, MIN_W, GAP);

            // ── Column headers ──────────────────────────────
            let header_h = 20.0;
            let (header_rect, _) = ui.allocate_exact_size(
                Vec2::new(available, header_h),
                egui::Sense::hover(),
            );

            ui.painter().rect_filled(header_rect, egui::CornerRadius::same(2), crate::theme::PANEL_BG);

            let mut x = header_rect.min.x;
            let header_y = header_rect.min.y;

            // Icon header (fixed width spacer)
            x += ICON_W;

            // Name header + drag handle
            ui.painter().text(
                egui::pos2(x + 4.0, header_y + header_h / 2.0),
                egui::Align2::LEFT_CENTER,
                "Name",
                egui::FontId::proportional(14.0),
                crate::theme::TEXT_PRIMARY,
            );
            x += widths.name;
            x = drag_handle(ui, x, header_y, header_h, HANDLE_W, |d| {
                app.config.column_widths.name = (app.config.column_widths.name + d).max(MIN_W);
            });
            x += GAP;

            // Dimensions header + drag handle
            ui.painter().text(
                egui::pos2(x + 4.0, header_y + header_h / 2.0),
                egui::Align2::LEFT_CENTER,
                "Dimensions",
                egui::FontId::proportional(14.0),
                crate::theme::TEXT_PRIMARY,
            );
            x += widths.dimensions;
            x = drag_handle(ui, x, header_y, header_h, HANDLE_W, |d| {
                app.config.column_widths.dimensions = (app.config.column_widths.dimensions + d).max(MIN_W);
            });
            x += GAP;

            // Size header + drag handle
            ui.painter().text(
                egui::pos2(x + 4.0, header_y + header_h / 2.0),
                egui::Align2::LEFT_CENTER,
                "Size",
                egui::FontId::proportional(14.0),
                crate::theme::TEXT_PRIMARY,
            );
            x += widths.size;
            x = drag_handle(ui, x, header_y, header_h, HANDLE_W, |d| {
                app.config.column_widths.size = (app.config.column_widths.size + d).max(MIN_W);
            });
            x += GAP;

            // Date header (no handle after)
            ui.painter().text(
                egui::pos2(x + 4.0, header_y + header_h / 2.0),
                egui::Align2::LEFT_CENTER,
                "Date",
                egui::FontId::proportional(14.0),
                crate::theme::TEXT_PRIMARY,
            );

            ui.separator();

            // ── Rows ────────────────────────────────────────
            for (i, path) in paths.iter().enumerate() {
                let is_selected = app.browser_state.selected_thumb == Some(i);
                let row_bg = if is_selected {
                    crate::theme::SELECTED_BG
                } else if i % 2 == 0 {
                    crate::theme::PANEL_BG
                } else {
                    crate::theme::CARD_BG
                };

                let row_h = 24.0;
                let (rect, response) = ui.allocate_exact_size(
                    Vec2::new(available, row_h),
                    egui::Sense::click(),
                );

                let actual_bg = if response.hovered() && !is_selected {
                    crate::theme::HOVER_BG
                } else {
                    row_bg
                };
                ui.painter().rect_filled(rect, egui::CornerRadius::same(2), actual_bg);

                // Row content
                let widths = col_widths(&app.config.column_widths, rect.width(), ICON_W, MIN_W, GAP);
                let mut x = rect.min.x;
                let cy = rect.center().y;

                // Icon
                ui.painter().text(
                    egui::pos2(x + ICON_W / 2.0, cy),
                    egui::Align2::CENTER_CENTER,
                    "\u{1F5BC}",
                    egui::FontId::proportional(12.0),
                    crate::theme::TEXT_SECONDARY,
                );
                x += ICON_W;

                // Name
                let name = path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let name_color = if is_selected { crate::theme::TEXT_PRIMARY } else { crate::theme::TEXT_SECONDARY };
                ui.painter().text(
                    egui::pos2(x + 4.0, cy),
                    egui::Align2::LEFT_CENTER,
                    &name,
                    egui::FontId::proportional(12.0),
                    name_color,
                );
                x += widths.name + GAP;

                // Dimensions
                ui.painter().text(
                    egui::pos2(x + widths.dimensions - 4.0, cy),
                    egui::Align2::RIGHT_CENTER,
                    "-",
                    egui::FontId::proportional(12.0),
                    crate::theme::TEXT_SECONDARY,
                );
                x += widths.dimensions + GAP;

                // Size
                let size_str = std::fs::metadata(path)
                    .ok()
                    .map(|m| format_size(m.len()))
                    .unwrap_or_else(|| "-".to_string());
                ui.painter().text(
                    egui::pos2(x + widths.size - 4.0, cy),
                    egui::Align2::RIGHT_CENTER,
                    &size_str,
                    egui::FontId::proportional(12.0),
                    crate::theme::TEXT_SECONDARY,
                );
                x += widths.size + GAP;

                // Date
                let date_str = std::fs::metadata(path)
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|dt| {
                        let secs = dt.as_secs();
                        let days = secs / 86400;
                        let time = secs % 86400;
                        let h = time / 3600;
                        let min = (time % 3600) / 60;
                        format!("{days}d {h:02}:{min:02}")
                    })
                    .unwrap_or_else(|| "-".to_string());
                ui.painter().text(
                    egui::pos2(x + widths.date - 4.0, cy),
                    egui::Align2::RIGHT_CENTER,
                    &date_str,
                    egui::FontId::proportional(12.0),
                    crate::theme::TEXT_SECONDARY,
                );

                if response.double_clicked() {
                    app.switch_to_viewer(i);
                    return;
                }
                if response.clicked() {
                    app.browser_state.selected_thumb = Some(i);
                }
            }
        });
}

fn col_widths(cw: &crate::config::ColumnWidths, available: f32, icon_w: f32, min_w: f32, gap: f32) -> ColumnWidthSet {
    let mut name = cw.name.max(min_w);
    let mut dimensions = cw.dimensions.max(min_w);
    let mut size = cw.size.max(min_w);
    let mut date = cw.date.max(min_w);
    let fixed = icon_w + name + gap + dimensions + gap + size + gap;
    if fixed + date < available {
        date += available - fixed - date;
    }
    ColumnWidthSet { name, dimensions, size, date }
}

struct ColumnWidthSet {
    name: f32,
    dimensions: f32,
    size: f32,
    date: f32,
}

fn drag_handle(
    ui: &mut egui::Ui,
    x: f32,
    header_y: f32,
    header_h: f32,
    handle_w: f32,
    mut on_drag: impl FnMut(f32),
) -> f32 {
    let handle_rect = egui::Rect::from_min_size(
        egui::pos2(x - handle_w / 2.0, header_y),
        egui::vec2(handle_w, header_h),
    );
    let resp = ui.interact(handle_rect, ui.next_auto_id(), egui::Sense::click_and_drag());

    ui.painter().vline(x, header_y..=(header_y + header_h), egui::Stroke::new(1.0, crate::theme::BORDER));

    if resp.drag_started() || resp.dragged() || resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeColumn);
    }
    if resp.dragged() {
        on_drag(resp.drag_delta().x);
    }

    x
}
```

Note: `draw_drag_handle` is a standalone helper function (not a method), placed before `show_list_view` or after it in the file.

- [ ] **Step 2: Update imports in `grid.rs`**

The `Vec2` import is already there. Remove unused `image::GenericImageView`, `Stroke`, `TextureOptions` from the import line if they become unused by checking `cargo check`. The `Stroke` type is now used in `draw_drag_handle`, so keep it. Remove `TextureOptions` and `GenericImageView` if they're only used in `show_thumbnail_grid` — actually they are used there, so keep them.

No import changes needed. The `image::GenericImageView` was only used in `show_thumbnail_grid` for dimensions, it's not needed for list view. Keep it since it's still used.

- [ ] **Step 3: Run `cargo check` to verify compilation**

Run: `cargo check`
Expected: clean compile

- [ ] **Step 4: Run existing tests**

Run: `cargo test`
Expected: all 11 tests pass (format_size tests)

- [ ] **Step 5: Commit**

```bash
git add src/browser/grid.rs
git commit -m "feat: add resizable columns to list view with drag handles"
```

---

### Task 3: Save config on column width changes

**Files:**
- Modify: `src/app.rs`

**Interfaces:**
- Consumes: `app.config.save()` already exists
- Produces: config saved after drag operations (in `on_exit` handler, which already exists)

- [ ] **Step 1: Verify `on_exit` already saves config**

The `on_exit` method in `app.rs:167-169` already calls `self.config.save()`. No change needed — column widths are part of `Config` and will be persisted on exit.

- [ ] **Step 2: Ensure config directory exists on save**

The `Config::save()` method already calls `std::fs::create_dir_all(path.parent())`. No change needed.

- [ ] **Step 3: Run `cargo check`**

Run: `cargo check`
Expected: clean compile

- [ ] **Step 4: Commit (if any changes were made; otherwise skip)**

```bash
git add src/app.rs
git commit -m "fix: ensure config save path supports ./cache/ directory"
```

(Only if changes were needed — likely no-op task.)
