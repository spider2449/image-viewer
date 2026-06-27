use std::path::Path;

pub struct ExifData {
    pub visible: bool,
    pub entries: Vec<(String, String)>,
}

impl ExifData {
    pub fn new() -> Self {
        Self {
            visible: false,
            entries: Vec::new(),
        }
    }

    pub fn parse(&mut self, path: &Path) {
        self.entries.clear();

        let ext = path.extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if ext != "jpg" && ext != "jpeg" && ext != "tif" && ext != "tiff" {
            self.entries.push(("Info".to_string(), "EXIF not available for this format".to_string()));
            return;
        }

        let file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(e) => {
                self.entries.push(("Error".to_string(), format!("Cannot read file: {e}")));
                return;
            }
        };
        let mut reader = std::io::BufReader::new(file);
        let exif = match exif::Reader::new().read_from_container(&mut reader) {
            Ok(e) => e,
            Err(_) => {
                self.entries.push(("Info".to_string(), "No EXIF data found".to_string()));
                return;
            }
        };

        let fields: Vec<(&str, exif::Tag)> = vec![
            ("Camera Make", exif::Tag::Make),
            ("Camera Model", exif::Tag::Model),
            ("Lens", exif::Tag::LensModel),
            ("Software", exif::Tag::Software),
            ("Date/Time", exif::Tag::DateTimeOriginal),
            ("Image Width", exif::Tag::PixelXDimension),
            ("Image Height", exif::Tag::PixelYDimension),
            ("Orientation", exif::Tag::Orientation),
            ("Exposure Time", exif::Tag::ExposureTime),
            ("F-Number", exif::Tag::FNumber),
            ("ISO", exif::Tag::PhotographicSensitivity),
            ("Focal Length", exif::Tag::FocalLength),
            ("Flash", exif::Tag::Flash),
        ];

        for (label, tag) in fields {
            if let Some(field) = exif.get_field(tag, exif::In::PRIMARY) {
                let value = format_exif_value(field);
                self.entries.push((label.to_string(), value));
            }
        }

        // GPS: need ref field for N/S/E/W
        if let Some(lat) = exif.get_field(exif::Tag::GPSLatitude, exif::In::PRIMARY) {
            let ref_val = exif.get_field(exif::Tag::GPSLatitudeRef, exif::In::PRIMARY)
                .and_then(|f| f.value.display_as(f.tag).to_string().into())
                .unwrap_or_default();
            let dir = if ref_val == "S" { "S" } else { "N" };
            self.entries.push(("GPS Latitude".to_string(), format_gps_coords(&lat, dir)));
        }
        if let Some(lon) = exif.get_field(exif::Tag::GPSLongitude, exif::In::PRIMARY) {
            let ref_val = exif.get_field(exif::Tag::GPSLongitudeRef, exif::In::PRIMARY)
                .and_then(|f| f.value.display_as(f.tag).to_string().into())
                .unwrap_or_default();
            let dir = if ref_val == "W" { "W" } else { "E" };
            self.entries.push(("GPS Longitude".to_string(), format_gps_coords(&lon, dir)));
        }

        if self.entries.is_empty() {
            self.entries.push(("Info".to_string(), "No EXIF data found".to_string()));
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

fn format_exif_value(field: &exif::Field) -> String {
    match field.tag {
        exif::Tag::Orientation => {
            match field.value.get_uint(0) {
                Some(1) => "Normal".to_string(),
                Some(2) => "Mirrored".to_string(),
                Some(3) => "Rotated 180\u{00B0}".to_string(),
                Some(4) => "Mirrored & rotated 180\u{00B0}".to_string(),
                Some(5) => "Mirrored & rotated 90\u{00B0} CW".to_string(),
                Some(6) => "Rotated 90\u{00B0} CW".to_string(),
                Some(7) => "Mirrored & rotated 90\u{00B0} CCW".to_string(),
                Some(8) => "Rotated 90\u{00B0} CCW".to_string(),
                _ => format!("{}", field.value.display_as(field.tag)),
            }
        }
        exif::Tag::ExposureTime => {
            if let exif::Value::Rational(rats) = &field.value {
                if let Some(r) = rats.first() {
                    let num = r.to_f64();
                    if num > 0.0 {
                        let denom = (1.0 / num).round() as u64;
                        return format!("1/{denom} sec");
                    }
                }
            }
            format!("{}", field.value.display_as(field.tag))
        }
        exif::Tag::FNumber => {
            if let exif::Value::Rational(rats) = &field.value {
                if let Some(r) = rats.first() {
                    let val = r.to_f64();
                    return format!("F/{val:.1}");
                }
            }
            format!("{}", field.value.display_as(field.tag))
        }
        exif::Tag::FocalLength => {
            if let exif::Value::Rational(rats) = &field.value {
                if let Some(r) = rats.first() {
                    let val = r.to_f64();
                    return format!("{val:.1} mm");
                }
            }
            format!("{}", field.value.display_as(field.tag))
        }
        exif::Tag::Flash => {
            match field.value.get_uint(0) {
                Some(0) => "Did not fire".to_string(),
                Some(1) => "Fired".to_string(),
                Some(5) => "Fired (return light detected)".to_string(),
                Some(7) => "Fired (return light not detected)".to_string(),
                Some(16) => "Did not fire (auto)".to_string(),
                _ => format!("{}", field.value.display_as(field.tag)),
            }
        }
        _ => {
            format!("{}", field.value.display_as(field.tag))
        }
    }
}

fn format_gps_coords(field: &exif::Field, dir: &str) -> String {
    if let exif::Value::Rational(rats) = &field.value {
        if rats.len() >= 3 {
            let deg = rats[0].to_f64();
            let min = rats[1].to_f64();
            let sec = rats[2].to_f64();
            return format!("{deg}\u{00B0} {min}' {sec:.1}\" {dir}");
        }
    }
    format!("{} {}", field.value.display_as(field.tag), dir)
}

pub fn show(data: &mut ExifData, ctx: &egui::Context) {
    if !data.visible {
        return;
    }

    egui::SidePanel::right("exif_panel")
        .resizable(true)
        .frame(egui::Frame {
            fill: crate::theme::PANEL_BG,
            inner_margin: egui::Margin::symmetric(8, 8),
            ..Default::default()
        })
        .default_width(280.0)
        .min_width(200.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("EXIF Data");
                ui.separator();
                for (i, (label, value)) in data.entries.iter().enumerate() {
                    let row_bg = if i % 2 == 0 {
                        crate::theme::PANEL_BG
                    } else {
                        crate::theme::CARD_BG
                    };
                    let (rect, _) = ui.allocate_exact_size(
                        egui::Vec2::new(ui.available_width(), 20.0),
                        egui::Sense::hover(),
                    );
                    ui.painter().rect_filled(rect, egui::CornerRadius::same(0), row_bg);
                    #[allow(deprecated)]
                    let mut child_ui = ui.child_ui(rect, *ui.layout(), None);
                    child_ui.horizontal(|ui| {
                        ui.colored_label(crate::theme::ACCENT, format!("{label}:"));
                        ui.colored_label(crate::theme::TEXT_PRIMARY, value);
                    });
                }
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_exif_for_png() {
        let mut data = ExifData::new();
        data.parse(&std::path::PathBuf::from("test.png"));
        assert!(!data.entries.is_empty());
        assert!(data.entries[0].1.contains("not available"));
    }

    #[test]
    fn test_no_exif_for_bmp() {
        let mut data = ExifData::new();
        data.parse(&std::path::PathBuf::from("test.bmp"));
        assert!(!data.entries.is_empty());
        assert!(data.entries[0].1.contains("not available"));
    }

    #[test]
    fn test_no_exif_for_missing_file() {
        let mut data = ExifData::new();
        data.parse(&std::path::PathBuf::from("nonexistent.jpg"));
        assert!(!data.entries.is_empty());
        assert!(data.entries[0].1.contains("Cannot read"));
    }

    #[test]
    fn test_clear() {
        let mut data = ExifData::new();
        data.entries.push(("Test".to_string(), "Value".to_string()));
        data.clear();
        assert!(data.entries.is_empty());
    }
}
