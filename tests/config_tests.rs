use snapnote::config::{Config, Mode};
#[test]
fn defaults_partial_config_and_atomic_preferences() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("configuration with spaces/config.toml");
    let mut config = Config::load(&path).unwrap();
    assert_eq!(config.clipboard.default_mode, Mode::Agent);
    config.clipboard.default_mode = Mode::Image;
    config.save(&path).unwrap();
    assert_eq!(
        Config::load(&path).unwrap().clipboard.default_mode,
        Mode::Image
    );
    std::fs::write(&path, "[clipboard]\ndefault_mode = 'context'\n").unwrap();
    let config = Config::load(&path).unwrap();
    assert_eq!(config.capture.hotkey, "Command+B");
    assert_eq!(config.clipboard.default_mode, Mode::Context);
}
#[test]
fn invalid_config_does_not_overwrite_original() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("config.toml");
    for text in [
        "[broken",
        "schema_version = 99",
        "[capture]\nhotkey = 'nonsense'",
        "[storage]\ndirectory = 'relative/path'",
    ] {
        std::fs::write(&path, text).unwrap();
        assert!(Config::load(&path).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
    }
}
