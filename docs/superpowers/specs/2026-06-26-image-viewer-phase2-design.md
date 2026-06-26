# Image Viewer — Phase 2 Design Spec: Editing

## Overview
Add image editing capabilities to the viewer: crop, rotate/flip, resize, format conversion, and undo/redo. Each operation is non-destructive (applied to a copy of the original image), accessible from the viewer toolbar.

## Architecture

### Module Structure
```
src/editor/
  mod.rs       — editor state, panel UI, edit mode integration
  operations.rs — EditOp enum + apply() function for each operation
  panel.rs     — right-side editing panel widget
```

### EditOp Enum
```rust
pub enum EditOp {
    Crop { x: u32, y: u32, width: u32, height: u32 },
    Rotate180,
    Rotate90Cw,
    Rotate90Ccw,
    FlipHorizontal,
    FlipVertical,
    Resize { width: u32, height: u32 },
    NoOp,
}
```

### Undo/Redo
- `image_edit_stack: Vec<(EditOp, DynamicImage)>` on `editor::State`
- Before applying any edit, push the current **pre-edit** image onto the stack (with the op description)
- Undo pops the last entry, restoring the previous image
- Redo pushes it back (use a second vec for redo)
- Limit stack to 50 entries

### Format Conversion
- New "Save As" dialog in viewer toolbar / file menu
- Dropdown for format: PNG, JPEG, BMP, WEBP
- On save, re-encode current image into chosen format using `image` crate's encoder
- JPEG quality slider (1-100)

### Data Flow
```
Original image (loaded via image_loader)
  → User initiates edit
  → Push current image to undo stack
  → Apply operation (returns new DynamicImage)
  → Re-encode to egui texture via image_loader::load_to_texture
  → Display updated image
```

## UI Layout
- **Edit button** in viewer toolbar — opens right-side panel
- **Panel:** vertically stacked operation buttons/sliders/inputs
  - Crop mode: toggle on → click-drag on image to select region → confirm
  - Rotate: 4 buttons (90° CW, 90° CCW, 180°, Flip H, Flip V)
  - Resize: width/height number inputs + aspect ratio lock checkbox + Apply
  - Undo / Redo buttons with shortcut indicators (Ctrl+Z / Ctrl+Shift+Z)
  - Save As button with format dropdown
- **Close (X)** button on panel to dismiss

## Out of Scope (Phase 2)
- Color adjustments (brightness, contrast, saturation) — Phase 3
- Red-eye removal
- Batch editing
- Layers / non-destructive editing beyond undo stack
- Selection tools beyond crop rectangle
