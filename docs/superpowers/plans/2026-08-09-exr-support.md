# EXR (.exr) View Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `.exr` (OpenEXR) files appear in the browser, decode + display in the Viewer, generate thumbnails, and be exportable to PNG/BMP/WebP/JPEG via existing Save As / batch convert — read-only source, no EXR writing.

**Architecture:** Enable the `image` crate's `exr` Cargo feature (pulls in the pure-Rust `exr` crate). All existing decode call sites already use `image::open` / `image::load_from_memory`, which pick up the new format automatically. The only source edits are two mapping entries in `format_ext.rs` (`extension_to_image_format`, `format_to_extension`) plus matching tests. HDR float pixels (Rgb32F / Rgba32F) are converted to 8-bit by the existing `img.to_rgba8()` call — a simple clamp tonemap (highlights clip to white, no gamma), per design choice.

**Tech Stack:** Rust 2021, `image = "0.25"` (adding `exr` feature), `egui`/`eframe 0.31`, `lru 0.12`.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-08-09-exr-support-design.md` (approved).
- Verification that matters: `cargo check`. Plus `cargo test` (unit tests in `format_ext.rs`). (`AGENTS.md`.)
- After this change, `image` features list is `["png", "jpeg", "bmp", "gif", "tiff", "webp", "exr"]` — `default-features = false` is preserved.
- Versioning convention (`AGENTS.md`): bump the patch version in `Cargo.toml` before each commit. Current is `0.1.51`; this plan bumps to `0.1.52`.
- All code, comments, tests, identifiers MUST be English.
- No new comments in source unless asked.
- No new files other than this plan + the spec already committed.

## File Structure

- Modify: `Cargo.toml` — add `"exr"` to `image` features; bump patch version.
- Modify: `src/format_ext.rs` — add `"exr"` => `ImageFormat::OpenExr` in `extension_to_image_format`; add `"exr" => "exr"` in `format_to_extension`; extend two existing unit tests.
- No other source files change (decode paths are format-agnostic).

---

### Task 1: Add EXR extension/format mapping and enable feature (TDD)

**Files:**
- Modify: `src/format_ext.rs:4-14` (`format_to_extension`), `src/format_ext.rs:16-26` (`extension_to_image_format`), `src/format_ext.rs:69-75` (`test_extension_to_image_format_case_insensitive`), `src/format_ext.rs:77-84` (`test_is_supported_extension`)
- Modify: `Cargo.toml:3` (version), `Cargo.toml:10` (`image` features)

**Interfaces:**
- Consumes: `image::ImageFormat::OpenExr` (becomes available once the `exr` feature is enabled in Cargo.toml; the variant exists in `image 0.25` but is only constructable when the feature is on).
- Produces: `crate::format_ext::is_supported_extension(&Path)` now returns `true` for paths ending in `.exr` / `.EXR` (case-insensitive). All existing callers (`app::scan_folder` at `src/app.rs:123`, `browser/grid.rs`, `batch`, `editor`, `disk_cache`) automatically gain EXR support with no further edits.

**Rationale for one task, not three:** the test assertions reference `ImageFormat::OpenExr`, which only compiles after the Cargo feature is enabled. Splitting "add tests" from "enable feature" would leave the project in a non-compiling state between commits, breaking `cargo check` — violating the "every commit builds" rule. The test additions and feature enable are therefore atomic; the mapping edits (which the tests assert) belong in the same commit so the tests pass on the first run.

- [ ] **Step 1: Write the failing tests**

In `src/format_ext.rs`, extend `test_extension_to_image_format_case_insensitive` to add the EXR case. The test currently ends at line 74 with `assert_eq!(extension_to_image_format("xyz"), None);`. Add the EXR assertion **before** that `None` line:

```rust
    #[test]
    fn test_extension_to_image_format_case_insensitive() {
        assert_eq!(extension_to_image_format("PNG"), Some(ImageFormat::Png));
        assert_eq!(extension_to_image_format("Jpg"), Some(ImageFormat::Jpeg));
        assert_eq!(extension_to_image_format("JPEG"), Some(ImageFormat::Jpeg));
        assert_eq!(extension_to_image_format("EXR"), Some(ImageFormat::OpenExr));
        assert_eq!(extension_to_image_format("xyz"), None);
    }
```

Then extend `test_is_supported_extension` to add two EXR cases (lowercase + uppercase). The current body is lines 78-83. Replace it with:

```rust
    #[test]
    fn test_is_supported_extension() {
        assert!(is_supported_extension(std::path::Path::new("a.PNG")));
        assert!(is_supported_extension(std::path::Path::new("a.Jpg")));
        assert!(is_supported_extension(std::path::Path::new("a.tiff")));
        assert!(is_supported_extension(std::path::Path::new("a.exr")));
        assert!(is_supported_extension(std::path::Path::new("A.EXR")));
        assert!(!is_supported_extension(std::path::Path::new("a.txt")));
        assert!(!is_supported_extension(std::path::Path::new("noext")));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib format_ext`
Expected: COMPILE ERROR — `ImageFormat::OpenExr` not found / no variant named `OpenExr`, because the `exr` feature is not yet enabled. Additionally, `extension_to_image_format("EXR")` returns `None`, so `test_extension_to_image_format_case_insensitive` would fail at runtime once the variant exists. Both are fixed by the next steps.

- [ ] **Step 3: Add the EXR mapping to `extension_to_image_format`**

In `src/format_ext.rs`, edit the `extension_to_image_format` match (lines 16-26). Add the `"exr"` arm before the `_ => None` arm:

```rust
pub fn extension_to_image_format(ext: &str) -> Option<ImageFormat> {
    match ext.to_lowercase().as_str() {
        "png" => Some(ImageFormat::Png),
        "jpg" | "jpeg" => Some(ImageFormat::Jpeg),
        "bmp" => Some(ImageFormat::Bmp),
        "gif" => Some(ImageFormat::Gif),
        "tif" | "tiff" => Some(ImageFormat::Tiff),
        "webp" => Some(ImageFormat::WebP),
        "exr" => Some(ImageFormat::OpenExr),
        _ => None,
    }
}
```

- [ ] **Step 4: Add the EXR mapping to `format_to_extension`**

In the same file, edit `format_to_extension` (lines 4-14). Add `"exr" => "exr"` before the `_ => ""` arm:

```rust
pub fn format_to_extension(format: &str) -> &'static str {
    match format {
        "jpeg" => "jpg",
        "png" => "png",
        "bmp" => "bmp",
        "webp" => "webp",
        "gif" => "gif",
        "tiff" => "tif",
        "exr" => "exr",
        _ => "",
    }
}
```

(Although `save_image` is not extended to write EXR — out of scope per the spec — the mapping is added for symmetry and any future UI that derives an extension label from the format string. No runtime consumer changes behavior.)

- [ ] **Step 5: Enable the `exr` feature and bump patch version in `Cargo.toml`**

Edit `Cargo.toml` line 3 and line 10.

```toml
[package]
name = "image-viewer"
version = "0.1.52"
edition = "2021"

[dependencies]
eframe = "0.31"
egui = "0.31"
exif = { version = "0.6", package = "kamadak-exif" }
image = { version = "0.25", default-features = false, features = ["png", "jpeg", "bmp", "gif", "tiff", "webp", "exr"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
lru = "0.12"
open = "5"
trash = "5"
arboard = "3"
```

- [ ] **Step 6: Run `cargo check` to confirm the feature enables the variant**

Run: `cargo check`
Expected: completes successfully, downloads and compiles the `exr` crate (first time only). No warnings or errors related to `OpenExr`.

- [ ] **Step 7: Run the unit tests to verify they pass**

Run: `cargo test --lib format_ext`
Expected: all 5 tests in `format_ext` pass, including the two extended with EXR cases (`test_extension_to_image_format_case_insensitive`, `test_is_supported_extension`).

- [ ] **Step 8: Run the full test suite to confirm no regressions**

Run: `cargo test`
Expected: all tests pass. Per `AGENTS.md`: "Tests: 11 unit tests in `editor::operations` and `browser::grid`" plus the `format_ext` tests and `app::tests::test_textures_lru_evicts_old_entries`. No new failures.

- [ ] **Step 9: Run `cargo build --release` to confirm the release binary compiles**

Run: `cargo build --release`
Expected: builds the single binary `target/release/image-viewer.exe` with the `exr` feature linked in.

- [ ] **Step 10: Commit**

```bash
git add Cargo.toml src/format_ext.rs
git commit -m "feat(format): support .exr (OpenEXR) files as viewable source

Enable the image crate's `exr` Cargo feature and add \"exr\" mappings to
extension_to_image_format / format_to_extension. All decode call sites
(image::open / image::load_from_memory) pick up the new format
automatically. HDR float pixels are clamp-tonemapped to 8-bit via the
existing img.to_rgba8() path. EXR is a read-only source; Save As / batch
convert re-use existing PNG/BMP/WebP/JPEG writers.

Bumps patch version 0.1.51 -> 0.1.52 per AGENTS.md convention."
```

---

## Post-Implementation Manual Verification (not in plan scope)

These are not automation steps — they confirm the integration end-to-end. The implementer or reviewer can run them after Task 1 is committed, using a known-good `.exr` sample (e.g. from openexr.com's test images, or any HDR render):

1. Launch `target/release/image-viewer.exe`.
2. Use the folder tree to navigate to a folder containing a `.exr` file.
3. Confirm: the `.exr` appears in the thumbnail grid; a thumbnail is generated within a second or two (background worker + disk cache path).
4. Double-click the `.exr` thumbnail to open the Viewer. Confirm: image displays (highlights may clip to white — expected for the simple-clamp tonemap), dimensions in the status bar match the file, bit depth reads 96 (RGB float) or 128 (RGBA float).
5. With an EXR open, open the editor panel and use **Save As** to export to PNG. Confirm a non-empty PNG is written beside the source and opens in any image viewer.
6. Open **Tools → Batch Convert**, select an EXR-containing folder, target PNG, run. Confirm output PNGs are written.

If any step fails, the issue is in the `image` crate's EXR decoder (upstream), not in this change set — the surface area added here is two match-arms and one Cargo feature.

## Spec Coverage

Spec section -> this plan:

- `Cargo.toml` (image feature + version bump) -> Task 1, Step 5
- `extension_to_image_format` mapping -> Task 1, Step 3
- `format_to_extension` mapping -> Task 1, Step 4
- Decode call sites "no edits" -> confirmed: no task touches them; embraced in Task 1's Produces/Consumes note
- Editor & batch "no edits" -> confirmed: no task
- Thumbnails "no edits" -> confirmed: no task
- Tests in `format_ext.rs` -> Task 1, Step 1
- Verification `cargo check` + `cargo test` + `cargo build --release` -> Task 1, Steps 6-9

All spec sections represented. No placeholders.
