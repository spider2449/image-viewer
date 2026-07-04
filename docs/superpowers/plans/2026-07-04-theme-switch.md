# Light/Dark Theme Switch Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a user-toggleable light/dark theme via a `Theme` enum + `ThemeColors` struct in `src/theme.rs`, persisted in `config.json` as a `theme: String` field (default `"dark"`), toggled from a View menu entry.

**Architecture:** Add new types alongside existing `const` colors in `theme.rs`. Refactor `theme_visuals()` to dispatch on `Theme`. Add an `App::theme_colors()` accessor and sweep ~50 call sites file-by-file to replace `crate::theme::PANEL_BG`-style references with `colors.panel_bg`. No new dependencies. No new modules.

**Tech Stack:** Rust 2021, eframe/egui 0.31, serde. No new crates.

**Reference spec:** `docs/superpowers/specs/2026-07-04-theme-switch-design.md`

## Global Constraints

- All existing 32 tests must continue to pass after every task.
- `cargo check` must be clean (the 1 pre-existing warning is acceptable; introduce no new warnings).
- Follow existing code style: no comments unless explicitly required, 4-space indent, `crate::` imports.
- Bump `Cargo.toml` patch version before each commit per AGENTS.md convention.
- Do not change egui version (0.31). No new external dependencies.
- Preserve immediate-mode GUI pattern: no async/await on the UI thread.
- The order of tasks matters: setup tasks (1-5) must complete before any call-site sweep (6-12).
- `Theme::from_str` returns `Theme::Dark` for any value other than `"light"` — this is the safe fallback for corrupted/old configs.

---

### Task 1: Add Theme enum, ThemeColors struct, palette(), and from_str with tests

**Files:**
- Modify: `src/theme.rs` (add new types and function at top of file; do NOT remove existing `const`s yet)
- Test: `src/theme.rs` (3 new tests in `#[cfg(test)] mod tests`)

**Interfaces:**
- Produces:
  - `pub enum Theme { Dark, Light }`
  - `pub struct ThemeColors { pub bg_dark, pub panel_bg, pub card_bg, pub hover_bg, pub accent, pub selected_bg, pub text_primary, pub text_secondary, pub border, pub danger: Color32 }`
  - `pub fn palette(theme: Theme) -> ThemeColors`
  - `impl Theme { pub fn from_str(s: &str) -> Theme }`

- [ ] **Step 1: Read the current theme.rs top**

Run: `Read src/theme.rs` lines 1-19

Confirm:
- Line 1: `use eframe::egui::{self, Color32, CornerRadius, Margin, Stroke, Style, Visuals, Vec2};`
- Lines 4-13: existing `const` declarations (BG_DARK, PANEL_BG, ...)
- Lines 15-18: `styled_icon` function
- Lines 21+: `theme_visuals`

- [ ] **Step 2: Add the new types after the const block**

Edit `src/theme.rs`. After the existing `DANGER` const (line 13) and before the `styled_icon` function (line 15), insert:

```rust
// ── Theme switch (light/dark) ─────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
}

impl Theme {
    pub fn from_str(s: &str) -> Theme {
        match s {
            "light" => Theme::Light,
            _ => Theme::Dark,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ThemeColors {
    pub bg_dark: Color32,
    pub panel_bg: Color32,
    pub card_bg: Color32,
    pub hover_bg: Color32,
    pub accent: Color32,
    pub selected_bg: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub border: Color32,
    pub danger: Color32,
}

pub fn palette(theme: Theme) -> ThemeColors {
    match theme {
        Theme::Dark => ThemeColors {
            bg_dark: Color32::from_rgb(0x1a, 0x1a, 0x1a),
            panel_bg: Color32::from_rgb(0x22, 0x22, 0x22),
            card_bg: Color32::from_rgb(0x2a, 0x2a, 0x2a),
            hover_bg: Color32::from_rgb(0x35, 0x35, 0x35),
            accent: Color32::from_rgb(0x4a, 0x9e, 0xff),
            selected_bg: Color32::from_rgb(0x2d, 0x5a, 0x8e),
            text_primary: Color32::from_rgb(0xe0, 0xe0, 0xe0),
            text_secondary: Color32::from_rgb(0x88, 0x88, 0x88),
            border: Color32::from_rgb(0x3a, 0x3a, 0x3a),
            danger: Color32::from_rgb(0xe7, 0x4c, 0x3c),
        },
        Theme::Light => ThemeColors {
            bg_dark: Color32::from_rgb(0xff, 0xff, 0xff),
            panel_bg: Color32::from_rgb(0xf5, 0xf5, 0xf5),
            card_bg: Color32::from_rgb(0xe8, 0xe8, 0xe8),
            hover_bg: Color32::from_rgb(0xdc, 0xdc, 0xdc),
            accent: Color32::from_rgb(0x1a, 0x6d, 0xd6),
            selected_bg: Color32::from_rgb(0xa8, 0xc8, 0xf0),
            text_primary: Color32::from_rgb(0x1a, 0x1a, 0x1a),
            text_secondary: Color32::from_rgb(0x66, 0x66, 0x66),
            border: Color32::from_rgb(0xcc, 0xcc, 0xcc),
            danger: Color32::from_rgb(0xc0, 0x39, 0x2b),
        },
    }
}
```

(Dark values exactly match the existing `const`s so we can later remove the `const`s without changing dark mode visually.)

- [ ] **Step 3: Add 3 unit tests at the end of theme.rs**

Edit `src/theme.rs`. Append at the very end of the file (after the closing brace of `theme_style`):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_palette_dark_matches_old_constants() {
        let c = palette(Theme::Dark);
        assert_eq!(c.bg_dark, Color32::from_rgb(0x1a, 0x1a, 0x1a));
        assert_eq!(c.panel_bg, Color32::from_rgb(0x22, 0x22, 0x22));
        assert_eq!(c.card_bg, Color32::from_rgb(0x2a, 0x2a, 0x2a));
        assert_eq!(c.hover_bg, Color32::from_rgb(0x35, 0x35, 0x35));
        assert_eq!(c.accent, Color32::from_rgb(0x4a, 0x9e, 0xff));
        assert_eq!(c.selected_bg, Color32::from_rgb(0x2d, 0x5a, 0x8e));
        assert_eq!(c.text_primary, Color32::from_rgb(0xe0, 0xe0, 0xe0));
        assert_eq!(c.text_secondary, Color32::from_rgb(0x88, 0x88, 0x88));
        assert_eq!(c.border, Color32::from_rgb(0x3a, 0x3a, 0x3a));
        assert_eq!(c.danger, Color32::from_rgb(0xe7, 0x4c, 0x3c));
    }

    #[test]
    fn test_palette_light_distinct_from_dark() {
        let dark = palette(Theme::Dark);
        let light = palette(Theme::Light);
        let diffs = [
            dark.bg_dark != light.bg_dark,
            dark.panel_bg != light.panel_bg,
            dark.card_bg != light.card_bg,
            dark.hover_bg != light.hover_bg,
            dark.accent != light.accent,
            dark.selected_bg != light.selected_bg,
            dark.text_primary != light.text_primary,
            dark.text_secondary != light.text_secondary,
            dark.border != light.border,
            dark.danger != light.danger,
        ];
        let count = diffs.iter().filter(|&&d| d).count();
        assert!(count >= 8, "expected at least 8 of 10 colors to differ, got {count}");
    }

    #[test]
    fn test_theme_from_str() {
        assert_eq!(Theme::from_str("dark"), Theme::Dark);
        assert_eq!(Theme::from_str("light"), Theme::Light);
        assert_eq!(Theme::from_str("nonsense"), Theme::Dark);
        assert_eq!(Theme::from_str(""), Theme::Dark);
    }
}
```

- [ ] **Step 4: Verify cargo check passes**

Run: `cargo check`
Expected: clean (the new symbols are unused for now — they will trigger `dead_code` warnings, but we accept those because Task 2 onward will use them. If the warning is unacceptable, add `#[allow(dead_code)]` to each new pub item temporarily; remove in Task 12.)

If dead_code warnings appear, add `#[allow(dead_code)]` attributes to `Theme`, `Theme::from_str`, `ThemeColors`, and `palette` to keep the build clean. These will be removed once callers are wired.

- [ ] **Step 5: Verify tests pass (3 new)**

Run: `cargo test`
Expected: 35 tests pass (32 + 3 new)

- [ ] **Step 6: Bump version and commit**

Edit `Cargo.toml`: `version = "0.1.25"` → `version = "0.1.26"`.

```bash
git add src/theme.rs Cargo.toml
git commit -m "feat: add Theme enum, ThemeColors struct, palette(), from_str

Foundation for light/dark theme switch. Adds Theme enum, ThemeColors
struct with 10 semantic colors, palette() dispatching on Theme, and
Theme::from_str for safe config parsing. Dark values match existing
const colors exactly. Light values per design spec. 3 unit tests cover
palette pinning, light/dark distinctness, and from_str fallback."
```

---

### Task 2: Refactor theme_visuals() to accept Theme argument

**Files:**
- Modify: `src/theme.rs:21-82` (theme_visuals signature and body)
- Modify: `src/app.rs:48` (only caller — pass Theme::Dark to preserve existing behavior)

**Interfaces:**
- Changes signature: `pub fn theme_visuals() -> Visuals` → `pub fn theme_visuals(theme: Theme) -> Visuals`
- Produces: `Visuals` filled from `palette(theme)` instead of hardcoded `const`s

- [ ] **Step 1: Read current theme_visuals body**

Run: `Read src/theme.rs` lines 21-82

Confirm every color reference is to a `const` (e.g. `BORDER`, `PANEL_BG`, `CARD_BG`, `TEXT_PRIMARY`, `TEXT_SECONDARY`, `ACCENT`, `SELECTED_BG`, `BG_DARK`, `DANGER`, `HOVER_BG`).

- [ ] **Step 2: Replace theme_visuals signature and body**

Edit `src/theme.rs`. Replace the entire `theme_visuals` function (lines 21-82) with:

```rust
pub fn theme_visuals(theme: Theme) -> Visuals {
    let c = palette(theme);
    Visuals {
        dark_mode: theme == Theme::Dark,
        override_text_color: Some(c.text_primary),
        window_corner_radius: CornerRadius::same(6),
        window_stroke: Stroke::new(1.0, c.border),
        panel_fill: c.panel_bg,
        faint_bg_color: c.bg_dark,
        extreme_bg_color: c.bg_dark,
        code_bg_color: c.card_bg,
        warn_fg_color: c.danger,
        error_fg_color: c.danger,
        hyperlink_color: c.accent,
        selection: egui::style::Selection {
            bg_fill: c.selected_bg,
            stroke: Stroke::new(1.0, c.accent),
        },
        widgets: egui::style::Widgets {
            noninteractive: egui::style::WidgetVisuals {
                bg_fill: c.card_bg,
                weak_bg_fill: c.panel_bg,
                bg_stroke: Stroke::new(1.0, c.border),
                corner_radius: CornerRadius::same(4),
                fg_stroke: Stroke::new(1.0, c.text_secondary),
                expansion: 0.0,
            },
            inactive: egui::style::WidgetVisuals {
                bg_fill: c.panel_bg,
                weak_bg_fill: c.card_bg,
                bg_stroke: Stroke::new(1.0, c.border),
                corner_radius: CornerRadius::same(4),
                fg_stroke: Stroke::new(1.0, c.text_primary),
                expansion: 0.0,
            },
            hovered: egui::style::WidgetVisuals {
                bg_fill: c.hover_bg,
                weak_bg_fill: c.hover_bg,
                bg_stroke: Stroke::new(1.0, c.accent),
                corner_radius: CornerRadius::same(4),
                fg_stroke: Stroke::new(1.5, c.accent),
                expansion: 1.0,
            },
            active: egui::style::WidgetVisuals {
                bg_fill: c.selected_bg,
                weak_bg_fill: c.selected_bg,
                bg_stroke: Stroke::new(1.0, c.accent),
                corner_radius: CornerRadius::same(4),
                fg_stroke: Stroke::new(2.0, c.accent),
                expansion: 1.0,
            },
            open: egui::style::WidgetVisuals {
                bg_fill: c.card_bg,
                weak_bg_fill: c.card_bg,
                bg_stroke: Stroke::new(1.0, c.border),
                corner_radius: CornerRadius::same(4),
                fg_stroke: Stroke::new(1.0, c.text_primary),
                expansion: 0.0,
            },
        },
        ..Default::default()
    }
}
```

- [ ] **Step 3: Update the sole caller in app.rs**

Edit `src/app.rs:48`. Replace:

```rust
cc.egui_ctx.set_visuals(crate::theme::theme_visuals());
```

with:

```rust
cc.egui_ctx.set_visuals(crate::theme::theme_visuals(crate::theme::Theme::Dark));
```

(Behavior-preserving: still dark. Task 5 will replace the hardcoded `Theme::Dark` with the config-driven value once `Config.theme` exists.)

- [ ] **Step 4: Verify cargo check passes**

Run: `cargo check`
Expected: clean

- [ ] **Step 5: Verify tests pass**

Run: `cargo test`
Expected: 35 tests pass

- [ ] **Step 6: Bump version and commit**

Edit `Cargo.toml`: `version = "0.1.26"` → `version = "0.1.27"`.

```bash
git add src/theme.rs src/app.rs Cargo.toml
git commit -m "refactor: theme_visuals accepts Theme argument and reads from palette

theme_visuals() -> theme_visuals(theme: Theme). Body now reads every
color from palette(theme) instead of the const declarations. The sole
caller in App::new hardcodes Theme::Dark to preserve existing behavior
until Config.theme lands in Task 4."
```

---

### Task 3: Refactor styled_icon to accept ThemeColors

**Files:**
- Modify: `src/theme.rs:15-18` (styled_icon signature)
- Modify: `src/browser/grid.rs:13` (only caller)

**Interfaces:**
- Changes signature: `pub fn styled_icon(codepoint: &str) -> egui::RichText` → `pub fn styled_icon(codepoint: &str, colors: &ThemeColors) -> egui::RichText`

- [ ] **Step 1: Read current styled_icon**

Run: `Read src/theme.rs` lines 15-18

Confirm body uses `ACCENT` constant.

- [ ] **Step 2: Refactor styled_icon**

Edit `src/theme.rs`. Replace lines 15-18:

```rust
pub fn styled_icon(codepoint: &str) -> egui::RichText {
    egui::RichText::new(codepoint).size(14.0).color(ACCENT)
}
```

with:

```rust
pub fn styled_icon(codepoint: &str, colors: &ThemeColors) -> egui::RichText {
    egui::RichText::new(codepoint).size(14.0).color(colors.accent)
}
```

- [ ] **Step 3: Update caller in browser/grid.rs**

Edit `src/browser/grid.rs:13`. The line currently reads:

```rust
        ui.label(crate::theme::styled_icon("\u{25C0}"));
```

This task does NOT yet introduce `colors` to `show_grid` (that happens in the sweep task for `browser/grid.rs`). To keep this task self-contained, change it to pass a dark palette explicitly (this will be replaced in Task 7):

```rust
        ui.label(crate::theme::styled_icon("\u{25C0}", &crate::theme::palette(crate::theme::Theme::Dark)));
```

(Task 7 will replace `palette(Theme::Dark)` with `app.theme_colors()` once that accessor exists.)

- [ ] **Step 4: Verify cargo check passes**

Run: `cargo check`
Expected: clean

- [ ] **Step 5: Verify tests pass**

Run: `cargo test`
Expected: 35 tests pass

- [ ] **Step 6: Bump version and commit**

Edit `Cargo.toml`: `version = "0.1.27"` → `version = "0.1.28"`.

```bash
git add src/theme.rs src/browser/grid.rs Cargo.toml
git commit -m "refactor: styled_icon accepts &ThemeColors instead of using const ACCENT

styled_icon(codepoint) -> styled_icon(codepoint, &ThemeColors). The
sole caller in show_grid temporarily passes palette(Theme::Dark); the
sweep task will replace it with app.theme_colors() once that accessor
exists."
```

---

### Task 4: Add Config.theme field with serde default

**Files:**
- Modify: `src/config.rs` (add field, default fn, and Default impl entry)
- Test: existing tests must pass; no new test (serde default is exercised at runtime; thumbnail config tests don't depend on theme)

**Interfaces:**
- Produces: `pub theme: String` field on `Config`, default `"dark"`

- [ ] **Step 1: Read config.rs**

Run: `Read src/config.rs`

Confirm:
- Lines 21-32: `Config` struct with fields `last_folder` through `thumb_size`
- Lines 34-48: `Default for Config` with field initializers

- [ ] **Step 2: Add default_theme helper**

Edit `src/config.rs`. At the top of the file (after line 2, before the `ColumnWidths` struct), add:

```rust
fn default_theme() -> String {
    "dark".to_string()
}
```

- [ ] **Step 3: Add theme field to Config struct**

Edit `src/config.rs`. After the `thumb_size: f32,` line (the last field in `Config`), add:

```rust
    #[serde(default = "default_theme")]
    pub theme: String,
```

- [ ] **Step 4: Add theme to Default impl**

Edit `src/config.rs`. In the `impl Default for Config` block, after the `thumb_size: 140.0,` line, add:

```rust
            theme: default_theme(),
```

- [ ] **Step 5: Verify cargo check passes**

Run: `cargo check`
Expected: clean (the field is never read yet — that's fine, it'll be read in Task 5)

- [ ] **Step 6: Verify tests pass**

Run: `cargo test`
Expected: 35 tests pass

- [ ] **Step 7: Bump version and commit**

Edit `Cargo.toml`: `version = "0.1.28"` → `version = "0.1.29"`.

```bash
git add src/config.rs Cargo.toml
git commit -m "feat: add Config.theme field with serde default to 'dark'

New optional field pub theme: String on Config, defaults to 'dark' via
#[serde(default = \"default_theme\")] to maintain backward compat with
existing configs (no field present → Dark theme, matching pre-existing
behavior). String type chosen for forward-compat (future themes)."
```

---

### Task 5: Wire App::theme_colors accessor and config-driven visuals

**Files:**
- Modify: `src/app.rs` (App::new uses config theme; add theme_colors method)

**Interfaces:**
- Produces: `App::theme_colors(&self) -> crate::theme::ThemeColors`

- [ ] **Step 1: Update App::new to use config theme value**

Edit `src/app.rs:48`. Replace:

```rust
cc.egui_ctx.set_visuals(crate::theme::theme_visuals(crate::theme::Theme::Dark));
```

with:

```rust
let theme = crate::theme::Theme::from_str(&config.theme);
cc.egui_ctx.set_visuals(crate::theme::theme_visuals(theme));
```

(Note: `config` is the local `let config = Config::load();` from earlier in `App::new`.)

- [ ] **Step 2: Add theme_colors accessor**

Edit `src/app.rs`. Add inside `impl App` (after `pub fn new(...)` closes, before `fn cache_dir`):

```rust
    pub fn theme_colors(&self) -> crate::theme::ThemeColors {
        crate::theme::palette(crate::theme::Theme::from_str(&self.config.theme))
    }
```

- [ ] **Step 3: Verify cargo check passes**

Run: `cargo check`
Expected: clean (the accessor may show as unused until callers in sweep tasks use it — add `#[allow(dead_code)]` to it if a warning appears; remove in Task 12)

- [ ] **Step 4: Verify tests pass**

Run: `cargo test`
Expected: 35 tests pass

- [ ] **Step 5: Bump version and commit**

Edit `Cargo.toml`: `version = "0.1.29"` → `version = "0.1.30"`.

```bash
git add src/app.rs Cargo.toml
git commit -m "feat: App::new uses config.theme; add App::theme_colors accessor

App::new reads config.theme (now a real field), parses via
Theme::from_str, and passes the resulting Theme to theme_visuals.
New accessor App::theme_colors() returns palette(Theme::from_str(...))
for use by UI rendering functions. Callers wired in subsequent sweep
tasks per file."
```

---

### Task 6: Sweep theme references in src/app.rs

**Files:**
- Modify: `src/app.rs` (5 theme references per grep)

**Interfaces:**
- Consumes: `App::theme_colors(&self)`

- [ ] **Step 1: Find all theme references in app.rs**

Run: `Grep "crate::theme::" src/app.rs`

Expected matches (from earlier grep):
- Line 206: `fill: crate::theme::PANEL_BG,` (menu bar frame)
- Line 211: `ui.style_mut().visuals.widgets.inactive.bg_fill = crate::theme::PANEL_BG;`
- Line 300-301: `ui.colored_label(crate::theme::ACCENT, "Key")` / `"Action"`
- Line 342: `ui.colored_label(crate::theme::TEXT_SECONDARY, "MIT License")`

- [ ] **Step 2: Replace each reference with theme_colors() access**

For each match, replace `crate::theme::<CONST>` with `self.theme_colors().<field>` (lowercase, no prefix). Field mapping:
- `PANEL_BG` → `panel_bg`
- `ACCENT` → `accent`
- `TEXT_SECONDARY` → `text_secondary`

Example: `fill: crate::theme::PANEL_BG,` → `fill: self.theme_colors().panel_bg,`

Apply this transformation to all 5 sites. Two sites (lines 206, 211) are within `update(&mut self, ...)` so `self.theme_colors()` works directly. Three sites (lines 300, 301, 342) are inside `egui::menu::bar(ui, |ui| { ... })` closures — since these closures capture `self` by reference (the outer scope has `&mut self`), `self.theme_colors()` is still callable. (Verify the closure form — if it's `|ui|` with `self` borrowed via the enclosing `&mut self`, this works; if the closure uses `move`, adjust.)

If a closure is `move`, capture the colors before the closure: `let colors = self.theme_colors();` then use `colors.panel_bg` inside.

- [ ] **Step 3: Verify cargo check passes**

Run: `cargo check`
Expected: clean

- [ ] **Step 4: Verify tests pass**

Run: `cargo test`
Expected: 35 tests pass

- [ ] **Step 5: Bump version and commit**

Edit `Cargo.toml`: `version = "0.1.30"` → `version = "0.1.31"`.

```bash
git add src/app.rs Cargo.toml
git commit -m "refactor: app.rs uses App::theme_colors() instead of theme consts

5 references to crate::theme::PANEL_BG/ACCENT/TEXT_SECONDARY replaced
with self.theme_colors().panel_bg/accent/text_secondary. Behavior
preserved (dark theme values identical to old const values)."
```

---

### Task 7: Sweep theme references in src/browser/grid.rs

**Files:**
- Modify: `src/browser/grid.rs` (28 theme references — the most of any file)

**Interfaces:**
- Consumes: `App::theme_colors(&self)`

- [ ] **Step 1: Find all theme references in grid.rs**

Run: `Grep "crate::theme::" src/browser/grid.rs`

Expected ~28 matches across `show_grid`, `show_thumbnail_grid`, `show_list_view`.

- [ ] **Step 2: Capture colors at top of each public grid function**

Edit `src/browser/grid.rs`. At the top of `pub fn show_grid(app: &mut App, ui: &mut egui::Ui) {` (line 9), as the first statement inside the function body, add:

```rust
let colors = app.theme_colors();
```

Similarly add `let colors = app.theme_colors();` at the top of `show_thumbnail_grid` and `show_list_view` (if `app` is in scope; both take `app: &mut App`).

For `drag_handle` and `col_widths` and other helper fns that don't take `app`: check if they reference `crate::theme::*`. The earlier grep showed line 541 references `crate::theme::BORDER` — locate which helper function contains this and add a `colors: &ThemeColors` parameter to that helper, OR inline-construct a dark palette (`crate::theme::palette(crate::theme::Theme::Dark)`) locally for the helper. Choose the parameter-passing approach for consistency.

If a helper fn references `crate::theme::BORDER` and is called from `show_list_view`, change its signature to accept `colors: &ThemeColors` and pass `&colors` from the caller.

- [ ] **Step 3: Replace every crate::theme::<CONST> reference with colors.<field>**

For each reference in grid.rs, replace `crate::theme::<CONST>` with `colors.<field>` (or `c.<field>` if you rename the local binding). Field mapping (from Task 6):

| Const          | Field           |
|----------------|-----------------|
| `BG_DARK`      | `bg_dark`       |
| `PANEL_BG`     | `panel_bg`      |
| `CARD_BG`      | `card_bg`       |
| `HOVER_BG`     | `hover_bg`      |
| `ACCENT`       | `accent`        |
| `SELECTED_BG`  | `selected_bg`   |
| `TEXT_PRIMARY` | `text_primary`  |
| `TEXT_SECONDARY` | `text_secondary` |
| `BORDER`       | `border`        |
| `DANGER`       | `danger`        |

Also: the `styled_icon` call at line 13 (from Task 3) currently passes `&palette(Theme::Dark)`. Replace with `&colors`:

```rust
ui.label(crate::theme::styled_icon("\u{25C0}", &colors));
```

The closure inside `egui::ScrollArea::vertical().show(ui, |ui| { ... })` captures `colors` by reference (the outer `let colors = app.theme_colors();` is in scope). Inside closures, use `colors.panel_bg` directly.

If a closure uses `move`, change the outer `let colors = app.theme_colors();` and capture `&colors` by reference inside the closure (or move `colors` in and use it directly — but then you can't use it after the closure).

- [ ] **Step 4: Verify cargo check passes**

Run: `cargo check`
Expected: clean (no new warnings)

- [ ] **Step 5: Verify tests pass**

Run: `cargo test`
Expected: 35 tests pass

- [ ] **Step 6: Bump version and commit**

Edit `Cargo.toml`: `version = "0.1.31"` → `version = "0.1.32"`.

```bash
git add src/browser/grid.rs Cargo.toml
git commit -m "refactor: browser/grid.rs uses App::theme_colors instead of consts

28 references to crate::theme::* replaced with colors.<field> from
app.theme_colors(). styled_icon call now passes &colors. Helpers that
need colors receive it as a parameter. Behavior preserved (dark values
identical)."
```

---

### Task 8: Sweep theme references in src/browser/tree.rs

**Files:**
- Modify: `src/browser/tree.rs` (5 theme references)

- [ ] **Step 1: Find theme references**

Run: `Grep "crate::theme::" src/browser/tree.rs`

Expected matches:
- Line 114: `crate::theme::SELECTED_BG`
- Line 116: `crate::theme::PANEL_BG`
- Line 146: `crate::theme::ACCENT`
- Line 156: `crate::theme::TEXT_PRIMARY`
- Line 158: `crate::theme::ACCENT`

- [ ] **Step 2: Capture colors in show_tree and pass to show_node**

Edit `src/browser/tree.rs`. At the top of `pub fn show_tree(app: &mut App, ui: &mut Ui) {` add as the first statement:

```rust
let colors = app.theme_colors();
```

`show_node` is recursive and takes `app: &mut App`, so it can call `app.theme_colors()` itself (or you can thread `colors: &ThemeColors` through). Simpler: inside `show_node`, also call `let colors = app.theme_colors();` at the top of the function body. (Slight perf overhead — calling `palette()` per recursive node — but `palette()` is just a struct construction, cheaper than a single rect paint call, and `show_node` is called few dozen times max.)

Edit `src/browser/tree.rs`. At the top of `fn show_node(...) {` body add:

```rust
let colors = app.theme_colors();
```

Then replace each `crate::theme::<CONST>` in `show_node` with `colors.<field>`.

- [ ] **Step 3: Verify cargo check passes**

Run: `cargo check`
Expected: clean

- [ ] **Step 4: Verify tests pass**

Run: `cargo test`
Expected: 35 tests pass

- [ ] **Step 5: Bump version and commit**

Edit `Cargo.toml`: `version = "0.1.32"` → `version = "0.1.33"`.

```bash
git add src/browser/tree.rs Cargo.toml
git commit -m "refactor: browser/tree.rs uses App::theme_colors instead of consts

5 references to crate::theme::SELECTED_BG/PANEL_BG/ACCENT/TEXT_PRIMARY
replaced with colors.<field>. show_tree and show_node each capture colors
locally via app.theme_colors()."
```

---

### Task 9: Sweep theme references in src/browser/mod.rs

**Files:**
- Modify: `src/browser/mod.rs` (2 theme references)

- [ ] **Step 1: Find theme references**

Run: `Grep "crate::theme::" src/browser/mod.rs`

Expected matches:
- Line 68: `crate::theme::PANEL_BG`
- Line 94: `crate::theme::BORDER`

- [ ] **Step 2: Capture colors and replace**

Read the function containing these lines (likely `show_side_panel` or similar):

Run: `Read src/browser/mod.rs` around lines 60-95 to identify the enclosing function and whether `app: &mut App` is in scope.

If `app` is in scope: at the top of that function, add `let colors = app.theme_colors();` and replace both `crate::theme::PANEL_BG` → `colors.panel_bg` and `crate::theme::BORDER` → `colors.border`.

If the function takes `app: &App` (immutable) but `theme_colors(&self)` is `&self`→ works.

- [ ] **Step 3: Verify cargo check passes**

Run: `cargo check`
Expected: clean

- [ ] **Step 4: Verify tests pass**

Run: `cargo test`
Expected: 35 tests pass

- [ ] **Step 5: Bump version and commit**

Edit `Cargo.toml`: `version = "0.1.33"` → `version = "0.1.34"`.

```bash
git add src/browser/mod.rs Cargo.toml
git commit -m "refactor: browser/mod.rs uses App::theme_colors instead of consts

2 references to crate::theme::PANEL_BG/BORDER replaced with
colors.panel_bg/colors.border."
```

---

### Task 10: Sweep theme references in src/viewer.rs

**Files:**
- Modify: `src/viewer.rs` (10 theme references)

- [ ] **Step 1: Find theme references**

Run: `Grep "crate::theme::" src/viewer.rs`

Expected matches:
- Line 51: `fill: crate::theme::PANEL_BG,`
- Line 57: `egui::RichText::new("\u{2190} Browser").color(crate::theme::ACCENT)`
- Line 78: `ui.colored_label(crate::theme::TEXT_SECONDARY, "Zoom:")`
- Line 117, 123, 191, 203: `TEXT_SECONDARY`
- Line 183: `PANEL_BG`
- Line 366: `BORDER`

Total: ~10 references.

- [ ] **Step 2: Capture colors at top of viewer rendering fn**

The viewer rendering is in a top-level function — check its signature:

Run: `Read src/viewer.rs` lines 1-50 to find the function signature (likely `pub fn draw_viewer(app: &mut App, ctx: &egui::Context)` or similar).

Edit `src/viewer.rs`. At the top of the viewer's main public function (the one called from `App::update`), add as the first statement:

```rust
let colors = app.theme_colors();
```

Then replace every `crate::theme::<CONST>` with `colors.<field>`.

For closures inside `egui::CentralPanel::default().show(ctx, |ui| { ... })`:
- The closure captures `app` by mutable reference (we see `app.viewer_state.is_slideshow`, etc. in the body).
- Use `app.theme_colors().<field>` directly inside the closure (recomputes the palette per call — negligible since palette is just a struct build), OR
- Capture `colors` outside the closure if possible. The issue: outside the closure, `app` is also borrowed by `CentralPanel::default().show(ctx, |ui| { /* uses app */ })`. You can't have `let colors = app.theme_colors();` outside the closure AND pass `app` into the closure — that's a double-borrow.

**Therefore: inside the closure, use `app.theme_colors().<field>` directly** for each replacement. (Reads fine because `theme_colors(&self)` is `&self`.)

For lines outside the `CentralPanel::show` closure (lines 51, 57 — likely in `TopBottomPanel` setup): check whether they're inside a different closure that also captures `app`. If they're inside `egui::TopBottomPanel::top(...).show(ctx, |ui| { /* app... */ })`, same approach: use `app.theme_colors().<field>`.

- [ ] **Step 3: Verify cargo check passes**

Run: `cargo check`
Expected: clean

- [ ] **Step 4: Verify tests pass**

Run: `cargo test`
Expected: 35 tests pass

- [ ] **Step 5: Bump version and commit**

Edit `Cargo.toml`: `version = "0.1.34"` → `version = "0.1.35"`.

```bash
git add src/viewer.rs Cargo.toml
git commit -m "refactor: viewer.rs uses App::theme_colors instead of consts

10 references to crate::theme::PANEL_BG/BORDER/ACCENT/TEXT_SECONDARY
replaced with app.theme_colors().<field>. Inside CentralPanel and
TopBottomPanel closures, app.theme_colors() is called per-access to
avoid double-borrow of app."
```

---

### Task 11: Sweep theme references in src/editor/mod.rs, src/batch/mod.rs, src/exif.rs

**Files:**
- Modify: `src/editor/mod.rs` (8 theme references)
- Modify: `src/batch/mod.rs` (4 theme references)
- Modify: `src/exif.rs` (5 theme references)

- [ ] **Step 1: Sweep editor/mod.rs**

Run: `Grep "crate::theme::" src/editor/mod.rs`

Expected matches (per earlier grep): `PANEL_BG` (line 67), `ACCENT` (lines 101, 123, 145, 171), `TEXT_SECONDARY` (lines 147, 154).

Read the function signature containing these:

Run: `Read src/editor/mod.rs` lines 1-70 to find the public function (likely `pub fn show_editor(app: &mut App, ctx: &egui::Context, ui: &mut egui::Ui)` or similar).

Edit `src/editor/mod.rs`. At the top of the editor's main function, add `let colors = app.theme_colors();` if `app` is exclusively available; otherwise use `app.theme_colors().<field>` inline.

Replace all 8 references with `colors.<field>`. (Same closure-borrowing caveat as Task 10 — if editor panel uses `.show(|ui| { /* app used */ })`, use inline `app.theme_colors().<field>` inside the closure.)

- [ ] **Step 2: Sweep batch/mod.rs**

Run: `Grep "crate::theme::" src/batch/mod.rs`

Expected matches: `TEXT_SECONDARY` (line 106), `ACCENT` (lines 151, 183, 215).

Read the function signature:

Run: `Read src/batch/mod.rs` lines 1-110 to find the public function (likely `pub fn show_batch_dialog(...)` taking `app: &mut App` and `ctx: &egui::Context`).

Edit `src/batch/mod.rs`. At the top of the batch dialog's main function, add `let colors = app.theme_colors();`. Replace each `crate::theme::<CONST>` with `colors.<field>`. Inside closures, use `app.theme_colors().<field>` inline.

- [ ] **Step 3: Sweep exif.rs**

Run: `Grep "crate::theme::" src/exif.rs`

Expected matches: `PANEL_BG` (lines 173, 185), `CARD_BG` (line 187), `ACCENT` (line 197), `TEXT_PRIMARY` (line 198).

Read the function containing these:

Run: `Read src/exif.rs` lines 160-200 to identify the function (likely `pub fn show_exif_panel(...)` or a similar rendering fn taking `app: &mut App` and `ui: &mut egui::Ui`).

Edit `src/exif.rs`. At top of the EXIF rendering fn, add `let colors = app.theme_colors();`. Replace all 5 references with `colors.<field>`.

(If `exif.rs` doesn't take `app` — for instance, takes `ui` only — you may need to thread `colors: &ThemeColors` as a parameter, or change the call site to pass `&app.theme_colors()`.)

- [ ] **Step 4: Verify cargo check passes**

Run: `cargo check`
Expected: clean

- [ ] **Step 5: Verify tests pass**

Run: `cargo test`
Expected: 35 tests pass

- [ ] **Step 6: Bump version and commit**

Edit `Cargo.toml`: `version = "0.1.35"` → `version = "0.1.36"`.

```bash
git add src/editor/mod.rs src/batch/mod.rs src/exif.rs Cargo.toml
git commit -m "refactor: editor/batch/exif use App::theme_colors instead of consts

17 total references replaced across editor/mod.rs (8), batch/mod.rs (4),
and exif.rs (5). Each module's main rendering function captures colors
via app.theme_colors() at the top, with app.theme_colors().<field> used
inline inside closures to avoid double-borrows."
```

---

### Task 12: Remove unused const declarations from theme.rs and add View menu toggle

**Files:**
- Modify: `src/theme.rs` (remove the 10 `const` declarations now that no callers reference them)
- Modify: `src/app.rs` (add View menu "Toggle Theme" entry)
- Remove any `#[allow(dead_code)]` added during setup

**Interfaces:**
- Removes: `pub const BG_DARK, PANEL_BG, CARD_BG, HOVER_BG, ACCENT, SELECTED_BG, TEXT_PRIMARY, TEXT_SECONDARY, BORDER, DANGER` from `theme.rs` (no external callers after Tasks 6-11)

- [ ] **Step 1: Verify no callers reference the consts**

Run: `Grep "crate::theme::BG_DARK|crate::theme::PANEL_BG|crate::theme::CARD_BG|crate::theme::HOVER_BG|crate::theme::ACCENT|crate::theme::SELECTED_BG|crate::theme::TEXT_PRIMARY|crate::theme::TEXT_SECONDARY|crate::theme::BORDER|crate::theme::DANGER"`

Expected: 0 matches in `src/` (only `theme.rs`'s own const declarations may still exist).

If any matches remain outside `theme.rs`, fix them first (a sweep task missed a site).

- [ ] **Step 2: Remove the const block from theme.rs**

Edit `src/theme.rs`. Delete lines 4-13 (the 10 const declarations):

```rust
// ── Color palette ──────────────────────────────────────────
pub const BG_DARK: Color32 = Color32::from_rgb(0x1a, 0x1a, 0x1a);
pub const PANEL_BG: Color32 = Color32::from_rgb(0x22, 0x22, 0x22);
pub const CARD_BG: Color32 = Color32::from_rgb(0x2a, 0x2a, 0x2a);
pub const HOVER_BG: Color32 = Color32::from_rgb(0x35, 0x35, 0x35);
pub const ACCENT: Color32 = Color32::from_rgb(0x4a, 0x9e, 0xff);
pub const SELECTED_BG: Color32 = Color32::from_rgb(0x2d, 0x5a, 0x8e);
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xe0, 0xe0, 0xe0);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(0x88, 0x88, 0x88);
pub const BORDER: Color32 = Color32::from_rgb(0x3a, 0x3a, 0x3a);
pub const DANGER: Color32 = Color32::from_rgb(0xe7, 0x4c, 0x3c);
```

(Their values are now generated by `palette()`.)

- [ ] **Step 3: Remove any #[allow(dead_code)] attributes added during setup**

If you added `#[allow(dead_code)]` to `Theme`, `ThemeColors`, `palette`, `from_str`, or `App::theme_colors` in earlier tasks, remove them now. They should all have live callers.

- [ ] **Step 4: Add View menu "Toggle Theme" entry**

Edit `src/app.rs`. Inside the `ui.menu_button("View", |ui| { ... })` block (after the existing "Toggle Grid/List" button, before the closing `});`), add:

```rust
                        ui.separator();
                        let next_theme = if self.config.theme == "light" { "Dark" } else { "Light" };
                        if ui.button(format!("Switch to {next_theme} Theme")).clicked() {
                            self.config.theme = next_theme.to_lowercase();
                            self.config.save();
                            ctx.set_visuals(crate::theme::theme_visuals(
                                crate::theme::Theme::from_str(&self.config.theme),
                            ));
                            ui.close_menu();
                        }
```

(The button shows the *target* theme: "Switch to Dark Theme" when currently light, and vice versa — clearer than a generic "Toggle Theme".)

- [ ] **Step 5: Verify cargo check passes**

Run: `cargo check`
Expected: clean (no new warnings — const removal should not produce unused-import warnings because `Color32` is still used by `palette` and `theme_visuals`)

- [ ] **Step 6: Verify tests pass**

Run: `cargo test`
Expected: 35 tests pass

- [ ] **Step 7: Bump version and commit**

Edit `Cargo.toml`: `version = "0.1.36"` → `version = "0.1.37"`.

```bash
git add src/theme.rs src/app.rs Cargo.toml
git commit -m "feat: View menu entry to switch themes; remove old theme consts

Adds 'Switch to {Dark|Light} Theme' button in View menu. Persists the
choice via config.save(), then calls ctx.set_visuals(theme_visuals(
from_str(...))) to apply immediately. With all callers now using
App::theme_colors(), the 10 pub const Color32 declarations in theme.rs
are removed; palette() is now the single source of truth."
```

---

### Task 13: Manual verification (both themes render correctly)

**Files:**
- None (runtime verification only)

- [ ] **Step 1: Build release binary**

Run: `cargo build --release`
Expected: clean build, no warnings beyond the pre-existing 1.

- [ ] **Step 2: Launch app and switch to light theme**

Launch the built binary. Click View → "Switch to Light Theme". Verify:
- Menu bar background is light (#f5f5f5)
- Folder tree panel background is light
- Thumbnail grid cards have light backgrounds (#e8e8e8 for unselected)
- Text is dark (#1a1a1a for primary, #666666 for secondary)
- Accent color (links, icons) is the darker blue (#1a6dd6) — readable on light
- Selected thumbnail row in list view uses light blue (#a8c8f0)
- Borders are light grey (#cccccc)

- [ ] **Step 3: Verify light theme persists across restart**

Quit the app. Relaunch. Confirm the light theme is still active (config.json `theme` field is `"light"`).

- [ ] **Step 4: Switch back to dark theme and verify**

Click View → "Switch to Dark Theme". Verify:
- All backgrounds return to dark greys (#1a, #22, #2a, #35)
- Text returns to light grey (#e0e0e0)
- Accent returns to original blue (#4a9eff)
- Borders return to #3a3a3a

- [ ] **Step 5: Verify dark theme persists across restart**

Quit, relaunch, confirm dark theme active.

- [ ] **Step 6: Open an image and toggle theme in viewer mode**

Open a folder, double-click an image to enter Viewer mode. Toggle theme from the View menu (available in viewer mode via the top menu). Verify:
- Image itself unchanged (it's pixel data, theme-independent)
- Status bar background switches light/dark
- Browser button (top-left "← Browser") uses correct accent color
- Zoom/pan state preserved across the toggle (no reset)

- [ ] **Step 7: Verify config.json backward compat**

Manually edit `cache/config.json` (in exe dir) — remove the `"theme"` field entirely. Relaunch app. Confirm:
- App starts in dark theme (serde default)
- No errors or panics

- [ ] **Step 8: Verify corrupt config fallback**

Manually edit `cache/config.json` — set `"theme": "purple"`. Relaunch app. Confirm:
- App starts in dark theme (Theme::from_str fallback)
- No errors or panics

- [ ] **Step 9: Bump version (final) and commit if any fixups were needed**

If any issues were found in Steps 1-8, fix them now with a new commit. Otherwise, no commit needed (the binary is already at v0.1.37 from Task 12).

Edit `Cargo.toml`: bump version if you made fixup commits (e.g. `version = "0.1.37"` → `version = "0.1.38"`).

---

## Self-Review Checklist

After all 13 tasks complete, verify:

- [ ] **All 35 tests pass**: `cargo test` exits 0
- [ ] **`cargo check` clean**: no new errors or warnings beyond the pre-existing 1
- [ ] **`cargo build --release` succeeds**: produces a working binary
- [ ] **git log shows 12+ commits**, each with a clear message and version bump
- [ ] **No new external dependencies** in Cargo.toml
- [ ] **All spec requirements covered**:
  - `Theme` enum + `ThemeColors` struct in `theme.rs` → Task 1
  - `palette()` function with 10 colors × 2 themes → Task 1
  - `Theme::from_str` fallback to Dark → Task 1 (with test)
  - `theme_visuals(theme)` refactor → Task 2
  - `styled_icon(codepoint, &ThemeColors)` → Task 3
  - `Config.theme: String` with serde default → Task 4
  - `App::theme_colors()` accessor → Task 5
  - Config-driven visuals in `App::new` → Task 5
  - Call-site sweep (~50 references) → Tasks 6-11
  - View menu entry → Task 12
  - Const removal → Task 12
  - 3 unit tests → Task 1
  - Manual runtime verification of both themes + restart + corrupt config → Task 13
- [ ] **YAGNI** preserved: no Solarized theme, no OS detection, no shortcut, no preview, no animation

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-07-04-theme-switch.md`. Two execution options:

**1. Subagent-Driven (recommended)** — Dispatch a fresh subagent per task, review between tasks, fast iteration. Best for call-site sweep where typo risk is high.

**2. Inline Execution** — Execute tasks in this session using `superpowers:executing-plans`, batch execution with checkpoints. Best if you want to watch progress live.

Which approach?
