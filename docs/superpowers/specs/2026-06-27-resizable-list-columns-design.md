# Resizable List-View Columns

**Date:** 2026-06-27
**Status:** Approved

## Summary

Add draggable column-width controls to the browser list view, allowing users to resize columns. Widths persist to a `./cache/config.json` file alongside the executable.

## Design

### Columns

| Column | Default width | Min width | Resizable |
|--------|--------------|-----------|-----------|
| Icon   | 24px         | —         | No (fixed)|
| Name   | 200px        | 60px      | Yes       |
| Dimensions | 100px    | 60px      | Yes       |
| Size   | 80px         | 60px      | Yes       |
| Date   | 150px        | 60px      | Yes       |

### Layout algorithm

All columns laid out left-to-right at their stored pixel widths. If total width < available viewport width, remaining space is added to the Date column (rightmost). No horizontal scrollbar needed.

### Drag handles

- 4px-wide vertical strip between each adjacent column pair, rendered in the **header row** (click target area slightly wider for usability)
- On hover: cursor changes to `CursorIcon::ResizeColumn`
- On drag: `response.drag_started()` / `response.drag_delta()` updates the left-side column's stored width
- Width is clamped to [min_width, ∞) — no max cap
- Header background extends behind the drag handle for visual clarity

On window resize, columns keep their absolute pixel widths; only the Date column's padding adjusts.

### Persistence

Config saved as JSON to `./cache/config.json` (relative to executable). The `Config` struct gains a `column_widths` field:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnWidths {
    pub name: f32,
    pub dimensions: f32,
    pub size: f32,
    pub date: f32,
}
```

Defaults applied if field missing or parse fails. Config path changes from `dirs_next::config_dir()` to executable-relative `./cache/`.

### State

Column widths stored in `browser::State`:

```rust
pub struct State {
    pub show_list_view: bool,
    pub column_widths: ColumnWidths,
    // ... existing fields
}
```

Loaded from config on app init. Saved to config whenever a drag completes (or on app close).

### Row rendering change

Rows switch from `right_to_left` layout to left-to-right fixed-width columns matching the header. Content clips to column bounds. Alternating row backgrounds unchanged.

## Scope

- No changes to thumbnail grid view
- No changes to sorting or context menus
- No horizontal scrolling (columns shrink-wrap to available width with Date taking remainder)

## Files changed

| File | Change |
|------|--------|
| `src/config.rs` | Add `ColumnWidths` struct, change config path to `./cache/config.json` |
| `src/browser/mod.rs` | Add `column_widths` field to `State` |
| `src/browser/grid.rs` | Rewrite header + row layout to fixed-width columns with drag handles |
| `src/app.rs` | Load/save column widths on init/exit |
