# Build, install, and launch SnapNote

These instructions target macOS Apple Silicon. Run commands in Terminal.

## Requirements

Install the stable Rust toolchain from https://rustup.rs if Rust is missing. Install Apple's command line tools if needed:

```sh
xcode-select --install
```

Confirm the toolchain is available:

```sh
export PATH="$HOME/.cargo/bin:$PATH"
rustc --version
cargo --version
```

## Rebuild and update the installed app

First choose **Quit** from SnapNote's menu-bar menu. Save and close any open capture first. Closing the popup alone leaves the resident running.

```sh
cd /Users/tdawg/code-repositories/SnapNote
./scripts/build-app.sh
```

The script builds both executables in release mode with the locked dependencies, bundles the editor, and ad-hoc signs `dist/SnapNote.app`. On the first build Cargo needs internet access to download crates.

**Update the copy in Applications before launching it:**

```sh
ditto dist/SnapNote.app /Applications/SnapNote.app
codesign --verify --deep --strict /Applications/SnapNote.app
open /Applications/SnapNote.app
```

If Terminal reports permission denied when copying, use Finder to replace `/Applications/SnapNote.app` with `dist/SnapNote.app`. For a user-only installation, copy to `~/Applications/SnapNote.app` instead and consistently launch that copy.

Rebuilding `dist/SnapNote.app` does not update `/Applications/SnapNote.app`. An already-running resident also continues using its original installed location. Avoid running multiple copies; the configuration lock prevents a second resident from starting.

## Launch without rebuilding

```sh
open /Applications/SnapNote.app
```

The app starts in the menu bar with no popup. Press **Cmd+B**, select a region, then type or dictate context. **Cmd+C** or **Ctrl+C** copies the image path followed by the message. The green checkmark or **Cmd+B** saves and closes. The red × button or right-click also saves and closes once a screenshot exists. **Escape** saves and closes the popup; Escape during native region selection cancels selection instead.

Default captures: `~/screenshots/snapshot/`. Shared settings: `~/Library/Application Support/SnapNote/config.toml`. Updating the app does not replace either directory.

## Permissions after a rebuild

An ad-hoc rebuild can invalidate macOS's saved Screen Recording authorization. If selection fails, open **System Settings → Privacy & Security → Screen & System Audio Recording**, remove the old SnapNote entry if necessary, add the exact installed app, and enable it. Then quit and relaunch SnapNote. Grant access to the main app, not the editor helper. The app does not bypass permissions.

## Checks and CLI

```sh
export PATH="$HOME/.cargo/bin:$PATH"
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo fmt --check

/Applications/SnapNote.app/Contents/MacOS/SnapNote latest
/Applications/SnapNote.app/Contents/MacOS/SnapNote list
```

The default tests do not capture the screen or modify your clipboard. Explicit native tests are documented in [README.md](README.md).

For source development without packaging:

```sh
cargo build --bins
cargo run
```

Quit the installed resident first. The packaged app remains the preferred real-world test because permissions and bundle identities differ from `cargo run`.

## Alternative renderer

```sh
./scripts/build-app.sh wgpu
```

This creates `dist/comparison/wgpu/SnapNote.app`, leaving the default glow bundle separate. Quit the current resident before launching a comparison. See [performance measurements](docs/performance.md) for renderer tradeoffs.
