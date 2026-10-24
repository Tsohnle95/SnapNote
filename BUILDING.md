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

Run one command:

```sh
cd /Users/tdawg/code-repositories/SnapNote
./scripts/build-and-install.sh
```

It builds both executables in release mode with the locked dependencies, signs `dist/SnapNote.app` with the stable local **SnapNote Local Signing** identity, quits the running resident, replaces `/Applications/SnapNote.app`, verifies the signature, and relaunches the installed copy. The first build creates the signing identity in your login keychain; later builds reuse it, so Screen Recording permission stays granted. On the first build Cargo needs internet access to download crates.

If a capture window is open, the script exits instead of discarding a possible unsaved draft. Close the window or quit SnapNote and rerun.

Manual equivalent:

```sh
./scripts/build-app.sh
./scripts/install-app.sh
```

`install-app.sh` installs `dist/SnapNote.app` and relaunches it. To use a user-only installation, set `SNAPNOTE_INSTALL_DIR="$HOME/Applications"` and consistently launch that copy.

## Launch without rebuilding

```sh
open -n /Applications/SnapNote.app
```

The app starts in the menu bar with no popup. Press **Cmd+B**, select a region, then type or dictate context. The input starts with the image path and `View this screenshot.` on separate lines, with the cursor on the following line. **Cmd+C** or **Ctrl+C** copies that complete text and your message. The green checkmark or **Cmd+B** saves and closes. Right-click also saves and closes once a screenshot exists. The red × deletes the unsaved capture and message, then closes. **Escape** saves and closes the popup; Escape during native region selection cancels selection instead.

Default captures: `~/screenshots/snapshot/`. Shared settings: `~/Library/Application Support/SnapNote/config.toml`. Updating the app does not replace either directory.

## Permissions after a rebuild

Local builds are signed with a stable self-signed **SnapNote Local Signing** certificate, so Screen Recording permission survives rebuilds. Ad-hoc signatures used to change identity on every build; that is why macOS kept asking. Keep the certificate in your login keychain and continue signing with it.

One-time migration from an older ad-hoc build:

```sh
tccutil reset ScreenCapture local.snapnote.app
./scripts/build-and-install.sh
```

Enable the app once in **System Settings → Privacy & Security → Screen & System Audio Recording**, then quit and relaunch if macOS asks. Later rebuilds and replacements do not need another grant. Grant access to the main app, not the editor helper. The app does not bypass permissions.

`dist/SnapNote.app` and `/Applications/SnapNote.app` share a bundle identifier, so macOS may resolve `open` to whichever copy it registered last. Launch the exact installed copy with `open -n /Applications/SnapNote.app`, or run `/Applications/SnapNote.app/Contents/MacOS/SnapNote` directly. Both copies use the same stable signing identity, so either keeps the permission.

If you reinstall the certificate or switch signing identities, reset the stale TCC entry once with `tccutil reset ScreenCapture local.snapnote.app` and grant again. Use `SNAPNOTE_ADHOC_SIGNING=1` only for a throwaway build: ad-hoc signing deliberately restores the old re-prompt behaviour.

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
