# SnapNote

A local-first Rust screenshot and context utility for macOS Apple Silicon. Select a region, describe it, and save a PNG, Markdown note, and JSON metadata. Copy text, a local screenshot reference, or actual image pixels for use in other applications.

The idle application is a windowless menu-bar process. The editor runs in a separate process, created when needed and terminated when closed. Its window, renderer, textures, and screenshot buffers are released on exit. There is no WebView, server, database, telemetry, upload, or continuous screenshot monitoring.

## Build and launch

See [BUILDING.md](BUILDING.md) for manual rebuild, installation, launch, and permission-recovery commands. Rebuilding `dist` alone does not update the installed Applications copy.

Requires a current stable Rust toolchain, macOS, and Xcode command line tools. The local release bundle is ad-hoc signed; it is not notarized for distribution.

```sh
./scripts/build-app.sh
open dist/SnapNote.app
```

The bundle includes the resident executable and an embedded editor helper. Keep the entire bundle together. To install, quit SnapNote and copy `dist/SnapNote.app` to `~/Applications/` or `/Applications/` using Finder. Grant permissions to the copy you actually run.

For development, build both executables before launching:

```sh
cargo build --bins
cargo run
```

## Capture and save

1. Press **Cmd+B** to start the native region selector, then drag and release. Escape cancels selection.
2. A small preview appears above the release cursor with a focused message bubble beneath it. The actual PNG path is already inserted; type optional context after it.
3. Press **Cmd+C** or **Ctrl+C** to copy the complete path and message, if wanted. The popup stays open. A green checkmark and “Copied” appear at the bottom right for one second after success.
4. Press **Cmd+B** again to save and close. **Escape** also saves and closes the popup. The green checkmark saves and closes. The red × deletes the unsaved capture and message, then closes. Closing never copies automatically. **Right-click anywhere in the popup** saves the completed capture and closes. Cancelling native region selection leaves no saved capture.

Drag the screenshot preview to move the popup. Its position is clamped to the capture display initially. The PNG is staged before the input appears, so a copied reference points to an existing file. Finalizing writes Markdown and metadata; the helper process exits and releases its renderer and textures. A failed save retains the popup and draft for recovery.

The original PNG is preserved without recompression; only the preview is downscaled. No context is required. The tray provides Settings, Recent Captures, Open Capture Folder, New Capture, and Quit. Settings are outside the capture flow. Quit during native selection waits for selection to finish or Escape to cancel it.

## Clipboard

The popup's **Cmd+C** and **Ctrl+C** copy exactly the editable message: an absolute PNG path, newline, and any context entered. Both copy shortcuts copy the full path and message even when text is selected. **Cmd+V** and dictation tools that paste through the system clipboard are supported. Local paths are references, not automatic image attachments; a receiving agent must read or attach the image. A remote agent cannot automatically access this Mac's files.

The CLI additionally supports context-only, path-only, agent text, actual image pixels, and save-only modes. Agent formatting avoids duplicating the path when the saved message already starts with it. Closing leaves the clipboard unchanged. Right-click, Escape, Cmd+B, and the green checkmark preserve the image and message; the red × deletes the unsaved capture. A previously copied reference to a deleted capture will no longer work.

## Storage and configuration

Default storage is `~/screenshots/snapshot/`. Each uniquely named snippet directory contains:

```text
2026-10-22-183700-Ab12Cd/
  capture.png
  context.md
  metadata.json
```

Snippet directories are private to the user on macOS. Metadata is committed only after image and Markdown writes complete. Ordinary save failures roll back the incomplete directory. A hard process termination can leave a directory without metadata; retrieval ignores it. Nothing is automatically deleted from the saved library. To remove a saved snippet, move its complete folder to Trash using Finder.

Settings allow changing the capture directory and shortcut. The compact popup always exits after saving. Saving settings preserves the current screenshot and context. Storage changes are available immediately for retry; shortcut changes apply when the editor closes. A conflicting shortcut is rejected while retaining the working shortcut. The initial theme is neutral dark; launch-at-login is not implemented.

Configuration is shared by GUI and CLI at:

```text
~/Library/Application Support/SnapNote/config.toml
```

Example:

```toml
schema_version = 1

[capture]
hotkey = "Command+B"

[storage]
directory = "~/screenshots/snapshot"

[clipboard]
default_mode = "agent"

[ui]
close_after_save = true
```

Use `--config /absolute/path/config.toml` or `SNAPNOTE_CONFIG` to select another configuration. The GUI explains malformed configuration and offers defaults without overwriting the original until preferences are saved. CLI operations report the error on stderr instead of silently reading a different library.

## CLI

Use the resident executable directly; these subcommands do not start a GUI or require the menu-bar app to be running:

```sh
./target/release/snapnote latest
./target/release/snapnote list
./target/release/snapnote show <id>
./target/release/snapnote path <id>
./target/release/snapnote copy <id> --mode agent
./target/release/snapnote open-folder
```

Copy modes are `context`, `path`, `agent`, `image`, and `save-only`. Quote configuration paths containing spaces. `path` writes only the image path and a newline. Exit codes are 0 for success, 1 for an operation failure, and 2 for invalid command-line arguments. Missing or unsafe IDs and missing snippet files are reported as errors.

The packaged CLI is `SnapNote.app/Contents/MacOS/SnapNote`. An optional shell alias or symlink can point to that executable; no PATH modification is performed automatically.

## Screen Recording permission

Enable **SnapNote** in System Settings → Privacy & Security → Screen & System Audio Recording. A permission request button is available in SnapNote Settings. Quit and relaunch after granting access if macOS requires it. Do not grant the editor helper screen access: capture belongs to the resident.

Ad-hoc rebuilds change the application's code-signing identity. An existing enabled toggle may refer to an older build and still fail permission checks. If that happens, remove the old SnapNote entry, add the final bundle you are running, enable it, and relaunch. Development and installed copies may receive different permission decisions. No permission is bypassed or changed automatically.

## Validation and renderer comparison

```sh
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo fmt --check
# Explicitly accesses and temporarily modifies the desktop pasteboard:
cargo test --locked --test native_clipboard_tests -- --ignored

# Build the alternative renderer in an isolated target/output directory:
./scripts/build-app.sh wgpu
```

The default build uses **glow**. The comparison bundle is `dist/comparison/wgpu/SnapNote.app`. The resident creates no renderer in either build. See [test report](docs/test-report.md), [performance measurements](docs/performance.md), and [architecture](docs/architecture.md) for measured results, pending integration checks, and limitations. Windows and Linux capture adapters are intentionally not implemented.

## Annotations and growing input

The input grows with its content up to 144 logical pixels, then scrolls. Hover over the screenshot for Move, Pen, Arrow, Box, Undo, and Redo. Move drags the popup; the drawing tools add red marks. Cmd+Z and Cmd+Shift+Z undo and redo annotations when the message field is not focused; normal text undo remains available while typing. PNG annotations are applied off the UI thread before copying its reference or saving. Unedited images keep their original PNG bytes.
