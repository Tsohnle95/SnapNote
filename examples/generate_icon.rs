// Code-native camera mark shared in spirit with the menu-bar glyph.
// Generates an iconset for Apple's iconutil; no runtime dependency on these files.
fn rounded(x: f32, y: f32, cx: f32, cy: f32, w: f32, h: f32, r: f32) -> bool {
    let dx = (x - cx).abs() - w / 2. + r;
    let dy = (y - cy).abs() - h / 2. + r;
    dx.max(0.).hypot(dy.max(0.)) + dx.max(dy).min(0.) <= r
}
fn main() -> anyhow::Result<()> {
    let directory = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("usage: generate_icon <iconset-directory>"))?;
    std::fs::create_dir_all(&directory)?;
    for size in [16, 32, 128, 256, 512] {
        for scale in [1, 2] {
            let n = size * scale;
            let factor = 4;
            let high = n * factor;
            let image = image::RgbaImage::from_fn(high, high, |x, y| {
                let x = x as f32 / high as f32;
                let y = y as f32 / high as f32;
                let mut color = [0, 0, 0, 0];
                if rounded(x, y, 0.5, 0.5, 0.90, 0.90, 0.20) {
                    color = [30, 33, 39, 255];
                }
                let camera = rounded(x, y, 0.50, 0.53, 0.61, 0.42, 0.055)
                    || rounded(x, y, 0.40, 0.31, 0.21, 0.11, 0.025);
                let inside = rounded(x, y, 0.50, 0.53, 0.55, 0.36, 0.035);
                let lens = (x - 0.52).hypot(y - 0.53);
                if (camera && !inside) || (0.09..0.12).contains(&lens) {
                    color = [231, 236, 240, 255];
                }
                if (x - 0.73).hypot(y - 0.40) < 0.021 {
                    color = [154, 206, 190, 255];
                }
                image::Rgba(color)
            });
            let image =
                image::imageops::resize(&image, n, n, image::imageops::FilterType::Lanczos3);
            let suffix = if scale == 2 { "@2x" } else { "" };
            image.save(format!("{directory}/icon_{size}x{size}{suffix}.png"))?;
        }
    }
    Ok(())
}
