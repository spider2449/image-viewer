# Image Viewer — Visual Polish Design Spec

## Overview
Refine the entire GUI with a consistent visual theme, improved colors, spacing, and per-module styling. No new features, panels, or behavior changes — pure visual polish.

## Theme System (`src/theme.rs`)
New module providing centralized color palette and pre-built egui visuals/style.

**Color palette:**
| Role | Hex | Usage |
|------|-----|-------|
| Background | `#1a1a1a` | Window background |
| Panel bg | `#222222` | Side panels, toolbars, status bar |
| Card bg | `#2a2a2a` | Thumbnail cards, list rows |
| Hover | `#353535` | Hover state on items |
| Accent | `#4a9eff` | Buttons, selection, active elements |
| Selected bg | `#2d5a8e` | Selection highlight |
| Text primary | `#e0e0e0` | Main labels |
| Text secondary | `#888888` | Subtext, metadata |
| Border | `#3a3a3a` | Panel borders, dividers |
| Danger | `#e74c3c` | Destructive actions |
| Success | `#2ecc71` | Success indicators |

**Style overrides:**
- Window padding: 12px
- Rounding: 4px on panels, 6px on buttons, 2px on inputs
- Spacing: 8px item spacing, 4px button padding
- Scrollbar: 6px wide, rounded thumb, accent hover color

The theme is applied once in `app.rs` during initialization via `ctx.set_visuals()` and `ctx.set_style()`.

## Per-Module Visual Refinements

### Menu Bar (`app.rs`)
- Subtle bottom border using accent color (1px stroke)
- Menu items inherit accent hover color
- Separator between menus for visual grouping

### Browser Mode

**Folder tree (`browser/tree.rs`):**
- Folder icon prefix (📁 `U+1F4C1`) before each directory name
- Selected folder highlighted with accent background pill (`Frame` with accent fill)
- Hover highlight on rows
- Indentation lines via painter (subtle vertical lines at each depth level)

**Thumbnail grid (`browser/grid.rs`):**
- Card-style thumbnails: card bg (`#2a2a2a`), subtle border (`#3a3a3a`), 4px corner radius
- Shadow effect via painter: semi-transparent black rect offset 2px below card
- Hover effect: border changes to accent color on hover
- Selection: accent border (2px) + subtle glow (larger semi-transparent accent rect behind card)
- Filename label: single-line truncation with `…`, secondary text color, 11px font
- Loading state: centered subtle spinner text (`…` in accent color)
- Error state: centered ✖ in muted red

**List view (`browser/grid.rs`):**
- Column headers styled with strong text, bottom accent underline
- Alternating row backgrounds (`#222` / `#2a2a2a`)
- Hover highlight on rows
- Selection: accent background on entire row
- Size/date columns right-aligned, monospace font

**Browser toolbar (`browser/grid.rs`):**
- Grouped controls with visual separators: Navigation | Sort | View | Refresh
- Back/Up buttons styled as icon-only with tooltips
- Sort dropdown and direction toggle consistently styled
- File count right-aligned, secondary text color

### Viewer Mode

**Viewer toolbar (`viewer.rs`):**
- Icon-style buttons with consistent sizing (28px height)
- Tooltips on all buttons
- Groups: Navigation (Browser, Prev, Next) | Zoom (Fit, 1:1, slider) | Display (Info, Exif, Edit) | Slideshow | Fullscreen
- File counter and name right-aligned, secondary text

**Image area (`viewer.rs`):**
- Checkerboard alpha background pattern (16px alternating light/dark gray squares) when image has transparency
- Subtle inner border (1px `#3a3a3a`) around image area
- Info overlay: semi-transparent dark background (`rgba(0,0,0,0.7)`), rounded corners (4px), 8px padding, positioned top-left with margin

**Status bar (`viewer.rs`):**
- Smaller font (12px), secondary text color
- Compact format: `Zoom: 100% | 1920x1080 | 2.3 MB`

### Editor Panel (`editor/mod.rs`)
- Section headers ("Edit", "Transform", "Resize", "Save As") with accent underline (2px)
- Collapsible sections via `collapsing` headers
- Consistent button width (fill available)
- Undo/Redo buttons grouped with close button right-aligned
- Crop toggle styled as selectable button with accent when active

### EXIF Panel (`exif.rs`)
- Key-value table layout with proper alignment
- Keys right-aligned in accent color with `:` suffix
- Values left-aligned in primary text color
- Alternating row backgrounds for readability
- "No EXIF data" message centered in secondary text

### Batch Modal (`batch/mod.rs`)
- Mode tabs styled as accent-colored segmented control
- File list with styled checkboxes, consistent spacing
- Apply button filled with accent color
- Log area with monospace font, muted background

## Icons
- Continue using unicode/emoji characters for icons
- All icons wrapped in `egui::RichText` with consistent size (14px) and accent color
- Helper function `theme::icon(codepoint)` returning `RichText` with standard styling
- Tooltips added to all icon buttons

## Implementation Plan

### Files to create:
- `src/theme.rs` — theme module

### Files to modify:
- `src/main.rs` — add `mod theme;`
- `src/app.rs` — apply theme on init, menu bar styling
- `src/browser/mod.rs` — panel styling
- `src/browser/tree.rs` — tree node visual refinements
- `src/browser/grid.rs` — thumbnail card styling, list view refinements, toolbar styling
- `src/viewer.rs` — toolbar styling, checkerboard background, info overlay, status bar
- `src/editor/mod.rs` — section headers, collapsible groups, button styling
- `src/exif.rs` — table layout, alternating rows
- `src/batch/mod.rs` — modal styling, tab styling, apply button

## Out of Scope
- Animations or transitions
- Custom window decorations / title bar
- Adding or removing any feature or panel
- Font changes (CJK font loading already handled)
- Dependencies — no new crates added
