# Complete Editor History Plan

**Date:** 2026-08-19
**Scope:** Make every edit in the current unsaved document reversible, including live adjustment previews.

## Problem

The current undo stack stores full image snapshots and discards old entries after 50 states or 64 MiB. A large image can therefore retain only one or two undo steps. Live adjustment previews are also outside the history model, so Undo can appear unavailable while the displayed image differs from the committed image.

## Implementation

1. Keep the image loaded from disk as the session baseline.
2. Replace bounded image snapshots with an ordered `EditOp` history and a cursor.
3. Rebuild an earlier state by replaying active operations from the baseline, so image size no longer causes old edits to be discarded.
4. Represent clipboard replacement as a history operation so paste is reversible through the same mechanism.
5. Treat Undo during a live adjustment preview as cancelling that preview first.
6. Add Revert All, preserving the full operation list so Redo can restore reverted edits.
7. Add Ctrl+Z, Ctrl+Y, and Ctrl+Shift+Z shortcuts while the editor is open.
8. Commit a pending visible adjustment before Save As so the saved file matches the preview.
9. Display the unsaved edit count and update regression tests for long histories, branching, paste, preview, revert, and redo.
10. Prevent image navigation or returning to Browser from silently clearing an unsaved editing session.

## Verification

```powershell
cargo test editor
cargo test
cargo check
```

## Result

- Replaced bounded image snapshots with complete operation history.
- Added preview-aware Undo, Revert All, redo-preserving history, and keyboard shortcuts.
- Save As now commits the visible live adjustment before encoding.
- Image navigation and returning to Browser now require explicit confirmation before discarding an unsaved session.
