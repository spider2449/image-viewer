use image::{DynamicImage, imageops};

#[derive(Clone, Debug)]
pub enum EditOp {
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

#[cfg(test)]
mod tests {
    use super::*;
    use image::DynamicImage;
    use image::GenericImageView;

    #[test]
    fn test_rotate_180() {
        let img = DynamicImage::new_rgba8(10, 20);
        let result = EditOp::Rotate180.apply(&img);
        assert_eq!(result.dimensions(), (10, 20));
    }

    #[test]
    fn test_rotate_90_cw() {
        let img = DynamicImage::new_rgba8(10, 20);
        let result = EditOp::Rotate90Cw.apply(&img);
        assert_eq!(result.dimensions(), (20, 10));
    }

    #[test]
    fn test_rotate_90_ccw() {
        let img = DynamicImage::new_rgba8(10, 20);
        let result = EditOp::Rotate90Ccw.apply(&img);
        assert_eq!(result.dimensions(), (20, 10));
    }

    #[test]
    fn test_flip_horizontal() {
        let img = DynamicImage::new_rgba8(10, 20);
        let result = EditOp::FlipHorizontal.apply(&img);
        assert_eq!(result.dimensions(), (10, 20));
    }

    #[test]
    fn test_flip_vertical() {
        let img = DynamicImage::new_rgba8(10, 20);
        let result = EditOp::FlipVertical.apply(&img);
        assert_eq!(result.dimensions(), (10, 20));
    }

    #[test]
    fn test_resize() {
        let img = DynamicImage::new_rgba8(100, 200);
        let result = EditOp::Resize { width: 50, height: 100 }.apply(&img);
        assert_eq!(result.dimensions(), (50, 100));
    }

    #[test]
    fn test_noop() {
        let img = DynamicImage::new_rgba8(10, 20);
        let result = EditOp::NoOp.apply(&img);
        assert_eq!(result.dimensions(), (10, 20));
    }
}
