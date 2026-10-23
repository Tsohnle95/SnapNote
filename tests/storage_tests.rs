mod common;
use snapnote::storage;
#[test]
fn complete_snippets_preserve_png_and_authored_context() {
    let (_temp, root, png) = common::setup();
    for text in [
        "",
        "  Unicode 日本語 🦀\nline two\n",
        "# Capture\n\n![Screenshot](./capture.png)\n",
    ] {
        let snippet = storage::save(&root, &png, text).unwrap();
        let read = storage::load(&root, &snippet.metadata.id).unwrap();
        assert_eq!(read.text, text);
        assert_eq!(read.metadata.image_width, 7);
        assert_eq!(read.metadata.image_height, 5);
        assert_eq!(
            std::fs::read(&png).unwrap(),
            std::fs::read(read.image_path()).unwrap()
        );
        assert_eq!(
            std::fs::read_to_string(read.directory.join("context.md")).unwrap(),
            storage::markdown(text)
        );
        assert_eq!(read.metadata.schema_version, 1);
        assert!(read.image_path().is_absolute());
    }
    assert_eq!(storage::list(&root).unwrap().len(), 3);
}
#[test]
fn rapid_saves_never_collide_or_overwrite() {
    let (_temp, root, png) = common::setup();
    let ids: std::collections::HashSet<_> = (0..40)
        .map(|_| storage::save(&root, &png, "context").unwrap().metadata.id)
        .collect();
    assert_eq!(ids.len(), 40);
    assert_eq!(storage::list(&root).unwrap().len(), 40);
}
#[test]
fn invalid_png_and_unwritable_destination_leave_no_snippet() {
    let (temp, root, png) = common::setup();
    let bad = temp.path().join("bad.png");
    std::fs::write(&bad, b"invalid PNG").unwrap();
    assert!(storage::save(&root, &bad, "test").is_err());
    assert!(storage::list(&root).unwrap().is_empty());
    assert!(storage::save(&png, &png, "test").is_err());
}
#[cfg(unix)]
#[test]
fn denied_destination_is_reported() {
    use std::os::unix::fs::PermissionsExt;
    let (_temp, root, png) = common::setup();
    std::fs::create_dir(&root).unwrap();
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o500)).unwrap();
    let result = storage::save(&root, &png, "test");
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert!(result.is_err());
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);
}
#[test]
fn incomplete_directories_are_not_exposed_as_saved() {
    let (_temp, root, png) = common::setup();
    std::fs::create_dir_all(root.join("2026-10-22-partial")).unwrap();
    std::fs::copy(png, root.join("2026-10-22-partial/capture.png")).unwrap();
    assert!(storage::list(&root).unwrap().is_empty());
    assert!(storage::load(&root, "2026-10-22-partial").is_err());
}
#[test]
fn traversal_and_missing_files_are_rejected() {
    let (_temp, root, png) = common::setup();
    let saved = storage::save(&root, &png, "").unwrap();
    assert!(storage::load(&root, "../outside").is_err());
    std::fs::remove_file(saved.image_path()).unwrap();
    assert!(storage::load(&root, &saved.metadata.id).is_err());
}
#[cfg(unix)]
#[test]
fn symlinks_cannot_escape_library() {
    use std::os::unix::fs::symlink;
    let (temp, root, png) = common::setup();
    let saved = storage::save(&root, &png, "").unwrap();
    symlink(temp.path(), root.join("outside")).unwrap();
    assert!(storage::load(&root, "outside").is_err());
    std::fs::remove_file(saved.image_path()).unwrap();
    symlink(&png, saved.image_path()).unwrap();
    assert!(storage::load(&root, &saved.metadata.id).is_err());
}
#[test]
fn ordering_compares_instants_across_timezone_changes() {
    let (_temp, root, png) = common::setup();
    let earlier = storage::save(&root, &png, "earlier").unwrap();
    let later = storage::save(&root, &png, "later").unwrap();
    for (mut snippet, date) in [
        (earlier, "2026-11-01T01:50:00-06:00"),
        (later, "2026-11-01T01:10:00-07:00"),
    ] {
        snippet.metadata.created_at = date.into();
        std::fs::write(
            snippet.directory.join("metadata.json"),
            serde_json::to_vec(&snippet.metadata).unwrap(),
        )
        .unwrap();
    }
    let snippets = storage::list(&root).unwrap();
    assert_eq!(snippets[0].text, "later");
}
#[test]
fn popup_draft_has_real_image_path_before_copy_and_commits_exact_message() {
    let (_temp, root, png) = common::setup();
    let mut draft = storage::begin(&root, &png).unwrap();
    let path = draft.image_path();
    assert!(path.is_file());
    assert!(path.is_absolute());
    assert!(storage::list(&root).unwrap().is_empty());
    let message = format!("{}\n日本語 — optional message\n", path.display());
    let saved = draft.commit(&message).unwrap();
    drop(draft);
    let loaded = storage::load(&root, &saved.metadata.id).unwrap();
    assert_eq!(loaded.image_path(), path);
    assert_eq!(loaded.text, message);
    assert_eq!(
        snapnote::clipboard::format(snapnote::config::Mode::Agent, &path, &message).unwrap(),
        message
    );
    assert_eq!(
        snapnote::clipboard::format(snapnote::config::Mode::Context, &path, &message).unwrap(),
        "日本語 — optional message\n"
    );
}
#[test]
fn uncommitted_draft_releases_its_owned_files() {
    let (_temp, root, png) = common::setup();
    let draft = storage::begin(&root, &png).unwrap();
    let image = draft.image_path();
    assert!(image.exists());
    drop(draft);
    assert!(!image.exists());
    assert!(png.exists());
}

#[test]
fn annotation_export_undo_and_commit_preserve_image_dimensions_and_context() {
    use snapnote::annotations::{Mark, Tool};
    let (_temp, root, png) = common::setup();
    let mut draft = storage::begin(&root, &png).unwrap();
    let original = image::open(draft.image_path()).unwrap().to_rgba8();
    let mark = Mark {
        tool: Tool::Arrow,
        points: vec![[0., 0.], [1., 1.]],
    };
    draft.render_annotations(&[mark]).unwrap();
    let annotated = image::open(draft.image_path()).unwrap().to_rgba8();
    assert_eq!(annotated.dimensions(), original.dimensions());
    assert_ne!(annotated, original);
    draft.render_annotations(&[]).unwrap();
    assert_eq!(
        image::open(draft.image_path()).unwrap().to_rgba8(),
        original
    );
    for tool in [Tool::Pen, Tool::Rectangle, Tool::Arrow] {
        draft
            .render_annotations(&[Mark {
                tool,
                points: vec![[0., 0.], [1., 1.]],
            }])
            .unwrap();
        assert_ne!(
            image::open(draft.image_path()).unwrap().to_rgba8(),
            original
        );
    }
    let saved = draft.commit("Annotated context").unwrap();
    drop(draft);
    assert_eq!(
        storage::load(&root, &saved.metadata.id).unwrap().text,
        "Annotated context"
    );
    assert_eq!(std::fs::read_dir(saved.directory).unwrap().count(), 3);
}
