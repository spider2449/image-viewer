# Light/Dark Theme Switch Design

**Date:** 2026-07-04
**Status:** Approved (pending implementation)
**Goal:** Add a user-toggleable light/dark theme to the image-viewer. Persist the choice in config.json. Default to dark for backward compatibility.

## Architecture

Single new module addition (`Theme` enum + `ThemeColors` struct) lives in `src/theme.rs`. No new dependencies, no new files outside `theme.rs`.

### Module layout

- `src/theme.rs`:
  - `pub enum Theme { Dark, Light }` — serializable to/from `"dark"`/`"light"` strings.
  - `pub struct ThemeColors { bg_dark, panel_bg, card_bg, hover_bg, accent, selected_bg, text_primary, text_secondary, border, danger }` — 10 `Color32` fields.
  - `pub fn palette(theme: Theme) -> ThemeColors` — single source of truth for color values per theme.
  - `pub fn theme_visuals(theme: Theme) -> Visuals` — replaces current `theme_visuals()`. Dispatches on theme, fills `Visuals` from `palette(theme)`. Sets `dark_mode: bool` accordingly.
  - `pub fn styled_icon(codepoint: &str, colors: &ThemeColors) -> egui::RichText` — replaces `styled_icon(codepoint)`. Color comes from `colors.accent` instead of constant `ACCENT`.
  - `pub fn theme_style() -> Style` — unchanged (spacing is theme-independent).
  - `pub fn Theme::from_str(s: &str) -> Theme` — `"dark"` → `Dark`, `"light"` → `Light`, any other value (including `""` and unknowns) → `Dark` (safe fallback).

- `src/config.rs`:
  - Add `pub theme: String` field to `Config`. Default value `"dark"` via `#[serde(default = "default_theme")]` where `fn default_theme() -> String { "dark".to_string() }`. Existing configs without the field deserialize to `"dark"`. No migration code.

- `src/app.rs`:
  - `App::new` calls `cc.egui_ctx.set_visuals(theme::theme_visuals(Theme::from_str(&config.theme)))` then `set_style(...)`.
  - New accessor `App::theme_colors(&self) -> ThemeColors` returning `theme::palette(Theme::from_str(&self.config.theme))`. Called at the top of every UI rendering function. Supersedes every direct `crate::theme::PANEL_BG`-style reference.
  - View menu gains a "Toggle Theme (Dark/Light)" entry. On click: flip `self.config.theme`, call `self.config.save()`, call `ctx.set_visuals(theme::theme_visuals(...))`, then `ui.close_menu()`.

### App state

`App` does **not** cache a `ThemeColors` field — it is computed on demand via the accessor. Rationale: the palette is pure data and cheap to build; caching would require invalidation on toggle, adding statefulness for no perf gain. The accessor reads `self.config.theme` (already the source of truth).

### Config schema

```rust
#[derive(Serialize, Deserialize)]
pub struct Config {
    // ...existing fields...
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_theme() -> String { "dark".to_string() }
```

The `String` type (rather than `Theme` enum directly) is intentionally chosen for forward-compat: adding future themes (e.g. `"solarized"`) does not require a schema migration. `Theme::from_str` is the parser; unknown strings fall back to `Dark`.

## Data flow

```
Config.theme (String "dark"|"light")
   ↓ loaded at startup, toggled via View menu
App::theme_colors() -> ThemeColors
   ↓ called at top of each UI rendering fn
palette(theme) -> ThemeColors { panel_bg, text_primary, ... }
   ↓ consumed by painter calls and widget colors
ctx.set_visuals(theme_visuals(theme)) -> updates global widget visuals
   ↓ called once on toggle (not every frame — egui retains visuals between frames)
```

`theme_visuals(theme)` returns a fully-populated `Visuals` (not just `dark_mode: bool` flipped) — every widget color is set from `palette(theme)` to preserve the bespoke look in both themes. `Visuals::dark_mode` is a hint affecting egui's built-in rendering defaults; set `true` for dark, `false` for light.

## Palette values

The 10 semantic color categories with their dark and light values:

| Color              | Dark (Hex)  | Light (Hex) | Rationale for light value |
|--------------------|------------|------------|---------------------------|
| `bg_dark`          | `#1a1a1a`  | `#ffffff`  | Pure white for extreme contrast areas |
| `panel_bg`         | `#222222`  | `#f5f5f5`  | Off-white panel, slightly distinguishes from pure white |
| `card_bg`          | `#2a2a2a`  | `#e8e8e8`  | Slightly darker than panel for table rows and cards |
| `hover_bg`         | `#353535`  | `#dcdcdc`  | Visible hover highlight without being too dark |
| `accent`           | `#4a9eff`  | `#1a6dd6`  | Darkened blue to maintain contrast on white (WCAG AA) |
| `selected_bg`      | `#2d5a8e`  | `#a8c8f0`  | Light blue, keeps dark text readable when selected |
| `text_primary`     | `#e0e0e0`  | `#1a1a1a`  | Near-black text on light bg |
| `text_secondary`   | `#888888`  | `#666666`  | Slightly darker grey for secondary text on light bg |
| `border`           | `#3a3a3a`  | `#cccccc`  | Visible but not heavy separators on light bg |
| `danger`           | `#e74c3c`  | `#c0392b`  | Slightly darker red for sufficient contrast on light |

**Design principle**: light-theme text is darker than dark-theme text; panel_bg becomes near-white instead of dark-grey. Accent darkened to maintain sufficient contrast (#4a9eff has poor contrast on white). Selected_bg lightened so highlighted rows remain readable with dark text.

## Error handling

- **Invalid theme string in config.json** (manual edit, corruption, downgrade): `Theme::from_str` returns `Theme::Dark` for any value other than `"light"`. No panic, no error log. Silently defaults to dark. Existing configs without a `theme` field: serde's `#[serde(default)]` yields `"dark"`.
- **`ctx.set_visuals` failures**: `set_visuals` is infallible — no error path needed.
- **Config save on toggle**: reuses existing `config.save()` path. One write per toggle (not per-frame); same I/O characteristics as the existing config save on column resize.

## Edge cases

- **First launch after upgrade**: config.json missing `theme` field → `#[serde(default)]` provides `"dark"`. No migration code needed.
- **Toggling while viewer is open**: visuals update on next frame; in-progress zoom/pan state preserved (color-only swap, no viewer state touched).
- **Toggling while editor has unsaved changes**: editor `current_image` and undo stack untouched.
- **Toggling mid-slideshow**: slideshow timer/index untouched; only colors change.
- **Slideshow mid-decode**: thumbnail workers don't read `App.theme`; thumbnails are RGB pixel data, theme-independent.
- **Menu display**: View menu label is static "Toggle Theme (Dark/Light)" — does not dynamically show current theme. (Showing current theme in the label would be a nice-to-have but is out of scope per Section 4.)

## Testing

Three unit tests in `src/theme.rs`:

1. **`test_palette_dark_matches_old_constants`** — assert `palette(Theme::Dark).panel_bg == Color32::from_rgb(0x22, 0x22, 0x22)` for every one of the 10 colors. Prevents accidental palette drift when refactoring `const`s into the struct. Pinning values, not just structure.

2. **`test_palette_light_distinct_from_dark`** — assert at least 8 of 10 colors differ between `palette(Dark)` and `palette(Light)`. Sanity check that light palette isn't a copy-paste of dark.

3. **`test_theme_from_str`** — assert `from_str("dark") == Theme::Dark`, `from_str("light") == Theme::Light`, `from_str("nonsense") == Theme::Dark`, `from_str("") == Theme::Dark`. Verifies the fallback behavior that protects against corrupted configs.

Cannot unit-test the actual visual swap (requires an egui context); palette dispatch is pure and covered by the above. No new integration tests. Existing 32 tests must still pass — the rename `theme_visuals()` → `theme_visuals(theme)` is a signature change internal to the crate; no external API impact (crate is a binary, no public API).

## Scope

### In scope
- New `Theme` enum + `ThemeColors` struct in `theme.rs`
- `palette(Theme) -> ThemeColors` with the 10 colors from the table above
- Refactor `theme_visuals()` → `theme_visuals(theme)`; `styled_icon(codepoint)` → `styled_icon(codepoint, &ThemeColors)`
- Update `App::new` to call `theme_visuals(Theme::from_str(&config.theme))`
- `App::theme_colors(&self) -> ThemeColors` accessor
- `Config.theme: String` field with `#[serde(default = "default_theme")]`
- View menu entry "Toggle Theme (Dark/Light)"
- Replace every `crate::theme::PANEL_BG`-style reference with `colors.panel_bg` (or equivalent), where `colors` is obtained from `app.theme_colors()` or passed through where `app` isn't directly available
- 3 unit tests in `theme.rs`

### Out of scope (YAGNI)
- More than 2 themes (solarized, gruvbox, custom themes, etc.) — design supports future addition; scope stays at 2
- Per-component theme overrides — no individual color picker
- OS preference detection — user chose dark default
- Keyboard shortcut — user chose menu-only
- Live preview — toggle is instant click
- Animated transition — instant color swap, no fade
- Theme indication in window title — outside View menu label
- Custom user themes via config file — out of scope
- Theme syncing across machines — already handled by config.json persistence

## Backward compatibility

- **Config schema**: forward-compatible. `theme: String` with serde default. Old configs without the field → dark theme (matches pre-existing behavior exactly).
- **Existing tests**: pass unchanged. Signature change `theme_visuals()` → `theme_visuals(theme)` is an internal call-site update, not a breaking public API change.
- **No public API breakage**: crate is a binary; no external consumers.
- **Runtime behavior**: identical to current for users who never toggle (they remain on dark theme).

## Risk & rollback

- **Risk**: largest single change since the review-fixes batch. Roughly 50 call sites touched across 8 files (`app.rs`, `viewer.rs`, `browser/grid.rs`, `browser/tree.rs`, `editor/mod.rs`, `batch/mod.rs`, `theme.rs`). Risk vector: typos in color swaps (e.g. `text_primary` accidentally becoming `text_secondary` in one file).
- **Mitigation**: grep-driven sweep to enumerate every `crate::theme::PANEL_BG`-style reference before refactoring; mechanical find-replace with post-edit grep verification; run `cargo test` after every task; manually run app and toggle theme at end of integration task.
- **Rollback**: each task commits independently. `git revert <commit>` per task is sufficient. No data migration to undo — config field has a safe default.

## Implementation order (high-level; full plan to follow)

Will be broken into bite-sized tasks in the implementation plan:

1. Add `Theme` enum, `ThemeColors` struct, `palette()` function, `Theme::from_str()` in `theme.rs` with the 3 unit tests (no existing code touched yet — new symbols sit alongside the old `const`s).
2. Refactor `theme_visuals()` → `theme_visuals(theme: Theme)`. Update its only caller (`App::new`) to pass `Theme::Dark` (preserves existing behavior; later task wires the real config value).
3. Refactor `styled_icon(codepoint)` → `styled_icon(codepoint, &ThemeColors)`. Update all callers.
4. Add `Config.theme` field with serde default.
5. Add `App::theme_colors()` accessor and `App::new` call to `theme_visuals(Theme::from_str(&config.theme))`.
6. Replace every direct `crate::theme::<CONST>` reference with the corresponding `colors.<field>` access — sweep per file (`app.rs`, `viewer.rs`, `browser/grid.rs`, `browser/tree.rs`, `editor/mod.rs`, `batch/mod.rs`).
7. Add View menu "Toggle Theme" entry with persist-on-toggle logic.
8. Manually run app, toggle theme, verify both themes render correctly.
