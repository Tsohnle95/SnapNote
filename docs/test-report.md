# Test report — macOS MVP implementation

Date: 2026-10-22. Platform: Apple M3 Pro, macOS 27.0.1, stable Rust 1.98.1.

## Status

The user-assisted native region-selection path now produced a real preview and saved the screenshot and entered context to `~/screenshots/snapshot/`. The subsequent compact-popup revision changes the interaction to Cmd+B capture/save-close and Ctrl+C copy. Its generated-fixture test verified focused input, appended text, exact clipboard readback, and Escape closing the helper. Physical capture through the revised popup remains pending.

The frameless preview initially could not be moved. It now starts a native window drag on mouse-down over the preview, before the egui drag threshold, as required by AppKit. This revision has not yet received a physical drag acceptance check.

## Automated verification

`cargo test --locked`: **23 passed**, with three explicit OS tests excluded from the default run. Coverage includes:

- Capture cancellation cleanup, invalid image rejection despite process success, actionable process-error guidance, and temporary-file ownership. Missing output plus diagnostics is rejected even with exit status 0.
- PNG byte preservation, Markdown text preservation, empty and Unicode context, JSON schema/dimensions, absolute paths containing spaces.
- Forty rapid saves without collisions, denied destination, invalid PNG, incomplete directory exclusion, missing image files, traversal and symlink rejection.
- An injected disk-full failure after PNG writing; the incomplete directory is removed and the source capture remains.
- Correct ordering across timezone offsets/daylight-saving transitions.
- Context/path/agent formatting, multiline text, special characters, and distinguishing image/save-only from text output.
- Clipboard failure preserving saved files.
- Partial configuration defaults, atomic preference persistence, malformed configuration/schema/shortcut/path rejection without overwriting the original.
- CLI latest/list/show/path ordering and output; missing IDs, missing images, invalid modes/arguments, stderr, exit codes, malformed config, and no GUI launch for CLI subcommands.

`cargo clippy --locked --all-targets -- -D warnings`: passed. `cargo fmt --check`: passed. Both release application bundles built. `codesign --verify --deep --strict` passed for both.

The following explicit real macOS OS tests passed outside the filesystem sandbox (inside it, the OS pasteboard was inaccessible):

- `cargo test --locked native_tool_writes_supported_private_temporary_filename -- --ignored`: native 20×20 PNG capture to the production temporary-file factory, valid pixels, private 0600 permissions, and automatic cleanup.
- `cargo test --locked native_text_and_image_round_trip -- --ignored`: text and two-pixel image round trips.
- `cargo test --locked --test native_clipboard_tests -- --ignored`: all five CLI modes, exact Unicode/multiline text, PNG pixels, empty context, and Save Only preserving the clipboard. Prior text or image contents were restored after each test; richer clipboard formats are outside that restoration helper's scope.

## Packaged GUI checks

Observed in the real packaged application:

- Tray/hotkey initialization and a resident process waiting in the native event loop.
- Physical global shortcut opens native region selection (user-assisted).
- The permission failure appears as actionable UI guidance.
- A separate editor helper displays the generated 1600×1000 PNG at preserved aspect ratio.
- Native accessibility exposes controls; the optional text field receives focus.
- Real GUI Save + Copy saves Unicode multiline context, reports “Screenshot reference and context copied,” and disables duplicate saving.
- The CLI retrieves the same saved path and exact context.
- Clicking the editor's native close button terminates the editor process while the resident survives. Reopening a fresh helper works.
- Both glow and wgpu preview windows rendered successfully; physical memory was sampled. See performance.md for actual numbers and limits.
- Source and native link inspection confirm the resident has no editor graphics context, no screenshot pixel decode, and no OpenGL/Metal renderer link. No continuous application repaint request or idle timer exists.

The earlier hidden-window feasibility design was replaced, not accepted as the final lifecycle. The final editor closes its process rather than hiding a retained native window.

## Pending manual checks and limitations

- Final compact-popup bundle: physical Cmd+B → region selection → focused context → Ctrl+C → Cmd+B → retrieval.
- Permission denial and recovery for installed copies and after rebuilds.
- Actual primary/secondary displays, Retina pixel dimensions, mixed DPI, fullscreen applications, and confirmation the editor is absent from captured pixels.
- Physical configurable-shortcut conflicts and recovery, repeated captures, GUI recapture, and parent-supervised close/reopen cycles.
- Physical Cmd+B, Ctrl+C, Escape, preview dragging, standard editing shortcuts, narrow resizing, and relevant agent/image-application manual paste tests. Programmatic pasteboard readback does not certify every receiving application.
- Full tray Quit/relaunch behavior throughout real captures, and longer-duration idle CPU/energy measurements. Quit during the native selector waits for its cancellation/completion; Escape cancels it.
- Disk-full testing is injected, not performed by filling the user's disk. Power-loss durability and hard-kill cleanup are not certified; uncommitted crash remnants are ignored on retrieval.
- Local bundles are ad-hoc signed, not notarized. Windows/Linux capture, login startup, annotation, OCR, cloud features, and external agent integrations are not implemented.

The revised compact workflow must not be described as fully accepted until its remaining physical checks pass. Earlier large-editor checks above are historical and do not certify the revised popup.

## Latest compact-popup build

Both renderer bundles rebuilt and passed deep strict code-signature verification. The default suite remains 23 passed, three explicit OS tests ignored; clippy with warnings denied and formatting checks passed. A generated-fixture popup saved and exited on Escape; CLI retrieval returned its single real path. Final resident sampling found 14.2M physical footprint, 0.0% CPU, and no remaining helper. Final physical shortcut and preview-drag acceptance is awaiting user feedback.

## Paste, full-message copying, and right-click cancellation

Reproduced the reported input failure in the packaged popup: an externally supplied native paste left the focused message unchanged. Inspection of the installed eframe/egui-winit sources identified the missing `clipboard` feature: without it the UI uses a local fallback clipboard. Enabled that feature explicitly and updated Cargo.lock. Hex source inspection confirms its macOS dictation insertion uses system-pasteboard text followed by Cmd+V; no Hex changes were made.

After rebuilding, real native paste inserted `Dictation paste regression — 日本語` and a second context line after the screenshot path. Ctrl+C followed by native clipboard readback returned the exact path, newline, and both context lines. Copy now snapshots after TextEdit processes pending input. The live Hex microphone-to-transcript path still requires user speech verification; native paste compatibility is verified.

Right-click in the rebuilt popup's message area exited the helper successfully and removed its entire uncommitted draft directory. No metadata or saved snippet remained, and only the resident process remained. Cmd+B and Escape retain save-and-close behavior. Right-click cancellation applies before save finalization begins; it does not undo an already requested save. Both renderer bundles rebuilt; 23 automated tests, clippy with warnings denied, formatting, and deep strict signature verification passed.

## One-second copy feedback

Success now sets a one-second deadline for a bottom-right green checkmark and green `Copied` label. Failure shows the error without success feedback. A single delayed repaint clears it; there is no animation loop. The checkmark is drawn with two strokes rather than relying on font glyph availability. Native GUI verification observed the green feedback and subsequent disappearance. Native clipboard readback was exactly the test image path plus newline plus `Testing, testing.`; the text clipboard from before this test was restored afterward. Right-click discarded the test popup and its image. Copy formatting uses the actual staged PNG path and avoids duplicating the already-prefilled path. The final stroked checkmark replaces the missing checkmark font glyph observed during that check.

## Both copy shortcuts

The popup now consumes both native `Event::Copy` (macOS Cmd+C) and Ctrl+C before TextEdit handles selection copying. Both dispatch the same full path-plus-message operation and one-second success feedback. Settings retain standard text-editing shortcuts.

## Save and cancel buttons

Added a compact green stroked checkmark and × beneath the input, aligned right, with accessible button names and hover descriptions. The checkmark calls save-and-close without copying; cancel shares right-click discard-and-close logic. Both disable during loading/finalization. Native packaged GUI verification clicked Save and close, confirmed helper exit and exact CLI retrieval of `Save button verification`, then clicked Cancel without saving on a new draft and confirmed helper exit and complete draft directory removal. Clippy, formatting, editor geometry test, and both renderer release builds passed.

## Installed build and close/reopen regression

The user was running an older `/Applications/SnapNote.app`, while previous button checks exercised `dist/SnapNote.app`. Updated the installed copy and verified its helper displays the accessible Save and close / Cancel without saving controls. Right-click exited it successfully and removed the draft. Only the installed resident remained afterward.

Fixed lifecycle hazards: a terminal closing helper ignores queued events and never shows itself again; a BrokenPipe from IPC racing helper exit is normal cleanup rather than a reason to launch error Settings; a press/release latch prevents held shortcut repeats from starting another capture after dismissal. Regression tests reproduce the broken pipe and repeated press conditions. The suite now passes 25 tests, with three OS tests explicitly ignored; clippy with warnings denied and formatting passed. Long-duration physical capture/close repetition remains a manual check.

## Growing input, red close button, annotations, and preservation

The red × and right-click now save and close after capture, as does Escape; the previous discard behavior is superseded. Cancellation in native selection still leaves no snippet. The input starts at its actual text height, grows to 144 logical pixels, then scrolls. Native GUI inspection verified the short initial field and capped scrolling with twenty lines. A fixed-height action row avoids resize feedback.

The screenshot hover toolbar provides Move, Pen, Arrow, Box, Undo, and Redo. Cmd+Z and Cmd+Shift+Z operate on annotations when the message is not focused; typing retains normal text undo. Native inspection verified the toolbar, arrow preview, and toolbar undo. A fast-drag issue found during rectangle testing was corrected by processing ordered pointer events rather than relying on the final pointer state; a regression covers same-frame press/move/release for all three drawing tools. PNG export/undo tests verify changed pixels, unchanged dimensions, restoration of original pixels, context persistence, and cleanup of the original backup after commit. PNG decoding/encoding occurs on workers only for annotation export; unedited PNGs are copied unchanged.

Final native retesting became blocked by `Sky Computer Use native pipe startup failed`, including after resetting the tool connection. The test popup was closed through its private control pipe. Final rectangle/pen gestures, Cmd+Z/Cmd+Shift+Z, and post-capture right-click preservation require physical acceptance. No complete native annotation acceptance is claimed.

Latest verification: 27 automated tests passed, three explicit native OS tests ignored; clippy with warnings denied and formatting passed. Installed the new glow bundle at `/Applications/SnapNote.app` and relaunched its resident. Final physical annotation acceptance is pending user feedback.

User-assisted acceptance: the user subsequently confirmed the updated workflow worked, including drawing and annotation undo/redo. The earlier automation connection failure remains a tooling limitation rather than an unresolved user-reported failure.
