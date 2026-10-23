mod common;
use snapnote::config::{Config, Mode};
#[test]
#[ignore = "mutates the real desktop pasteboard; run explicitly"]
fn all_cli_clipboard_modes_round_trip() {
    let (_temp, root, png) = common::setup();
    let config_path = root.parent().unwrap().join("config.toml");
    let mut config = Config::default();
    config.storage.directory = root.clone();
    config.save(&config_path).unwrap();
    let snippet = snapnote::storage::save(&root, &png, "  日本語\nmultiline context\n").unwrap();
    let mut clipboard = arboard::Clipboard::new().unwrap();
    let prior_image = clipboard.get_image().ok();
    let prior_text = clipboard.get_text().ok();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        for (mode, name) in [
            (Mode::Context, "context"),
            (Mode::Path, "path"),
            (Mode::Agent, "agent"),
            (Mode::Image, "image"),
            (Mode::SaveOnly, "save-only"),
        ] {
            clipboard.set_text("untouched marker").unwrap();
            let output = std::process::Command::new(env!("CARGO_BIN_EXE_snapnote"))
                .arg("--config")
                .arg(&config_path)
                .args(["copy", &snippet.metadata.id, "--mode", name])
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(output.stdout.is_empty());
            match mode {
                Mode::Image => {
                    let read = clipboard.get_image().unwrap();
                    assert_eq!((read.width, read.height), (7, 5));
                    assert_eq!(
                        read.bytes.as_ref(),
                        image::open(&png).unwrap().to_rgba8().as_raw()
                    );
                }
                Mode::SaveOnly => assert_eq!(clipboard.get_text().unwrap(), "untouched marker"),
                _ => assert_eq!(
                    clipboard.get_text().unwrap(),
                    snapnote::clipboard::format(mode, &snippet.image_path(), &snippet.text)
                        .unwrap()
                ),
            }
            assert!(snippet.image_path().is_file());
        }
        clipboard.set_text("empty context test").unwrap();
        let empty = snapnote::storage::save(&root, &png, "").unwrap();
        snapnote::clipboard::copy(&empty, Mode::Context).unwrap();
        assert_eq!(clipboard.get_text().unwrap(), "");
    }));
    if let Some(image) = prior_image {
        clipboard.set_image(image).unwrap();
    } else if let Some(text) = prior_text {
        clipboard.set_text(text).unwrap();
    }
    if let Err(error) = result {
        std::panic::resume_unwind(error);
    }
}
