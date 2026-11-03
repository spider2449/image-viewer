pub mod icons;
pub mod render;
pub mod shapes;

pub use shapes::{ImgRect, Shape, Style};

pub const MIN_DRAG_PX: f32 = 4.0;
const HISTORY_LIMIT: usize = 100;
const PASTE_STEP: egui::Vec2 = egui::vec2(16.0, 16.0);
// Written to the system clipboard on copy. egui only emits Event::Paste when the
// system clipboard holds text, and comparing against this tells our own objects
// apart from text copied in another application afterwards.
pub const CLIPBOARD_MARKER: &str = "[image-viewer annotation objects]";

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
    Callout,
    Step,
    BlurRegion,
}

impl Tool {
    // Tools whose object is a rectangle, so Auto Region can size it from a click.
    pub fn takes_region(self) -> bool {
        matches!(self, Tool::Rect | Tool::Ellipse | Tool::Highlight | Tool::BlurRegion)
    }

    // Tools that drop an object on click instead of sizing it by dragging.
    pub fn is_stamp(self) -> bool {
        matches!(self, Tool::Text | Tool::Callout | Tool::Step)
    }
}

// What a Select-tool drag is currently doing to the selected shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragMode {
    Move,
    Handle(usize),
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
    // Selected object indices, sorted ascending. Resize handles and text editing
    // need exactly one; see `single_selected`.
    pub selected: Vec<usize>,
    pub style: Style,
    pub draft: Option<Draft>,
    pub text_buffer: String,
    // BlurRegion tool effect: false = Blur, true = Mosaic.
    pub mosaic: bool,
    // Select-tool drag in progress, and whether its undo snapshot was taken yet.
    pub drag_mode: Option<DragMode>,
    pub drag_snapped: bool,
    // True while a style control is being dragged, so one gesture = one undo step.
    pub style_edit_open: bool,
    // Curve anchor under the pointer when the context menu was opened.
    pub menu_anchor: Option<usize>,
    // Rubber-band selection in progress: (press point, current point).
    pub marquee: Option<(egui::Pos2, egui::Pos2)>,
    // Copied objects. Deliberately survives `clear` so they can be pasted onto the
    // next image.
    clipboard: Vec<Shape>,
    paste_offset: egui::Vec2,
    undo_stack: Vec<Vec<Shape>>,
    redo_stack: Vec<Vec<Shape>>,
}

impl AnnotateState {
    pub fn new() -> Self {
        Self::default()
    }

    // Full reset (new image, discard, or after the objects were burned in).
    pub fn clear(&mut self) {
        self.shapes.clear();
        self.selected.clear();
        self.draft = None;
        self.marquee = None;
        // Pasting onto a fresh image lands where the objects were copied from.
        self.paste_offset = egui::Vec2::ZERO;
        self.text_buffer.clear();
        self.drag_mode = None;
        self.drag_snapped = false;
        self.style_edit_open = false;
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    // Remove every object but keep it undoable.
    pub fn clear_objects(&mut self) {
        if self.shapes.is_empty() {
            return;
        }
        self.snapshot();
        self.shapes.clear();
        self.selected.clear();
        self.draft = None;
    }

    // Record the current objects so the next mutation can be undone.
    pub fn snapshot(&mut self) {
        self.undo_stack.push(self.shapes.clone());
        if self.undo_stack.len() > HISTORY_LIMIT {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo(&mut self) -> bool {
        let Some(prev) = self.undo_stack.pop() else {
            return false;
        };
        self.redo_stack
            .push(std::mem::replace(&mut self.shapes, prev));
        self.after_history_jump();
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo_stack.pop() else {
            return false;
        };
        self.undo_stack
            .push(std::mem::replace(&mut self.shapes, next));
        self.after_history_jump();
        true
    }

    fn after_history_jump(&mut self) {
        self.draft = None;
        self.drag_mode = None;
        self.marquee = None;
        let len = self.shapes.len();
        self.selected.retain(|i| *i < len);
    }

    // The selected index when exactly one object is selected.
    pub fn single_selected(&self) -> Option<usize> {
        match self.selected.as_slice() {
            [i] => Some(*i).filter(|i| *i < self.shapes.len()),
            _ => None,
        }
    }

    pub fn delete_selected(&mut self) {
        let selected = std::mem::take(&mut self.selected);
        if selected.is_empty() {
            return;
        }
        self.snapshot();
        // Ascending order, so removing from the back keeps earlier indices valid.
        for i in selected.into_iter().rev() {
            if i < self.shapes.len() {
                self.shapes.remove(i);
            }
        }
    }

    // Remove one anchor of the selected curve. A curve keeps at least 2 anchors;
    // use Delete Selected to remove the whole object.
    pub fn can_delete_anchor(&self, anchor: usize) -> bool {
        self.single_selected()
            .and_then(|i| self.shapes.get(i))
            .is_some_and(|s| matches!(s, Shape::Pencil { points, .. }
                if points.len() > 2 && anchor < points.len()))
    }

    pub fn delete_anchor(&mut self, anchor: usize) -> bool {
        if !self.can_delete_anchor(anchor) {
            return false;
        }
        let Some(i) = self.single_selected() else {
            return false;
        };
        self.snapshot();
        if let Shape::Pencil { points, .. } = &mut self.shapes[i] {
            points.remove(anchor);
        }
        true
    }

    fn selected_shapes(&self) -> Vec<Shape> {
        self.selected
            .iter()
            .filter_map(|i| self.shapes.get(*i).cloned())
            .collect()
    }

    // Append `items` as one undo step and select them. Step stamps get fresh
    // numbers so the sequence stays unique.
    fn insert_copies(&mut self, items: Vec<Shape>) {
        self.snapshot();
        self.selected.clear();
        for mut shape in items {
            if let Shape::Step { number, .. } = &mut shape {
                *number = next_step_number(&self.shapes);
            }
            self.shapes.push(shape);
            self.selected.push(self.shapes.len() - 1);
        }
        self.sync_style_from_single();
    }

    // Copy the selected objects, offset a little, and select the copies.
    pub fn duplicate_selected(&mut self) {
        let mut items = self.selected_shapes();
        if items.is_empty() {
            return;
        }
        for shape in &mut items {
            shape.translate(PASTE_STEP);
        }
        self.insert_copies(items);
    }

    // Returns false when nothing is selected, leaving the clipboard untouched.
    pub fn copy_selected(&mut self) -> bool {
        let items = self.selected_shapes();
        if items.is_empty() {
            return false;
        }
        self.clipboard = items;
        self.paste_offset = PASTE_STEP;
        true
    }

    pub fn cut_selected(&mut self) -> bool {
        if !self.copy_selected() {
            return false;
        }
        self.delete_selected();
        // The originals are gone, so the first paste restores them in place.
        self.paste_offset = egui::Vec2::ZERO;
        true
    }

    pub fn can_paste(&self) -> bool {
        !self.clipboard.is_empty()
    }

    // Paste the copied objects as one group, kept inside `bounds` (image size).
    // Repeated pastes cascade so the copies do not stack exactly.
    pub fn paste(&mut self, bounds: egui::Vec2) -> bool {
        let mut items = self.clipboard.clone();
        let Some(extent) = items
            .iter()
            .map(|s| s.bounding_rect())
            .reduce(ImgRect::union)
        else {
            return false;
        };
        let offset = self.paste_offset;
        let delta = offset + fit_delta(translated(extent, offset), bounds);
        for shape in &mut items {
            shape.translate(delta);
        }
        self.insert_copies(items);
        self.paste_offset = offset + PASTE_STEP;
        true
    }

    // Drop text from the system clipboard as a new Text object.
    pub fn paste_text(&mut self, text: &str, at: egui::Pos2, bounds: egui::Vec2) {
        if text.trim().is_empty() {
            return;
        }
        let extent = shapes::text_bounds(at, text, self.style.font_size);
        self.push_shape(Shape::Text {
            at: at + fit_delta(extent, bounds),
            text: text.to_string(),
            style: self.style,
        });
    }

    pub fn nudge_selected(&mut self, delta: egui::Vec2, new_undo_step: bool) {
        if self.selected.is_empty() {
            return;
        }
        if new_undo_step {
            self.snapshot();
        }
        self.translate_selected(delta);
    }

    pub fn translate_selected(&mut self, delta: egui::Vec2) {
        for i in &self.selected {
            if let Some(shape) = self.shapes.get_mut(*i) {
                shape.translate(delta);
            }
        }
    }

    // Move the selected objects one step toward the front (`up`) or back. An object
    // already blocked by the end of the stack keeps its place, and the group's
    // relative order is preserved.
    pub fn reorder_selected(&mut self, up: bool) {
        let len = self.shapes.len();
        let target = |i: usize| if up { i + 1 } else { i.wrapping_sub(1) };
        let selected = &self.selected;
        let movable = |i: usize| target(i) < len && !selected.contains(&target(i));
        if !selected.iter().any(|i| *i < len && movable(*i)) {
            return;
        }
        self.snapshot();
        // Walk from the moving edge so each object swaps into an already-freed slot.
        let order: Vec<usize> = if up {
            (0..self.selected.len()).rev().collect()
        } else {
            (0..self.selected.len()).collect()
        };
        for pos in order {
            let i = self.selected[pos];
            let j = target(i);
            if i < len && j < len && !self.selected.contains(&j) {
                self.shapes.swap(i, j);
                self.selected[pos] = j;
            }
        }
    }

    // Load the style of a lone selected object into the panel controls. With several
    // selected the panel keeps its values: only edited fields are applied to them.
    fn sync_style_from_single(&mut self) {
        if let Some(style) = self
            .single_selected()
            .and_then(|i| self.shapes.get(i))
            .and_then(|s| s.style())
        {
            self.style = *style;
            // Rotation belongs to the object, not to the panel style.
            self.style.angle = 0.0;
        }
    }

    // Select by index and load its style into the panel controls.
    pub fn select(&mut self, index: Option<usize>) {
        self.selected = index.into_iter().collect();
        self.sync_style_from_single();
    }

    pub fn select_all(&mut self) {
        self.selected = (0..self.shapes.len()).collect();
        self.sync_style_from_single();
    }

    // Click: replace the selection, or with Shift toggle the object under the pointer.
    pub fn click_select(&mut self, hit: Option<usize>, additive: bool) {
        match (hit, additive) {
            (Some(i), true) => {
                match self.selected.binary_search(&i) {
                    Ok(pos) => {
                        self.selected.remove(pos);
                    }
                    Err(pos) => self.selected.insert(pos, i),
                }
                self.sync_style_from_single();
            }
            (None, true) => {}
            (hit, false) => self.select(hit),
        }
    }

    // Press on an object to drag it: an already-selected object keeps the whole
    // group selected so the group moves together.
    pub fn grab(&mut self, i: usize, additive: bool) {
        match self.selected.binary_search(&i) {
            Ok(_) => {}
            Err(pos) if additive => {
                self.selected.insert(pos, i);
                self.sync_style_from_single();
            }
            Err(_) => self.select(Some(i)),
        }
    }

    // Add every object touched by the rubber band to the selection.
    pub fn finish_marquee(&mut self) {
        let Some((a, b)) = self.marquee.take() else {
            return;
        };
        let band = ImgRect::from_points(a, b);
        for (i, shape) in self.shapes.iter().enumerate() {
            if shape.bounding_rect().intersects(band) && !self.selected.contains(&i) {
                self.selected.push(i);
            }
        }
        self.selected.sort_unstable();
        self.sync_style_from_single();
    }

    // Push a new object and select it.
    fn push_shape(&mut self, shape: Shape) {
        self.snapshot();
        self.shapes.push(shape);
        self.selected = vec![self.shapes.len() - 1];
    }

    // Apply the panel edits made since `before` to every selected object.
    pub fn apply_style_to_selected(&mut self, before: &Style) {
        let after = self.style;
        for i in &self.selected {
            if let Some(style) = self.shapes.get_mut(*i).and_then(|s| s.style_mut()) {
                style.apply_changes(before, &after);
            }
        }
    }
}

// Copy the selection, and mark the system clipboard so Ctrl+V reaches us. Must not
// be called from inside a `ctx.input` closure: `copy_text` would deadlock on it.
pub fn copy_to_clipboard(state: &mut AnnotateState, ctx: &egui::Context) {
    if state.copy_selected() {
        ctx.copy_text(CLIPBOARD_MARKER.to_string());
    }
}

// Same contract as `copy_to_clipboard`.
pub fn cut_to_clipboard(state: &mut AnnotateState, ctx: &egui::Context) {
    if state.cut_selected() {
        ctx.copy_text(CLIPBOARD_MARKER.to_string());
    }
}

fn translated(r: ImgRect, delta: egui::Vec2) -> ImgRect {
    ImgRect {
        min: r.min + delta,
        max: r.max + delta,
    }
}

// Shift that brings `r` inside a `bounds`-sized image. When `r` is larger than the
// image, its top-left corner wins so the start of the content stays visible.
fn fit_delta(r: ImgRect, bounds: egui::Vec2) -> egui::Vec2 {
    let axis = |lo: f32, hi: f32, size: f32| {
        let back = (size - hi).min(0.0);
        if lo + back < 0.0 {
            -lo
        } else {
            back
        }
    };
    egui::vec2(axis(r.min.x, r.max.x, bounds.x), axis(r.min.y, r.max.y, bounds.y))
}

// Chaikin corner cutting: rounds a jittery freehand path, keeping both endpoints.
pub fn smooth_points(points: &[egui::Pos2], rounds: usize) -> Vec<egui::Pos2> {
    let mut pts = points.to_vec();
    for _ in 0..rounds {
        if pts.len() < 3 {
            break;
        }
        let mut out = Vec::with_capacity(pts.len() * 2);
        out.push(pts[0]);
        for w in pts.windows(2) {
            out.push(w[0] + (w[1] - w[0]) * 0.25);
            out.push(w[0] + (w[1] - w[0]) * 0.75);
        }
        out.push(*pts.last().unwrap());
        pts = out;
    }
    pts
}

// Next free step number: one above the highest stamp on the canvas.
pub fn next_step_number(shapes: &[Shape]) -> u32 {
    shapes
        .iter()
        .filter_map(|s| match s {
            Shape::Step { number, .. } => Some(*number),
            _ => None,
        })
        .max()
        .map_or(1, |n| n + 1)
}

// True when the in-progress drag is big enough to become a shape.
pub fn draft_ready(d: &Draft) -> bool {
    match d.tool {
        Tool::Pencil => d.points.len() >= 2,
        // Stamps are placed at the press point regardless of drag size.
        t if t.is_stamp() => true,
        _ => (d.current - d.start).length() >= MIN_DRAG_PX,
    }
}

// Build the object a stamp tool drops at `at`.
fn stamp_shape(state: &AnnotateState, tool: Tool, at: egui::Pos2) -> Option<Shape> {
    let style = state.style;
    let typed = state.text_buffer.trim();
    match tool {
        Tool::Text => Some(Shape::Text {
            at,
            text: if typed.is_empty() {
                "Text".to_string()
            } else {
                state.text_buffer.clone()
            },
            style,
        }),
        Tool::Callout => {
            let text = if typed.is_empty() {
                "Note".to_string()
            } else {
                state.text_buffer.clone()
            };
            let size = shapes::callout_fit_size(&text, style.font_size);
            let rect = shapes::callout_box(at, size);
            // Default tail points down-left below the box.
            let tail = egui::pos2(
                rect.min.x + rect.width() * 0.25,
                rect.max.y + style.font_size * 1.2,
            );
            Some(Shape::Callout {
                at,
                size,
                text,
                tail,
                style,
            })
        }
        Tool::Step => Some(Shape::Step {
            at,
            number: next_step_number(&state.shapes),
            style,
        }),
        _ => None,
    }
}

// Drop a stamp object (text / callout / step number) at `at`.
pub fn place_stamp(state: &mut AnnotateState, at: egui::Pos2) {
    if !state.active {
        return;
    }
    if let Some(shape) = stamp_shape(state, state.tool, at) {
        state.push_shape(shape);
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
        Tool::Line => Shape::Line {
            a: d.start,
            b: d.current,
            style,
        },
        Tool::Arrow => Shape::Arrow {
            a: d.start,
            b: d.current,
            style,
        },
        Tool::Rect => Shape::Rect {
            rect: shapes::ImgRect::from_points(d.start, d.current),
            style,
        },
        Tool::Ellipse => Shape::Ellipse {
            rect: shapes::ImgRect::from_points(d.start, d.current),
            style,
        },
        Tool::Pencil => Shape::Pencil {
            // Reduce the freehand trace to a few editable anchors of a vector curve.
            points: shapes::simplify_rdp(&smooth_points(&d.points, 2), 2.0 + style.width * 0.3),
            style,
        },
        Tool::Highlight => Shape::Highlight {
            rect: shapes::ImgRect::from_points(d.start, d.current),
            style,
        },
        Tool::BlurRegion => Shape::BlurRegion {
            rect: shapes::ImgRect::from_points(d.start, d.current),
            kind: if state.mosaic {
                shapes::BlurKind::Mosaic
            } else {
                shapes::BlurKind::Blur
            },
        },
        Tool::Text | Tool::Callout | Tool::Step => {
            match stamp_shape(state, d.tool, d.start) {
                Some(s) => s,
                None => return,
            }
        }
    };
    state.push_shape(shape);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft(tool: Tool, a: (f32, f32), b: (f32, f32)) -> Draft {
        Draft {
            tool,
            start: egui::pos2(a.0, a.1),
            current: egui::pos2(b.0, b.1),
            points: vec![],
        }
    }

    fn active(tool: Tool) -> AnnotateState {
        let mut s = AnnotateState::new();
        s.active = true;
        s.tool = tool;
        s
    }

    #[test]
    fn tiny_rect_drag_is_dropped() {
        let mut s = active(Tool::Rect);
        s.draft = Some(draft(Tool::Rect, (10.0, 10.0), (11.0, 11.0)));
        commit_draft(&mut s);
        assert!(s.shapes.is_empty());
        assert!(s.draft.is_none());
    }

    #[test]
    fn mosaic_flag_selects_mosaic_region() {
        let mut s = active(Tool::BlurRegion);
        s.mosaic = true;
        s.draft = Some(draft(Tool::BlurRegion, (0.0, 0.0), (20.0, 20.0)));
        commit_draft(&mut s);
        assert_eq!(s.shapes.len(), 1);
        assert!(matches!(
            s.shapes[0],
            Shape::BlurRegion {
                kind: shapes::BlurKind::Mosaic,
                ..
            }
        ));
    }

    #[test]
    fn rect_drag_commits_one_normalized_shape() {
        let mut s = active(Tool::Rect);
        s.draft = Some(draft(Tool::Rect, (50.0, 50.0), (10.0, 10.0)));
        commit_draft(&mut s);
        assert_eq!(s.shapes.len(), 1);
    }

    #[test]
    fn undo_redo_walks_history() {
        let mut s = active(Tool::Rect);
        for _ in 0..2 {
            s.draft = Some(draft(Tool::Rect, (0.0, 0.0), (30.0, 30.0)));
            commit_draft(&mut s);
        }
        assert_eq!(s.shapes.len(), 2);
        assert!(s.undo());
        assert_eq!(s.shapes.len(), 1);
        assert!(s.redo());
        assert_eq!(s.shapes.len(), 2);
        assert!(!s.redo());
    }

    #[test]
    fn new_edit_after_undo_drops_redo() {
        let mut s = active(Tool::Rect);
        s.draft = Some(draft(Tool::Rect, (0.0, 0.0), (30.0, 30.0)));
        commit_draft(&mut s);
        s.undo();
        assert!(s.can_redo());
        s.draft = Some(draft(Tool::Rect, (0.0, 0.0), (30.0, 30.0)));
        commit_draft(&mut s);
        assert!(!s.can_redo());
    }

    #[test]
    fn step_stamps_count_up() {
        let mut s = active(Tool::Step);
        for _ in 0..3 {
            place_stamp(&mut s, egui::pos2(10.0, 10.0));
        }
        let nums: Vec<u32> = s
            .shapes
            .iter()
            .filter_map(|sh| match sh {
                Shape::Step { number, .. } => Some(*number),
                _ => None,
            })
            .collect();
        assert_eq!(nums, vec![1, 2, 3]);
    }

    #[test]
    fn text_stamp_uses_default_when_buffer_empty() {
        let mut s = active(Tool::Text);
        place_stamp(&mut s, egui::pos2(5.0, 5.0));
        assert!(matches!(&s.shapes[0], Shape::Text { text, .. } if text == "Text"));
    }

    #[test]
    fn select_loads_style_and_restyle_applies_back() {
        let mut s = active(Tool::Rect);
        s.style.width = 9.0;
        s.draft = Some(draft(Tool::Rect, (0.0, 0.0), (30.0, 30.0)));
        commit_draft(&mut s);
        s.style.width = 2.0;
        s.select(Some(0));
        assert_eq!(s.style.width, 9.0);
        let before = s.style;
        s.style.dashed = true;
        s.apply_style_to_selected(&before);
        assert!(s.shapes[0].style().unwrap().dashed);
    }

    #[test]
    fn smoothing_keeps_endpoints_and_adds_points() {
        let pts = [
            egui::pos2(0.0, 0.0),
            egui::pos2(10.0, 10.0),
            egui::pos2(20.0, 0.0),
        ];
        let out = smooth_points(&pts, 2);
        assert!(out.len() > pts.len());
        assert_eq!(out[0], pts[0]);
        assert_eq!(*out.last().unwrap(), pts[2]);
        // The sharp corner is cut: nothing reaches the original apex.
        assert!(out.iter().all(|p| p.y < 10.0));
    }

    #[test]
    fn rotating_a_rect_keeps_opposite_corner_fixed_when_resizing() {
        let mut s = Shape::Rect {
            rect: shapes::ImgRect {
                min: egui::pos2(0.0, 0.0),
                max: egui::pos2(40.0, 20.0),
            },
            style: Style {
                angle: 0.7,
                ..Style::default()
            },
        };
        let anchor = s.handles()[0]; // top-left, opposite of handle 4
        s.drag_handle(4, s.handles()[4] + egui::vec2(15.0, 9.0));
        let after = s.handles()[0];
        assert!((anchor - after).length() < 0.01, "{anchor:?} vs {after:?}");
    }

    #[test]
    fn rotate_handle_sets_angle_and_hit_test_follows() {
        let mut s = Shape::Rect {
            rect: shapes::ImgRect {
                min: egui::pos2(0.0, 0.0),
                max: egui::pos2(100.0, 10.0),
            },
            style: Style::default(),
        };
        // Pointing straight right of center = 90 degrees clockwise.
        s.drag_handle(8, egui::pos2(150.0, 5.0));
        let angle = s.style().unwrap().angle;
        assert!((angle - std::f32::consts::FRAC_PI_2).abs() < 0.01);
        // The long axis now runs vertically through the center (50, 5).
        assert!(s.hit_test(egui::pos2(50.0, 45.0), 1.0));
        assert!(!s.hit_test(egui::pos2(95.0, 5.0), 1.0));
    }

    #[test]
    fn reorder_swaps_neighbours() {
        let mut s = active(Tool::Rect);
        s.draft = Some(draft(Tool::Rect, (0.0, 0.0), (30.0, 30.0)));
        commit_draft(&mut s);
        s.tool = Tool::Ellipse;
        s.draft = Some(draft(Tool::Ellipse, (0.0, 0.0), (30.0, 30.0)));
        commit_draft(&mut s);
        s.select(Some(0));
        s.reorder_selected(true);
        assert!(matches!(s.shapes[1], Shape::Rect { .. }));
        assert_eq!(s.selected, vec![1]);
    }

    const BIG: egui::Vec2 = egui::vec2(1000.0, 1000.0);

    // Rects at x = 0, 100, 200, ... each 30 px wide.
    fn with_rects(n: usize) -> AnnotateState {
        let mut s = active(Tool::Rect);
        for k in 0..n {
            let x = k as f32 * 100.0;
            s.draft = Some(draft(Tool::Rect, (x, 0.0), (x + 30.0, 30.0)));
            commit_draft(&mut s);
        }
        s.select(None);
        s
    }

    #[test]
    fn region_draft_commits_with_exact_bounds() {
        let mut s = active(Tool::BlurRegion);
        assert!(s.tool.takes_region());
        s.draft = Some(draft(Tool::BlurRegion, (6.0, 0.0), (10.0, 10.0)));
        commit_draft(&mut s);
        assert_eq!(
            s.shapes[0].bounding_rect(),
            ImgRect {
                min: egui::pos2(6.0, 0.0),
                max: egui::pos2(10.0, 10.0),
            }
        );
    }

    fn min_x(s: &AnnotateState, i: usize) -> f32 {
        s.shapes[i].bounding_rect().min.x
    }

    #[test]
    fn copy_paste_cascades_and_selects_the_copies() {
        let mut s = with_rects(2);
        s.select_all();
        assert!(s.copy_selected());
        assert!(s.paste(BIG));
        assert_eq!(s.shapes.len(), 4);
        assert_eq!(s.selected, vec![2, 3]);
        assert_eq!(min_x(&s, 2), PASTE_STEP.x);
        assert!(s.paste(BIG));
        assert_eq!(min_x(&s, 4), PASTE_STEP.x * 2.0);
        s.undo();
        assert_eq!(s.shapes.len(), 4);
    }

    #[test]
    fn copy_without_selection_leaves_nothing_to_paste() {
        let mut s = with_rects(1);
        assert!(!s.copy_selected());
        assert!(!s.can_paste());
        assert!(!s.paste(BIG));
        assert_eq!(s.shapes.len(), 1);
    }

    #[test]
    fn cut_removes_then_first_paste_restores_in_place() {
        let mut s = with_rects(2);
        s.select(Some(1));
        assert!(s.cut_selected());
        assert_eq!(s.shapes.len(), 1);
        assert!(s.selected.is_empty());
        assert!(s.paste(BIG));
        assert_eq!(min_x(&s, 1), 100.0);
    }

    #[test]
    fn clipboard_survives_image_switch() {
        let mut s = with_rects(1);
        s.select(Some(0));
        s.copy_selected();
        s.clear();
        assert!(s.paste(BIG));
        assert_eq!(s.shapes.len(), 1);
        assert_eq!(min_x(&s, 0), 0.0);
    }

    #[test]
    fn paste_stays_inside_a_smaller_image() {
        let mut s = with_rects(2);
        s.select(Some(1));
        s.copy_selected();
        s.clear();
        s.paste(egui::vec2(50.0, 50.0));
        let b = s.shapes[0].bounding_rect();
        assert!(b.min.x >= 0.0 && b.max.x <= 50.0);
    }

    #[test]
    fn pasted_step_stamps_get_fresh_numbers() {
        let mut s = active(Tool::Step);
        place_stamp(&mut s, egui::pos2(10.0, 10.0));
        s.copy_selected();
        s.paste(BIG);
        s.paste(BIG);
        let numbers: Vec<u32> = s
            .shapes
            .iter()
            .map(|x| match x {
                Shape::Step { number, .. } => *number,
                _ => 0,
            })
            .collect();
        assert_eq!(numbers, vec![1, 2, 3]);
    }

    #[test]
    fn shift_click_toggles_and_plain_click_replaces() {
        let mut s = with_rects(3);
        s.click_select(Some(2), true);
        s.click_select(Some(0), true);
        assert_eq!(s.selected, vec![0, 2]);
        s.click_select(None, true);
        assert_eq!(s.selected, vec![0, 2]);
        s.click_select(Some(2), true);
        assert_eq!(s.selected, vec![0]);
        s.click_select(Some(1), false);
        assert_eq!(s.selected, vec![1]);
        s.click_select(None, false);
        assert!(s.selected.is_empty());
    }

    #[test]
    fn grabbing_a_group_member_keeps_the_group() {
        let mut s = with_rects(3);
        s.select_all();
        s.grab(1, false);
        assert_eq!(s.selected, vec![0, 1, 2]);
        s.translate_selected(egui::vec2(5.0, 0.0));
        assert_eq!(min_x(&s, 2), 205.0);
        s.select(Some(0));
        s.grab(2, false);
        assert_eq!(s.selected, vec![2]);
    }

    #[test]
    fn marquee_selects_touched_objects_and_adds_to_selection() {
        let mut s = with_rects(3);
        s.selected = vec![2];
        // Dragged right-to-left, touching only the first rect.
        s.marquee = Some((egui::pos2(50.0, 50.0), egui::pos2(20.0, 20.0)));
        s.finish_marquee();
        assert_eq!(s.selected, vec![0, 2]);
        assert!(s.marquee.is_none());
    }

    #[test]
    fn delete_and_undo_handle_a_group() {
        let mut s = with_rects(3);
        s.selected = vec![0, 2];
        s.delete_selected();
        assert_eq!(s.shapes.len(), 1);
        assert_eq!(min_x(&s, 0), 100.0);
        s.undo();
        assert_eq!(s.shapes.len(), 3);
    }

    #[test]
    fn undo_drops_selection_indices_that_no_longer_exist() {
        let mut s = with_rects(2);
        s.select_all();
        s.duplicate_selected();
        assert_eq!(s.selected, vec![2, 3]);
        s.undo();
        assert!(s.selected.is_empty());
    }

    #[test]
    fn reorder_moves_a_group_and_keeps_its_order() {
        let mut s = with_rects(4);
        s.selected = vec![1, 2];
        s.reorder_selected(true);
        assert_eq!(s.selected, vec![2, 3]);
        assert_eq!(min_x(&s, 1), 300.0);
        assert_eq!(min_x(&s, 2), 100.0);
        assert_eq!(min_x(&s, 3), 200.0);
        // Already at the top: no change and no undo step.
        let undo_depth = s.undo_stack.len();
        s.reorder_selected(true);
        assert_eq!(s.undo_stack.len(), undo_depth);
        s.reorder_selected(false);
        assert_eq!(s.selected, vec![1, 2]);
        assert_eq!(min_x(&s, 1), 100.0);
    }

    #[test]
    fn group_style_edit_changes_only_the_edited_field() {
        let mut s = with_rects(2);
        s.shapes[0].style_mut().unwrap().width = 2.0;
        s.shapes[1].style_mut().unwrap().width = 8.0;
        s.select_all();
        let before = s.style;
        s.style.dashed = true;
        s.apply_style_to_selected(&before);
        for (i, width) in [(0, 2.0), (1, 8.0)] {
            let style = s.shapes[i].style().unwrap();
            assert!(style.dashed);
            assert_eq!(style.width, width);
        }
    }

    #[test]
    fn nudge_makes_one_undo_step_per_key_press() {
        let mut s = with_rects(1);
        s.select(Some(0));
        let depth = s.undo_stack.len();
        s.nudge_selected(egui::vec2(1.0, 0.0), true);
        s.nudge_selected(egui::vec2(1.0, 0.0), false);
        assert_eq!(s.undo_stack.len(), depth + 1);
        assert_eq!(min_x(&s, 0), 2.0);
    }

    #[test]
    fn paste_text_adds_a_selected_text_object() {
        let mut s = with_rects(0);
        s.paste_text("   ", egui::pos2(5.0, 5.0), BIG);
        assert!(s.shapes.is_empty());
        s.paste_text("hello", egui::pos2(5.0, 5.0), BIG);
        assert!(matches!(&s.shapes[0], Shape::Text { text, .. } if text == "hello"));
        assert_eq!(s.selected, vec![0]);
    }
}
