mod common;
use std::process::{Command, Output};
fn cli(config: &std::path::Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_snapnote"))
        .arg("--config")
        .arg(config)
        .args(args)
        .output()
        .unwrap()
}
#[test]
fn retrieval_order_plain_output_and_errors() {
    let (temp, root, png) = common::setup();
    let config_path = temp.path().join("config.toml");
    let mut config = snapnote::config::Config::default();
    config.storage.directory = root.clone();
    config.save(&config_path).unwrap();
    let empty = cli(&config_path, &["latest"]);
    assert!(!empty.status.success());
    assert!(empty.stdout.is_empty());
    assert!(!empty.stderr.is_empty());
    let first = snapnote::storage::save(&root, &png, "First context").unwrap();
    let second = snapnote::storage::save(&root, &png, "Latest 日本語\nsecond line").unwrap();
    let latest = cli(&config_path, &["latest"]);
    assert!(latest.status.success());
    assert!(latest.stderr.is_empty());
    let output = String::from_utf8(latest.stdout).unwrap();
    assert!(output.contains("Latest 日本語\nsecond line"));
    assert!(output.contains(second.image_path().to_str().unwrap()));
    let list = cli(&config_path, &["list"]);
    assert!(list.status.success());
    let output = String::from_utf8(list.stdout).unwrap();
    assert!(output.starts_with(&second.metadata.id));
    assert!(output.contains(&first.metadata.id));
    let path = cli(&config_path, &["path", &second.metadata.id]);
    assert_eq!(
        String::from_utf8(path.stdout).unwrap(),
        format!("{}\n", second.image_path().display())
    );
    assert!(
        cli(&config_path, &["show", &first.metadata.id])
            .status
            .success()
    );
    for args in [
        &["show", "missing"][..],
        &["path", "../outside"][..],
        &["unknown"][..],
        &["copy", "missing", "--mode", "bad"][..],
    ] {
        let output = cli(&config_path, args);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
    std::fs::remove_file(second.image_path()).unwrap();
    let missing = cli(&config_path, &["path", &second.metadata.id]);
    assert!(!missing.status.success());
    assert!(missing.stdout.is_empty());
}
#[test]
fn cli_rejects_malformed_config_without_launching_gui() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("config.toml");
    std::fs::write(&path, "[invalid").unwrap();
    let output = cli(&path, &["list"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Malformed"));
}
