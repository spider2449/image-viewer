# Editor Image Tools Implementation Plan

**Date:** 2026-08-19
**Scope:** Expand the existing single-image editor into a practical general-purpose image adjustment workflow while preserving the current architecture and history model.

## Goals

- Add previewable color and tone controls: exposure, brightness, contrast, saturation, hue, temperature, tint, and gamma.
- Add common one-click corrections and effects: auto contrast, grayscale, sepia, invert, blur, and sharpen.
- Expand resize with exact, fit, and fill modes; selectable resampling; percentages; and common size presets.
- Keep every committed edit compatible with undo/redo and Save As.
- Preserve alpha while applying pixel-level color operations.

## Design

1. Add typed adjustment, resize-mode, and resize-filter values to `editor::operations` so the processing code remains independent of egui.
2. Implement color adjustment as one pixel pass followed by hue rotation, keeping alpha unchanged and clamping output channels.
3. Preview color controls against the current committed image. Slider movement only replaces the viewer texture; Apply creates exactly one history entry, while Reset restores the committed texture.
4. Keep effect controls explicit: configure blur/sharpen parameters, then commit one operation.
5. Add resize presets and percentage helpers to the UI, with aspect-aware dimensions and clearly defined exact/fit/fill behavior.
6. Reset pending preview controls whenever the underlying committed image changes through load, edit, undo, redo, or paste.
7. Add focused unit tests for dimensions, resize modes, alpha preservation, neutral adjustments, tone changes, automatic contrast, and effects.

## Verification

```powershell
cargo fmt --check
cargo test editor
cargo test
cargo check
```

The project-defined required gate is `cargo check`; the tests provide regression coverage for the added processing behavior.

## Result

- Implemented all planned adjustment, effect, resize, preview, and history behavior.
- Added processing and history regression tests.
- Verified the final implementation with the required `cargo check` gate and the complete test suite.
