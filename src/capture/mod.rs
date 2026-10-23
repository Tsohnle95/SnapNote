#[cfg(target_os = "macos")]
mod macos;
#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    #[error(
        "Screen Recording permission is required. Enable SnapNote in System Settings → Privacy & Security → Screen & System Audio Recording, then quit and relaunch SnapNote."
    )]
    PermissionDenied,
    #[error(
        "Screenshot process failed: {0}. Check Screen Recording permission in System Settings if this persists."
    )]
    Process(String),
    #[error("Screenshot file operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("Screenshot output is not a valid PNG: {0}")]
    InvalidImage(#[from] image::ImageError),
    #[error("Region capture is currently supported on macOS only")]
    Unsupported,
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct CursorAnchor {
    pub x: f64,
    pub y: f64,
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
}

// The resident owns only a file handle and timing data, never decoded screenshot pixels.
pub struct CapturedFile {
    pub anchor: Option<CursorAnchor>,
    pub file: tempfile::NamedTempFile,
    pub completed: std::time::Instant,
}
fn temporary_file() -> Result<tempfile::NamedTempFile, CaptureError> {
    // macOS 27 screencapture rejects dot-prefixed filenames, sometimes with exit 0.
    Ok(tempfile::Builder::new()
        .prefix("snapnote-capture-")
        .suffix(".png")
        .tempfile()?)
}

fn inspect(
    file: tempfile::NamedTempFile,
    _success: bool,
    diagnostics: &[u8],
) -> Result<Option<CapturedFile>, CaptureError> {
    let length = match std::fs::metadata(file.path()) {
        Ok(metadata) => metadata.len(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => 0,
        Err(error) => return Err(error.into()),
    };
    if length == 0 {
        let message = String::from_utf8_lossy(diagnostics);
        if !message.trim().is_empty() {
            return Err(CaptureError::Process(message.trim().into()));
        }
        return Ok(None);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(file.path(), std::fs::Permissions::from_mode(0o600))?;
    }
    let reader = image::ImageReader::with_format(
        std::io::BufReader::new(std::fs::File::open(file.path())?),
        image::ImageFormat::Png,
    );
    reader.into_dimensions()?;
    Ok(Some(CapturedFile {
        anchor: None,
        file,
        completed: std::time::Instant::now(),
    }))
}

pub struct Capture {
    // Ownership keeps the private temporary PNG alive until save or cancellation.
    pub file: tempfile::NamedTempFile,
    pub image: image::RgbaImage,
    pub completed: std::time::Instant,
}

fn validate(
    file: tempfile::NamedTempFile,
    success: bool,
    stderr: &[u8],
) -> Result<Option<Capture>, CaptureError> {
    let bytes = std::fs::read(file.path())?;
    if bytes.is_empty() {
        let error = String::from_utf8_lossy(stderr);
        if !success && !error.trim().is_empty() {
            return Err(CaptureError::Process(error.trim().into()));
        }
        return Ok(None);
    }
    let decoded = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)?;
    let image = if decoded.width() > 1600 || decoded.height() > 1600 {
        decoded.thumbnail(1600, 1600)
    } else {
        decoded
    }
    .to_rgba8();
    Ok(Some(Capture {
        file,
        image,
        completed: std::time::Instant::now(),
    }))
}

pub fn adopt(path: std::path::PathBuf) -> Result<Option<Capture>, CaptureError> {
    let parent = std::fs::canonicalize(
        path.parent()
            .ok_or_else(|| std::io::Error::other("Invalid capture path"))?,
    )?;
    let temp = std::fs::canonicalize(std::env::temp_dir())?;
    if parent != temp
        || !path
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with("snapnote-capture-"))
        || std::fs::symlink_metadata(&path)?.file_type().is_symlink()
    {
        return Err(
            std::io::Error::other("Editor accepts only SnapNote-owned temporary captures").into(),
        );
    }
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)?;
    validate(
        tempfile::NamedTempFile::from_parts(file, tempfile::TempPath::try_from_path(path)?),
        true,
        b"",
    )
}

pub fn region() -> Result<Option<CapturedFile>, CaptureError> {
    #[cfg(target_os = "macos")]
    {
        macos::region()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err(CaptureError::Unsupported)
    }
}
pub fn request_permission() -> bool {
    #[cfg(target_os = "macos")]
    {
        macos::request_permission()
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_diagnostic_is_an_error_even_with_exit_zero_and_missing_file() {
        let file = temporary_file().unwrap();
        std::fs::remove_file(file.path()).unwrap();
        assert!(
            inspect(
                file,
                true,
                b"screencapture: cannot write file to intended destination"
            )
            .is_err()
        );
        let file = temporary_file().unwrap();
        std::fs::remove_file(file.path()).unwrap();
        assert!(inspect(file, false, b"").unwrap().is_none());
    }
    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "captures a 20x20 top-left region using the real macOS tool"]
    fn native_tool_writes_supported_private_temporary_filename() {
        use std::os::unix::fs::PermissionsExt;
        let file = temporary_file().unwrap();
        let output = std::process::Command::new("/usr/sbin/screencapture")
            .args(["-x", "-R0,0,20,20"])
            .arg(file.path())
            .output()
            .unwrap();
        let diagnostics = [output.stderr, output.stdout].concat();
        let result = inspect(file, output.status.success(), &diagnostics)
            .unwrap()
            .unwrap();
        assert_eq!(
            std::fs::metadata(result.file.path())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        let decoded = image::open(result.file.path()).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (20, 20));
    }
    #[test]
    fn cancellation_removes_temporary_output() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let path = file.path().to_owned();
        assert!(validate(file, true, b"").unwrap().is_none());
        assert!(!path.exists());
    }
    #[test]
    fn permission_error_has_recovery_guidance_and_cleans_up() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let path = file.path().to_owned();
        let error = validate(file, false, b"could not create image from display")
            .err()
            .unwrap();
        assert!(error.to_string().contains("System Settings"));
        assert!(!path.exists());
    }
    #[test]
    fn exit_success_does_not_make_invalid_image_valid() {
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), b"not an image").unwrap();
        let path = file.path().to_owned();
        assert!(validate(file, true, b"").is_err());
        assert!(!path.exists());
    }
    #[test]
    fn validated_png_is_retained_until_capture_is_dropped() {
        let file = tempfile::NamedTempFile::new().unwrap();
        image::RgbaImage::from_pixel(3, 2, image::Rgba([20, 30, 40, 255]))
            .save_with_format(file.path(), image::ImageFormat::Png)
            .unwrap();
        let capture = validate(file, true, b"").unwrap().unwrap();
        let path = capture.file.path().to_owned();
        assert_eq!(capture.image.dimensions(), (3, 2));
        assert!(path.exists());
        drop(capture);
        assert!(!path.exists());
    }
}
