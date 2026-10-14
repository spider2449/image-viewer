# Annotation Drawing (Phase 1) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add FastStone-like vector annotation drawing (Phase 1 toolset) to image-viewer with session-editable shapes and burn-on-Apply.

**Architecture:** New `src/annotate/` module (state + shapes + dual render). Interactive shapes paint as an egui overlay in `viewer.rs`; Apply burns them to pixels via `imageproc` and commits through the existing `EditOp::Replace` pipeline, so undo / unsaved-dialog / save need no changes.

**Tech Stack:** Rust 2021, egui/eframe 0.31, image 0.25, imageproc 0.25, ab_glyph 0.2.

**Spec:** `docs/superpowers/specs/2026-10-14-annotation-drawing-design.md` — this plan implements Phase 1 only. Phase 2 (StepNumber, Callout, Magnifier, shadow/outline, dash styles, stamps) gets its own follow-up plan.

## Global Constraints

- All code and code comments entirely in English (repo rule).
- `cargo check` is the verification that matters; `cargo test` must stay green (11 existing tests).
- Bump the patch version in `Cargo.toml` by 1 as part of every commit in this plan (repo manual versioning convention).
- Shapes store coordinates in image-pixel `f32` space, never screen space.
- Never panic on out-of-bounds geometry: clamp rects to image bounds.
- `crop_active` and annotate mode are mutually exclusive.

---

## File Structure

- Create `src/annotate/mod.rs` — `Tool`, `Draft`, `AnnotateState`, `draft_ready()`, `commit_draft()`. Single owner of annotation session state.
- Create `src/annotate/shapes.rs` — `Pos`/`ImgRect`/`Style`/`Shape`/`BlurKind`, geometry helpers (`normalize`, `clamp`, `hit_test`, `translate`, `text_bounds`). Pure logic, no egui painting, no image mutation.
- Create `src/annotate/render.rs` — `paint_shapes()` + `paint_draft()` (egui overlay), `arrow_head_points()` pure helper, `burn()` (CPU raster via imageproc), `load_burn_font()`.
- Modify `src/main.rs` — add `mod annotate;`.
- Modify `src/editor/mod.rs` — add `annotate: AnnotateState` field, init in `new()`, reset in `load_image()`/`discard_changes()`, include shapes in `has_unsaved_changes()`, add Annotate panel section with Apply/Clear.
- Modify `src/viewer.rs` — annotate drag handling + overlay paint call + pan suppression while annotating.
- Modify `src/font_loader.rs` — expose `cjk_font_bytes() -> Option<Vec<u8>>` alongside existing `load_cjk_font()`.
- Modify `Cargo.toml` — add `imageproc = "0.25"`, add `ab_glyph = "0.2"`.

---

### Task 1: Dependencies

**Files:**
- Modify: `Cargo.toml:7-17`
- Test: `cargo check` (no unit test; compile is the test)

**Interfaces:**
- Consumes: nothing
- Produces: `imageproc::drawing` + `ab_glyph::{Font, FontArc}` available to later tasks

- [ ] **Step 1: Add dependencies and bump version**

```toml
imageproc = "0.25"
ab_glyph = "0.2"
```

Append after the `image = {...}` line. Bump `version = "0.1.67"` patch by 1.

- [ ] **Step 2: Verify resolution**

Run: `cargo check`
Expected: PASS (new deps compile; imageproc 0.25.x pairs with image 0.25).

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "feat: add imageproc and ab_glyph deps for annotation burn"
```

---

### Task 2: Annotation state (`annotate/mod.rs`)

**Files:**
- Create: `src/annotate/mod.rs`
- Modify: `src/main.rs:1-16` (add `mod annotate;`), `src/editor/mod.rs:10-33` (add field `pub annotate: crate::annotate::AnnotateState`), `src/editor/mod.rs:36-61` (init `annotate: crate::annotate::AnnotateState::new()` in `new()`)
- Test: unit tests inside `src/annotate/mod.rs`

**Interfaces:**
- Consumes: `shapes::{Shape, Style}` (Task 3 defines the enum; if implementing strictly in order, write Task 3 first — or stub the enum in this task and flesh it out in Task 3. Recommended order: Task 3 before Task 2.)
- Produces: `AnnotateState::new()`, `AnnotateState::clear()`, `Tool` (default `Select`), `Draft`, `draft_ready(&Draft) -> bool`, `commit_draft(&mut AnnotateState)` — used by Tasks 7 (viewer) and 8 (panel).

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn tiny_rect_drag_is_dropped() {
    let mut s = AnnotateState::new();
    s.active = true;
    s.tool = Tool::Rect;
    s.draft = Some(Draft {
        tool: Tool::Rect,
        start: egui::pos2(10.0, 10.0),
        current: egui::pos2(11.0, 11.0),
        points: vec![],
    });
    commit_draft(&mut s);
    assert!(s.shapes.is_empty());
    assert!(s.draft.is_none());
}

#[test]
fn rect_drag_commits_one_normalized_shape() {
    let mut s = AnnotateState::new();
    s.active = true;
    s.tool = Tool::Rect;
    s.draft = Some(Draft {
        tool: Tool::Rect,
        start: egui::pos2(50.0, 50.0),
        current: egui::pos2(10.0, 10.0),
        points: vec![],
    });
    commit_draft(&mut s);
    assert_eq!(s.shapes.len(), 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test annotate:: 2>&1 | tail -20`
Expected: FAIL with "no function or associated item named `commit_draft` found" (module does not exist yet).

- [ ] **Step 3: Write minimal implementation**

```rust
pub mod render;
pub mod shapes;

pub use shapes::{Shape, Style};

pub const MIN_DRAG_PX: f32 = 4.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tool {
    #[default]
    Select,
    Line,
    Arrow,
    Rect,
    Ellipse,
    Pencil,
    Highlight,
    Text,
    BlurRegion,
}

#[derive(Clone, Debug)]
pub struct Draft {
    pub tool: Tool,
    pub start: egui::Pos2,
    pub current: egui::Pos2,
    pub points: Vec<egui::Pos2>,
}

#[derive(Clone, Debug, Default)]
pub struct AnnotateState {
    pub active: bool,
    pub tool: Tool,
    pub shapes: Vec<Shape>,
    pub selected: Option<usize>,
    pub style: Style,
    pub draft: Option<Draft>,
    pub text_buffer: String,
}

impl AnnotateState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.shapes.clear();
        self.selected = None;
        self.draft = None;
        self.text_buffer.clear();
    }
}

// True when the in-progress drag is big enough to become a shape.
pub fn draft_ready(d: &Draft) -> bool {
    match d.tool {
        Tool::Pencil => d.points.len() >= 2,
        Tool::Text => true, // text commits from the panel input, not from drag size
        _ => (d.current - d.start).length() >= MIN_DRAG_PX,
    }
}

// Push the draft as a Shape using the current style, or drop it.
// Always clears the draft.
pub fn commit_draft(state: &mut AnnotateState) {
    let Some(d) = state.draft.take() else { return };
    if !state.active || !draft_ready(&d) {
        return;
    }
    let style = state.style;
    let shape = match d.tool {
        Tool::Select => return,
        Tool::Line => Shape::Line { a: d.start, b: d.current, style },
        Tool::Arrow => Shape::Arrow { a: d.start, b: d.current, style },
        Tool::Rect => Shape::Rect { rect: shapes::ImgRect::from_points(d.start, d.current), style },
        Tool::Ellipse => Shape::Ellipse { rect: shapes::ImgRect::from_points(d.start, d.current), style },
        Tool::Pencil => Shape::Pencil { points: d.points, style },
        Tool::Highlight => Shape::Highlight { rect: shapes::ImgRect::from_points(d.start, d.current), style },
        Tool::BlurRegion => Shape::BlurRegion { rect: shapes::ImgRect::from_points(d.start, d.current), kind: shapes::BlurKind::Blur },
        Tool::Text => {
            if state.text_buffer.trim().is_empty() {
                return;
            }
            Shape::Text { at: d.start, text: state.text_buffer.clone(), style }
        }
    };
    state.shapes.push(shape);
    state.selected = Some(state.shapes.len() - 1);
}
```

Wire `mod annotate;` in `main.rs` and the `annotate` field + init in `editor::State`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test annotate:: 2>&1 | tail -10`
Expected: PASS (2 tests). Then `cargo test 2>&1 | tail -5` — all existing tests still green.

- [ ] **Step 5: Commit (include patch version bump)**

```bash
git add src/annotate/mod.rs src/main.rs src/editor/mod.rs Cargo.toml
git commit -m "feat: add annotation session state and draft commit"
```

---

### Task 3: Shapes and geometry (`annotate/shapes.rs`)

**Files:**
- Create: `src/annotate/shapes.rs`
- Test: unit tests inside `src/annotate/shapes.rs`

**Interfaces:**
- Consumes: nothing (pure geometry over `egui::Pos2`/`egui::Vec2`/`egui::Color32`)
- Produces: `ImgRect::{from_points, normalize, clamp, width, height}`, `Style::{default, effective_color}`, `Shape::{bounding_rect, translate, hit_test}`, `text_bounds()`, `BlurKind` — used by Tasks 2, 4, 5.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn rect_from_reversed_points_normalizes() {
    let r = ImgRect::from_points(egui::pos2(50.0, 50.0), egui::pos2(10.0, 20.0));
    assert_eq!(r.min, egui::pos2(10.0, 20.0));
    assert_eq!(r.max, egui::pos2(50.0, 50.0));
}

#[test]
fn clamp_keeps_rect_inside_image() {
    let r = ImgRect { min: egui::pos2(-5.0, 10.0), max: egui::pos2(500.0, 60.0) };
    let c = r.clamp(100.0, 80.0);
    assert_eq!((c.min.x, c.min.y, c.max.x, c.max.y), (0.0, 10.0, 100.0, 60.0));
}

#[test]
fn line_hit_test_uses_tolerance() {
    let s = Shape::Line { a: egui::pos2(0.0, 0.0), b: egui::pos2(100.0, 0.0), style: Style::default() };
    assert!(s.hit_test(egui::pos2(50.0, 3.0), 5.0));
    assert!(!s.hit_test(egui::pos2(50.0, 30.0), 5.0));
}

#[test]
fn translate_moves_rect_shape() {
    let mut s = Shape::Rect {
        rect: ImgRect { min: egui::pos2(10.0, 10.0), max: egui::pos2(20.0, 20.0) },
        style: Style::default(),
    };
    s.translate(egui::vec2(5.0, -5.0));
    assert_eq!(s.bounding_rect().min, egui::pos2(15.0, 5.0));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test annotate::shapes 2>&1 | tail -10`
Expected: FAIL — module does not exist.

- [ ] **Step 3: Write minimal implementation**

```rust
use eframe::egui;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImgRect {
    pub min: egui::Pos2,
    pub max: egui::Pos2,
}

impl ImgRect {
    // Build from any two corners; result is always normalized.
    pub fn from_points(a: egui::Pos2, b: egui::Pos2) -> Self {
        Self {
            min: egui::pos2(a.x.min(b.x), a.y.min(b.y)),
            max: egui::pos2(a.x.max(b.x), a.y.max(b.y)),
        }
    }

    pub fn normalize(mut self) -> Self {
        let (lox, hix) = (self.min.x.min(self.max.x), self.min.x.max(self.max.x));
        let (loy, hiy) = (self.min.y.min(self.max.y), self.min.y.max(self.max.y));
        self.min = egui::pos2(lox, loy);
        self.max = egui::pos2(hix, hiy);
        self
    }

    // Clamp inside a (width, height) image; keeps at least 1px where possible.
    pub fn clamp(mut self, w: f32, h: f32) -> Self {
        self.min.x = self.min.x.clamp(0.0, w);
        self.min.y = self.min.y.clamp(0.0, h);
        self.max.x = self.max.x.clamp(0.0, w);
        self.max.y = self.max.y.clamp(0.0, h);
        self.normalize()
    }

    pub fn width(self) -> f32 {
        (self.max.x - self.min.x).max(0.0)
    }

    pub fn height(self) -> f32 {
        (self.max.y - self.min.y).max(0.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    pub color: egui::Color32,
    pub width: f32,     // stroke width in image px
    pub filled: bool,   // rect / ellipse fill
    pub opacity: f32,   // 0.0..=1.0
    pub font_size: f32, // image px
}

impl Default for Style {
    fn default() -> Self {
        Self {
            color: egui::Color32::RED,
            width: 3.0,
            filled: false,
            opacity: 1.0,
            font_size: 24.0,
        }
    }
}

impl Style {
    // Color with opacity applied to the alpha channel.
    pub fn effective_color(self) -> egui::Color32 {
        let a = (self.color.a() as f32 * self.opacity.clamp(0.0, 1.0)) as u8;
        egui::Color32::from_rgba_premultiplied(self.color.r(), self.color.g(), self.color.b(), a)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlurKind {
    Blur,
    Mosaic,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    Line { a: egui::Pos2, b: egui::Pos2, style: Style },
    Arrow { a: egui::Pos2, b: egui::Pos2, style: Style },
    Pencil { points: Vec<egui::Pos2>, style: Style },
    Rect { rect: ImgRect, style: Style },
    Ellipse { rect: ImgRect, style: Style },
    Highlight { rect: ImgRect, style: Style },
    Text { at: egui::Pos2, text: String, style: Style },
    BlurRegion { rect: ImgRect, kind: BlurKind },
}

// Approximate text bounds for hit-testing (monospace estimate).
pub fn text_bounds(at: egui::Pos2, text: &str, font_size: f32) -> ImgRect {
    let lines = text.lines().count().max(1) as f32;
    let widest = text.lines().map(|l| l.chars().count()).max().unwrap_or(0) as f32;
    ImgRect {
        min: at,
        max: egui::pos2(at.x + widest * font_size * 0.6, at.y + lines * font_size * 1.2),
    }
}

fn dist_point_segment(p: egui::Pos2, a: egui::Pos2, b: egui::Pos2) -> f32 {
    let ab = b - a;
    let denom = ab.length_sq().max(1e-6);
    let t = ((p - a).dot(ab) / denom).clamp(0.0, 1.0);
    (a + ab * t - p).length()
}

impl Shape {
    pub fn bounding_rect(&self) -> ImgRect {
        match self {
            Shape::Line { a, b, .. } | Shape::Arrow { a, b, .. } => ImgRect::from_points(*a, *b),
            Shape::Pencil { points, .. } => {
                let mut r = ImgRect::from_points(points[0], points[0]);
                for p in &points[1..] {
                    r.min.x = r.min.x.min(p.x);
                    r.min.y = r.min.y.min(p.y);
                    r.max.x = r.max.x.max(p.x);
                    r.max.y = r.max.y.max(p.y);
                }
                r
            }
            Shape::Rect { rect, .. } | Shape::Ellipse { rect, .. }
            | Shape::Highlight { rect, .. } | Shape::BlurRegion { rect, .. } => *rect,
            Shape::Text { at, text, style } => text_bounds(*at, text, style.font_size),
        }
    }

    pub fn translate(&mut self, delta: egui::Vec2) {
        let mv = |p: &mut egui::Pos2| *p += delta;
        match self {
            Shape::Line { a, b, .. } | Shape::Arrow { a, b, .. } => { mv(a); mv(b); }
            Shape::Pencil { points, .. } => points.iter_mut().for_each(mv),
            Shape::Rect { rect, .. } | Shape::Ellipse { rect, .. }
            | Shape::Highlight { rect, .. } | Shape::BlurRegion { rect, .. } => {
                rect.min += delta;
                rect.max += delta;
            }
            Shape::Text { at, .. } => mv(at),
        }
    }

    // Selection hit test in image-pixel space; tolerance is in image px.
    pub fn hit_test(&self, pos: egui::Pos2, tolerance: f32) -> bool {
        match self {
            Shape::Line { a, b, .. } | Shape::Arrow { a, b, .. } => {
                dist_point_segment(pos, *a, *b) <= tolerance
            }
            Shape::Pencil { points, .. } => points
                .windows(2)
                .any(|w| dist_point_segment(pos, w[0], w[1]) <= tolerance),
            Shape::Rect { rect, .. } | Shape::Highlight { rect, .. } | Shape::BlurRegion { rect, .. } => {
                pos.x >= rect.min.x - tolerance && pos.x <= rect.max.x + tolerance
                    && pos.y >= rect.min.y - tolerance && pos.y <= rect.max.y + tolerance
            }
            Shape::Ellipse { rect, .. } => {
                let c = egui::pos2((rect.min.x + rect.max.x) / 2.0, (rect.min.y + rect.max.y) / 2.0);
                let rx = (rect.width() / 2.0 + tolerance).max(1e-3);
                let ry = (rect.height() / 2.0 + tolerance).max(1e-3);
                let dx = (pos.x - c.x) / rx;
                let dy = (pos.y - c.y) / ry;
                dx * dx + dy * dy <= 1.0
            }
            Shape::Text { at, text, style } => {
                let b = text_bounds(*at, text, style.font_size);
                pos.x >= b.min.x - tolerance && pos.x <= b.max.x + tolerance
                    && pos.y >= b.min.y - tolerance && pos.y <= b.max.y + tolerance
            }
        }
    }
}
```

Note: `Pencil` with fewer than 2 points never reaches here (`draft_ready` drops it), but `bounding_rect` indexes `points[0]` — keep the `draft_ready` guard as the invariant and note it in a comment.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test annotate:: 2>&1 | tail -10`
Expected: PASS (all Task 2 + Task 3 tests).

- [ ] **Step 5: Commit (include patch version bump)**

```bash
git add src/annotate/shapes.rs src/annotate/mod.rs Cargo.toml
git commit -m "feat: add annotation shapes and geometry helpers"
```

---

### Task 4: Overlay painting (`render.rs` part 1)

**Files:**
- Create: `src/annotate/render.rs` (paint functions + `arrow_head_points` in this task; `burn` comes in Task 5)
- Test: unit test for `arrow_head_points` inside `src/annotate/render.rs`

**Interfaces:**
- Consumes: `shapes::{Shape, Style, ImgRect}`, `egui::Painter`
- Produces: `paint_shapes(painter, shapes, selected, to_screen)`, `paint_draft(painter, draft, style, to_screen)`, `arrow_head_points(tip, dir, size)` — used by Task 7.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn arrowhead_wings_sit_behind_tip() {
    let wings = arrow_head_points(egui::pos2(10.0, 0.0), egui::vec2(1.0, 0.0), 6.0);
    assert_eq!(wings, [egui::pos2(4.0, 3.0), egui::pos2(4.0, -3.0)]);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test annotate::render 2>&1 | tail -10`
Expected: FAIL — module does not exist.

- [ ] **Step 3: Write minimal implementation**

```rust
use super::shapes::{Shape, Style};
use eframe::egui;

// Two wing tips of an arrowhead: `dir` is the unit flight direction.
pub fn arrow_head_points(tip: egui::Pos2, dir: egui::Vec2, size: f32) -> [egui::Pos2; 2] {
    let base = tip - dir * size;
    let perp = egui::vec2(-dir.y, dir.x) * size * 0.5;
    [base + perp, base - perp]
}

fn stroke(style: Style, zoom: f32) -> egui::Stroke {
    egui::Stroke::new((style.width * zoom).max(1.0), style.effective_color())
}

// Paint all shapes; `to_screen` maps image px -> screen px.
pub fn paint_shapes(
    painter: &egui::Painter,
    shapes: &[Shape],
    selected: Option<usize>,
    zoom: f32,
    to_screen: impl Fn(egui::Pos2) -> egui::Pos2,
) {
    for (i, shape) in shapes.iter().enumerate() {
        paint_shape(painter, shape, zoom, &to_screen);
        if selected == Some(i) {
            let b = shape.bounding_rect();
            painter.rect_stroke(
                egui::Rect::from_min_max(to_screen(b.min), to_screen(b.max)),
                egui::CornerRadius::ZERO,
                egui::Stroke::new(1.5, egui::Color32::WHITE),
                egui::StrokeKind::Outside,
            );
        }
    }
}

fn paint_shape(
    painter: &egui::Painter,
    shape: &Shape,
    zoom: f32,
    to_screen: &impl Fn(egui::Pos2) -> egui::Pos2,
) {
    match shape {
        Shape::Line { a, b, style } => {
            painter.line_segment([to_screen(*a), to_screen(*b)], stroke(*style, zoom));
        }
        Shape::Arrow { a, b, style } => {
            let (sa, sb) = (to_screen(*a), to_screen(*b));
            painter.line_segment([sa, sb], stroke(*style, zoom));
            let dir = (*b - *a).normalized().unwrap_or(egui::vec2(1.0, 0.0));
            let size = (style.width * 4.0 + 6.0) * zoom;
            let [w1, w2] = arrow_head_points(sb, dir, size);
            painter.line_segment([sb, w1], stroke(*style, zoom));
            painter.line_segment([sb, w2], stroke(*style, zoom));
        }
        Shape::Pencil { points, style } => {
            let pts: Vec<egui::Pos2> = points.iter().map(|p| to_screen(*p)).collect();
            painter.add(egui::Shape::line(pts, stroke(*style, zoom)));
        }
        Shape::Rect { rect, style } => {
            let r = egui::Rect::from_min_max(to_screen(rect.min), to_screen(rect.max));
            if style.filled {
                painter.rect_filled(r, egui::CornerRadius::ZERO, style.effective_color());
            } else {
                painter.rect_stroke(r, egui::CornerRadius::ZERO, stroke(*style, zoom), egui::StrokeKind::Inside);
            }
        }
        Shape::Ellipse { rect, style } => {
            let r = egui::Rect::from_min_max(to_screen(rect.min), to_screen(rect.max));
            if style.filled {
                painter.ellipse_filled(r.center(), r.size() / 2.0, style.effective_color());
            } else {
                painter.ellipse_stroke(r.center(), r.size() / 2.0, stroke(*style, zoom));
            }
        }
        Shape::Highlight { rect, style } => {
            let r = egui::Rect::from_min_max(to_screen(rect.min), to_screen(rect.max));
            let mut hl = *style;
            hl.opacity = (hl.opacity * 0.4).clamp(0.05, 0.6);
            painter.rect_filled(r, egui::CornerRadius::ZERO, hl.effective_color());
        }
        Shape::Text { at, text, style } => {
            painter.text(
                to_screen(*at),
                egui::Align2::LEFT_TOP,
                text,
                egui::FontId::proportional((style.font_size * zoom).max(4.0)),
                style.effective_color(),
            );
        }
        Shape::BlurRegion { rect, .. } => {
            let r = egui::Rect::from_min_max(to_screen(rect.min), to_screen(rect.max));
            painter.rect_filled(r, egui::CornerRadius::ZERO, egui::Color32::from_black_alpha(80));
            painter.rect_stroke(r, egui::CornerRadius::ZERO, egui::Stroke::new(1.0, egui::Color32::WHITE), egui::StrokeKind::Outside);
        }
    }
}

// Paint the in-progress drag as a preview shape.
pub fn paint_draft(
    painter: &egui::Painter,
    draft: &super::Draft,
    style: Style,
    zoom: f32,
    to_screen: impl Fn(egui::Pos2) -> egui::Pos2,
) {
    let preview = match draft.tool {
        super::Tool::Select => return,
        super::Tool::Line => Shape::Line { a: draft.start, b: draft.current, style },
        super::Tool::Arrow => Shape::Arrow { a: draft.start, b: draft.current, style },
        super::Tool::Rect => Shape::Rect { rect: super::shapes::ImgRect::from_points(draft.start, draft.current), style },
        super::Tool::Ellipse => Shape::Ellipse { rect: super::shapes::ImgRect::from_points(draft.start, draft.current), style },
        super::Tool::Pencil => Shape::Pencil { points: draft.points.clone(), style },
        super::Tool::Highlight => Shape::Highlight { rect: super::shapes::ImgRect::from_points(draft.start, draft.current), style },
        super::Tool::BlurRegion => Shape::BlurRegion { rect: super::shapes::ImgRect::from_points(draft.start, draft.current), kind: super::shapes::BlurKind::Blur },
        super::Tool::Text => return, // text previews via the panel input, not drag
    };
    paint_shape(painter, &preview, zoom, &to_screen);
}
```

Note: `egui::Vec2::normalized()` returns `Option<Vec2>` in egui 0.31 — the `unwrap_or` above handles the zero-length case.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test annotate:: 2>&1 | tail -5` then `cargo check 2>&1 | tail -3`
Expected: PASS, no new warnings in annotate module.

- [ ] **Step 5: Commit (include patch version bump)**

```bash
git add src/annotate/render.rs Cargo.toml
git commit -m "feat: paint annotation overlay in egui"
```

---

### Task 5: CPU burn (`render.rs` part 2)

**Files:**
- Modify: `src/annotate/render.rs` (append `burn` + blend helpers)
- Test: unit tests inside `src/annotate/render.rs`

**Interfaces:**
- Consumes: `shapes::{Shape, Style, BlurKind}`, `image::{DynamicImage, Rgba}`, `imageproc::drawing`, `image::imageops`
- Produces: `burn(base: &DynamicImage, shapes: &[Shape]) -> DynamicImage` — used by Task 8 (Apply). Text shapes without an available font are skipped (logged once via `eprintln!`); font loading itself is Task 6.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn burn_empty_preserves_pixels_exactly() {
    let img = white_image(100, 80);
    assert_eq!(burn(&img, &[]).to_rgba8(), img.to_rgba8());
}

#[test]
fn burn_line_changes_pixels_and_keeps_size() {
    let img = white_image(100, 80);
    let shape = Shape::Line {
        a: egui::pos2(10.0, 10.0),
        b: egui::pos2(90.0, 10.0),
        style: red_style(),
    };
    let out = burn(&img, &[shape]);
    assert_eq!(out.dimensions(), (100, 80));
    assert_ne!(out.to_rgba8(), img.to_rgba8());
}

#[test]
fn burn_clamps_out_of_bounds_rect() {
    let img = white_image(100, 80);
    let shape = Shape::Rect {
        rect: ImgRect { min: egui::pos2(-50.0, -50.0), max: egui::pos2(500.0, 500.0) },
        style: red_style(),
    };
    let out = burn(&img, &[shape]); // must not panic
    assert_eq!(out.dimensions(), (100, 80));
}

#[test]
fn burn_blur_region_keeps_size_and_softens_checker() {
    let img = checker_image(64, 64);
    let shape = Shape::BlurRegion {
        rect: ImgRect { min: egui::pos2(0.0, 0.0), max: egui::pos2(64.0, 64.0) },
        kind: BlurKind::Blur,
    };
    let out = burn(&img, &[shape]);
    assert_eq!(out.dimensions(), (64, 64));
    assert_ne!(out.to_rgba8(), img.to_rgba8());
}
```

Helpers for tests: `white_image(w, h)` (all-white RGBA), `checker_image` (8px black/white squares), `red_style()` (`Style { color: RED, width: 3.0, ..Default::default() }`).

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test annotate::render::tests::burn 2>&1 | tail -10`
Expected: FAIL — `burn` undefined.

- [ ] **Step 3: Write minimal implementation**

```rust
use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
use imageproc::drawing::{
    draw_filled_ellipse_mut, draw_filled_rect_mut, draw_hollow_ellipse_mut,
    draw_hollow_rect_mut, draw_line_segment_mut, draw_polygon_mut,
};
use imageproc::rect::Rect as IRect;

// Standard-alpha blend of src over dst (both RGBA).
fn blend(dst: Rgba<u8>, src: Rgba<u8>) -> Rgba<u8> {
    let a = src[3] as f32 / 255.0;
    Rgba([
        (src[0] as f32 * a + dst[0] as f32 * (1.0 - a)) as u8,
        (src[1] as f32 * a + dst[1] as f32 * (1.0 - a)) as u8,
        (src[2] as f32 * a + dst[2] as f32 * (1.0 - a)) as u8,
        dst[3].max(src[3]),
    ])
}

fn to_rgba(c: egui::Color32) -> Rgba<u8> {
    Rgba([c.r(), c.g(), c.b(), c.a()])
}

fn clamp_rect(rect: super::shapes::ImgRect, w: u32, h: u32) -> Option<IRect> {
    let c = rect.clamp(w as f32, h as f32);
    if c.width() < 1.0 || c.height() < 1.0 {
        return None;
    }
    IRect::at(c.min.x as i32, c.min.y as i32)
        .of_size(c.width() as u32, c.height() as u32)
}

// Rasterize shapes onto a copy of `base`. Never panics on geometry.
pub fn burn(base: &DynamicImage, shapes: &[super::shapes::Shape]) -> DynamicImage {
    use super::shapes::Shape;
    if shapes.is_empty() {
        return base.clone();
    }
    let (w, h) = base.dimensions();
    let mut img: RgbaImage = base.to_rgba8();
    for shape in shapes {
        match shape {
            Shape::Line { .. } => {
                burn_one(&mut img, shape, w, h);
            }
            Shape::Arrow { a, b, style } => {
                // Shaft reuses the Line path; head is a filled triangle.
                let shaft = Shape::Line { a: *a, b: *b, style: *style };
                burn_one(&mut img, &shaft, w, h);
                let len = (b.x - a.x).hypot(b.y - a.y).max(1e-6);
                let dir = egui::vec2((b.x - a.x) / len, (b.y - a.y) / len);
                let size = style.width * 4.0 + 6.0;
                let [w1, w2] = arrow_head_points(*b, dir, size);
                let tri = vec![
                    imageproc::point::Point::new(b.x as i32, b.y as i32),
                    imageproc::point::Point::new(w1.x as i32, w1.y as i32),
                    imageproc::point::Point::new(w2.x as i32, w2.y as i32),
                ];
                draw_polygon_mut(&mut img, &tri, to_rgba(style.effective_color()));
            }
            Shape::Pencil { points, style } => {
                for win in points.windows(2) {
                    let seg = Shape::Line { a: win[0], b: win[1], style: *style };
                    burn_one(&mut img, &seg, w, h);
                }
            }
            Shape::Rect { rect, style } => {
                let Some(r) = clamp_rect(*rect, w, h) else { continue };
                let color = to_rgba(style.effective_color());
                if style.filled {
                    draw_filled_rect_mut(&mut img, r, color);
                } else {
                    draw_hollow_rect_mut(&mut img, r, color);
                }
            }
            Shape::Ellipse { rect, style } => {
                let Some(r) = clamp_rect(*rect, w, h) else { continue };
                let center = ((r.left() + r.width() as i32 / 2), (r.top() + r.height() as i32 / 2));
                let (rx, ry) = (r.width() as i32 / 2, r.height() as i32 / 2);
                let color = to_rgba(style.effective_color());
                if style.filled {
                    draw_filled_ellipse_mut(&mut img, center, rx, ry, color);
                } else {
                    draw_hollow_ellipse_mut(&mut img, center, rx, ry, color);
                }
            }
            Shape::Highlight { rect, style } => {
                // Translucent fill regardless of the filled flag.
                let Some(r) = clamp_rect(*rect, w, h) else { continue };
                let mut hl = *style;
                hl.opacity = (hl.opacity * 0.4).clamp(0.05, 0.6);
                blend_rect(&mut img, r, to_rgba(hl.effective_color()));
            }
            Shape::Text { .. } => {
                // Text rendering lands in Task 6 (needs the burn font);
                // until then Text shapes are skipped so every commit compiles.
            }
            Shape::BlurRegion { rect, kind } => {
                let Some(r) = clamp_rect(*rect, w, h) else { continue };
                burn_region(&mut img, r, *kind);
            }
        }
    }
    DynamicImage::ImageRgba8(img)
}
```

Supporting helpers (same file, all bounds-checked so burn never panics):

```rust
// Single-shape fast path reused for Line/Arrow-shaft/Pencil segments.
fn burn_one(img: &mut RgbaImage, shape: &super::shapes::Shape, _w: u32, _h: u32) {
    use super::shapes::Shape;
    let Shape::Line { a, b, style } = shape else { return };
    let color = to_rgba(style.effective_color());
    let dist = (b.x - a.x).hypot(b.y - a.y);
    let steps = (dist / 1.0).ceil().max(1.0) as i32;
    let r = (style.width / 2.0).max(0.5) as i32;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        draw_filled_circle_blend(
            img,
            ((a.x + (b.x - a.x) * t) as i32, (a.y + (b.y - a.y) * t) as i32),
            r,
            color,
        );
    }
}

// Disc stamp with per-pixel blending; silently skips out-of-bounds pixels.
fn draw_filled_circle_blend(img: &mut RgbaImage, center: (i32, i32), radius: i32, color: Rgba<u8>) {
    let (w, h) = (img.width() as i32, img.height() as i32);
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dy * dy > radius * radius {
                continue;
            }
            let (x, y) = (center.0 + dx, center.1 + dy);
            if x < 0 || y < 0 || x >= w || y >= h {
                continue;
            }
            let px = img.get_pixel_mut(x as u32, y as u32);
            *px = blend(*px, color);
        }
    }
}

// Blend a solid color over every pixel inside `r`.
fn blend_rect(img: &mut RgbaImage, r: IRect, color: Rgba<u8>) {
    for y in r.top()..r.bottom() {
        for x in r.left()..r.right() {
            if let Some(px) = img.get_pixel_mut_checked(x as u32, y as u32) {
                *px = blend(*px, color);
            }
        }
    }
}

// Destructive region effect: Blur softens, Mosaic pixelates.
fn burn_region(img: &mut RgbaImage, r: IRect, kind: super::shapes::BlurKind) {
    use super::shapes::BlurKind;
    let (x, y, w, h) = (r.left().max(0) as u32, r.top().max(0) as u32, r.width(), r.height());
    if w == 0 || h == 0 {
        return;
    }
    let sub = image::imageops::crop_imm(img, x, y, w, h).to_image();
    let out = match kind {
        BlurKind::Blur => image::imageops::blur(&sub, 8.0),
        BlurKind::Mosaic => {
            let tiny_w = (w / 12).max(1);
            let tiny_h = (h / 12).max(1);
            let tiny = image::imageops::resize(&sub, tiny_w, tiny_h, image::imageops::FilterType::Nearest);
            image::imageops::resize(&tiny, w, h, image::imageops::FilterType::Nearest)
        }
    };
    let _ = image::imageops::replace(img, &out, x as i64, y as i64);
}
```

(The rect is pre-clamped inside the image, so the paste is always in bounds.)

Wait — ordering rule: Task 6 must come before Task 5's Text test passes. Simplest: Task 5's tests exclude Text; Task 6 adds `burn_text` + its tests. In Task 5, the Text arm calls `burn_text(&mut img, shape)` and this task also defines the minimal version:

```rust
// Minimal version: skipped rendering lands in Task 6 with font support.
fn burn_text(_img: &mut RgbaImage, _shape: &super::shapes::Shape) {}
```

Hmm, that is a temporary stub which the skill forbids... but it is resolved by the very next task and tests never cover the stub. Cleaner alternative: merge font loading into Task 5. But tasks should be small. Decision: define `load_burn_font()` in Task 5 too (it is 15 lines, reuses `font_loader` paths), and make Task 6 purely "wire CJK font bytes + text tests". Actually simplest honest split:

- Task 5: burn for all non-text shapes + helpers + `burn_text` skeleton that only handles font-missing skip... still a stub.

Alternative: Task 5 implements `burn` WITHOUT the Text arm (Text shapes are skipped with a comment "rendered in Task 6"), Task 6 adds the Text arm + `burn_text` + `load_burn_font` + tests. A match that skips a variant needs a `Shape::Text { .. } => {}` arm with explanatory comment — that is a documented sequencing step, not a placeholder. Acceptable and keeps every commit green. Go with that.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test annotate:: 2>&1 | tail -5` then `cargo test 2>&1 | tail -3`
Expected: PASS everywhere.

- [ ] **Step 5: Commit (include patch version bump)**

```bash
git add src/annotate/render.rs Cargo.toml
git commit -m "feat: burn annotation shapes to pixels"
```

---

### Task 6: Text burn with CJK font

**Files:**
- Modify: `src/font_loader.rs` (add `cjk_font_bytes()`), `src/annotate/render.rs` (add `load_burn_font`, `burn_text`, wire Text arm)
- Test: unit tests inside `src/annotate/render.rs`

**Interfaces:**
- Consumes: `font_loader::cjk_font_bytes()`, `ab_glyph::{FontArc, PxScale}`, `imageproc::drawing::draw_text_mut`
- Produces: Text shapes render in `burn()` output.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn burn_text_never_panics_and_keeps_size() {
    let img = white_image(200, 100);
    let shape = Shape::Text {
        at: egui::pos2(10.0, 10.0),
        text: "Hi 測試".to_string(),
        style: Style { font_size: 24.0, ..red_style() },
    };
    let out = burn(&img, &[shape]); // must not panic with or without a system font
    assert_eq!(out.dimensions(), (200, 100));
}

#[test]
fn burn_text_renders_when_font_available() {
    if load_burn_font().is_none() {
        eprintln!("SKIP: no CJK font on this machine");
        return;
    }
    let img = white_image(200, 100);
    let shape = Shape::Text {
        at: egui::pos2(10.0, 10.0),
        text: "Hi".to_string(),
        style: Style { font_size: 32.0, ..red_style() },
    };
    assert_ne!(burn(&img, &[shape]).to_rgba8(), img.to_rgba8());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test annotate::render::tests::burn_text 2>&1 | tail -10`
Expected: FAIL — no such tests / Text arm is a no-op so the render test fails.

- [ ] **Step 3: Write minimal implementation**

In `font_loader.rs`, extract the candidate scan so both UI and burn share it:

```rust
pub fn cjk_font_path() -> Option<std::path::PathBuf> {
    let candidates: &[&str] = if cfg!(target_os = "windows") {
        &[
            "C:\\Windows\\Fonts\\msyh.ttc",
            "C:\\Windows\\Fonts\\msyhbd.ttc",
            "C:\\Windows\\Fonts\\simsun.ttc",
            "C:\\Windows\\Fonts\\simhei.ttf",
            "C:\\Windows\\Fonts\\yahei.ttf",
        ]
    } else if cfg!(target_os = "macos") {
        &[
            "/System/Library/Fonts/PingFang.ttc",
            "/System/Library/Fonts/STHeiti Light.ttc",
            "/Library/Fonts/Arial Unicode.ttf",
        ]
    } else {
        &[
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
        ]
    };
    candidates
        .iter()
        .map(std::path::PathBuf::from)
        .find(|p| p.exists())
}

pub fn cjk_font_bytes() -> Option<Vec<u8>> {
    cjk_font_path().and_then(|p| std::fs::read(p).ok())
}

pub fn load_cjk_font() -> Option<FontData> {
    cjk_font_bytes().map(FontData::from_owned)
}
```

In `render.rs`:

```rust
use ab_glyph::{FontArc, PxScale};
use imageproc::drawing::draw_text_mut;

fn load_burn_font() -> Option<FontArc> {
    let bytes = crate::font_loader::cjk_font_bytes()?;
    // Leak to get the 'static lifetime FontArc::try_from_vec needs.
    FontArc::try_from_vec(bytes).ok()
}

fn burn_text(img: &mut RgbaImage, at: egui::Pos2, text: &str, style: super::shapes::Style) {
    let Some(font) = load_burn_font() else {
        eprintln!("annotate: no CJK font found, skipping text burn");
        return;
    };
    let scale = PxScale::from(style.font_size.max(4.0));
    let color = to_rgba(style.effective_color());
    let mut y = at.y.max(0.0);
    for line in text.lines() {
        draw_text_mut(img, color, at.x.max(0.0) as i32, y as i32, scale, &font, line);
        y += style.font_size * 1.2;
    }
}
```

Change the `Shape::Text { .. } => {}` arm from Task 5 to destructure and call `burn_text`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test annotate:: 2>&1 | tail -5`
Expected: PASS (second test renders or clean-skips depending on machine fonts).

- [ ] **Step 5: Commit (include patch version bump)**

```bash
git add src/font_loader.rs src/annotate/render.rs Cargo.toml
git commit -m "feat: burn text annotations with CJK font"
```

---

### Task 7: Viewer integration (draw + select + overlay)

**Files:**
- Modify: `src/viewer.rs:407-471` (drag handling + overlay paint; exact lines: the `crop_active` drag block and crop overlay block)
- Test: unit test for `screen_delta_to_image` helper at bottom of `src/viewer.rs` (existing `#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `annotate::{AnnotateState, Draft, Tool, commit_draft, draft_ready}`, `annotate::render::{paint_shapes, paint_draft}`, existing `screen_to_image_pos` / `image_to_screen_pos`
- Produces: mouse draws/selects/moves shapes; overlay visible. Used by Task 8 (panel toggles `annotate.active`).

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn screen_delta_converts_by_zoom() {
    assert_eq!(
        screen_delta_to_image(egui::vec2(10.0, -6.0), 2.0),
        egui::vec2(5.0, -3.0)
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test viewer::tests::screen_delta 2>&1 | tail -8`
Expected: FAIL — function undefined.

- [ ] **Step 3: Write minimal implementation**

Add helper (bottom of file, near `screen_to_image_pos`):

```rust
fn screen_delta_to_image(delta: egui::Vec2, zoom: f32) -> egui::Vec2 {
    delta / zoom.max(1e-3)
}
```

In `draw_image`, replace the drag block with mode-aware handling. Current code:

```rust
if crop_active {
    ...crop drag...
} else if drag.dragged() {
    app.viewer_state.pan_offset += drag.drag_delta();
}
```

New logic (insert after `let crop_active = ...`, also read annotate state):

```rust
let annot_active = app.editor_state.annotate.active;
```

Rules:
- If `crop_active`, existing crop behavior unchanged; ensure activating annotate clears `crop_active` (done in Task 8 panel) and vice versa.
- Else if `annot_active` and tool != Select: `drag_started` → `draft = Some(Draft { tool, start: img_pos, current: img_pos, points: vec![img_pos] })`; `dragged` → update `current`, push to `points` for Pencil (dedupe points closer than 1.0 image px); on `drag_stopped()` (check `drag.drag_stopped()`) → `commit_draft(&mut app.editor_state.annotate)`.
- Else if `annot_active` and tool == Select: click (use `ui.input` click or `drag.drag_started` + `drag_stopped` without motion) → hit-test topmost-first (`shapes.iter().rfind`) with tolerance `8.0 / zoom`, set `selected`; drag with a selection → `translate(screen_delta_to_image(drag.drag_delta(), zoom))`.
- Else: existing pan behavior.

After the crop overlay block, paint the annotation overlay:

```rust
{
    let ann = &app.editor_state.annotate;
    if ann.active || !ann.shapes.is_empty() {
        let to_screen = |p: egui::Pos2| image_to_screen_pos(p, draw_rect, tex_size);
        crate::annotate::render::paint_shapes(ui.painter(), &ann.shapes, ann.selected, app.viewer_state.zoom, to_screen);
        if let Some(d) = &ann.draft {
            crate::annotate::render::paint_draft(ui.painter(), d, ann.style, app.viewer_state.zoom, to_screen);
        }
    }
}
```

Note on borrow: `app.editor_state.annotate` is borrowed while `ui.painter()` is used — copy needed values (`zoom`) to locals first; `paint_*` take `&ann.shapes` while painter borrows `ui` — restructure to end the `app` borrow before painting (clone style, collect nothing; just scope the borrow: read shapes ref after `let ann = &app.editor_state.annotate;` — `ui` and `app` are independent borrows, this compiles).

While `annot_active`, skip the pan branch so drawing never pans.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test 2>&1 | tail -3` then `cargo check 2>&1 | tail -3`
Expected: PASS, no errors (fix borrow issues until clean).

- [ ] **Step 5: Commit (include patch version bump)**

```bash
git add src/viewer.rs Cargo.toml
git commit -m "feat: draw and select annotations in viewer"
```

---

### Task 8: Editor panel (tools, style, Apply, lifecycle)

**Files:**
- Modify: `src/editor/mod.rs` (`State::new`, `load_image`, `discard_changes`, `has_unsaved_changes`, `show()` panel, new `annotate_controls` + `apply_annotations` fns)
- Test: unit tests inside `src/editor/mod.rs` `#[cfg(test)]`

**Interfaces:**
- Consumes: `annotate::{AnnotateState, Tool}`, `annotate::render::burn`, existing `apply_op`/`sync_editor_image` flow
- Produces: user-facing Annotate section; Apply burns shapes into one undo step; lifecycle keeps state consistent.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn discard_clears_annotations() {
    let mut state = state_with_image(colored_image(4, 4, 10));
    state.annotate.shapes.push(crate::annotate::shapes::Shape::Rect {
        rect: crate::annotate::shapes::ImgRect {
            min: eframe::egui::pos2(1.0, 1.0),
            max: eframe::egui::pos2(2.0, 2.0),
        },
        style: Default::default(),
    });
    assert!(state.has_unsaved_changes());
    state.discard_changes();
    assert!(state.annotate.shapes.is_empty());
    assert!(!state.has_unsaved_changes());
}

#[test]
fn unapplied_shapes_count_as_unsaved() {
    let mut state = state_with_image(colored_image(4, 4, 10));
    assert!(!state.has_unsaved_changes());
    state.annotate.active = true;
    state.annotate.shapes.push(crate::annotate::shapes::Shape::Rect {
        rect: crate::annotate::shapes::ImgRect {
            min: eframe::egui::pos2(0.0, 0.0),
            max: eframe::egui::pos2(1.0, 1.0),
        },
        style: Default::default(),
    });
    assert!(state.has_unsaved_changes());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test editor::tests::discard_clears_annotations 2>&1 | tail -8`
Expected: FAIL — `annotate` field does not exist on the test `State` (Task 2 added it to the real struct; tests reference it but `discard_changes`/`has_unsaved_changes` ignore it).

- [ ] **Step 3: Write minimal implementation**

State changes:

```rust
fn discard_changes(&mut self) {
    self.current_image = self.original_image.clone();
    self.history.clear();
    self.history_index = 0;
    self.adjustments = ColorAdjustments::default();
    self.annotate.clear();
}
```

In `load_image`, after the existing resets: `self.annotate.clear();`

```rust
pub(crate) fn has_unsaved_changes(&self) -> bool {
    self.history_index > 0
        || !self.adjustments.is_neutral()
        || !self.annotate.shapes.is_empty()
}
```

Panel section (inside the scroll area, after Transform, before Resize):

```rust
ui.separator();
egui::CollapsingHeader::new(
    egui::RichText::new("Annotate").strong().color(colors.accent),
)
.default_open(false)
.show(ui, |ui| annotate_controls(app, ctx, ui));
```

`annotate_controls`:

```rust
fn annotate_controls(app: &mut App, ctx: &egui::Context, ui: &mut egui::Ui) {
    use crate::annotate::Tool;
    let tools = [
        (Tool::Select, "Select"),
        (Tool::Line, "Line"),
        (Tool::Arrow, "Arrow"),
        (Tool::Rect, "Rect"),
        (Tool::Ellipse, "Ellipse"),
        (Tool::Pencil, "Pencil"),
        (Tool::Highlight, "Highlight"),
        (Tool::Text, "Text"),
        (Tool::BlurRegion, "Blur"),
    ];
    ui.horizontal_wrapped(|ui| {
        for (tool, label) in tools {
            if ui.selectable_label(app.editor_state.annotate.tool == tool, label).clicked() {
                app.editor_state.annotate.tool = tool;
                if !app.editor_state.annotate.active {
                    app.editor_state.annotate.active = true;
                    app.editor_state.crop_active = false; // mutual exclusion
                }
            }
        }
    });
    ui.horizontal(|ui| {
        if ui
            .selectable_label(app.editor_state.annotate.active, "Draw Mode")
            .clicked()
        {
            app.editor_state.annotate.active = !app.editor_state.annotate.active;
            if app.editor_state.annotate.active {
                app.editor_state.crop_active = false;
            } else {
                app.editor_state.annotate.draft = None;
                app.editor_state.annotate.selected = None;
            }
        }
    });
    let style = &mut app.editor_state.annotate.style;
    let mut color = style.color;
    if ui.color_edit_button_srgba(&mut color).changed() {
        style.color = color;
    }
    ui.add(egui::Slider::new(&mut style.width, 1.0..=30.0).text("Width"));
    ui.add(egui::Slider::new(&mut style.opacity, 0.05..=1.0).text("Opacity"));
    ui.add(egui::Slider::new(&mut style.font_size, 8.0..=120.0).text("Font size"));
    ui.checkbox(&mut style.filled, "Fill rect / ellipse");
    if app.editor_state.annotate.tool == Tool::Text {
        ui.add(
            egui::TextEdit::multiline(&mut app.editor_state.annotate.text_buffer)
                .hint_text("Text to place (click image to set position)")
                .desired_rows(2),
        );
        if ui.button("Place Text At Center").clicked() {
            if let Some(img) = app.editor_state.current_image.as_ref() {
                let (w, h) = image::GenericImageView::dimensions(img);
                app.editor_state.annotate.shapes.push(
                    crate::annotate::shapes::Shape::Text {
                        at: egui::pos2(w as f32 / 2.0, h as f32 / 2.0),
                        text: app.editor_state.annotate.text_buffer.clone(),
                        style: app.editor_state.annotate.style,
                    },
                );
            }
        }
    }
    ui.label(format!("{} object(s)", app.editor_state.annotate.shapes.len()));
    ui.horizontal(|ui| {
        let has = !app.editor_state.annotate.shapes.is_empty();
        if ui.add_enabled(has, egui::Button::new("Apply")).clicked() {
            apply_annotations(app, ctx);
        }
        if ui.add_enabled(has, egui::Button::new("Delete Selected")).clicked() {
            if let Some(i) = app.editor_state.annotate.selected.take() {
                if i < app.editor_state.annotate.shapes.len() {
                    app.editor_state.annotate.shapes.remove(i);
                }
            }
        }
        if ui.add_enabled(has, egui::Button::new("Clear")).clicked() {
            app.editor_state.annotate.clear();
            app.editor_state.annotate.active = true; // stay in draw mode
        }
    });
}

fn apply_annotations(app: &mut App, ctx: &egui::Context) {
    let base = match app.editor_state.current_image.as_ref() {
        Some(i) => i.clone(),
        None => return,
    };
    if app.editor_state.annotate.shapes.is_empty() {
        return;
    }
    let burned = crate::annotate::render::burn(&base, &app.editor_state.annotate.shapes);
    app.editor_state.annotate.clear();
    app.editor_state.annotate.active = false;
    apply_op(app, ctx, EditOp::Replace(std::sync::Arc::new(burned)));
}
```

Note: `apply_op` resets adjustments and calls `sync_editor_image` — Apply becomes exactly one undo step, and the existing unsaved-navigation dialog covers it.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test 2>&1 | tail -3` then `cargo check 2>&1 | tail -3`
Expected: PASS.

- [ ] **Step 5: Commit (include patch version bump)**

```bash
git add src/editor/mod.rs Cargo.toml
git commit -m "feat: annotation panel with apply and lifecycle"
```

---

### Task 9: Final verification and manual pass

**Files:**
- Touch: none (verification only)
- Test: full suite + release build

- [ ] **Step 1: Run the full suite**

Run: `cargo test 2>&1 | tail -5`
Expected: all tests PASS (11 pre-existing + new annotate tests).

- [ ] **Step 2: Release build**

Run: `cargo build --release 2>&1 | tail -3`
Expected: success, single binary produced.

- [ ] **Step 3: Manual pass** (no commit; report results)

1. Open a folder, enter Viewer, toggle Edit → Annotate → Draw Mode.
2. Draw one of each Phase-1 tool at 25%, 100%, and 400% zoom; shapes must track the image under zoom/pan.
3. Select + drag a shape to move it; Delete Selected removes it.
4. Apply → Undo restores pixels; unsaved dialog appears on navigation with unapplied shapes.
5. Save As PNG and JPEG; output contains the burned annotations.

---

## Self-Review

1. **Spec coverage:** data model (§3.1/§3.2) → Tasks 2–3; overlay + burn (§3.3) → Tasks 4–6; viewer interaction (§3.4, mutual exclusion) → Task 7; panel + Apply/Clear (§3.5) → Task 8; Replace-based commit (§3.6) → Task 8 `apply_annotations`; error handling (§5) → clamp helpers, font-missing skip, `draft_ready` guard; testing (§7) → unit tests per task + Task 9. Amendment: unapplied shapes count as unsaved (Task 8) — stricter than spec, prevents silent loss; flag in review.
2. **Placeholder scan:** every step has concrete code/commands; no TBD/TODO; version bumps are explicit `+1` instructions.
3. **Type consistency:** `egui::Pos2` positions everywhere; `ImgRect` shared by shapes/render; `burn(&DynamicImage, &[Shape]) -> DynamicImage`; `commit_draft(&mut AnnotateState)`; style copied by value (`Style: Copy` — ensure `#[derive(Clone, Copy)]` on Style; `Shape` is Clone only due to String/Vec).

## Out of Scope (follow-up plan)

Phase 2 per spec §6: StepNumber, Callout/curved text, Magnifier, shadow/outline, dash styles, emoji/stamp — additive `Shape` variants + panel controls, planned separately after Phase 1 lands.
