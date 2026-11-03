use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn default_theme() -> String {
    "dark".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnWidths {
    pub name: f32,
    pub size: f32,
    pub date: f32,
}

impl Default for ColumnWidths {
    fn default() -> Self {
        Self {
            name: 200.0,
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
    pub thumb_size: f32,
    #[serde(default = "default_theme")]
    pub theme: String,
    // Default keeps config files written before this field loadable.
    #[serde(default, alias = "auto_region")]
    pub magic_wand: crate::selection::WandParams,
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
            thumb_size: 140.0,
            theme: default_theme(),
            magic_wand: Default::default(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_without_magic_wand_still_loads_with_defaults() {
        let mut value = serde_json::to_value(Config::default()).unwrap();
        value.as_object_mut().unwrap().remove("magic_wand");
        value["thumb_size"] = serde_json::json!(99.0);
        let config: Config = serde_json::from_value(value).unwrap();
        assert_eq!(config.thumb_size, 99.0);
        assert_eq!(config.magic_wand, Default::default());
    }

    #[test]
    fn magic_wand_settings_round_trip() {
        let mut config = Config::default();
        config.magic_wand.tolerance = 77;
        config.magic_wand.expand = -3;
        config.magic_wand.contiguous = false;
        let text = serde_json::to_string(&config).unwrap();
        let loaded: Config = serde_json::from_str(&text).unwrap();
        assert_eq!(loaded.magic_wand, config.magic_wand);
    }

    #[test]
    fn old_auto_region_settings_carry_over() {
        let mut value = serde_json::to_value(Config::default()).unwrap();
        let object = value.as_object_mut().unwrap();
        object.remove("magic_wand");
        object.insert(
            "auto_region".to_string(),
            serde_json::json!({ "tolerance": 77, "padding": -3, "min_size": 9, "diagonal": true }),
        );
        let config: Config = serde_json::from_value(value).unwrap();
        assert_eq!(config.magic_wand.tolerance, 77);
        assert_eq!(config.magic_wand.expand, -3);
        assert!(config.magic_wand.diagonal);
        assert!(config.magic_wand.contiguous);
    }
}
