# Apply Save As filename input feature

$filePath = "src/editor/mod.rs"
$content = Get-Content $filePath -Raw

# 1. Add save_as_filename field to State struct
$content = $content.Replace(
    "pub save_jpeg_quality: u8,",
    "pub save_jpeg_quality: u8,`n    pub save_as_filename: String,"
)

# 2. Initialize save_as_filename in State::new()
$content = $content.Replace(
    "save_jpeg_quality: 90,",
    "save_jpeg_quality: 90,`n            save_as_filename: String::new(),"
)

# 3. Add text input in UI before Save As button
$content = $content.Replace(
    'if ui.button("Save As...").clicked() {`n                    save_as(app);`n                }',
    '                // Filename input`n                egui::TextEdit::singleline(&mut app.editor_state.save_as_filename)`n                    .hint_text("Enter filename")`n                    .show(ui);`n                if ui.button("Save As...").clicked() && !app.editor_state.save_as_filename.is_empty() {`n                    save_as(app);`n                }'
)

# 4. Replace save_as function to use the filename
$oldSaveAs = @'
fn save_as(app: &mut App) {
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
}
'@

$newSaveAs = @'
fn save_as(app: &mut App) {
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
}
'@

$content = $content.Replace($oldSaveAs, $newSaveAs)

Set-Content $filePath -Value $content -NoNewline
Write-Host "Applied Save As filename input feature"
