# Annotation Drawing — Design Spec (2026-10-14)

## 1. Background & Goal

Add FastStone-Capture-like basic drawing/annotation to `image-viewer`.
Reference: https://www.faststone.org/FSCaptureDetail.htm (Draw annotation
objects: callouts, text, arrowed lines, highlights, rectangles, circles,
step numbers, blur, magnifier, etc.)

User decisions:
- Scope: full annotation set, delivered in two phases under one spec.
- Model: vector object layer, re-editable within the session
  (select / move / restyle / delete).
- Persistence: burn into the exported image on Apply/Save; NO sidecar
  annotation file (no FSC equivalent).

## 2. Architecture (chosen: Approach A)

New `src/annotate/` module with dual rendering:

- Interactive rendering: egui `Painter` overlay on top of the viewer image.
- Burn rendering: CPU rasterizer producing a `DynamicImage` for Apply/Save.

```
viewer.rs (overlay + mouse) ──image-px coords──> annotate::AnnotateState
editor/mod.rs (Annotate panel) ──tool/style──> annotate::AnnotateState
annotate::render::paint_shapes ──egui overlay──> screen
annotate::render::burn ──DynamicImage──> EditOp::Replace ──> existing
    undo / has_unsaved_changes / save_as pipeline (zero changes there)
```

All shape coordinates are stored in **image-pixel space**, independent of
zoom/pan. The existing `viewer.rs` helpers `screen_to_image_pos` /
`image_to_screen_pos` are reused for conversion.

New dependencies: `imageproc` (geometry + mosaic/blur region) and
`ab_glyph` (text rasterization for burn; reuses CJK font path from
`font_loader.rs`).

## 3. Components

### 3.1 `src/annotate/mod.rs` — `AnnotateState`

```rust
pub struct AnnotateState {
    pub active: bool,              // annotate mode on/off (mutually exclusive with crop_active)
    pub tool: Tool,                // Select | Line | Arrow | Rect | Ellipse | Pencil | Highlight | Text | BlurRegion (+ Phase 2)
    pub shapes: Vec<Shape>,
    pub selected: Vec<usize>,      // indices into shapes (single-select in Phase 1, multi ok if cheap)
    pub style: Style,              // color, width, fill, opacity, font_size
    pub draft: Option<Draft>,      // in-progress drag (start + current in image px)
}
```

- `AnnotateState` lives as a field on `editor::State`
(`pub annotate: AnnotateState`), so `load_image` / `discard_changes` reset
it alongside history and adjustments.
- `Tool` and `Style` are plain data; the panel edits `style`, the viewer
  reads `tool` + `style` when creating shapes on drag-end.
- Single-select in Phase 1 (`selected` holds at most one index).

### 3.2 `src/annotate/shapes.rs` — `Shape` + `Style`

```rust
pub struct Style { pub color: egui::Color32, pub width: f32, pub fill: bool, pub opacity: f32, pub font_size: f32 }
pub enum Shape {
    Line { a: Pos, b: Pos, style: Style },
    Arrow { a: Pos, b: Pos, style: Style },
    Pencil { points: Vec<Pos>, style: Style },
    Rect { rect: ImgRect, style: Style },
    Ellipse { rect: ImgRect, style: Style },
    Highlight { rect: ImgRect, style: Style },  // translucent overlay (any color); freehand highlight = Pencil with low opacity
    Text { at: Pos, text: String, style: Style },
    BlurRegion { rect: ImgRect, kind: BlurKind }, // Blur | Mosaic
    // Phase 2: StepNumber, Callout, Magnifier, shadow/outline, dash styles, stamps
}
```

Positions in `f32` image pixels. Hit-test helpers (`hit(shape, pos,
tolerance)`) for selection, plus move-by-delta.

### 3.3 `src/annotate/render.rs` — dual rendering

- `paint_shapes(painter, shapes, selected, img_to_screen_fn)`: egui lines /
  rects / ellipses / arrowheads / text galley / translucent highlight /
  selection outline + handles.
- `burn(base: &DynamicImage, shapes: &[Shape]) -> DynamicImage`: CPU
  rasterization via `imageproc` (+ `ab_glyph` for text). BlurRegion uses
  existing `image::blur` on the cropped region (consistent with current
  Blur op); Mosaic = downscale/upscale with Nearest.
- Text burn: resolve font via `font_loader` path; fallback to embedded
  default if CJK font missing. Font size maps px ≈ `font_size` at 1:1.

### 3.4 `viewer.rs` changes

- After the crop-drag block, add annotate-drag handling on the same
  `viewer_image_drag` interact rect:
  - If `annotate.active` and tool != Select: drag-start sets `draft`,
    drag-update extends it, drag-end pushes a `Shape` (with min-size guard).
  - If tool == Select: drag on a selected shape moves it (image-px delta =
    screen delta / zoom); click selects via hit-test (topmost first).
- While `annotate.active`, panning is disabled (same pattern as
  `crop_active`); `crop_active` and `annotate.active` are mutually exclusive.
- Paint order in `draw_image`: image → crop overlay (existing) → annotate
  overlay (`paint_shapes`).

### 3.5 `editor/mod.rs` changes

- New "Annotate" collapsing section: tool buttons, color picker, width /
  opacity / font-size sliders, fill checkbox, object list with delete,
  "Apply" (burn → single `EditOp::Replace`, reuses `apply_op` so undo,
  resize sync, texture upload all work) and "Clear".
- `load_image` clears `annotate_state`; `discard_changes` clears it too.

### 3.6 `operations.rs`

- No new op needed: burn result is committed via existing
  `EditOp::Replace(Arc<DynamicImage>)`. This keeps `rebuild_current`,
  `has_unsaved_changes`, and `save_as` untouched.

## 4. Data flow

1. User toggles Annotate in editor panel → `annotate.active = true`.
2. Drag on image → image-px shape draft → mouse-up commits `Shape`.
3. Each frame: egui overlay paints shapes via `img_to_screen` mapping.
4. Apply: `burn(current_image, shapes)` → `apply_op(Replace)` →
   `sync_editor_image` → shapes cleared (burned).
5. Save: existing `save_as` path; annotations already burned, so output
   formats (png/jpeg/bmp/webp) need no changes.

## 5. Error handling

- Empty/drag-too-small shapes are dropped with no state change.
- Text with empty string is not created.
- Missing CJK font for burn: fall back to a built-in scalable default;
  overlay text (egui) always works regardless.
- `imageproc` failures (out-of-bounds rects): clamp rects to image bounds,
  never panic.

## 6. Phasing

- Phase 1: Select/move/delete, Line, Arrow, Rect, Ellipse (hollow+filled),
  Pencil, Highlighter, Text, Blur/Mosaic region, style controls.
- Phase 2: StepNumber, Callout/curved text, Magnifier, shadow/outline,
  dash styles, emoji/stamp. (Same module, additive enum variants.)

## 7. Testing

- Unit: image↔screen round-trip at various zooms; each Phase-1 shape burns
  without changing dimensions and changes at least one pixel; empty draft
  produces no shape; Apply pushes exactly one undo step and undo restores
  pixels.
- Verification: `cargo check`, `cargo test`, `cargo build --release`.
- Manual: draw each tool at 25%/100%/400% zoom, move selected shape,
  Apply → Undo → Save As PNG/JPEG.

## 8. Out of scope

- Sidecar/FSC-like annotation persistence (explicitly rejected).
- Freehand/region screen capture, scrolling capture, video recorder.
- Crop-drag already exists; no changes to crop behavior besides mutual
  exclusion with annotate mode.
