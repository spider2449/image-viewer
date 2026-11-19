use eframe::egui::{self, ColorImage, Context, Image, Rect, TextureHandle, Ui, Vec2};

const LOGO: &[u8] = include_bytes!("../assets/logo.png");
const ATLAS: &[u8] = include_bytes!("../assets/icons.png");

fn pixels(bytes: &[u8]) -> image::RgbaImage {
    image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .expect("valid embedded artwork")
        .into_rgba8()
}

pub fn window_icon() -> egui::IconData {
    let image = pixels(LOGO);
    egui::IconData {
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
    }
}

fn texture(ctx: &Context, name: &'static str, bytes: &[u8]) -> TextureHandle {
    let id = egui::Id::new(name);
    if let Some(handle) = ctx.data(|d| d.get_temp::<TextureHandle>(id)) {
        return handle;
    }
    let rgba = pixels(bytes);
    let size = [rgba.width() as usize, rgba.height() as usize];
    let handle = ctx.load_texture(
        name,
        ColorImage::from_rgba_unmultiplied(size, &rgba),
        egui::TextureOptions::LINEAR,
    );
    ctx.data_mut(|d| d.insert_temp(id, handle.clone()));
    handle
}

pub fn logo(ui: &mut Ui, size: f32) {
    let tex = texture(ui.ctx(), "image-viewer-logo", LOGO);
    ui.image((tex.id(), Vec2::splat(size)));
}

// Row-major cells in the 8 by 4 artwork sheet, assets/icons.svg.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Folder,
    Document,
    Photo,
    Up,
    List,
    Refresh,
    Prev,
    Next,
    Back,
    Fullscreen,
    Play,
    Stop,
    Down,
    Error,
    Select,
    Pencil,
    Line,
    Arrow,
    Rect,
    Ellipse,
    Highlight,
    Text,
    Callout,
    Step,
    BlurRegion,
    Undo,
    Redo,
    Save,
    Adjust,
    Crop,
    Wand,
    Loading,
}

impl Icon {
    pub fn image(self, ui: &Ui, size: f32) -> Image<'static> {
        let tex = texture(ui.ctx(), "image-viewer-icons", ATLAS);
        let index = self as usize;
        let cell = egui::vec2(1.0 / 8.0, 1.0 / 4.0);
        let uv = Rect::from_min_size(
            egui::pos2((index % 8) as f32 * cell.x, (index / 8) as f32 * cell.y),
            cell,
        );
        Image::new((tex.id(), Vec2::splat(size))).uv(uv)
    }

    pub fn paint(self, ui: &Ui, center: egui::Pos2, size: f32) {
        self.image(ui, size)
            .paint_at(ui, Rect::from_center_size(center, Vec2::splat(size)));
    }

    pub fn button(self, ui: &Ui, label: &str) -> egui::Button<'static> {
        egui::Button::image_and_text(self.image(ui, 16.0), label.to_owned())
    }
}
