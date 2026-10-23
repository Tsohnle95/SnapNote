fn main() -> anyhow::Result<()> {
    let file = tempfile::Builder::new()
        .prefix("snapnote-capture-")
        .suffix(".png")
        .tempfile()?;
    let image = image::RgbaImage::from_fn(1600, 1000, |x, y| {
        let color = if x < 260 {
            [35, 40, 48, 255]
        } else if y < 120 {
            [50, 60, 72, 255]
        } else if y % 85 < 42 && x % 560 > 30 {
            [110 + (x % 80) as u8, 145, 160, 255]
        } else {
            [27, 30, 36, 255]
        };
        image::Rgba(color)
    });
    image.save_with_format(file.path(), image::ImageFormat::Png)?;
    let (_, path) = file.keep()?;
    println!("{}", path.display());
    Ok(())
}
