# Architecture and feasibility

The specification's eframe/tray spike compiled and opened a packaged editor, but retaining a hidden eframe window also retains graphics resources. The subsequent requirement to release those resources on close changes lifecycle ownership.

## Process ownership

- `snapnote`: CLI dispatcher or windowless resident. Its macOS winit event loop owns tray-icon and global-hotkey on the main thread. `ControlFlow::Wait` and event-loop proxies handle events; it has no native editor window, egui context, OpenGL context, Metal device, texture, or decoded screenshot pixels.
- `snapnote-editor`: on-demand eframe/egui helper. It owns preview decoding, text, storage work, clipboard operations, the window, and its renderer. Closing or completing a save exits the process. Escape in the popup saves; right-click discards its uncommitted draft and exits. Escape in native selection cancels. The OS then reclaims the entire address space and graphics resources, rather than relying on allocator trimming in a resident UI process.

Both executables live in one application bundle. The helper has its own bundle identifier so macOS accessibility and application inspection can distinguish it from the windowless resident. The resident links no OpenGL, Metal, or wgpu library. No additional application framework was introduced: eframe already uses winit 0.30.

A kernel advisory file lock prevents duplicate residents for the same configuration. CLI subcommands dispatch before native event-loop initialization and require no resident. Development builds locate the sibling helper executable; packaged builds locate the embedded helper bundle.

## Capture and editor transitions

The resident starts native `screencapture -i -s -x` on a worker. The temporary PNG has a random, non-hidden name and private permissions. A real macOS regression showed that `screencapture` rejects dot-prefixed filenames, emits a diagnostic, and can still exit 0; therefore both output streams and the actual PNG are checked. Missing/empty output without diagnostics is cancellation, including when the native selector removes the temporary file. The resident checks output existence, nonzero size, and a readable PNG header. It never decodes pixels. The helper adopts that owned temporary file, fully decodes and validates the image on a worker, prepares a preview capped at 1600 pixels per dimension, and releases the decoded source after transferring the preview texture. Save actions are unavailable until validation succeeds.

The parent retains only the temporary pathname for crash cleanup. The helper removes the temporary capture after successful saving or cancellation. The parent also removes it after helper exit if necessary. A native selection cancellation creates no persistent snippet. Permission denial is distinguished using CoreGraphics preflight after a failed/empty capture; errors provide recovery guidance. A permission request is only initiated by the user's Settings button, in the resident process.

When a helper already exists, the resident sends short commands over its private stdin pipe: capture, finish, settings, recent, or quit. The helper wakes egui through a channel and `request_repaint`; there is no polling loop. Recapture exits with internal status 10. Only after the child has exited does the resident invoke the native selector again. Internal status 11 requests the system permission dialog in the resident. These statuses are not public CLI exit codes.

There is no socket, network listener, web server, or shared global screenshot buffer. No perpetual repaint request or animated busy spinner is used. Rendering follows window/input events, worker completions, and explicit control messages. The editor's text cursor may request normal visible-window blink repaints.

## Persistence and clipboard boundaries

Storage, formatting, and OS clipboard writes are independent library operations. A `Draft` exclusively reserves a collision-resistant directory, copies and syncs the original PNG, and exposes its real absolute path before text input is shown. The popup starts with that path followed by a newline. Cmd+B sends `finish` to an existing capture helper; otherwise it starts selection. Ctrl+C copies the complete message without closing. Preview dragging delegates to the native window manager.

Finalization writes the exact editable message in a Markdown wrapper, then atomically publishes `metadata.json` as the commit marker. Retrieval ignores uncommitted directories. Save failure returns ownership of the draft for retry. Normal abandoned drafts roll back through TempDir ownership; a hard kill can leave an ignored incomplete directory. Clipboard errors do not delete or finalize the draft. The temporary native screenshot is released after PNG staging; full-resolution decoded pixels are released after texture creation.

Configuration uses a schema version, defaults, syntax validation, and a synced temporary file followed by atomic replacement. Settings preserve the current capture; the resident applies shortcut changes after editor exit. The resident registers a new shortcut before unregistering the old one; a conflict retains the working shortcut and restores its configuration value. A malformed configuration is never automatically overwritten.

## Platform and dependency decisions

The native capture and CoreGraphics permission calls are isolated under `src/capture/macos.rs`. The shared storage/clipboard/formatting/CLI layers contain no macOS capture API. Future adapters can return the same temporary-file result without redesigning these layers. Windows/Linux adapters remain deferred.

The release builds use stable Rust 1.98.1, eframe/egui 0.37.0, winit 0.30.13, tray-icon 0.26.1, global-hotkey 0.9.0, arboard 3.7.0, image 0.25.10, clap 4.6.7, TOML 1.1.8, and serde. Cargo.lock records exact transitive versions. Native accessibility support is enabled. The glow and wgpu Cargo features are independent; the comparison builds include one renderer each. Diagnostic timings and errors go to stderr; screenshot pixels and context are never logged by the app.

Primary API references checked during implementation:

- https://docs.rs/eframe/0.37.0/eframe/trait.App.html
- https://docs.rs/eframe/0.37.0/eframe/struct.NativeOptions.html
- https://docs.rs/tray-icon/0.26.1/tray_icon/
- https://docs.rs/global-hotkey/0.9.0/global_hotkey/
- https://docs.rs/winit/0.30.13/winit/application/trait.ApplicationHandler.html
- https://developer.apple.com/documentation/coregraphics/cgpreflightscreencaptureaccess()
- https://developer.apple.com/documentation/coregraphics/cgrequestscreencaptureaccess()

Native clipboard support is explicitly enabled on eframe. Without that feature, egui-winit silently uses an application-local clipboard and external Cmd+V/Hex dictation cannot insert text. Ctrl+C is handled after TextEdit consumes pending input, so its full-message snapshot includes text pasted in the same frame.
