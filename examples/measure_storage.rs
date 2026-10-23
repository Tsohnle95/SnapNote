fn main() -> anyhow::Result<()> {
    let temp = tempfile::tempdir()?;
    let png = temp.path().join("input.png");
    let mut state = 0x12345678u32;
    let image = image::RgbaImage::from_fn(1200, 800, |_, _| {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        image::Rgba([state as u8, (state >> 8) as u8, (state >> 16) as u8, 255])
    });
    image.save(&png)?;
    let mut samples = vec![];
    for _ in 0..10 {
        let started = std::time::Instant::now();
        let snippet = snapnote::storage::save(
            &temp.path().join("library"),
            &png,
            "Performance measurement context",
        )?;
        samples.push(started.elapsed().as_secs_f64() * 1000.);
        assert!(snippet.image_path().is_file());
    }
    samples.sort_by(f64::total_cmp);
    println!(
        "{}",
        serde_json::json!({"image_width":1200,"image_height":800,"png_bytes":std::fs::metadata(&png)?.len(),"save_samples_ms":samples,"median_save_ms":(samples[4]+samples[5])/2.,"max_save_ms":samples[9]})
    );
    Ok(())
}
