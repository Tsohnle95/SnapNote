use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub schema_version: u32,
    pub id: String,
    pub created_at: String,
    pub image: String,
    pub context: String,
    pub image_width: u32,
    pub image_height: u32,
    pub platform: String,
}
#[derive(Debug)]
pub struct Snippet {
    pub metadata: Metadata,
    pub directory: PathBuf,
    pub text: String,
}
impl Snippet {
    pub fn image_path(&self) -> PathBuf {
        self.directory.join("capture.png")
    }
}
pub fn markdown(text: &str) -> String {
    format!("# Capture\n\n{text}\n\n![Screenshot](./capture.png)\n")
}
fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}
/// A screenshot with a stable, real path, not yet visible to CLI retrieval.
/// The PNG exists before the popup is shown so copied references are immediately readable.
pub struct Draft {
    guard: Option<tempfile::TempDir>,
    original: Option<tempfile::NamedTempFile>,
    directory: PathBuf,
    metadata: Metadata,
}
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file =
        tempfile::NamedTempFile::new_in(path.parent().context("Missing parent directory")?)?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    file.persist(path)?;
    Ok(())
}
impl Draft {
    pub fn render_annotations(&mut self, marks: &[crate::annotations::Mark]) -> Result<()> {
        if marks.is_empty() && self.original.is_none() {
            return Ok(());
        }
        if self.original.is_none() {
            let mut original = tempfile::NamedTempFile::new_in(&self.directory)?;
            std::io::copy(
                &mut std::fs::File::open(self.image_path())?,
                original.as_file_mut(),
            )?;
            self.original = Some(original);
        }
        let mut pixels = image::ImageReader::open(self.original.as_ref().unwrap().path())?
            .with_guessed_format()?
            .decode()?
            .to_rgba8();
        crate::annotations::render(&mut pixels, marks);
        let mut output = tempfile::NamedTempFile::new_in(&self.directory)?;
        image::DynamicImage::ImageRgba8(pixels)
            .write_to(output.as_file_mut(), image::ImageFormat::Png)?;
        output.as_file().sync_all()?;
        output.persist(self.image_path())?;
        Ok(())
    }

    pub fn image_path(&self) -> PathBuf {
        self.directory.join("capture.png")
    }
    pub fn commit(&mut self, message: &str) -> Result<Snippet> {
        atomic_write(
            &self.directory.join("context.md"),
            markdown(message).as_bytes(),
        )?;
        self.finalize(message)
    }
    fn finalize(&mut self, message: &str) -> Result<Snippet> {
        atomic_write(
            &self.directory.join("metadata.json"),
            &serde_json::to_vec_pretty(&self.metadata)?,
        )?;
        std::fs::File::open(&self.directory)?.sync_all()?;
        if let Some(guard) = self.guard.take() {
            let _ = guard.keep();
        }
        Ok(Snippet {
            metadata: self.metadata.clone(),
            directory: self.directory.clone(),
            text: message.into(),
        })
    }
}
pub fn begin(root: &Path, png: &Path) -> Result<Draft> {
    std::fs::create_dir_all(root)
        .with_context(|| format!("Create capture directory {}", root.display()))?;
    let root = std::fs::canonicalize(root)?;
    let dimensions = image::ImageReader::open(png)?
        .with_guessed_format()?
        .into_dimensions()
        .context("Validate screenshot dimensions")?;
    let now = chrono::Local::now();
    let guard = tempfile::Builder::new()
        .prefix(&format!("{}-", now.format("%Y-%m-%d-%H%M%S")))
        .tempdir_in(root)?;
    let directory = guard.path().to_owned();
    let id = directory
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let metadata = Metadata {
        schema_version: 1,
        id,
        created_at: now.to_rfc3339(),
        image: "capture.png".into(),
        context: "context.md".into(),
        image_width: dimensions.0,
        image_height: dimensions.1,
        platform: std::env::consts::OS.into(),
    };
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join("capture.png"))?;
    std::io::copy(&mut std::fs::File::open(png)?, &mut file)?;
    file.sync_all()?;
    Ok(Draft {
        guard: Some(guard),
        original: None,
        directory,
        metadata,
    })
}
pub fn save(root: &Path, png: &Path, text: &str) -> Result<Snippet> {
    save_with_context_writer(root, png, text, write)
}
fn save_with_context_writer(
    root: &Path,
    png: &Path,
    text: &str,
    write_context: impl FnOnce(&Path, &[u8]) -> Result<()>,
) -> Result<Snippet> {
    let mut draft = begin(root, png)?;
    write_context(
        &draft.directory.join("context.md"),
        markdown(text).as_bytes(),
    )?;
    draft.finalize(text)
}
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() < 128 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}
pub fn load(root: &Path, id: &str) -> Result<Snippet> {
    if !valid_id(id) {
        bail!("Invalid snippet ID: {id}");
    }
    let directory =
        std::fs::canonicalize(root.join(id)).with_context(|| format!("Find snippet {id}"))?;
    if directory.parent() != Some(std::fs::canonicalize(root)?.as_path()) {
        bail!("Snippet directory escapes storage root");
    }
    let metadata: Metadata = serde_json::from_reader(
        std::fs::File::open(directory.join("metadata.json")).context("Missing snippet metadata")?,
    )?;
    if metadata.schema_version != 1
        || metadata.id != id
        || metadata.image != "capture.png"
        || metadata.context != "context.md"
    {
        bail!("Unsupported or invalid snippet metadata for {id}");
    }
    for name in ["capture.png", "context.md", "metadata.json"] {
        let path = directory.join(name);
        if std::fs::symlink_metadata(&path)?.file_type().is_symlink() {
            bail!("Snippet file must not be a symbolic link: {name}");
        }
        if !path.is_file() {
            bail!("Missing snippet file: {name}");
        }
    }
    chrono::DateTime::parse_from_rfc3339(&metadata.created_at)
        .context("Invalid snippet creation date")?;
    let markdown = std::fs::read_to_string(directory.join("context.md"))?;
    let text = markdown
        .strip_prefix("# Capture\n\n")
        .and_then(|s| s.strip_suffix("\n\n![Screenshot](./capture.png)\n"))
        .context("Unrecognized context.md format")?
        .to_owned();
    Ok(Snippet {
        metadata,
        directory,
        text,
    })
}
pub fn list(root: &Path) -> Result<Vec<Snippet>> {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(e).context("Read capture directory"),
    };
    let mut snippets = vec![];
    for entry in entries {
        let entry = entry?;
        let id = entry.file_name().to_string_lossy().into_owned();
        if !valid_id(&id) || !entry.file_type()?.is_dir() {
            continue;
        }
        match std::fs::metadata(entry.path().join("metadata.json")) {
            Ok(_) => snippets.push(load(root, &id)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e).context("Read snippet commit marker"),
        }
    }
    snippets.sort_by(|a, b| {
        let a_date = chrono::DateTime::parse_from_rfc3339(&a.metadata.created_at)
            .expect("validated creation date");
        let b_date = chrono::DateTime::parse_from_rfc3339(&b.metadata.created_at)
            .expect("validated creation date");
        b_date
            .cmp(&a_date)
            .then_with(|| b.metadata.id.cmp(&a.metadata.id))
    });
    Ok(snippets)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mid_save_disk_full_rolls_back_image_and_directory() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("library");
        let png = temp.path().join("input.png");
        image::RgbaImage::new(2, 2).save(&png).unwrap();
        let result = save_with_context_writer(&root, &png, "keep this", |path, _| {
            assert!(path.parent().unwrap().join("capture.png").exists());
            Err(std::io::Error::from(std::io::ErrorKind::StorageFull).into())
        });
        assert!(result.is_err());
        assert!(png.exists());
        assert_eq!(std::fs::read_dir(root).unwrap().count(), 0);
    }
}
