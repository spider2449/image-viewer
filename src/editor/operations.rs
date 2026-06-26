use image::{DynamicImage, imageops};

#[derive(Clone, Debug)]
pub enum EditOp {
    #[allow(dead_code)]
    Crop { x: u32, y: u32, width: u32, height: u32 },
    Rotate180,
    Rotate90Cw,
    Rotate90Ccw,
    FlipHorizontal,
    FlipVertical,
    Resize { width: u32, height: u32 },
    #[allow(dead_code)]
    NoOp,
}

impl EditOp {
    #[allow(dead_code)]
    pub fn label(&self) -> &str {
        match self {
            EditOp::Crop { .. } => "Crop",
            EditOp::Rotate180 => "Rotate 180\u{00B0}",
            EditOp::Rotate90Cw => "Rotate 90\u{00B0} CW",
            EditOp::Rotate90Ccw => "Rotate 90\u{00B0} CCW",
            EditOp::FlipHorizontal => "Flip H",
            EditOp::FlipVertical => "Flip V",
            EditOp::Resize { .. } => "Resize",
            EditOp::NoOp => "",
        }
    }

    pub fn apply(&self, img: &DynamicImage) -> DynamicImage {
        match self {
            EditOp::Crop { x, y, width, height } => {
                img.crop_imm(*x, *y, *width, *height)
            }
            EditOp::Rotate180 => img.rotate180(),
            EditOp::Rotate90Cw => img.rotate90(),
            EditOp::Rotate90Ccw => img.rotate270(),
            EditOp::FlipHorizontal => img.fliph(),
            EditOp::FlipVertical => img.flipv(),
            EditOp::Resize { width, height } => {
                img.resize_exact(*width, *height, imageops::FilterType::Lanczos3)
            }
            EditOp::NoOp => img.clone(),
        }
    }
}
