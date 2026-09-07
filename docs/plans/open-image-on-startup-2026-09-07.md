# Open an image passed at startup

## Goal

When Windows launches the image viewer for a file opened from Explorer, show
that image immediately in Viewer mode instead of starting in Browser mode.

## Plan

1. Read the first command-line argument in `main.rs` and pass it to the app
   creator.
2. In `App::new`, validate and canonicalize the startup image, scan its parent
   directory, and select the matching image so adjacent-image navigation keeps
   working.
3. Keep the current last-folder restoration path for normal launches and
   document the command-line/file-association behavior.
4. Run formatting, `cargo check`, and the existing test suite.
