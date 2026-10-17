pub mod icons;
pub mod render;
pub mod shapes;

pub use shapes::{Shape, Style};

pub const MIN_DRAG_PX: f32 = 4.0;
const HISTORY_LIMIT: usize = 100;

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
    pub selected: Option<usize>,
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
        self.selected = None;
        self.draft = None;
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
        self.selected = None;
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
        if self.selected.is_some_and(|i| i >= self.shapes.len()) {
            self.selected = None;
        }
    }

    pub fn delete_selected(&mut self) {
        if let Some(i) = self.selected.filter(|i| *i < self.shapes.len()) {
            self.snapshot();
            self.shapes.remove(i);
        }
        self.selected = None;
    }

    // Copy the selected object, offset a little, and select the copy.
    pub fn duplicate_selected(&mut self) {
        let Some(i) = self.selected.filter(|i| *i < self.shapes.len()) else {
            return;
        };
        self.snapshot();
        let mut copy = self.shapes[i].clone();
        copy.translate(egui::vec2(16.0, 16.0));
        if let Shape::Step { number, .. } = &mut copy {
            *number = next_step_number(&self.shapes);
        }
        self.shapes.push(copy);
        self.selected = Some(self.shapes.len() - 1);
    }

    // Move the selected object one step toward the front (`up`) or back.
    pub fn reorder_selected(&mut self, up: bool) {
        let Some(i) = self.selected.filter(|i| *i < self.shapes.len()) else {
            return;
        };
        let j = if up { i + 1 } else { i.wrapping_sub(1) };
        if j >= self.shapes.len() {
            return;
        }
        self.snapshot();
        self.shapes.swap(i, j);
        self.selected = Some(j);
    }

    // Select by index and load its style into the panel controls.
    pub fn select(&mut self, index: Option<usize>) {
        self.selected = index;
        if let Some(style) = index
            .and_then(|i| self.shapes.get(i))
            .and_then(|s| s.style())
        {
            self.style = *style;
            // Rotation belongs to the object, not to the panel style.
            self.style.angle = 0.0;
        }
    }

    // Push a new object and select it.
    fn push_shape(&mut self, shape: Shape) {
        self.snapshot();
        self.shapes.push(shape);
        self.selected = Some(self.shapes.len() - 1);
    }

    // Apply the panel's current style to the selected object.
    pub fn apply_style_to_selected(&mut self) {
        let style = self.style;
        if let Some(s) = self
            .selected
            .and_then(|i| self.shapes.get_mut(i))
            .and_then(|s| s.style_mut())
        {
            let angle = s.angle;
            *s = style;
            s.angle = angle;
        }
    }
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
            let rect = shapes::callout_rect(at, &text, style.font_size);
            // Default tail points down-left below the box.
            let tail = egui::pos2(
                rect.min.x + rect.width() * 0.25,
                rect.max.y + style.font_size * 1.2,
            );
            Some(Shape::Callout {
                at,
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
            points: smooth_points(&d.points, 2),
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
        s.style.dashed = true;
        s.apply_style_to_selected();
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
        assert_eq!(s.selected, Some(1));
    }
}
