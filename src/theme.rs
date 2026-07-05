use eframe::egui::{self, Color32, CornerRadius, Margin, Stroke, Style, Visuals, Vec2};

// ── Theme switch (light/dark) ─────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
}

impl Theme {
    pub fn from_str(s: &str) -> Theme {
        match s {
            "light" => Theme::Light,
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
            bg_dark: Color32::from_rgb(0x1a, 0x1a, 0x1a),
            panel_bg: Color32::from_rgb(0x22, 0x22, 0x22),
            card_bg: Color32::from_rgb(0x2a, 0x2a, 0x2a),
            hover_bg: Color32::from_rgb(0x35, 0x35, 0x35),
            accent: Color32::from_rgb(0x4a, 0x9e, 0xff),
            selected_bg: Color32::from_rgb(0x2d, 0x5a, 0x8e),
            text_primary: Color32::from_rgb(0xe0, 0xe0, 0xe0),
            text_secondary: Color32::from_rgb(0x88, 0x88, 0x88),
            border: Color32::from_rgb(0x3a, 0x3a, 0x3a),
            danger: Color32::from_rgb(0xe7, 0x4c, 0x3c),
        },
        Theme::Light => ThemeColors {
            bg_dark: Color32::from_rgb(0xff, 0xff, 0xff),
            panel_bg: Color32::from_rgb(0xf5, 0xf5, 0xf5),
            card_bg: Color32::from_rgb(0xe8, 0xe8, 0xe8),
            hover_bg: Color32::from_rgb(0xdc, 0xdc, 0xdc),
            accent: Color32::from_rgb(0x1a, 0x6d, 0xd6),
            selected_bg: Color32::from_rgb(0xa8, 0xc8, 0xf0),
            text_primary: Color32::from_rgb(0x1a, 0x1a, 0x1a),
            text_secondary: Color32::from_rgb(0x66, 0x66, 0x66),
            border: Color32::from_rgb(0xcc, 0xcc, 0xcc),
            danger: Color32::from_rgb(0xc0, 0x39, 0x2b),
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
    let base = if theme == Theme::Dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    Visuals {
        dark_mode: theme == Theme::Dark,
        override_text_color: Some(c.text_primary),
        window_corner_radius: CornerRadius::same(6),
        window_stroke: Stroke::new(1.0, c.border),
        window_fill: c.panel_bg,
        panel_fill: c.panel_bg,
        faint_bg_color: c.bg_dark,
        extreme_bg_color: c.bg_dark,
        code_bg_color: c.card_bg,
        warn_fg_color: c.danger,
        error_fg_color: c.danger,
        hyperlink_color: c.accent,
        selection: egui::style::Selection {
            bg_fill: c.selected_bg,
            stroke: Stroke::new(1.0, c.accent),
        },
        widgets: egui::style::Widgets {
            noninteractive: egui::style::WidgetVisuals {
                bg_fill: c.card_bg,
                weak_bg_fill: c.panel_bg,
                bg_stroke: Stroke::new(1.0, c.border),
                corner_radius: CornerRadius::same(4),
                fg_stroke: Stroke::new(1.0, c.text_secondary),
                expansion: 0.0,
            },
            inactive: egui::style::WidgetVisuals {
                bg_fill: c.panel_bg,
                weak_bg_fill: c.card_bg,
                bg_stroke: Stroke::new(1.0, c.border),
                corner_radius: CornerRadius::same(4),
                fg_stroke: Stroke::new(1.0, c.text_primary),
                expansion: 0.0,
            },
            hovered: egui::style::WidgetVisuals {
                bg_fill: c.hover_bg,
                weak_bg_fill: c.hover_bg,
                bg_stroke: Stroke::new(1.0, c.accent),
                corner_radius: CornerRadius::same(4),
                fg_stroke: Stroke::new(1.5, c.accent),
                expansion: 1.0,
            },
            active: egui::style::WidgetVisuals {
                bg_fill: c.selected_bg,
                weak_bg_fill: c.selected_bg,
                bg_stroke: Stroke::new(1.0, c.accent),
                corner_radius: CornerRadius::same(4),
                fg_stroke: Stroke::new(2.0, c.accent),
                expansion: 1.0,
            },
            open: egui::style::WidgetVisuals {
                bg_fill: c.card_bg,
                weak_bg_fill: c.card_bg,
                bg_stroke: Stroke::new(1.0, c.border),
                corner_radius: CornerRadius::same(4),
                fg_stroke: Stroke::new(1.0, c.text_primary),
                expansion: 0.0,
            },
        },
        ..base
    }
}

// ── Build the global Style ─────────────────────────────────
pub fn theme_style() -> Style {
    Style {
        spacing: egui::style::Spacing {
            item_spacing: Vec2::new(8.0, 8.0),
            button_padding: Vec2::new(8.0, 4.0),
            indent: 16.0,
            scroll: egui::style::ScrollStyle {
                bar_width: 6.0,
                ..Default::default()
            },
            window_margin: Margin::same(12),
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
    fn test_palette_dark_matches_old_constants() {
        let c = palette(Theme::Dark);
        assert_eq!(c.bg_dark, Color32::from_rgb(0x1a, 0x1a, 0x1a));
        assert_eq!(c.panel_bg, Color32::from_rgb(0x22, 0x22, 0x22));
        assert_eq!(c.card_bg, Color32::from_rgb(0x2a, 0x2a, 0x2a));
        assert_eq!(c.hover_bg, Color32::from_rgb(0x35, 0x35, 0x35));
        assert_eq!(c.accent, Color32::from_rgb(0x4a, 0x9e, 0xff));
        assert_eq!(c.selected_bg, Color32::from_rgb(0x2d, 0x5a, 0x8e));
        assert_eq!(c.text_primary, Color32::from_rgb(0xe0, 0xe0, 0xe0));
        assert_eq!(c.text_secondary, Color32::from_rgb(0x88, 0x88, 0x88));
        assert_eq!(c.border, Color32::from_rgb(0x3a, 0x3a, 0x3a));
        assert_eq!(c.danger, Color32::from_rgb(0xe7, 0x4c, 0x3c));
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
        assert_eq!(Theme::from_str("nonsense"), Theme::Dark);
        assert_eq!(Theme::from_str(""), Theme::Dark);
    }
}
