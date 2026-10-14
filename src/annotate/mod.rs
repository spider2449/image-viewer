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
    // BlurRegion tool effect: false = Blur, true = Mosaic.
    pub mosaic: bool,
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
        // Text commits from the panel input, not from drag size.
        Tool::Text => true,
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
            points: d.points,
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
        Tool::Text => {
            if state.text_buffer.trim().is_empty() {
                return;
            }
            Shape::Text {
                at: d.start,
                text: state.text_buffer.clone(),
                style,
            }
        }
    };
    state.shapes.push(shape);
    state.selected = Some(state.shapes.len() - 1);
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn mosaic_flag_selects_mosaic_region() {
        let mut s = AnnotateState::new();
        s.active = true;
        s.tool = Tool::BlurRegion;
        s.mosaic = true;
        s.draft = Some(Draft {
            tool: Tool::BlurRegion,
            start: egui::pos2(0.0, 0.0),
            current: egui::pos2(20.0, 20.0),
            points: vec![],
        });
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
}
