use eframe::egui::{self, Pos2, Rect, Vec2};

#[derive(Clone, Copy)]
pub enum Part {
    New,
    Move,
    Resize { x: i8, y: i8 },
}

impl Part {
    pub fn cursor(self) -> egui::CursorIcon {
        match self {
            Self::New => egui::CursorIcon::Crosshair,
            Self::Move => egui::CursorIcon::Move,
            Self::Resize { x: 0, .. } => egui::CursorIcon::ResizeVertical,
            Self::Resize { y: 0, .. } => egui::CursorIcon::ResizeHorizontal,
            Self::Resize { x, y } if x == y => egui::CursorIcon::ResizeNwSe,
            Self::Resize { .. } => egui::CursorIcon::ResizeNeSw,
        }
    }
}

pub fn handles(rect: Rect) -> [(Pos2, Part); 8] {
    let mut result = [(Pos2::ZERO, Part::New); 8];
    for (i, (x, y)) in [
        (-1, -1),
        (1, -1),
        (1, 1),
        (-1, 1),
        (0, -1),
        (1, 0),
        (0, 1),
        (-1, 0),
    ]
    .into_iter()
    .enumerate()
    {
        let pos = rect.center() + rect.size() * Vec2::new(x as f32, y as f32) * 0.5;
        result[i] = (pos, Part::Resize { x, y });
    }
    result
}

// Hit testing uses screen points so handles stay easy to grab at every zoom.
pub fn hit_test(rect: Rect, pos: Pos2) -> Part {
    if let Some((_, part)) = handles(rect)
        .into_iter()
        .filter(|(p, _)| p.distance(pos) <= 8.0)
        .min_by(|(a, _), (b, _)| a.distance(pos).total_cmp(&b.distance(pos)))
    {
        return part;
    }
    if rect.expand(5.0).contains(pos) {
        if (pos.x - rect.left()).abs() <= 5.0 {
            return Part::Resize { x: -1, y: 0 };
        }
        if (pos.x - rect.right()).abs() <= 5.0 {
            return Part::Resize { x: 1, y: 0 };
        }
        if (pos.y - rect.top()).abs() <= 5.0 {
            return Part::Resize { x: 0, y: -1 };
        }
        if (pos.y - rect.bottom()).abs() <= 5.0 {
            return Part::Resize { x: 0, y: 1 };
        }
    }
    if rect.contains(pos) {
        Part::Move
    } else {
        Part::New
    }
}

#[derive(Clone, Copy)]
pub struct Drag {
    pub part: Part,
    pub origin: Pos2,
    pub rect: Rect,
}

impl Drag {
    pub fn update(self, pos: Pos2, size: Vec2) -> Rect {
        let pos = Pos2::new(pos.x.clamp(0.0, size.x), pos.y.clamp(0.0, size.y));
        let mut rect = self.rect;
        match self.part {
            Part::New => Rect::from_two_pos(self.origin, pos),
            Part::Move => {
                let delta = pos - self.origin;
                rect.translate(Vec2::new(
                    delta
                        .x
                        .clamp(-rect.min.x, (size.x - rect.max.x).max(-rect.min.x)),
                    delta
                        .y
                        .clamp(-rect.min.y, (size.y - rect.max.y).max(-rect.min.y)),
                ))
            }
            Part::Resize { x, y } => {
                // Preserve the grab offset and keep at least one image pixel.
                let delta = pos - self.origin;
                if x < 0 {
                    rect.min.x = (rect.min.x + delta.x).clamp(0.0, (rect.max.x - 1.0).max(0.0));
                }
                if x > 0 {
                    rect.max.x =
                        (rect.max.x + delta.x).clamp((rect.min.x + 1.0).min(size.x), size.x);
                }
                if y < 0 {
                    rect.min.y = (rect.min.y + delta.y).clamp(0.0, (rect.max.y - 1.0).max(0.0));
                }
                if y > 0 {
                    rect.max.y =
                        (rect.max.y + delta.y).clamp((rect.min.y + 1.0).min(size.y), size.y);
                }
                rect
            }
        }
    }
}
