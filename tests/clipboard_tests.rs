use snapnote::{clipboard, config::Mode};
use std::path::Path;
#[test]
fn text_modes_keep_whitespace_and_unicode() {
    let path = Path::new("/Users/example/Pictures/My Captures/capture.png");
    for text in ["", "日本語 & <tag>\n  text\n", "# Heading\n\nline"] {
        assert_eq!(clipboard::format(Mode::Context, path, text).unwrap(), text);
        assert_eq!(
            clipboard::format(Mode::Path, path, text).unwrap(),
            path.to_str().unwrap()
        );
        assert_eq!(
            clipboard::format(Mode::Agent, path, text).unwrap(),
            format!("{}\n{}", path.display(), text)
        );
    }
    assert!(clipboard::format(Mode::Image, path, "text").is_err());
    assert!(clipboard::format(Mode::SaveOnly, path, "text").is_err());
}
#[test]
fn clipboard_failure_does_not_touch_storage() {
    let temp = tempfile::tempdir().unwrap();
    let png = temp.path().join("test.png");
    image::RgbaImage::new(2, 2).save(&png).unwrap();
    let snippet = snapnote::storage::save(&temp.path().join("library"), &png, "keep me").unwrap();
    // A missing image forces a deterministic failure before accessing the OS pasteboard.
    std::fs::rename(snippet.image_path(), snippet.directory.join("backup.png")).unwrap();
    assert!(clipboard::copy(&snippet, Mode::Image).is_err());
    assert!(snippet.directory.join("context.md").is_file());
    assert!(snippet.directory.join("metadata.json").is_file());
    assert!(snippet.directory.join("backup.png").is_file());
    assert_eq!(snippet.text, "keep me");
}
