#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod batch;
mod browser;
mod config;
mod disk_cache;
mod editor;
mod exif;
mod font_loader;
mod image_loader;
mod theme;
mod thumbnail_cache;
mod viewer;

use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([600.0, 400.0])
            .with_title("Image Viewer"),
        ..Default::default()
    };

    eframe::run_native(
        "Image Viewer",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
