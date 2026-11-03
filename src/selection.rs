// Pixel selections: the Magic Wand fill, mask arithmetic and the selection outline.

use crate::annotate::ImgRect;
use eframe::egui;
use image::DynamicImage;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock};

// Beyond this many outline edges (a very noisy selection) marching ants would be too
// slow to rebuild while animating, so the selection is shown as a static tint.
const MAX_ANT_SEGMENTS: usize = 100_000;
// On + off length of one marching-ants dash, in points.
const ANT_PERIOD: f32 = 8.0;
pub const ANT_FRAME_SECONDS: f64 = 0.1;

// Tunable Magic Wand settings, saved in config.json. The aliases keep the values of
// config files written when this was still called Auto Region.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WandParams {
    // Largest per-channel difference still counted as similar to the clicked pixel.
    pub tolerance: u8,
    // Select only pixels connected to the clicked one, else every similar pixel.
    pub contiguous: bool,
    // Compare each pixel with its neighbour instead of the clicked pixel, so the
    // fill can walk along a soft gradient. Contiguous only.
    pub follow_gradient: bool,
    // Let the fill cross diagonal pixel corners (8-way instead of 4-way). Contiguous only.
    pub diagonal: bool,
    // Grow (positive) or shrink (negative) each click's selection, in image px.
    #[serde(alias = "padding")]
    pub expand: i32,
}

impl Default for WandParams {
    fn default() -> Self {
        Self {
            tolerance: 32,
            contiguous: true,
            follow_gradient: false,
            diagonal: false,
            expand: 0,
        }
    }
}

// Which pixels are selected. The GPU copy used for painting is created on first use
// and released together with the mask.
pub struct PixelMask {
    width: usize,
    height: usize,
    bits: Vec<bool>,
    texture: OnceLock<egui::TextureHandle>,
}

impl PartialEq for PixelMask {
    fn eq(&self, other: &Self) -> bool {
        self.width == other.width && self.height == other.height && self.bits == other.bits
    }
}

impl std::fmt::Debug for PixelMask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PixelMask({}x{})", self.width, self.height)
    }
}

impl PixelMask {
    pub(crate) fn new(width: usize, height: usize, bits: Vec<bool>) -> Self {
        debug_assert_eq!(bits.len(), width * height);
        Self {
            width,
            height,
            bits,
            texture: OnceLock::new(),
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn contains(&self, x: usize, y: usize) -> bool {
        self.bits[y * self.width + x]
    }

    fn count(&self) -> usize {
        self.bits.iter().filter(|b| **b).count()
    }

    // Smallest box around the selected pixels as (x, y, width, height).
    fn bounds(&self) -> Option<(u32, u32, u32, u32)> {
        let (mut min_x, mut min_y, mut max_x, mut max_y) = (self.width, self.height, 0, 0);
        for (i, _) in self.bits.iter().enumerate().filter(|(_, b)| **b) {
            let (x, y) = (i % self.width, i / self.width);
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
        (min_x <= max_x && min_y <= max_y).then(|| {
            (
                min_x as u32,
                min_y as u32,
                (max_x - min_x + 1) as u32,
                (max_y - min_y + 1) as u32,
            )
        })
    }

    fn crop(&self, (x, y, w, h): (u32, u32, u32, u32)) -> PixelMask {
        let (x, y, w, h) = (x as usize, y as usize, w as usize, h as usize);
        let mut bits = Vec::with_capacity(w * h);
        for row in y..y + h {
            let start = row * self.width + x;
            bits.extend_from_slice(&self.bits[start..start + w]);
        }
        PixelMask::new(w, h, bits)
    }

    fn inverted(&self) -> PixelMask {
        PixelMask::new(
            self.width,
            self.height,
            self.bits.iter().map(|b| !*b).collect(),
        )
    }

    fn merged(&self, other: &PixelMask, op: impl Fn(bool, bool) -> bool) -> PixelMask {
        let bits = self
            .bits
            .iter()
            .zip(&other.bits)
            .map(|(a, b)| op(*a, *b))
            .collect();
        PixelMask::new(self.width, self.height, bits)
    }

    // Grow by `amount` px on every side, or shrink when negative. Pixels on the image
    // edge are not eaten by shrinking, since nothing unselected lies beyond them.
    fn expanded(self, amount: i32) -> PixelMask {
        match amount {
            0 => self,
            a if a > 0 => self.dilated(a as usize),
            a => self.inverted().dilated(a.unsigned_abs() as usize).inverted(),
        }
    }

    // Square dilation, one sliding-window pass per axis, so a large radius costs the
    // same as a small one.
    fn dilated(&self, radius: usize) -> PixelMask {
        let (w, h) = (self.width, self.height);
        let mut across = vec![false; w * h];
        for y in 0..h {
            spread_line(w, radius, |x| self.bits[y * w + x], |x| across[y * w + x] = true);
        }
        let mut bits = vec![false; w * h];
        for x in 0..w {
            spread_line(h, radius, |y| across[y * w + x], |y| bits[y * w + x] = true);
        }
        PixelMask::new(w, h, bits)
    }

    // Whether the mask, stretched over `rect`, covers the image-space point (x, y).
    // Nearest sampling, matching the texture filter used on screen.
    pub fn sample(&self, rect: ImgRect, x: f32, y: f32) -> bool {
        if rect.width() <= 0.0 || rect.height() <= 0.0 {
            return false;
        }
        let u = (x - rect.min.x) / rect.width();
        let v = (y - rect.min.y) / rect.height();
        if !(0.0..1.0).contains(&u) || !(0.0..1.0).contains(&v) {
            return false;
        }
        let mx = ((u * self.width as f32) as usize).min(self.width - 1);
        let my = ((v * self.height as f32) as usize).min(self.height - 1);
        self.bits[my * self.width + mx]
    }

    // Boundary between selected and unselected pixels as axis-aligned segments in
    // image px, with collinear neighbours merged.
    fn outline(&self) -> Vec<(egui::Pos2, egui::Pos2)> {
        let (w, h) = (self.width, self.height);
        let on = |x: isize, y: isize| {
            x >= 0
                && y >= 0
                && (x as usize) < w
                && (y as usize) < h
                && self.bits[y as usize * w + x as usize]
        };
        let mut segments = Vec::new();
        // Horizontal edges lie on the line above row `y`.
        for y in 0..=h as isize {
            let mut start = None;
            for x in 0..=w as isize {
                let edge = x < w as isize && on(x, y - 1) != on(x, y);
                push_run(&mut start, edge, x, |a, b| {
                    segments.push((
                        egui::pos2(a as f32, y as f32),
                        egui::pos2(b as f32, y as f32),
                    ));
                });
            }
        }
        // Vertical edges lie on the line left of column `x`.
        for x in 0..=w as isize {
            let mut start = None;
            for y in 0..=h as isize {
                let edge = y < h as isize && on(x - 1, y) != on(x, y);
                push_run(&mut start, edge, y, |a, b| {
                    segments.push((
                        egui::pos2(x as f32, a as f32),
                        egui::pos2(x as f32, b as f32),
                    ));
                });
            }
        }
        segments
    }

    pub fn texture(&self, ctx: &egui::Context) -> &egui::TextureHandle {
        self.texture.get_or_init(|| {
            let pixels = self
                .bits
                .iter()
                .map(|b| {
                    if *b {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::TRANSPARENT
                    }
                })
                .collect();
            let image = egui::ColorImage {
                size: [self.width, self.height],
                pixels,
            };
            ctx.load_texture("selection_mask", image, egui::TextureOptions::NEAREST)
        })
    }
}

// Mark every position within `radius` of a set position along one row or column.
fn spread_line(
    len: usize,
    radius: usize,
    is_set: impl Fn(usize) -> bool,
    mut mark: impl FnMut(usize),
) {
    // One past the last position any set position so far reaches.
    let mut reach = 0;
    for i in 0..len {
        if is_set(i) {
            for j in i.saturating_sub(radius).max(reach)..(i + radius + 1).min(len) {
                mark(j);
            }
            reach = (i + radius + 1).min(len);
        }
    }
}

// Close the open run of edge cells when the current cell is not part of it.
fn push_run(
    start: &mut Option<isize>,
    edge: bool,
    at: isize,
    mut emit: impl FnMut(isize, isize),
) {
    match (*start, edge) {
        (None, true) => *start = Some(at),
        (Some(a), false) => {
            emit(a, at);
            *start = None;
        }
        _ => {}
    }
}

// What a wand click does to the existing selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WandMode {
    Replace,
    Add,
    Subtract,
}

// The pixels a wand click at `seed` picks up.
fn wand_fill(image: &DynamicImage, seed: (u32, u32), params: &WandParams) -> Option<PixelMask> {
    let rgba = image.to_rgba8();
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
    if w == 0 || h == 0 {
        return None;
    }
    let px = rgba.as_raw();
    let start = (seed.1 as usize).min(h - 1) * w + (seed.0 as usize).min(w - 1);
    let similar = |a: usize, b: usize| {
        px[a * 4..a * 4 + 4]
            .iter()
            .zip(&px[b * 4..b * 4 + 4])
            .all(|(a, b)| a.abs_diff(*b) <= params.tolerance)
    };

    let bits = if params.contiguous {
        let mut visited = vec![false; w * h];
        visited[start] = true;
        let mut stack = vec![start];
        while let Some(i) = stack.pop() {
            let (x, y) = (i % w, i / w);
            let reference = if params.follow_gradient { i } else { start };
            for (dx, dy) in NEIGHBOURS {
                if !params.diagonal && dx != 0 && dy != 0 {
                    continue;
                }
                let (Some(nx), Some(ny)) = (x.checked_add_signed(dx), y.checked_add_signed(dy))
                else {
                    continue;
                };
                if nx >= w || ny >= h {
                    continue;
                }
                let n = ny * w + nx;
                if !visited[n] && similar(n, reference) {
                    visited[n] = true;
                    stack.push(n);
                }
            }
        }
        visited
    } else {
        (0..w * h).map(|i| similar(i, start)).collect()
    };
    Some(PixelMask::new(w, h, bits).expanded(params.expand))
}

const NEIGHBOURS: [(isize, isize); 8] = [
    (-1, 0),
    (1, 0),
    (0, -1),
    (0, 1),
    (-1, -1),
    (1, -1),
    (-1, 1),
    (1, 1),
];

// A non-empty selection with the summary the UI reads every frame.
pub struct Selection {
    pub mask: Arc<PixelMask>,
    pub outline: Vec<(egui::Pos2, egui::Pos2)>,
    pub count: usize,
    pub bounds: (u32, u32, u32, u32),
}

impl Selection {
    fn new(mask: PixelMask) -> Option<Self> {
        let bounds = mask.bounds()?;
        Some(Self {
            count: mask.count(),
            bounds,
            outline: mask.outline(),
            mask: Arc::new(mask),
        })
    }

    pub fn bounds_rect(&self) -> ImgRect {
        let (x, y, w, h) = self.bounds;
        ImgRect {
            min: egui::pos2(x as f32, y as f32),
            max: egui::pos2((x + w) as f32, (y + h) as f32),
        }
    }

    // The mask for an annotation object covering the bounds. None means the selection
    // is a solid rectangle, which needs no mask.
    pub fn object_mask(&self) -> Option<Arc<PixelMask>> {
        let (_, _, w, h) = self.bounds;
        (self.count != w as usize * h as usize).then(|| Arc::new(self.mask.crop(self.bounds)))
    }

    pub fn animates(&self) -> bool {
        self.outline.len() <= MAX_ANT_SEGMENTS
    }
}

#[derive(Default)]
pub struct MagicWand {
    pub active: bool,
    pub selection: Option<Selection>,
}

impl MagicWand {
    pub fn click(
        &mut self,
        image: &DynamicImage,
        seed: (u32, u32),
        params: &WandParams,
        mode: WandMode,
    ) {
        let Some(fresh) = wand_fill(image, seed, params) else {
            return;
        };
        // A selection from an image of another size cannot be combined with.
        let current = self
            .selection
            .as_ref()
            .map(|s| &*s.mask)
            .filter(|m| m.width == fresh.width && m.height == fresh.height);
        let combined = match (mode, current) {
            (WandMode::Add, Some(current)) => current.merged(&fresh, |a, b| a || b),
            (WandMode::Subtract, Some(current)) => current.merged(&fresh, |a, b| a && !b),
            (WandMode::Subtract, None) => return,
            (WandMode::Replace | WandMode::Add, _) => fresh,
        };
        self.selection = Selection::new(combined);
    }

    pub fn invert(&mut self) {
        if let Some(selection) = self.selection.take() {
            self.selection = Selection::new(selection.mask.inverted());
        }
    }
}

// Paint a mask stretched over a screen rect in one color.
pub fn paint_mask(
    painter: &egui::Painter,
    mask: &PixelMask,
    rect: egui::Rect,
    color: egui::Color32,
) {
    let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    painter.image(mask.texture(painter.ctx()).id(), rect, uv, color);
}

// Paint the live selection: marching ants, or a static tint when the outline is too
// detailed to animate. `screen` is the image's rect on screen.
pub fn paint_selection(
    painter: &egui::Painter,
    selection: &Selection,
    screen: egui::Rect,
    to_screen: impl Fn(egui::Pos2) -> egui::Pos2,
) {
    if !selection.animates() {
        let tint = egui::Color32::from_rgba_unmultiplied(80, 160, 255, 110);
        paint_mask(painter, &selection.mask, screen, tint);
        return;
    }
    let ctx = painter.ctx();
    let ppp = ctx.pixels_per_point();
    let thickness = 1.0 / ppp;
    let snap = |v: f32| (v * ppp).round() / ppp;
    let frame = (ctx.input(|i| i.time) / ANT_FRAME_SECONDS) as u64 % ANT_PERIOD as u64;
    let phase = frame as f32;
    let clip = painter.clip_rect();

    let mut mesh = egui::Mesh::default();
    for (a, b) in &selection.outline {
        let horizontal = a.y == b.y;
        let (a, b) = (to_screen(*a), to_screen(*b));
        // `lo..hi` runs along the segment; `line` is its fixed coordinate.
        let (lo, hi, line, clip_lo, clip_hi) = if horizontal {
            (a.x, b.x, snap(a.y), clip.min.x, clip.max.x)
        } else {
            (a.y, b.y, snap(a.x), clip.min.y, clip.max.y)
        };
        let (line_lo, line_hi) = if horizontal {
            (clip.min.y, clip.max.y)
        } else {
            (clip.min.x, clip.max.x)
        };
        if hi < clip_lo || lo > clip_hi || line < line_lo || line > line_hi {
            continue;
        }
        let lo = snap(lo.max(clip_lo));
        let hi = snap(hi.min(clip_hi)).max(lo + thickness);
        let bar = |from: f32, to: f32| {
            if horizontal {
                egui::Rect::from_min_max(egui::pos2(from, line), egui::pos2(to, line + thickness))
            } else {
                egui::Rect::from_min_max(egui::pos2(line, from), egui::pos2(line + thickness, to))
            }
        };
        mesh.add_colored_rect(bar(lo, hi), egui::Color32::WHITE);
        // Dashes depend on the absolute screen position, so they line up across the
        // many short segments of a jagged edge.
        let offset = phase + line;
        let mut dash = ((lo + offset) / ANT_PERIOD).floor() * ANT_PERIOD - offset;
        while dash < hi {
            let (from, to) = (dash.max(lo), (dash + ANT_PERIOD / 2.0).min(hi));
            if to > from {
                mesh.add_colored_rect(bar(from, to), egui::Color32::BLACK);
            }
            dash += ANT_PERIOD;
        }
    }
    painter.add(egui::Shape::mesh(mesh));
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    // 20x20 white image with a black outlined box whose inside is grey.
    fn boxed_image() -> DynamicImage {
        DynamicImage::ImageRgba8(RgbaImage::from_fn(20, 20, |x, y| {
            let inside = (5..15).contains(&x) && (4..12).contains(&y);
            let border = inside && (x == 5 || x == 14 || y == 4 || y == 11);
            Rgba(match (inside, border) {
                (_, true) => [0, 0, 0, 255],
                (true, false) => [120, 120, 120, 255],
                _ => [255, 255, 255, 255],
            })
        }))
    }

    // Two separate black squares on white.
    fn two_squares() -> DynamicImage {
        DynamicImage::ImageRgba8(RgbaImage::from_fn(20, 10, |x, y| {
            let on = (2..5).contains(&y) && ((2..5).contains(&x) || (10..13).contains(&x));
            Rgba(if on {
                [0, 0, 0, 255]
            } else {
                [255, 255, 255, 255]
            })
        }))
    }

    fn with_tolerance(tolerance: u8) -> WandParams {
        WandParams {
            tolerance,
            ..WandParams::default()
        }
    }

    fn fill(image: &DynamicImage, seed: (u32, u32), params: &WandParams) -> PixelMask {
        wand_fill(image, seed, params).unwrap()
    }

    fn click(wand: &mut MagicWand, image: &DynamicImage, seed: (u32, u32), mode: WandMode) {
        wand.click(image, seed, &with_tolerance(10), mode);
    }

    #[test]
    fn fill_selects_the_exact_pixels_not_their_bounding_box() {
        // An L-shaped area: its bounds also contain unselected pixels.
        let image = DynamicImage::ImageRgba8(RgbaImage::from_fn(10, 10, |x, y| {
            let on = x == 2 || y == 8;
            Rgba(if on { [0, 0, 0, 255] } else { [255, 255, 255, 255] })
        }));
        let mask = fill(&image, (2, 2), &with_tolerance(10));
        assert_eq!(mask.bounds(), Some((0, 0, 10, 10)));
        assert_eq!(mask.count(), 19);
        assert!(mask.bits[2 * 10 + 2]);
        assert!(!mask.bits[2 * 10 + 5]);
    }

    #[test]
    fn fill_stops_at_the_outline_and_tolerance_decides_what_merges() {
        let image = boxed_image();
        assert_eq!(fill(&image, (9, 8), &with_tolerance(10)).bounds(), Some((6, 5, 8, 6)));
        // Inside and border differ by 120: a high tolerance swallows the border.
        assert_eq!(fill(&image, (9, 8), &with_tolerance(130)).bounds(), Some((5, 4, 10, 8)));
    }

    #[test]
    fn open_background_and_a_single_pixel_are_valid_selections() {
        let image = boxed_image();
        let background = fill(&image, (0, 0), &with_tolerance(10));
        assert_eq!(background.bounds(), Some((0, 0, 20, 20)));
        // The box is not part of the background.
        assert!(!background.bits[8 * 20 + 9]);
        let speck = DynamicImage::ImageRgba8(RgbaImage::from_fn(20, 20, |x, y| {
            Rgba(if (x, y) == (9, 9) {
                [0, 0, 0, 255]
            } else {
                [255, 255, 255, 255]
            })
        }));
        assert_eq!(fill(&speck, (9, 9), &with_tolerance(10)).count(), 1);
    }

    #[test]
    fn contiguous_off_selects_every_similar_pixel() {
        let image = two_squares();
        assert_eq!(fill(&image, (3, 3), &with_tolerance(10)).count(), 9);
        let global = WandParams {
            contiguous: false,
            ..with_tolerance(10)
        };
        assert_eq!(fill(&image, (3, 3), &global).count(), 18);
    }

    #[test]
    fn diagonal_setting_crosses_pixel_corners() {
        let image = DynamicImage::ImageRgba8(RgbaImage::from_fn(20, 20, |x, y| {
            Rgba(if x == y && (5..12).contains(&x) {
                [0, 0, 0, 255]
            } else {
                [255, 255, 255, 255]
            })
        }));
        assert_eq!(fill(&image, (5, 5), &with_tolerance(10)).count(), 1);
        let diagonal = WandParams {
            diagonal: true,
            ..with_tolerance(10)
        };
        assert_eq!(fill(&image, (5, 5), &diagonal).count(), 7);
    }

    #[test]
    fn follow_gradient_walks_a_soft_ramp() {
        let image = DynamicImage::ImageRgba8(RgbaImage::from_fn(20, 20, |x, _| {
            let v = if x < 10 { (x * 10) as u8 } else { 255 };
            Rgba([v, v, v, 255])
        }));
        assert_eq!(fill(&image, (0, 0), &with_tolerance(15)).bounds(), Some((0, 0, 2, 20)));
        let follow = WandParams {
            follow_gradient: true,
            ..with_tolerance(15)
        };
        assert_eq!(fill(&image, (0, 0), &follow).bounds(), Some((0, 0, 10, 20)));
    }

    #[test]
    fn seed_on_the_far_edge_is_clamped() {
        let image = DynamicImage::ImageRgba8(RgbaImage::from_fn(10, 10, |x, _| {
            Rgba(if x >= 6 {
                [0, 0, 0, 255]
            } else {
                [255, 255, 255, 255]
            })
        }));
        assert_eq!(fill(&image, (10, 10), &with_tolerance(10)).bounds(), Some((6, 0, 4, 10)));
    }

    #[test]
    fn expand_grows_shrinks_and_keeps_the_image_edge() {
        let expanded = |expand| WandParams {
            expand,
            ..with_tolerance(10)
        };
        let image = boxed_image();
        assert_eq!(fill(&image, (9, 8), &expanded(2)).bounds(), Some((4, 3, 12, 10)));
        assert_eq!(fill(&image, (9, 8), &expanded(-1)).bounds(), Some((7, 6, 6, 4)));
        assert_eq!(fill(&image, (9, 8), &expanded(-10)).bounds(), None);
        assert_eq!(fill(&image, (9, 8), &expanded(50)).bounds(), Some((0, 0, 20, 20)));
        // The left half touches the image edge, which shrinking leaves alone.
        let half = DynamicImage::ImageRgba8(RgbaImage::from_fn(10, 10, |x, _| {
            Rgba(if x < 5 {
                [0, 0, 0, 255]
            } else {
                [255, 255, 255, 255]
            })
        }));
        assert_eq!(fill(&half, (0, 0), &expanded(-1)).bounds(), Some((0, 0, 4, 10)));
    }

    #[test]
    fn shift_adds_and_alt_subtracts() {
        let image = two_squares();
        let mut wand = MagicWand::default();
        click(&mut wand, &image, (3, 3), WandMode::Replace);
        assert_eq!(wand.selection.as_ref().unwrap().count, 9);

        click(&mut wand, &image, (11, 3), WandMode::Add);
        let selection = wand.selection.as_ref().unwrap();
        assert_eq!(selection.count, 18);
        assert_eq!(selection.bounds, (2, 2, 11, 3));

        click(&mut wand, &image, (3, 3), WandMode::Subtract);
        assert_eq!(wand.selection.as_ref().unwrap().bounds, (10, 2, 3, 3));

        click(&mut wand, &image, (11, 3), WandMode::Replace);
        assert_eq!(wand.selection.as_ref().unwrap().bounds, (10, 2, 3, 3));

        // Subtracting everything leaves no selection at all.
        click(&mut wand, &image, (11, 3), WandMode::Subtract);
        assert!(wand.selection.is_none());
        click(&mut wand, &image, (11, 3), WandMode::Subtract);
        assert!(wand.selection.is_none());
    }

    #[test]
    fn invert_swaps_selected_and_unselected() {
        let image = two_squares();
        let mut wand = MagicWand::default();
        click(&mut wand, &image, (3, 3), WandMode::Replace);
        wand.invert();
        assert_eq!(wand.selection.as_ref().unwrap().count, 20 * 10 - 9);
        // Inverting a full selection selects nothing.
        let solid = DynamicImage::ImageRgba8(RgbaImage::from_pixel(4, 4, Rgba([9, 9, 9, 255])));
        click(&mut wand, &solid, (0, 0), WandMode::Replace);
        wand.invert();
        assert!(wand.selection.is_none());
    }

    #[test]
    fn a_selection_from_another_image_size_is_replaced_not_combined() {
        let mut wand = MagicWand::default();
        click(&mut wand, &two_squares(), (3, 3), WandMode::Replace);
        let solid = DynamicImage::ImageRgba8(RgbaImage::from_pixel(4, 4, Rgba([9, 9, 9, 255])));
        click(&mut wand, &solid, (0, 0), WandMode::Add);
        assert_eq!(wand.selection.as_ref().unwrap().count, 16);
    }

    #[test]
    fn object_mask_is_cropped_to_bounds_and_skipped_for_solid_rectangles() {
        let image = two_squares();
        let mut wand = MagicWand::default();
        click(&mut wand, &image, (3, 3), WandMode::Replace);
        assert!(wand.selection.as_ref().unwrap().object_mask().is_none());

        click(&mut wand, &image, (11, 3), WandMode::Add);
        let selection = wand.selection.as_ref().unwrap();
        let mask = selection.object_mask().unwrap();
        assert_eq!((mask.width(), mask.height()), (11, 3));
        let rect = selection.bounds_rect();
        assert!(mask.sample(rect, 2.5, 2.5));
        assert!(!mask.sample(rect, 7.5, 2.5));
        assert!(mask.sample(rect, 12.5, 4.5));
        assert!(!mask.sample(rect, 13.5, 4.5));
    }

    #[test]
    fn sample_follows_a_moved_and_resized_rect() {
        let mask = PixelMask::new(2, 1, vec![true, false]);
        let rect = ImgRect {
            min: egui::pos2(100.0, 100.0),
            max: egui::pos2(140.0, 110.0),
        };
        assert!(mask.sample(rect, 119.0, 105.0));
        assert!(!mask.sample(rect, 121.0, 105.0));
        assert!(!mask.sample(rect, 99.0, 105.0));
    }

    #[test]
    fn outline_traces_a_merged_boundary() {
        // A single selected square gives exactly its four sides.
        let image = two_squares();
        let mut wand = MagicWand::default();
        click(&mut wand, &image, (3, 3), WandMode::Replace);
        let mut outline = wand.selection.as_ref().unwrap().outline.clone();
        outline.sort_by(|a, b| {
            (a.0.y, a.0.x, a.1.x)
                .partial_cmp(&(b.0.y, b.0.x, b.1.x))
                .unwrap()
        });
        assert_eq!(
            outline,
            vec![
                (egui::pos2(2.0, 2.0), egui::pos2(2.0, 5.0)),
                (egui::pos2(2.0, 2.0), egui::pos2(5.0, 2.0)),
                (egui::pos2(5.0, 2.0), egui::pos2(5.0, 5.0)),
                (egui::pos2(2.0, 5.0), egui::pos2(5.0, 5.0)),
            ]
        );
    }

    // Run one headless frame around `paint` and return what it drew.
    fn painted_meshes(
        paint: impl Fn(&egui::Painter),
    ) -> (Vec<egui::epaint::Mesh>, egui::TexturesDelta) {
        let ctx = egui::Context::default();
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| paint(ui.painter()));
        });
        let meshes = output
            .shapes
            .into_iter()
            .filter_map(|clipped| match clipped.shape {
                egui::Shape::Mesh(mesh) => Some((*mesh).clone()),
                _ => None,
            })
            .collect();
        (meshes, output.textures_delta)
    }

    #[test]
    fn marching_ants_cover_the_outline_in_black_and_white() {
        let mut wand = MagicWand::default();
        click(&mut wand, &two_squares(), (3, 3), WandMode::Replace);
        let selection = wand.selection.as_ref().unwrap();
        let screen = egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(200.0, 100.0));
        let (meshes, _) = painted_meshes(|painter| {
            let to_screen = |p: egui::Pos2| screen.min + p.to_vec2() * 10.0;
            paint_selection(painter, selection, screen, to_screen);
        });
        let mesh = meshes.iter().find(|m| !m.vertices.is_empty()).unwrap();
        let has = |color| mesh.vertices.iter().any(|v| v.color == color);
        assert!(has(egui::Color32::WHITE));
        assert!(has(egui::Color32::BLACK));
        // The selected square spans screen (30, 30) to (60, 60).
        let bounds = mesh.calc_bounds();
        assert!((bounds.min.x - 30.0).abs() <= 1.0 && (bounds.min.y - 30.0).abs() <= 1.0);
        assert!((bounds.max.x - 60.0).abs() <= 2.0 && (bounds.max.y - 60.0).abs() <= 2.0);
    }

    #[test]
    fn paint_mask_uploads_its_texture_once() {
        let mask = PixelMask::new(2, 1, vec![true, false]);
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(20.0, 10.0));
        let (meshes, textures) = painted_meshes(|painter| {
            paint_mask(painter, &mask, rect, egui::Color32::RED);
            paint_mask(painter, &mask, rect, egui::Color32::BLUE);
        });
        assert_eq!(meshes.len(), 2);
        let id = mask.texture.get().unwrap().id();
        assert!(meshes.iter().all(|m| m.texture_id == id));
        assert_eq!(textures.set.iter().filter(|(t, _)| *t == id).count(), 1);
    }

    #[test]
    fn dilation_marks_overlapping_windows_without_gaps() {
        let mut bits = vec![false; 12];
        bits[1] = true;
        bits[3] = true;
        bits[10] = true;
        let mask = PixelMask::new(12, 1, bits).dilated(2);
        let expected: Vec<bool> = (0..12).map(|x| x <= 5 || x >= 8).collect();
        assert_eq!(mask.bits, expected);
    }
}
