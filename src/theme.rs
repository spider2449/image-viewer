use eframe::egui::{self, Color32, CornerRadius, Margin, Stroke, Style, Visuals, Vec2};

// ── Theme switch ──────────────────────────────────────────
// Every subcomponent (menu bar, folder tree, thumbnail grid, viewer,
// editor panel, EXIF view, batch dialog) reads its colors from
// `palette(theme)`, so adding a variant here automatically re-skins the
// whole app. No hardcoded UI colors outside this module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
    Nord,
    TokyoNight,
    Gruvbox,
    Sepia,
}

impl Theme {
    /// All available themes in menu/gallery order.
    pub fn all() -> &'static [Theme] {
        &[
            Theme::Dark,
            Theme::Light,
            Theme::Nord,
            Theme::TokyoNight,
            Theme::Gruvbox,
            Theme::Sepia,
        ]
    }

    /// Stable id persisted in `config.json`.
    pub fn as_str(self) -> &'static str {
        match self {
            Theme::Dark => "dark",
            Theme::Light => "light",
            Theme::Nord => "nord",
            Theme::TokyoNight => "tokyo-night",
            Theme::Gruvbox => "gruvbox",
            Theme::Sepia => "sepia",
        }
    }

    /// Human-readable name for menus and the gallery.
    pub fn name(self) -> &'static str {
        match self {
            Theme::Dark => "Dark",
            Theme::Light => "Light",
            Theme::Nord => "Nord Frost",
            Theme::TokyoNight => "Tokyo Night",
            Theme::Gruvbox => "Gruvbox",
            Theme::Sepia => "Sepia Paper",
        }
    }

    /// One-line hint shown in the gallery.
    pub fn description(self) -> &'static str {
        match self {
            Theme::Dark => "Default violet-tinted dark",
            Theme::Light => "Default clean light",
            Theme::Nord => "Cool polar dark, easy on the eyes",
            Theme::TokyoNight => "Deep blue dark for night viewing",
            Theme::Gruvbox => "Warm retro dark with orange accent",
            Theme::Sepia => "Warm paper light for daytime browsing",
        }
    }

    pub fn is_dark(self) -> bool {
        match self {
            Theme::Dark | Theme::Nord | Theme::TokyoNight | Theme::Gruvbox => true,
            Theme::Light | Theme::Sepia => false,
        }
    }

    pub fn from_str(s: &str) -> Theme {
        match s {
            "light" => Theme::Light,
            "nord" => Theme::Nord,
            "tokyo-night" | "tokyonight" | "tokyo_night" => Theme::TokyoNight,
            "gruvbox" => Theme::Gruvbox,
            "sepia" => Theme::Sepia,
            "dark" => Theme::Dark,
            _ => Theme::Dark,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ThemeColors {
    pub bg_dark: Color32,
    pub panel_bg: Color32,
    pub card_bg: Color32,
    pub hover_bg: Color32,
    pub accent: Color32,
    pub selected_bg: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub border: Color32,
    pub danger: Color32,
}

pub fn palette(theme: Theme) -> ThemeColors {
    match theme {
        Theme::Dark => ThemeColors {
            bg_dark: Color32::from_rgb(0x13, 0x11, 0x1c),
            panel_bg: Color32::from_rgba_unmultiplied(0x1b, 0x18, 0x27, 235),
            card_bg: Color32::from_rgba_unmultiplied(0x25, 0x20, 0x35, 235),
            hover_bg: Color32::from_rgb(0x35, 0x2c, 0x4c),
            accent: Color32::from_rgb(0x8a, 0x63, 0xff),
            selected_bg: Color32::from_rgba_unmultiplied(0x8a, 0x63, 0xff, 0x40),
            text_primary: Color32::from_rgb(0xef, 0xec, 0xf7),
            text_secondary: Color32::from_rgb(0x9c, 0x93, 0xb8),
            border: Color32::from_rgb(0x3c, 0x32, 0x54),
            danger: Color32::from_rgb(0xff, 0x5c, 0x7a),
        },
        Theme::Light => ThemeColors {
            bg_dark: Color32::from_rgb(0xfd, 0xfc, 0xff),
            panel_bg: Color32::from_rgb(0xf3, 0xef, 0xfb),
            card_bg: Color32::from_rgb(0xe8, 0xe0, 0xf9),
            hover_bg: Color32::from_rgb(0xdb, 0xcd, 0xf7),
            accent: Color32::from_rgb(0x6d, 0x3d, 0xe0),
            selected_bg: Color32::from_rgb(0xcd, 0xba, 0xf5),
            text_primary: Color32::from_rgb(0x22, 0x1c, 0x33),
            text_secondary: Color32::from_rgb(0x6b, 0x60, 0x84),
            border: Color32::from_rgb(0xd6, 0xc7, 0xf0),
            danger: Color32::from_rgb(0xd6, 0x2b, 0x54),
        },
        Theme::Nord => ThemeColors {
            bg_dark: Color32::from_rgb(0x2e, 0x34, 0x40),
            panel_bg: Color32::from_rgb(0x3b, 0x42, 0x52),
            card_bg: Color32::from_rgb(0x43, 0x4c, 0x5e),
            hover_bg: Color32::from_rgb(0x4c, 0x56, 0x6a),
            accent: Color32::from_rgb(0x88, 0xc0, 0xd0),
            selected_bg: Color32::from_rgba_unmultiplied(0x88, 0xc0, 0xd0, 0x40),
            text_primary: Color32::from_rgb(0xec, 0xef, 0xf4),
            text_secondary: Color32::from_rgb(0x9a, 0xa3, 0xb2),
            border: Color32::from_rgb(0x4c, 0x56, 0x6a),
            danger: Color32::from_rgb(0xbf, 0x61, 0x6a),
        },
        Theme::TokyoNight => ThemeColors {
            bg_dark: Color32::from_rgb(0x1a, 0x1b, 0x26),
            panel_bg: Color32::from_rgb(0x24, 0x28, 0x3b),
            card_bg: Color32::from_rgb(0x2f, 0x35, 0x49),
            hover_bg: Color32::from_rgb(0x41, 0x48, 0x68),
            accent: Color32::from_rgb(0x7a, 0xa2, 0xf7),
            selected_bg: Color32::from_rgba_unmultiplied(0x7a, 0xa2, 0xf7, 0x40),
            text_primary: Color32::from_rgb(0xc0, 0xca, 0xf5),
            text_secondary: Color32::from_rgb(0x9a, 0xa5, 0xce),
            border: Color32::from_rgb(0x3b, 0x42, 0x61),
            danger: Color32::from_rgb(0xf7, 0x76, 0x8e),
        },
        Theme::Gruvbox => ThemeColors {
            bg_dark: Color32::from_rgb(0x28, 0x28, 0x28),
            panel_bg: Color32::from_rgb(0x32, 0x30, 0x2f),
            card_bg: Color32::from_rgb(0x3c, 0x38, 0x36),
            hover_bg: Color32::from_rgb(0x50, 0x49, 0x45),
            accent: Color32::from_rgb(0xfe, 0x80, 0x19),
            selected_bg: Color32::from_rgba_unmultiplied(0xfe, 0x80, 0x19, 0x40),
            text_primary: Color32::from_rgb(0xeb, 0xdb, 0xb2),
            text_secondary: Color32::from_rgb(0xa8, 0x99, 0x84),
            border: Color32::from_rgb(0x50, 0x49, 0x45),
            danger: Color32::from_rgb(0xfb, 0x49, 0x34),
        },
        Theme::Sepia => ThemeColors {
            bg_dark: Color32::from_rgb(0xf7, 0xf1, 0xe3),
            panel_bg: Color32::from_rgb(0xf0, 0xe7, 0xd3),
            card_bg: Color32::from_rgb(0xe9, 0xdc, 0xc0),
            hover_bg: Color32::from_rgb(0xde, 0xd0, 0xb4),
            accent: Color32::from_rgb(0x9c, 0x66, 0x44),
            selected_bg: Color32::from_rgb(0xd9, 0xc6, 0xa5),
            text_primary: Color32::from_rgb(0x4a, 0x42, 0x38),
            text_secondary: Color32::from_rgb(0x8a, 0x7d, 0x6b),
            border: Color32::from_rgb(0xd8, 0xc9, 0xac),
            danger: Color32::from_rgb(0xb3, 0x26, 0x1e),
        },
    }
}

// ── Convenient icon wrapper ────────────────────────────────
pub fn styled_icon(codepoint: &str, colors: &ThemeColors) -> egui::RichText {
    egui::RichText::new(codepoint).size(14.0).color(colors.accent)
}

// ── Build the global Visuals ───────────────────────────────
pub fn theme_visuals(theme: Theme) -> Visuals {
    let c = palette(theme);
    let base = if theme.is_dark() {
        Visuals::dark()
    } else {
        Visuals::light()
    };

    let window_shadow = if theme.is_dark() {
        egui::epaint::Shadow {
            offset: [0, 8],
            blur: 24,
            spread: 0,
            color: Color32::from_black_alpha(120),
        }
    } else {
        egui::epaint::Shadow {
            offset: [0, 4],
            blur: 10,
            spread: 0,
            color: Color32::from_black_alpha(40),
        }
    };

    Visuals {
        dark_mode: theme.is_dark(),
        override_text_color: Some(c.text_primary),
        window_corner_radius: CornerRadius::same(10),
        window_stroke: Stroke::new(1.0_f32, c.border),
        window_fill: c.panel_bg,
        panel_fill: c.panel_bg,
        window_shadow,
        popup_shadow: window_shadow,
        faint_bg_color: c.bg_dark,
        extreme_bg_color: c.bg_dark,
        code_bg_color: c.card_bg,
        warn_fg_color: c.danger,
        error_fg_color: c.danger,
        hyperlink_color: c.accent,
        selection: egui::style::Selection {
            bg_fill: c.selected_bg,
            stroke: Stroke::new(1.0_f32, c.accent),
        },
        widgets: egui::style::Widgets {
            noninteractive: egui::style::WidgetVisuals {
                bg_fill: c.card_bg,
                weak_bg_fill: c.panel_bg,
                bg_stroke: Stroke::new(1.0_f32, c.border),
                corner_radius: CornerRadius::same(8),
                fg_stroke: Stroke::new(1.0_f32, c.text_secondary),
                expansion: 0.0,
            },
            inactive: egui::style::WidgetVisuals {
                bg_fill: c.panel_bg,
                weak_bg_fill: c.card_bg,
                bg_stroke: Stroke::new(1.0_f32, c.border),
                corner_radius: CornerRadius::same(8),
                fg_stroke: Stroke::new(1.0_f32, c.text_primary),
                expansion: 0.0,
            },
            hovered: egui::style::WidgetVisuals {
                bg_fill: c.hover_bg,
                weak_bg_fill: c.hover_bg,
                bg_stroke: Stroke::new(1.0_f32, c.accent),
                corner_radius: CornerRadius::same(8),
                fg_stroke: Stroke::new(1.2_f32, c.accent),
                expansion: 0.5,
            },
            active: egui::style::WidgetVisuals {
                bg_fill: c.selected_bg,
                weak_bg_fill: c.selected_bg,
                bg_stroke: Stroke::new(1.0_f32, c.accent),
                corner_radius: CornerRadius::same(8),
                fg_stroke: Stroke::new(2.0_f32, c.accent),
                expansion: 1.0,
            },
            open: egui::style::WidgetVisuals {
                bg_fill: c.card_bg,
                weak_bg_fill: c.card_bg,
                bg_stroke: Stroke::new(1.0_f32, c.border),
                corner_radius: CornerRadius::same(8),
                fg_stroke: Stroke::new(1.0_f32, c.text_primary),
                expansion: 0.0,
            },
        },
        ..base
    }
}

// ── Apply a theme to the egui context ──────────────────────
// `Context::set_style`/`set_visuals` only write to whichever theme slot
// `ctx.theme()` currently resolves to (which defaults to following the OS
// and may not match `theme` yet). Pin the preference and target the slot
// explicitly so both the style and visuals always land on the right theme.
pub fn apply_theme(ctx: &egui::Context, theme: Theme) {
    let egui_theme = if theme.is_dark() {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    };
    ctx.set_theme(egui_theme);
    ctx.set_style_of(egui_theme, theme_style());
    ctx.set_visuals_of(egui_theme, theme_visuals(theme));
}

// ── Build the global Style ─────────────────────────────────
pub fn theme_style() -> Style {
    Style {
        spacing: egui::style::Spacing {
            item_spacing: Vec2::new(10.0, 10.0),
            button_padding: Vec2::new(10.0, 6.0),
            indent: 16.0,
            scroll: egui::style::ScrollStyle {
                bar_width: 6.0,
                ..Default::default()
            },
            window_margin: Margin::same(14),
            ..Default::default()
        },
        interaction: egui::style::Interaction {
            resize_grab_radius_side: 4.0,
            resize_grab_radius_corner: 4.0,
            ..Default::default()
        },
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_palette_dark_matches_current_constants() {
        let c = palette(Theme::Dark);
        assert_eq!(c.bg_dark, Color32::from_rgb(0x13, 0x11, 0x1c));
        assert_eq!(c.panel_bg, Color32::from_rgba_unmultiplied(0x1b, 0x18, 0x27, 235));
        assert_eq!(c.card_bg, Color32::from_rgba_unmultiplied(0x25, 0x20, 0x35, 235));
        assert_eq!(c.hover_bg, Color32::from_rgb(0x35, 0x2c, 0x4c));
        assert_eq!(c.accent, Color32::from_rgb(0x8a, 0x63, 0xff));
        assert_eq!(c.selected_bg, Color32::from_rgba_unmultiplied(0x8a, 0x63, 0xff, 0x40));
        assert_eq!(c.text_primary, Color32::from_rgb(0xef, 0xec, 0xf7));
        assert_eq!(c.text_secondary, Color32::from_rgb(0x9c, 0x93, 0xb8));
        assert_eq!(c.border, Color32::from_rgb(0x3c, 0x32, 0x54));
        assert_eq!(c.danger, Color32::from_rgb(0xff, 0x5c, 0x7a));
    }

    #[test]
    fn test_palette_light_matches_new_constants() {
        let c = palette(Theme::Light);
        assert_eq!(c.bg_dark, Color32::from_rgb(0xfd, 0xfc, 0xff));
        assert_eq!(c.panel_bg, Color32::from_rgb(0xf3, 0xef, 0xfb));
        assert_eq!(c.card_bg, Color32::from_rgb(0xe8, 0xe0, 0xf9));
        assert_eq!(c.hover_bg, Color32::from_rgb(0xdb, 0xcd, 0xf7));
        assert_eq!(c.accent, Color32::from_rgb(0x6d, 0x3d, 0xe0));
        assert_eq!(c.selected_bg, Color32::from_rgb(0xcd, 0xba, 0xf5));
        assert_eq!(c.text_primary, Color32::from_rgb(0x22, 0x1c, 0x33));
        assert_eq!(c.text_secondary, Color32::from_rgb(0x6b, 0x60, 0x84));
        assert_eq!(c.border, Color32::from_rgb(0xd6, 0xc7, 0xf0));
        assert_eq!(c.danger, Color32::from_rgb(0xd6, 0x2b, 0x54));
    }

    #[test]
    fn test_palette_light_distinct_from_dark() {
        let dark = palette(Theme::Dark);
        let light = palette(Theme::Light);
        let diffs = [
            dark.bg_dark != light.bg_dark,
            dark.panel_bg != light.panel_bg,
            dark.card_bg != light.card_bg,
            dark.hover_bg != light.hover_bg,
            dark.accent != light.accent,
            dark.selected_bg != light.selected_bg,
            dark.text_primary != light.text_primary,
            dark.text_secondary != light.text_secondary,
            dark.border != light.border,
            dark.danger != light.danger,
        ];
        let count = diffs.iter().filter(|&&d| d).count();
        assert!(count >= 8, "expected at least 8 of 10 colors to differ, got {count}");
    }

    #[test]
    fn test_theme_from_str() {
        assert_eq!(Theme::from_str("dark"), Theme::Dark);
        assert_eq!(Theme::from_str("light"), Theme::Light);
        assert_eq!(Theme::from_str("nord"), Theme::Nord);
        assert_eq!(Theme::from_str("tokyo-night"), Theme::TokyoNight);
        assert_eq!(Theme::from_str("gruvbox"), Theme::Gruvbox);
        assert_eq!(Theme::from_str("sepia"), Theme::Sepia);
        assert_eq!(Theme::from_str("nonsense"), Theme::Dark);
        assert_eq!(Theme::from_str(""), Theme::Dark);
    }

    #[test]
    fn test_theme_id_roundtrip() {
        for theme in Theme::all() {
            assert_eq!(Theme::from_str(theme.as_str()), *theme);
        }
    }

    #[test]
    fn test_all_themes_have_unique_ids_and_names() {
        let all = Theme::all();
        assert_eq!(all.len(), 6);
        let mut ids = std::collections::HashSet::new();
        let mut names = std::collections::HashSet::new();
        for theme in all {
            assert!(ids.insert(theme.as_str()), "duplicate id: {}", theme.as_str());
            assert!(names.insert(theme.name()), "duplicate name: {}", theme.name());
            assert!(!theme.description().is_empty());
        }
    }

    #[test]
    fn test_new_palettes_differ_from_dark_and_light() {
        let dark = palette(Theme::Dark);
        let light = palette(Theme::Light);
        for theme in [Theme::Nord, Theme::TokyoNight, Theme::Gruvbox, Theme::Sepia] {
            let c = palette(theme);
            let vs_dark = [
                dark.bg_dark != c.bg_dark,
                dark.panel_bg != c.panel_bg,
                dark.accent != c.accent,
                dark.text_primary != c.text_primary,
            ];
            assert!(
                vs_dark.iter().filter(|&&d| d).count() >= 3,
                "{:?} palette too close to dark",
                theme
            );
            let vs_light = [
                light.bg_dark != c.bg_dark,
                light.panel_bg != c.panel_bg,
                light.accent != c.accent,
            ];
            // Sepia is light-family; the others must differ from light.
            if theme != Theme::Sepia {
                assert!(
                    vs_light.iter().all(|&d| d),
                    "{:?} palette too close to light",
                    theme
                );
            }
        }
    }

    #[test]
    fn test_is_dark_matches_visuals() {
        for theme in Theme::all() {
            assert_eq!(theme_visuals(*theme).dark_mode, theme.is_dark());
        }
        assert!(Theme::Dark.is_dark());
        assert!(Theme::Nord.is_dark());
        assert!(Theme::TokyoNight.is_dark());
        assert!(Theme::Gruvbox.is_dark());
        assert!(!Theme::Light.is_dark());
        assert!(!Theme::Sepia.is_dark());
    }
}
