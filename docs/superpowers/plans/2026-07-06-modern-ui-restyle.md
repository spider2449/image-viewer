# Modern UI Restyle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Modernize `src/theme.rs`'s color palette, spacing, corner radii, and depth effects (shadows, alpha layering) for both dark ("Dark Glass") and light ("clean flat") themes, with no changes to `app.rs` or `viewer.rs`.

**Architecture:** All changes are confined to `src/theme.rs`: the `palette()` function's `ThemeColors` values, `theme_style()`'s `Spacing`/`Interaction`, and `theme_visuals()`'s `Visuals` (corner radii, shadows, widget states). Existing unit tests in `theme.rs` are updated to match the new palette values.

**Tech Stack:** Rust, egui 0.31.1 / epaint 0.31.1 (`Shadow { offset: [i8;2], blur: u8, spread: u8, color: Color32 }`).

## Global Constraints

- All code, comments, identifiers must be in English only (per user's global CLAUDE.md).
- No new crate dependencies.
- Do not touch `CornerRadius::ZERO` literal usages in `viewer.rs` (checkerboard cells, image border) — those are intentional pixel-perfect drawing, out of scope.
- Do not change layout/structure in `app.rs` / `viewer.rs`.
- Dark theme uses alpha-layered fills (glass feel); light theme stays fully opaque (flat feel).
- Accent color: dark `#5AA9FF` (rgb 90,169,255), light `#1F6FE5` (rgb 31,111,229).

---

### Task 1: Update palette colors for both themes

**Files:**
- Modify: `src/theme.rs:33-60` (the `palette()` function)
- Modify: `src/theme.rs:161-207` (existing tests — update expected values)

**Interfaces:**
- Produces: `palette(Theme) -> ThemeColors` unchanged signature; only field values change. Later tasks (`theme_visuals`, `theme_style`) consume `ThemeColors` fields by the same names (`bg_dark`, `panel_bg`, `card_bg`, `hover_bg`, `accent`, `selected_bg`, `text_primary`, `text_secondary`, `border`, `danger`).

- [ ] **Step 1: Update the failing test expectations first**

Replace the two color-assertion tests in `src/theme.rs` (lines ~165-198) with new expected values:

```rust
    #[test]
    fn test_palette_dark_matches_new_constants() {
        let c = palette(Theme::Dark);
        assert_eq!(c.bg_dark, Color32::from_rgb(0x14, 0x14, 0x14));
        assert_eq!(c.panel_bg, Color32::from_rgb(0x1c, 0x1c, 0x1c));
        assert_eq!(c.card_bg, Color32::from_rgb(0x24, 0x24, 0x24));
        assert_eq!(c.hover_bg, Color32::from_rgb(0x2e, 0x2e, 0x2e));
        assert_eq!(c.accent, Color32::from_rgb(0x5a, 0xa9, 0xff));
        assert_eq!(c.selected_bg, Color32::from_rgba_unmultiplied(0x5a, 0xa9, 0xff, 0x40));
        assert_eq!(c.text_primary, Color32::from_rgb(0xe8, 0xe8, 0xe8));
        assert_eq!(c.text_secondary, Color32::from_rgb(0x8f, 0x8f, 0x8f));
        assert_eq!(c.border, Color32::from_rgb(0x3a, 0x3a, 0x3a));
        assert_eq!(c.danger, Color32::from_rgb(0xe7, 0x4c, 0x3c));
    }

    #[test]
    fn test_palette_light_matches_new_constants() {
        let c = palette(Theme::Light);
        assert_eq!(c.bg_dark, Color32::from_rgb(0xff, 0xff, 0xff));
        assert_eq!(c.panel_bg, Color32::from_rgb(0xf7, 0xf7, 0xf8));
        assert_eq!(c.card_bg, Color32::from_rgb(0xee, 0xf0, 0xf2));
        assert_eq!(c.hover_bg, Color32::from_rgb(0xe2, 0xe5, 0xe9));
        assert_eq!(c.accent, Color32::from_rgb(0x1f, 0x6f, 0xe5));
        assert_eq!(c.selected_bg, Color32::from_rgb(0xcf, 0xe0, 0xfa));
        assert_eq!(c.text_primary, Color32::from_rgb(0x1a, 0x1a, 0x1a));
        assert_eq!(c.text_secondary, Color32::from_rgb(0x5a, 0x5a, 0x5a));
        assert_eq!(c.border, Color32::from_rgb(0xd6, 0xd8, 0xdb));
        assert_eq!(c.danger, Color32::from_rgb(0xc0, 0x39, 0x2b));
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
```

Keep the existing `test_theme_from_str` test as-is.

- [ ] **Step 2: Run tests to verify the new palette tests fail**

Run: `cargo test --lib theme::tests -- --nocapture`
Expected: FAIL (`test_palette_dark_matches_new_constants` and `test_palette_light_matches_new_constants` fail because `palette()` still returns old values; old test names are gone so no stale-pass risk).

- [ ] **Step 3: Update `palette()` implementation**

Replace `src/theme.rs:33-60` with:

```rust
pub fn palette(theme: Theme) -> ThemeColors {
    match theme {
        Theme::Dark => ThemeColors {
            bg_dark: Color32::from_rgb(0x14, 0x14, 0x14),
            panel_bg: Color32::from_rgb(0x1c, 0x1c, 0x1c),
            card_bg: Color32::from_rgb(0x24, 0x24, 0x24),
            hover_bg: Color32::from_rgb(0x2e, 0x2e, 0x2e),
            accent: Color32::from_rgb(0x5a, 0xa9, 0xff),
            selected_bg: Color32::from_rgba_unmultiplied(0x5a, 0xa9, 0xff, 0x40),
            text_primary: Color32::from_rgb(0xe8, 0xe8, 0xe8),
            text_secondary: Color32::from_rgb(0x8f, 0x8f, 0x8f),
            border: Color32::from_rgb(0x3a, 0x3a, 0x3a),
            danger: Color32::from_rgb(0xe7, 0x4c, 0x3c),
        },
        Theme::Light => ThemeColors {
            bg_dark: Color32::from_rgb(0xff, 0xff, 0xff),
            panel_bg: Color32::from_rgb(0xf7, 0xf7, 0xf8),
            card_bg: Color32::from_rgb(0xee, 0xf0, 0xf2),
            hover_bg: Color32::from_rgb(0xe2, 0xe5, 0xe9),
            accent: Color32::from_rgb(0x1f, 0x6f, 0xe5),
            selected_bg: Color32::from_rgb(0xcf, 0xe0, 0xfa),
            text_primary: Color32::from_rgb(0x1a, 0x1a, 0x1a),
            text_secondary: Color32::from_rgb(0x5a, 0x5a, 0x5a),
            border: Color32::from_rgb(0xd6, 0xd8, 0xdb),
            danger: Color32::from_rgb(0xc0, 0x39, 0x2b),
        },
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib theme::tests -- --nocapture`
Expected: PASS (all tests in `theme::tests` green).

- [ ] **Step 5: Commit**

```bash
git add src/theme.rs
git commit -m "feat: modernize theme palette with brighter accent and glass-ready dark tones"
```

---

### Task 2: Update spacing and shape in `theme_style()`

**Files:**
- Modify: `src/theme.rs:139-159` (the `theme_style()` function)

**Interfaces:**
- Consumes: nothing new (still returns `egui::Style`).
- Produces: `theme_style() -> Style` unchanged signature; callers in `app.rs`/`main.rs` (wherever it's applied via `ctx.set_style`) are unaffected since the signature doesn't change.

- [ ] **Step 1: Update `theme_style()` implementation**

Replace `src/theme.rs:139-159` with:

```rust
pub fn theme_style() -> Style {
    Style {
        spacing: egui::style::Spacing {
            item_spacing: Vec2::new(10.0, 10.0),
            button_padding: Vec2::new(10.0, 6.0),
            indent: 16.0,
            scroll: egui::style::ScrollStyle {
                bar_width: 6.0,
                ..Default::default()
            },
            window_margin: Margin::same(14),
            ..Default::default()
        },
        interaction: egui::style::Interaction {
            resize_grab_radius_side: 4.0,
            resize_grab_radius_corner: 4.0,
            ..Default::default()
        },
        ..Default::default()
    }
}
```

(Only `item_spacing`, `button_padding`, and `window_margin` values changed from the original; `indent`, `scroll`, and `interaction` are unchanged.)

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/theme.rs
git commit -m "feat: increase spacing and padding in theme_style for a roomier modern layout"
```

---

### Task 3: Add corner radius, shadows, and glass/flat visuals in `theme_visuals()`

**Files:**
- Modify: `src/theme.rs:68-136` (the `theme_visuals()` function)

**Interfaces:**
- Consumes: `ThemeColors` fields from Task 1 (`bg_dark`, `panel_bg`, `card_bg`, `hover_bg`, `accent`, `selected_bg`, `text_primary`, `text_secondary`, `border`, `danger`) — same names as before, only values changed.
- Produces: `theme_visuals(Theme) -> Visuals` unchanged signature.

- [ ] **Step 1: Update `theme_visuals()` implementation**

Replace `src/theme.rs:68-136` with:

```rust
pub fn theme_visuals(theme: Theme) -> Visuals {
    let c = palette(theme);
    let base = if theme == Theme::Dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };

    let window_shadow = if theme == Theme::Dark {
        egui::epaint::Shadow {
            offset: [0, 8],
            blur: 24,
            spread: 0,
            color: Color32::from_black_alpha(120),
        }
    } else {
        egui::epaint::Shadow {
            offset: [0, 4],
            blur: 10,
            spread: 0,
            color: Color32::from_black_alpha(40),
        }
    };

    Visuals {
        dark_mode: theme == Theme::Dark,
        override_text_color: Some(c.text_primary),
        window_corner_radius: CornerRadius::same(10),
        window_stroke: Stroke::new(1.0, c.border),
        window_fill: c.panel_bg,
        panel_fill: c.panel_bg,
        window_shadow,
        popup_shadow: window_shadow,
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
                corner_radius: CornerRadius::same(8),
                fg_stroke: Stroke::new(1.0, c.text_secondary),
                expansion: 0.0,
            },
            inactive: egui::style::WidgetVisuals {
                bg_fill: c.panel_bg,
                weak_bg_fill: c.card_bg,
                bg_stroke: Stroke::new(1.0, c.border),
                corner_radius: CornerRadius::same(8),
                fg_stroke: Stroke::new(1.0, c.text_primary),
                expansion: 0.0,
            },
            hovered: egui::style::WidgetVisuals {
                bg_fill: c.hover_bg,
                weak_bg_fill: c.hover_bg,
                bg_stroke: Stroke::new(1.0, c.accent),
                corner_radius: CornerRadius::same(8),
                fg_stroke: Stroke::new(1.2, c.accent),
                expansion: 0.5,
            },
            active: egui::style::WidgetVisuals {
                bg_fill: c.selected_bg,
                weak_bg_fill: c.selected_bg,
                bg_stroke: Stroke::new(1.0, c.accent),
                corner_radius: CornerRadius::same(8),
                fg_stroke: Stroke::new(2.0, c.accent),
                expansion: 1.0,
            },
            open: egui::style::WidgetVisuals {
                bg_fill: c.card_bg,
                weak_bg_fill: c.card_bg,
                bg_stroke: Stroke::new(1.0, c.border),
                corner_radius: CornerRadius::same(8),
                fg_stroke: Stroke::new(1.0, c.text_primary),
                expansion: 0.0,
            },
        },
        ..base
    }
}
```

- [ ] **Step 2: Run full test suite and check compilation**

Run: `cargo test --lib`
Expected: PASS, all `theme::tests` green, no compile warnings about unused variables.

- [ ] **Step 3: Commit**

```bash
git add src/theme.rs
git commit -m "feat: add layered shadows and rounder widget corners to theme visuals"
```

---

### Task 4: Manual visual verification

**Files:** none (manual run only)

- [ ] **Step 1: Build and run the app**

Run: `cargo run`
Expected: app launches without panics.

- [ ] **Step 2: Verify dark theme**

In the running app, ensure the theme is set to Dark (default). Check:
- Panels/buttons show visibly rounded corners (8-10px, not the old sharp 4-6px).
- Hovering a button shows an accent-blue border glow.
- Selecting a thumbnail/item shows an accent-tinted highlight (not flat navy).
- Menus/popups show a soft shadow beneath them.

- [ ] **Step 3: Verify light theme**

Use the View menu to switch to Light theme. Check:
- Backgrounds are crisp white/near-white, no dark leftovers.
- Borders are subtle gray, shadows are small and light (not heavy black).
- Accent color reads as a clear blue (`#1F6FE5`) on buttons/links/selection.

- [ ] **Step 4: Report result**

If any visual regression is found (e.g., unreadable text, invisible borders), note the specific widget/state and fix the corresponding color/stroke in `src/theme.rs` before considering this task done. Otherwise, no commit needed for this task (verification only).

---

## Self-Review Notes

- Spec coverage: palette (Task 1), spacing/corner radius (Task 2), shadows/glass depth/hover-active states (Task 3), manual verification (Task 4) — all spec sections covered.
- `CornerRadius::ZERO` literals in `viewer.rs` are untouched — confirmed out of scope, no task modifies `viewer.rs` or `app.rs`.
- Type/signature consistency: `palette`, `theme_style`, `theme_visuals` keep their original signatures across all tasks; `ThemeColors` field names unchanged so Task 3 compiles against Task 1's struct.
