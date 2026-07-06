# Modern UI Restyle Design

## Goal
Modernize the visual style of the image viewer's egui theme (`src/theme.rs`) across both dark and light modes, without changing layout/structure of `app.rs` or `viewer.rs`.

## Style Direction
- **Dark theme**: "Dark Glass" — deep layered backgrounds, semi-transparent panel/card fills to simulate glass depth, soft accent glow on hover/active borders, soft drop shadows on floating panels/windows.
- **Light theme**: Clean flat modern — crisp near-white backgrounds, no transparency tricks, subtle gray borders, tighter/lighter shadows.
- **Accent color**: Keep blue family but brighten/increase saturation.
  - Dark: `#5AA9FF`
  - Light: `#1F6FE5`

## Palette Changes (`palette()`)
Both `Theme::Dark` and `Theme::Light` `ThemeColors` get updated values:
- `accent` updated per above.
- Dark: base backgrounds shift slightly deeper (`bg_dark` ~`#141414`, `panel_bg` ~`#1c1c1c`, `card_bg` ~`#242424`, `hover_bg` ~`#2e2e2e`), `selected_bg` becomes accent-tinted at moderate alpha rather than flat solid.
- Light: backgrounds stay near-white/light-gray but with refined steps for clearer layering (`bg_dark` white, `panel_bg` `#f7f7f8`, `card_bg` `#eef0f2`, `hover_bg` `#e2e5e9`).
- `danger`/`text_primary`/`text_secondary`/`border` get minor tuning for contrast consistency with new accent, not a full repaint.

## Shape & Spacing (`theme_style()`)
- `window_margin`: 12 → 14
- `item_spacing`: (8,8) → (10,10)
- `button_padding`: (8,4) → (10,6)
- Widget corner radius: 4 → 8 (all widget states)
- Window/panel corner radius: 6 → 10

Existing literal `CornerRadius::ZERO` usages in `viewer.rs` (checkerboard cells, image border, thumbnail-adjacent drawing) are intentional pixel-perfect drawing and are left unchanged.

## Depth Effects (`theme_visuals()`)
- Add `egui::epaint::Shadow` configuration for `window_shadow` / `popup_shadow`:
  - Dark: larger, softer, higher-alpha black shadow (offset small, blur ~16, alpha ~90).
  - Light: smaller, lower-alpha shadow (blur ~8, alpha ~40).
- Dark-only: `panel_fill`, `card_bg`-derived fills use alpha (~235/255) instead of fully opaque, to read as layered glass. Light theme fills stay fully opaque.
- Hover state: border stroke uses accent color at slightly reduced width/expansion (softer glow feel) instead of the current stronger 1.5px/expansion 1.0.
- Active/selected state: `selected_bg` in dark mode becomes accent color at low alpha (glow tint) rather than a flat navy block; light mode keeps a light solid tint (flat style, no transparency).

## Out of Scope
- No changes to `app.rs` / `viewer.rs` layout, widget structure, or menu logic.
- No new dependencies (no blur libraries — egui doesn't support real backdrop blur; "glass" is simulated via alpha layering + shadow + border glow).
- Existing tests in `theme.rs` (`test_palette_dark_matches_old_constants`, etc.) will need updated expected values since palette constants change — update assertions to match new palette rather than removing coverage.

## Testing
- Update/extend `theme.rs` unit tests to assert new color values and that dark/light palettes still differ sufficiently.
- Manual verification: run the app (`cargo run`), toggle theme via View menu, confirm panels, buttons, thumbnails, hover/selection states render correctly in both themes.
