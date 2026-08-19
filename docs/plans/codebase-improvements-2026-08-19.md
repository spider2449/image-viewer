# Codebase Improvement Plan

**Date:** 2026-08-19
**Scope:** Resolve the six findings from the 2026-08-19 codebase review without changing the single-crate architecture or supported image formats.

## Goals

- Make edit undo/redo behavior correct and regression-tested.
- Give Fit and 1:1 zoom unambiguous, consistent semantics.
- Prevent failed image writes and folder deletion from causing unrecoverable data loss.
- Keep the GUI responsive during batch operations and expose real progress.
- Invalidate disk thumbnails whenever the source content signature changes.

## Constraints

- Keep all code, code comments, test names, and user-facing technical descriptions in English.
- Preserve the existing egui/eframe 0.31 application structure.
- Use `cargo check` as the required verification gate and run `cargo test` for regression coverage.
- Before every commit, bump the patch version in `Cargo.toml`. Do not combine multiple commits under one version bump.
- Keep each commit focused and leave unrelated working-tree changes untouched.

## Stage 1: Make image overwrites recoverable

**Priority:** P1 — data-loss prevention

**Files:**

- `src/format_ext.rs`
- `src/batch/operations.rs`
- Existing save call sites in `src/editor/mod.rs` and `src/browser/grid.rs`

### Design

1. Split the current save implementation into:
   - an internal encoder that writes a `DynamicImage` to a specified new path; and
   - a public transactional save helper used by every save/overwrite path.
2. Encode into a uniquely named temporary file in the destination directory so the temporary and destination files are on the same filesystem.
3. Do not touch an existing destination until encoding has completed successfully.
4. For replacement on Windows, move the original to a same-directory backup, move the completed temporary file into place, and restore the backup if replacement fails.
5. Remove the backup only after replacement succeeds. Clean up temporary files on every error path.
6. Route batch resize through the same helper instead of opening the source with `File::create`.

### Tests

- Existing destinations remain byte-for-byte unchanged when encoding fails.
- Successful replacement produces a decodable image with the expected dimensions.
- JPEG replacement still honors quality and converts alpha-bearing images to RGB.
- Temporary and backup files are removed after success.
- A simulated replacement failure restores the original. Isolate filesystem replacement behind a small private helper so this path can be tested deterministically.

### Verification

```powershell
cargo test format_ext
cargo test batch::operations
cargo test
cargo check
```

## Stage 2: Correct the editor history model

**Priority:** P1 — core editor behavior

**Files:**

- `src/editor/mod.rs`
- Optionally `src/editor/operations.rs` if a small reusable history type is extracted

### Design

1. Treat each undo entry as the state to restore and each redo entry as the state produced by the undone edit.
2. On edit:
   - push the current image onto undo;
   - clear redo;
   - apply the operation and install the result.
3. On undo:
   - move the current edited image onto redo;
   - restore the image popped from undo.
4. On redo:
   - move the current image back onto undo;
   - restore the edited image popped from redo.
5. Apply the same transition to clipboard paste.
6. Centralize the 50-entry and 64 MiB limits so edit, undo, redo, and paste cannot bypass them. Count the actual image buffer size rather than assuming every image is RGBA8 where practical.
7. Keep texture upload and resize-field synchronization as UI-side effects after a successful history transition.

### Tests

- Rotate -> undo -> redo restores the rotated pixels and dimensions.
- Resize -> undo -> redo restores both states in order.
- Paste/replacement -> undo -> redo restores the pasted image.
- A new edit after undo clears redo.
- History limits evict the oldest state without removing the newest usable undo state.

Avoid tests that only push and pop tuples; exercise the complete state transition API.

### Verification

```powershell
cargo test editor
cargo test
cargo check
```

## Stage 3: Normalize Fit, 1:1, and manual zoom

**Priority:** P1 — core viewer behavior

**Files:**

- `src/viewer.rs`
- `src/app.rs` if image selection resets the zoom mode

### Design

1. Define zoom as an absolute texture-to-screen scale:
   - `1.0` means one screen point per source pixel (1:1);
   - `fit_zoom` is `min(available_width / image_width, available_height / image_height)`.
2. Render with `display_size = texture_size * zoom`; do not multiply by the fit scale twice.
3. Add an explicit zoom mode, preferably `Fit` and `Manual`:
   - Fit mode uses the newly computed `fit_zoom` each frame and therefore follows window/panel resizing.
   - 1:1, slider changes, and mouse-wheel changes switch to Manual mode.
4. Enter Fit mode when opening a new image unless a deliberate product decision is made to preserve manual zoom across navigation.
5. Keep pan reset behavior for Fit and 1:1. Clamp manual zoom to the existing 0.1..=32.0 range.
6. Update the status display so its percentage matches the absolute zoom.

### Tests

Extract pure scale calculations and test:

- A 4000x3000 image in a 1000x750 area has `fit_zoom == 0.25` and displays at 1000x750 in Fit mode.
- The same image displays at 4000x3000 at 1:1.
- A small image receives the explicitly chosen behavior: upscale-to-fit or cap Fit at 1.0. Document that choice in the test name.
- Resizing the viewport recomputes Fit mode but does not overwrite Manual mode.

### Verification

```powershell
cargo test viewer
cargo test
cargo check
```

Perform a manual smoke test with one image larger than the window and one smaller than the window.

## Stage 4: Make folder deletion recoverable and explicit

**Priority:** P1 — data-loss prevention

**Files:**

- `src/browser/tree.rs`
- `src/browser/mod.rs` for pending-action and error state
- `src/browser/files.rs` if folder deletion is routed through the existing file-operation boundary

### Design

1. Replace direct `remove_dir_all` with a pending delete request stored in browser state.
2. Show a confirmation dialog containing the full folder path and an explicit cancel action.
3. On confirmation, use `trash::delete` so supported platforms move the folder to the recycle bin/trash.
4. Surface failures in the UI; do not discard the result or rely only on stderr in a release GUI application.
5. After success, clear the active folder and image state when the deleted folder is equal to or an ancestor of `current_folder` (`current_folder.starts_with(deleted_path)`).
6. Rebuild the tree and invalidate affected thumbnail, texture, metadata, editor, and EXIF state.

### Tests

- Requesting deletion does not mutate the filesystem before confirmation.
- Cancel leaves the folder untouched.
- Successful deletion clears an exact current-folder selection.
- Successful deletion also clears a current folder nested below the deleted folder.
- A failed trash operation retains application state and produces a visible error message.

Use a replaceable private deletion function or operation result handler so tests do not depend on the operating system's recycle-bin service.

### Verification

```powershell
cargo test browser::files
cargo test browser
cargo test
cargo check
```

Manually verify confirmation and recycle-bin recovery on Windows.

## Stage 5: Move batch work off the egui thread

**Priority:** P2 — responsiveness and honest progress

**Files:**

- `src/batch/mod.rs`
- `src/batch/operations.rs`
- `src/app.rs` only if polling is centralized there

### Design

1. Keep the image-processing functions independent of egui and make them report one result per file.
2. When Apply is pressed, snapshot all inputs into owned, `Send` values and start one background worker thread.
3. Send typed events back to the UI, for example:
   - `Started { total }`
   - `FileFinished { path, result }`
   - `Finished`
4. Poll events with `try_recv` during normal updates. Update `progress_current`, `progress_total`, and the error log from those events.
5. Display a determinate progress bar because the selected file count is known.
6. Add a cooperative cancellation flag checked between files. Label cancellation accurately; do not imply that the currently encoding file can be interrupted safely.
7. Disable mode/input changes while a job is running, but keep the window movable, repainting, and cancellable.
8. Rescan the folder once after `Finished`, not after each file.
9. Ensure closing the batch window does not silently abandon ownership of a running receiver. Either keep the task visible until completion or label the action as detaching and preserve result polling elsewhere.

### Tests

- Progress events are emitted once for every selected file.
- Mixed success/failure batches reach `Finished` and preserve all error messages.
- Cancellation stops before starting the next file.
- An empty selection finishes without hanging.
- The UI state reaches `running == false` after worker completion or disconnection.

### Verification

```powershell
cargo test batch
cargo test
cargo check
```

Manually run a batch large enough to confirm that the window remains responsive and progress advances.

## Stage 6: Strengthen thumbnail source signatures

**Priority:** P2 — stale cache correctness

**Files:**

- `src/disk_cache.rs`

### Design

1. Replace whole-second mtime keys with a source signature containing:
   - path hash;
   - modification time at nanosecond precision where the platform provides it;
   - source file length;
   - requested thumbnail size.
2. Keep path hash as the prefix so `delete_old_entries` continues to remove obsolete versions for one source.
3. Treat metadata or timestamp failures as cache misses.
4. Continue accepting existing cache files as orphaned legacy entries only for cleanup; do not return them without the new signature.
5. Keep cache writes off decoder threads as in the current architecture.

### Tests

- Changing source length within the same second produces a different cache path and a cache miss.
- Different requested sizes produce different cache paths.
- Replacing a source causes the new thumbnail to be returned, not the previous JPEG.
- Storing a new signature removes older entries sharing the path-hash prefix.

### Verification

```powershell
cargo test disk_cache
cargo test
cargo check
```

## Final Integration Pass

1. Run formatting without accepting unrelated rewrites:

```powershell
cargo fmt --check
```

2. Run the complete verification suite:

```powershell
cargo test
cargo check
```

3. Build the distributable binary:

```powershell
cargo build --release
```

4. Perform a Windows GUI smoke test covering:
   - large and small image Fit/1:1 behavior;
   - edit -> undo -> redo for rotate, resize, crop, and paste;
   - Save As over an existing destination;
   - batch progress, cancellation, and mixed errors;
   - confirmed folder deletion and recycle-bin recovery;
   - thumbnail refresh immediately after editing a source.

5. Review README controls after implementation and update only statements affected by the final behavior.

## Recommended Commit Boundaries

Use one patch-version bump per commit:

1. `fix: make image replacement recoverable`
2. `fix: restore correct editor redo states`
3. `fix: normalize viewer zoom semantics`
4. `fix: confirm and trash folder deletions`
5. `feat: run batch operations in background`
6. `fix: strengthen thumbnail cache invalidation`
7. `docs: align controls and behavior` only if documentation changes are necessary; this commit requires its own patch bump under the project convention.

If implementation reveals that a stage cannot remain independently buildable, combine only the tightly coupled code changes and still leave the repository passing `cargo check` at every commit boundary.
