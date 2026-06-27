use eframe::egui::{self, Color32, CornerRadius, Stroke, Style, Visuals, Vec2};

// ── Color palette ──────────────────────────────────────────
pub const BG_DARK: Color32 = Color32::from_rgb(0x1a, 0x1a, 0x1a);
pub const PANEL_BG: Color32 = Color32::from_rgb(0x22, 0x22, 0x22);
pub const CARD_BG: Color32 = Color32::from_rgb(0x2a, 0x2a, 0x2a);
pub const HOVER_BG: Color32 = Color32::from_rgb(0x35, 0x35, 0x35);
pub const ACCENT: Color32 = Color32::from_rgb(0x4a, 0x9e, 0xff);
pub const SELECTED_BG: Color32 = Color32::from_rgb(0x2d, 0x5a, 0x8e);
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xe0, 0xe0, 0xe0);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(0x88, 0x88, 0x88);
pub const BORDER: Color32 = Color32::from_rgb(0x3a, 0x3a, 0x3a);
pub const DANGER: Color32 = Color32::from_rgb(0xe7, 0x4c, 0x3c);
pub const SUCCESS: Color32 = Color32::from_rgb(0x2e, 0xcc, 0x71);

// ── Convenient icon wrapper ────────────────────────────────
pub fn styled_icon(codepoint: &str) -> egui::RichText {
    egui::RichText::new(codepoint).size(14.0).color(ACCENT)
}

// ── Build the global Visuals ───────────────────────────────
pub fn theme_visuals() -> Visuals {
    Visuals {
        dark_mode: true,
        override_text_color: Some(TEXT_PRIMARY),
        window_corner_radius: CornerRadius::same(6),
        window_stroke: Stroke::new(1.0, BORDER),
        panel_fill: PANEL_BG,
        faint_bg_color: BG_DARK,
        extreme_bg_color: BG_DARK,
        code_bg_color: CARD_BG,
        warn_fg_color: DANGER,
        error_fg_color: DANGER,
        hyperlink_color: ACCENT,
        selection: egui::style::Selection {
            bg_fill: SELECTED_BG,
            stroke: Stroke::new(1.0, ACCENT),
        },
        widgets: egui::style::Widgets {
            noninteractive: egui::style::WidgetVisuals {
                bg_fill: CARD_BG,
                weak_bg_fill: PANEL_BG,
                bg_stroke: Stroke::new(1.0, BORDER),
                corner_radius: CornerRadius::same(4),
                fg_stroke: Stroke::new(1.0, TEXT_SECONDARY),
                expansion: 0.0,
            },
            inactive: egui::style::WidgetVisuals {
                bg_fill: PANEL_BG,
                weak_bg_fill: CARD_BG,
                bg_stroke: Stroke::new(1.0, BORDER),
                corner_radius: CornerRadius::same(4),
                fg_stroke: Stroke::new(1.0, TEXT_PRIMARY),
                expansion: 0.0,
            },
            hovered: egui::style::WidgetVisuals {
                bg_fill: HOVER_BG,
                weak_bg_fill: HOVER_BG,
                bg_stroke: Stroke::new(1.0, ACCENT),
                corner_radius: CornerRadius::same(4),
                fg_stroke: Stroke::new(1.5, ACCENT),
                expansion: 1.0,
            },
            active: egui::style::WidgetVisuals {
                bg_fill: SELECTED_BG,
                weak_bg_fill: SELECTED_BG,
                bg_stroke: Stroke::new(1.0, ACCENT),
                corner_radius: CornerRadius::same(4),
                fg_stroke: Stroke::new(2.0, ACCENT),
                expansion: 1.0,
            },
            open: egui::style::WidgetVisuals {
                bg_fill: CARD_BG,
                weak_bg_fill: CARD_BG,
                bg_stroke: Stroke::new(1.0, BORDER),
                corner_radius: CornerRadius::same(4),
                fg_stroke: Stroke::new(1.0, TEXT_PRIMARY),
                expansion: 0.0,
            },
        },
        ..Default::default()
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
