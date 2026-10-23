#![allow(dead_code)]
use std::path::{Path, PathBuf};
pub fn png(root: &Path) -> PathBuf {
    let path = root.join("input image.png");
    image::RgbaImage::from_pixel(7, 5, image::Rgba([64, 128, 255, 255]))
        .save(&path)
        .unwrap();
    path
}
pub fn setup() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("Capture Library With Spaces");
    let png = png(temp.path());
    (temp, root, png)
}
