#!/usr/bin/env python3
"""Apply Save As filename input feature."""

import re

filepath = "src/editor/mod.rs"

with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()

# 1. Add save_as_filename field to State struct
content = content.replace(
    "pub save_jpeg_quality: u8,",
    "pub save_jpeg_quality: u8,\n    pub save_as_filename: String,"
)

# 2. Initialize save_as_filename in State::new()
content = content.replace(
    "save_jpeg_quality: 90,",
    "save_jpeg_quality: 90,\n            save_as_filename: String::new(),"
)

# 3. Add text input in UI before Save As button
old_ui = '''                if ui.button("Save As...").clicked() {
                    save_as(app);
                }'''

new_ui = '''                // Filename input
                egui::TextEdit::singleline(&mut app.editor_state.save_as_filename)
                    .hint_text("Enter filename")
                    .show(ui);
                if ui.button("Save As...").clicked() && !app.editor_state.save_as_filename.is_empty() {
                    save_as(app);
                }'''

if old_ui in content:
    content = content.replace(old_ui, new_ui)
    print("UI replaced")
else:
    print("UI pattern not found")

# 4. Replace save_as function to use the filename
old_save_as = '''fn save_as(app: &mut App) {
    let img = match &app.editor_state.current_image {
        Some(i) => i.clone(),
        None => return,
    };

    let path = match app.image_files.get(app.selected_image_index) {
        Some(p) => p.clone(),
        None => return,
    };

    let save_format = app.editor_state.save_format;
    let new_ext = crate::format_ext::format_to_extension(save_format);
    let new_name = path.with_extension(new_ext);
    match crate::format_ext::save_image(&img, &new_name, save_format, app.editor_state.save_jpeg_quality) {
        Ok(()) => app.scan_folder(),
        Err(e) => eprintln!("Save failed: {e}"),
    }
}'''

new_save_as = '''fn save_as(app: &mut App) {
    let img = match &app.editor_state.current_image {
        Some(i) => i.clone(),
        None => return,
    };

    let base_path = match app.image_files.get(app.selected_image_index) {
        Some(p) => p.clone(),
        None => return,
    };

    let save_format = app.editor_state.save_format;
    let new_ext = crate::format_ext::format_to_extension(save_format);
    let filename = &app.editor_state.save_as_filename;
    let new_name = base_path.with_file_name(format!("{}.{}", filename, new_ext));
    match crate::format_ext::save_image(&img, &new_name, save_format, app.editor_state.save_jpeg_quality) {
        Ok(()) => {
            app.scan_folder();
            app.editor_state.save_as_filename.clear();
        }
        Err(e) => eprintln!("Save failed: {e}"),
    }
}'''

if old_save_as in content:
    content = content.replace(old_save_as, new_save_as)
    print("save_as function replaced")
else:
    print("save_as pattern not found")

with open(filepath, 'w', encoding='utf-8') as f:
    f.write(content)

print("Done")
