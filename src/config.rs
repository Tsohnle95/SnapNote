use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Context,
    Path,
    #[default]
    Agent,
    Image,
    SaveOnly,
}
impl Mode {
    pub const ALL: [Self; 5] = [
        Self::Context,
        Self::Path,
        Self::Agent,
        Self::Image,
        Self::SaveOnly,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Context => "Context only",
            Self::Path => "Screenshot path",
            Self::Agent => "Screenshot reference + context",
            Self::Image => "Image data",
            Self::SaveOnly => "Save only",
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub schema_version: u32,
    pub capture: CaptureConfig,
    pub storage: StorageConfig,
    pub clipboard: ClipboardConfig,
    pub ui: UiConfig,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CaptureConfig {
    pub hotkey: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StorageConfig {
    pub directory: PathBuf,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ClipboardConfig {
    pub default_mode: Mode,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub close_after_save: bool,
}
impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            hotkey: "Command+B".into(),
        }
    }
}
impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            directory: PathBuf::from("~/screenshots/snapshot"),
        }
    }
}
impl Default for UiConfig {
    fn default() -> Self {
        Self {
            close_after_save: true,
        }
    }
}
impl Default for Config {
    fn default() -> Self {
        Self {
            schema_version: 1,
            capture: Default::default(),
            storage: Default::default(),
            clipboard: Default::default(),
            ui: Default::default(),
        }
    }
}
pub fn home() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .context("HOME is unavailable")
}
pub fn default_path() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("SNAPNOTE_CONFIG") {
        return Ok(path.into());
    }
    Ok(home()?.join("Library/Application Support/SnapNote/config.toml"))
}
pub fn expand(path: &Path) -> Result<PathBuf> {
    let path = if path == Path::new("~") {
        home()?
    } else if let Ok(rest) = path.strip_prefix("~/") {
        home()?.join(rest)
    } else {
        path.to_owned()
    };
    if !path.is_absolute() {
        bail!("Storage directory must be an absolute path or begin with ~/");
    }
    Ok(path)
}
impl Config {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1 {
            bail!("Unsupported configuration schema {}", self.schema_version);
        }
        let _: global_hotkey::hotkey::HotKey = self
            .capture
            .hotkey
            .parse()
            .context("Invalid capture shortcut")?;
        expand(&self.storage.directory)?;
        Ok(())
    }
    pub fn load(path: &Path) -> Result<Self> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e).context("Read configuration"),
        };
        let config: Self = toml::from_str(&text).context("Malformed configuration TOML")?;
        config.validate()?;
        Ok(config)
    }
    pub fn save(&self, path: &Path) -> Result<()> {
        use std::io::Write;
        self.validate()?;
        let parent = path
            .parent()
            .context("Configuration needs a parent directory")?;
        std::fs::create_dir_all(parent)?;
        let mut temp = tempfile::NamedTempFile::new_in(parent)?;
        temp.write_all(toml::to_string_pretty(self)?.as_bytes())?;
        temp.as_file().sync_all()?;
        temp.persist(path)
            .context("Replace configuration atomically")?;
        Ok(())
    }
}
