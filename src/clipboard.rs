use crate::{config::Mode, storage::Snippet};
use anyhow::{Result, bail};
use std::path::Path;

pub fn format(mode: Mode, image: &Path, context: &str) -> Result<String> {
    match mode {
        Mode::Context => Ok(context
            .strip_prefix(&format!("{}\n", image.display()))
            .unwrap_or(context)
            .to_owned()),
        Mode::Path => Ok(image.to_string_lossy().into_owned()),
        Mode::Agent => {
            let prefix = format!("{}\n", image.display());
            if context.starts_with(&prefix) || context == image.to_string_lossy() {
                Ok(context.to_owned())
            } else {
                Ok(format!("{prefix}{context}"))
            }
        }
        Mode::Image | Mode::SaveOnly => bail!("This mode does not produce text"),
    }
}
pub fn copy_text(text: String) -> Result<()> {
    arboard::Clipboard::new()?.set_text(text)?;
    Ok(())
}
pub fn copy_image(path: &Path) -> Result<()> {
    let image = image::open(path)?.to_rgba8();
    arboard::Clipboard::new()?.set_image(arboard::ImageData {
        width: image.width() as usize,
        height: image.height() as usize,
        bytes: std::borrow::Cow::Borrowed(image.as_raw()),
    })?;
    Ok(())
}
pub fn copy(snippet: &Snippet, mode: Mode) -> Result<&'static str> {
    match mode {
        Mode::SaveOnly => return Ok("Saved; clipboard unchanged"),
        Mode::Image => copy_image(&snippet.image_path())?,
        _ => copy_text(format(mode, &snippet.image_path(), &snippet.text)?)?,
    }
    Ok(match mode {
        Mode::Context => "Context copied",
        Mode::Path => "Screenshot path copied",
        Mode::Agent => "Screenshot reference and context copied",
        Mode::Image => "Image data copied",
        Mode::SaveOnly => unreachable!(),
    })
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "uses the real macOS pasteboard; run explicitly in the desktop session"]
    fn native_text_and_image_round_trip() {
        let mut clipboard = arboard::Clipboard::new().unwrap();
        let prior_image = clipboard.get_image().ok().map(|i| arboard::ImageData {
            width: i.width,
            height: i.height,
            bytes: std::borrow::Cow::Owned(i.bytes.into_owned()),
        });
        let prior_text = clipboard.get_text().ok();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let text = "SnapNote clipboard verification\nUnicode: 日本語";
            clipboard.set_text(text).unwrap();
            assert_eq!(clipboard.get_text().unwrap(), text);
            let pixels = vec![255, 0, 0, 255, 0, 255, 0, 255];
            clipboard
                .set_image(arboard::ImageData {
                    width: 2,
                    height: 1,
                    bytes: std::borrow::Cow::Owned(pixels.clone()),
                })
                .unwrap();
            let image = clipboard.get_image().unwrap();
            assert_eq!((image.width, image.height), (2, 1));
            assert_eq!(image.bytes.as_ref(), pixels.as_slice());
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
}
