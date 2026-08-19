pub mod operations;

use crate::app::App;
use eframe::egui;
use operations::Operation;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};

#[derive(Clone, Copy, PartialEq)]
pub enum BatchMode {
    Convert,
    Rename,
    Resize,
}

#[derive(Debug)]
enum Event {
    Started {
        total: usize,
    },
    FileFinished {
        path: PathBuf,
        result: Result<(), String>,
    },
    Finished {
        cancelled: bool,
    },
}

pub struct State {
    pub visible: bool,
    pub mode: BatchMode,
    pub checked: HashSet<PathBuf>,
    pub select_all: bool,
    pub convert_format: &'static str,
    pub jpeg_quality: u8,
    pub rename_pattern: String,
    #[allow(dead_code)]
    pub rename_preview: Vec<(PathBuf, PathBuf)>,
    pub resize_width: u32,
    pub resize_height: u32,
    pub resize_lock_aspect: bool,
    pub running: bool,
    pub progress_current: usize,
    pub progress_total: usize,
    pub log: Vec<String>,
    receiver: Option<mpsc::Receiver<Event>>,
    cancel: Option<Arc<AtomicBool>>,
}

impl State {
    pub fn new() -> Self {
        Self {
            visible: false,
            mode: BatchMode::Convert,
            checked: HashSet::new(),
            select_all: true,
            convert_format: "png",
            jpeg_quality: 90,
            rename_pattern: "{name}_modified".to_string(),
            rename_preview: Vec::new(),
            resize_width: 800,
            resize_height: 600,
            resize_lock_aspect: true,
            running: false,
            progress_current: 0,
            progress_total: 0,
            log: Vec::new(),
            receiver: None,
            cancel: None,
        }
    }

    pub fn open(&mut self, files: &[PathBuf]) {
        self.visible = true;
        self.checked = files.iter().cloned().collect();
        self.select_all = true;
        self.log.clear();
    }

    fn start(&mut self, files: Vec<PathBuf>, operation: Operation) {
        if self.running {
            return;
        }
        let (sender, receiver) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = Arc::clone(&cancel);
        self.running = true;
        self.progress_current = 0;
        self.progress_total = files.len();
        self.log.clear();
        self.receiver = Some(receiver);
        self.cancel = Some(cancel);
        std::thread::spawn(move || run_job(files, operation, worker_cancel, sender));
    }

    fn request_cancel(&self) {
        if let Some(cancel) = &self.cancel {
            cancel.store(true, Ordering::Relaxed);
        }
    }

    fn poll_events(&mut self) -> bool {
        let mut rescan = false;
        let mut disconnected = false;
        if let Some(receiver) = &self.receiver {
            loop {
                match receiver.try_recv() {
                    Ok(Event::Started { total }) => self.progress_total = total,
                    Ok(Event::FileFinished { path, result }) => {
                        self.progress_current += 1;
                        if let Err(error) = result {
                            self.log.push(format!("{}: {error}", path.display()));
                        }
                    }
                    Ok(Event::Finished { cancelled }) => {
                        self.running = false;
                        if cancelled {
                            self.log
                                .push("Cancelled before starting the next file.".to_string());
                        }
                        rescan = true;
                        disconnected = true;
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        if self.running {
                            self.log
                                .push("Batch worker disconnected unexpectedly.".to_string());
                            self.running = false;
                        }
                        rescan = true;
                        disconnected = true;
                        break;
                    }
                }
            }
        }
        if disconnected {
            self.receiver = None;
            self.cancel = None;
        }
        rescan
    }
}

fn run_job(
    files: Vec<PathBuf>,
    operation: Operation,
    cancel: Arc<AtomicBool>,
    sender: mpsc::Sender<Event>,
) {
    let _ = sender.send(Event::Started { total: files.len() });
    let mut cancelled = false;
    for (index, path) in files.into_iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            cancelled = true;
            break;
        }
        let result = operations::process_file(&operation, &path, index);
        if sender.send(Event::FileFinished { path, result }).is_err() {
            return;
        }
    }
    let _ = sender.send(Event::Finished { cancelled });
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    if app.batch_state.poll_events() {
        app.scan_folder();
    }
    if !app.batch_state.visible {
        return;
    }
    if app.batch_state.running {
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
    }

    let colors = app.theme_colors();
    let mut open = true;
    egui::Window::new("Batch Tool")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_size([600.0, 500.0])
        .show(ctx, |ui| {
            let running = app.batch_state.running;
            ui.add_enabled_ui(!running, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut app.batch_state.mode, BatchMode::Convert, "Convert");
                    ui.selectable_value(&mut app.batch_state.mode, BatchMode::Rename, "Rename");
                    ui.selectable_value(&mut app.batch_state.mode, BatchMode::Resize, "Resize");
                });
            });
            ui.separator();

            let files = app.image_files.clone();
            if app.batch_state.checked.is_empty() && app.batch_state.select_all && !running {
                app.batch_state.checked.extend(files.iter().cloned());
            }
            ui.add_enabled_ui(!running, |ui| {
                ui.horizontal(|ui| {
                    if ui.link("Select All").clicked() {
                        app.batch_state.checked.extend(files.iter().cloned());
                        app.batch_state.select_all = true;
                    }
                    ui.separator();
                    if ui.link("Unselect All").clicked() {
                        app.batch_state.checked.clear();
                        app.batch_state.select_all = false;
                    }
                });
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        for path in &files {
                            let name = path
                                .file_name()
                                .map(|n| n.to_string_lossy())
                                .unwrap_or_default();
                            let mut checked = app.batch_state.checked.contains(path);
                            if ui.checkbox(&mut checked, name).changed() {
                                if checked {
                                    app.batch_state.checked.insert(path.clone());
                                } else {
                                    app.batch_state.checked.remove(path);
                                }
                            }
                        }
                    });
            });

            let selected: Vec<PathBuf> = files
                .into_iter()
                .filter(|p| app.batch_state.checked.contains(p))
                .collect();
            ui.colored_label(
                colors.text_secondary,
                format!("{}/{} selected", selected.len(), app.image_files.len()),
            );
            ui.separator();
            let mut operation = None;
            ui.add_enabled_ui(!running, |ui| match app.batch_state.mode {
                BatchMode::Convert => {
                    egui::ComboBox::new("batch_format", "Format")
                        .selected_text(app.batch_state.convert_format)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut app.batch_state.convert_format, "png", "PNG");
                            ui.selectable_value(
                                &mut app.batch_state.convert_format,
                                "jpeg",
                                "JPEG",
                            );
                            ui.selectable_value(&mut app.batch_state.convert_format, "bmp", "BMP");
                            ui.selectable_value(
                                &mut app.batch_state.convert_format,
                                "webp",
                                "WEBP",
                            );
                        });
                    if app.batch_state.convert_format == "jpeg" {
                        ui.add(
                            egui::Slider::new(&mut app.batch_state.jpeg_quality, 1..=100)
                                .text("Quality"),
                        );
                    }
                    if ui.button("Apply").clicked() {
                        operation = Some(Operation::Convert {
                            format: app.batch_state.convert_format.to_string(),
                            jpeg_quality: app.batch_state.jpeg_quality,
                        });
                    }
                }
                BatchMode::Rename => {
                    ui.horizontal(|ui| {
                        ui.label("Pattern:");
                        ui.text_edit_singleline(&mut app.batch_state.rename_pattern);
                    });
                    if ui.button("Apply").clicked() {
                        operation = Some(Operation::Rename {
                            pattern: app.batch_state.rename_pattern.clone(),
                        });
                    }
                }
                BatchMode::Resize => {
                    ui.add(
                        egui::DragValue::new(&mut app.batch_state.resize_width)
                            .range(1..=16384)
                            .prefix("W: "),
                    );
                    ui.add(
                        egui::DragValue::new(&mut app.batch_state.resize_height)
                            .range(1..=16384)
                            .prefix("H: "),
                    );
                    ui.checkbox(&mut app.batch_state.resize_lock_aspect, "Lock aspect ratio");
                    if ui.button("Apply").clicked() {
                        operation = Some(Operation::Resize {
                            width: app.batch_state.resize_width,
                            height: app.batch_state.resize_height,
                            lock_aspect: app.batch_state.resize_lock_aspect,
                        });
                    }
                }
            });
            if let Some(operation) = operation {
                app.batch_state.start(selected, operation);
            }

            if app.batch_state.running {
                let fraction = if app.batch_state.progress_total == 0 {
                    0.0
                } else {
                    app.batch_state.progress_current as f32 / app.batch_state.progress_total as f32
                };
                ui.add(egui::ProgressBar::new(fraction).text(format!(
                    "{} / {}",
                    app.batch_state.progress_current, app.batch_state.progress_total
                )));
                if ui.button("Cancel after current file").clicked() {
                    app.batch_state.request_cancel();
                }
            }
            if !app.batch_state.log.is_empty() {
                ui.separator();
                egui::ScrollArea::vertical()
                    .id_salt("batch_log")
                    .max_height(100.0)
                    .show(ui, |ui| {
                        for line in &app.batch_state.log {
                            ui.label(line);
                        }
                    });
            }
        });
    if !open && !app.batch_state.running {
        app.batch_state.visible = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect_events(files: Vec<PathBuf>, cancel: Arc<AtomicBool>) -> Vec<Event> {
        let (sender, receiver) = mpsc::channel();
        run_job(
            files,
            Operation::Convert {
                format: "png".to_string(),
                jpeg_quality: 90,
            },
            cancel,
            sender,
        );
        receiver.into_iter().collect()
    }

    #[test]
    fn test_empty_job_starts_and_finishes() {
        let events = collect_events(Vec::new(), Arc::new(AtomicBool::new(false)));
        assert!(matches!(
            events.as_slice(),
            [
                Event::Started { total: 0 },
                Event::Finished { cancelled: false }
            ]
        ));
    }

    #[test]
    fn test_mixed_results_emit_one_event_per_file_and_finish() {
        let dir = std::env::temp_dir().join("batch_event_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let valid = dir.join("valid.jpg");
        image::DynamicImage::new_rgb8(2, 2).save(&valid).unwrap();
        let missing = dir.join("missing.jpg");
        let events = collect_events(vec![valid, missing], Arc::new(AtomicBool::new(false)));
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, Event::FileFinished { .. }))
                .count(),
            2
        );
        assert!(matches!(
            events.last(),
            Some(Event::Finished { cancelled: false })
        ));
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::FileFinished { result: Err(_), .. })));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_pre_cancelled_job_stops_before_first_file() {
        let events = collect_events(
            vec![PathBuf::from("never.png")],
            Arc::new(AtomicBool::new(true)),
        );
        assert!(!events
            .iter()
            .any(|e| matches!(e, Event::FileFinished { .. })));
        assert!(matches!(
            events.last(),
            Some(Event::Finished { cancelled: true })
        ));
    }


    #[test]
    fn test_ui_state_stops_running_after_worker_finishes() {
        let mut state = State::new();
        state.start(
            Vec::new(),
            Operation::Convert {
                format: "png".to_string(),
                jpeg_quality: 90,
            },
        );
        for _ in 0..100 {
            state.poll_events();
            if !state.running {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(!state.running);
    }
}
