use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnWidths {
    pub name: f32,
    pub dimensions: f32,
    pub size: f32,
    pub date: f32,
}

impl Default for ColumnWidths {
    fn default() -> Self {
        Self {
            name: 200.0,
            dimensions: 100.0,
            size: 80.0,
            date: 150.0,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub last_folder: Option<String>,
    pub window_pos: Option<[f32; 2]>,
    pub window_size: Option<[f32; 2]>,
    pub sort_by: String,
    pub sort_descending: bool,
    pub slideshow_interval_secs: u32,
    pub zoom_default: f32,
    pub column_widths: ColumnWidths,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            last_folder: None,
            window_pos: None,
            window_size: None,
            sort_by: "name".to_string(),
            sort_descending: false,
            slideshow_interval_secs: 5,
            zoom_default: 1.0,
            column_widths: ColumnWidths::default(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = Self::path();
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Some(path) = Self::path().parent() {
            std::fs::create_dir_all(path).ok();
        }
        if let Ok(s) = serde_json::to_string_pretty(self) {
            std::fs::write(Self::path(), s).ok();
        }
    }

    fn path() -> PathBuf {
        let mut p = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
        p.pop(); // exe dir
        p.push("cache");
        p.push("config.json");
        p
    }
}
