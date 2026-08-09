# EXR (.exr) File Type Support — Design

**Date:** 2026-08-09
**Status:** Approved
**Scope:** Add OpenEXR (`.exr`) as a viewable image format. Read-only source; Save As / batch convert to existing 8-bit formats works for free. No EXR writing.

## Goals

- Open and display `.exr` files in Browser (thumbnails) and Viewer (full image).
- EXR appears as a source format in batch convert / Save As (exportable to PNG/BMP/WebP/JPEG).
- Report correct bit depth in the status bar for HDR pixels.
- Minimal, idiomatic change — reuse the existing `image` crate decode path.

## Non-Goals

- EXR **write** support (encoding pipeline, HDR memory in editor).
- Manual exposure slider / interactive tone mapping UI.
- sRGB gamma on the HDR-to-8-bit conversion.
- Caching EXR-specific metadata (chromaticities, display window, etc.).
- New UI surfaces or architecture changes.

## Background

The project (`AGENTS.md`) currently uses `image = "0.25"` with features
`["png", "jpeg", "bmp", "gif", "tiff", "webp"]`. The `image` crate exposes an
opt-in `exr` feature that pulls in the pure-Rust `exr` crate (no unsafe, no C
dependencies, MIT/Apache-2.0). Enabling it makes `image::ImageFormat::OpenExr`
available, and both `image::open` and `image::load_from_memory` transparently
decode `.exr` files into `DynamicImage::ImageRgb32F` / `ImageRgba32F`.

All existing decode call sites use these generic entry points:

- `image_loader.rs` — `decode_to_colorimage`, `load_thumbnail`
- `editor/mod.rs:48` — `image::open(path)`
- `batch/operations.rs:11,63` — `image::open(path)`
- `batch/operations.rs:167,182,197` — tests
- `browser/grid.rs:303` — inline `image::open(path)`
- `disk_cache.rs:38` — `image::load_from_memory(&bytes)`

…so enabling the feature is sufficient; no per-callsite edits are required.

The `bit_depth` match in `decode_to_colorimage` already covers the float
variants:

- `DynamicImage::ImageRgb32F(_)  => 96`
- `DynamicImage::ImageRgba32F(_) => 128`

…so EXR's bit depth will be reported correctly in the Viewer status bar with no
new code.

`decode_to_colorimage` and `load_thumbnail` both call `img.to_rgba8()` to
produce the egui `ColorImage`. For `Rgb32F` / `Rgba32F` inputs the `image`
crate's `to_rgba8` clamps each float channel to `[0.0, 1.0]` and scales to
`0..=255`. This is the *simple clamp* tonemap the user selected: highlights
above 1.0 clip to white, no gamma applied.

Thumbnail generation uses `img.thumbnail(w, h)` which also operates on float
images — EXR thumbnails get the same clamp-tonemapped 8-bit representation,
matching what the Viewer shows.

## Approach

**Approach A (chosen): enable the `image` crate's `exr` feature.**

Rejected alternatives:

- **B — Direct `exr` crate dependency / custom decode path:** would require a
  bespoke `ColorImage` builder and thumbnail path, duplicating the existing
  pipeline. Gives more tone-mapping control, but the chosen scope is "simple
  clamp", so the extra surface is unjustified.
- **C — Gate EXR behind a Cargo feature flag:** clutters `format_ext.rs` with
  `#[cfg]` attributes; the project has no feature-flag precedent today
  (AGENTS.md: "Single-crate Rust project"), so adding one solely for EXR
  increases complexity for little binary-size benefit on a hobby project.

## Changes

### 1. `Cargo.toml`

- Add `"exr"` to the `image` features list.
- Bump patch version `0.1.51` -> `0.1.52` (per AGENTS.md versioning convention).

```toml
image = { version = "0.25", default-features = false, features = ["png", "jpeg", "bmp", "gif", "tiff", "webp", "exr"] }
```

### 2. `src/format_ext.rs`

Two mapping additions (no behavioral code):

```rust
// In extension_to_image_format:
"exr" => Some(ImageFormat::OpenExr),

// In format_to_extension:
"exr" => "exr",
```

`format_to_extension("exr")` is included for symmetry and for any future UI that
labels formats from this map; `save_image` is intentionally **not** extended to
write EXR (out of scope).

### 3. Decode call sites

No edits. All call sites already use `image::open` / `image::load_from_memory`,
which pick up the new format automatically once the feature is on.

### 4. Editor & batch (Save As / Convert)

No edits. The editor's Save As and batch convert already convert any
`DynamicImage` to PNG/BMP/WebP/JPEG via `save_image`. HDR->8-bit happens
implicitly when the editor crops/rotates and re-encodes to a non-HDR format —
works for free.

### 5. Thumbnails

No edits. `load_thumbnail` uses `image::load_from_memory` + `img.thumbnail()`,
both of which handle float images. EXR thumbnails display as clamp-tonemapped
8-bit, consistent with the Viewer.

### 6. Tests (`src/format_ext.rs`)

Extend two existing unit tests:

```rust
// test_extension_to_image_format_case_insensitive:
assert_eq!(extension_to_image_format("EXR"), Some(ImageFormat::OpenExr));

// test_is_supported_extension:
assert!(is_supported_extension(std::path::Path::new("a.exr")));
assert!(is_supported_extension(std::path::Path::new("A.EXR")));
```

No new EXR-fixture decode test is added: there is no `.exr` test asset in the
repo, and the `image` crate already tests its EXR decoder upstream. Adding a
fixture binary is out of scope.

## Verification

1. `cargo check`
2. `cargo test` — the two extended tests must pass; all other existing tests
   must remain green.
3. `cargo build --release` — produces the single binary.
4. Manual: open a sample `.exr` (e.g. from openexr.com's test assets) in the
   built viewer — confirm file appears in browser grid (thumbnail generated),
   opens in Viewer (correct dimensions, sensible 8-bit display with clipped
   highlights), Save As PNG produces a valid PNG.

## Risks & Tradeoffs

- **Compile time:** the `exr` crate adds a non-trivial pure-Rust dependency.
  Acceptable for a desktop viewer; no runtime/unsafe cost.
- **Display fidelity:** linear clamp without sRGB gamma means mid-tones look
  dark and highlights blow out. Documented as a known limitation; future
  enhancement, not this scope.
- **No EXR write:** confirmed out of scope; the editor has no HDR pipeline and
  there is no demand signal for one.
- **Memory:** `Rgb32F` / `Rgba32F` images are 12-16 bytes/pixel — larger than
  8-bit formats. The existing LRU texture cache (`app.textures`, capacity 8) and
  thumbnail cache (`ThumbnailCache::new(512, 4, ..)`) bound memory; no change
  needed.

## Out of Scope (deferred)

- EXR encoding (Save As EXR).
- Exposure / tone-mapping slider.
- sRGB gamma on HDR->8-bit.
- EXR metadata display (chromaticities, display window, channels).
- Abundant EXR channel layouts (ℓ-only, multi-part, deep) — covered by whatever
  `image` 0.25's `exr` feature supports; no extra handling here.
