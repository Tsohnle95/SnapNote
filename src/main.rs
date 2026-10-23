mod resident;
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use snapnote::{
    clipboard,
    config::{self, Config, Mode},
    storage,
};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(version, about = "Local screenshot and context snippets")]
struct Args {
    #[arg(
        long,
        global = true,
        help = "Configuration TOML path (also SNAPNOTE_CONFIG)"
    )]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Commands>,
}
#[derive(Subcommand)]
enum Commands {
    Latest,
    List,
    Show {
        id: String,
    },
    Path {
        id: String,
    },
    Copy {
        id: String,
        #[arg(long, value_enum, default_value = "agent")]
        mode: Mode,
    },
    OpenFolder,
}
pub fn open_folder(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path)?;
    let program = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        "explorer"
    } else {
        "xdg-open"
    };
    let status = std::process::Command::new(program)
        .arg(path)
        .status()
        .context("Open capture directory")?;
    anyhow::ensure!(status.success(), "File manager exited with {status}");
    Ok(())
}
fn run() -> Result<i32> {
    let args = Args::parse();
    let config_path = args.config.map(Ok).unwrap_or_else(config::default_path)?;
    let config_path = if config_path.is_absolute() {
        config_path
    } else {
        std::env::current_dir()?.join(config_path)
    };
    let loaded = Config::load(&config_path);
    let Some(command) = args.command else {
        resident::run(config_path)?;
        return Ok(0);
    };
    // CLI dispatch happens before native event-loop or tray initialization.
    let config = loaded?;
    let root = config::expand(&config.storage.directory)?;
    match command {
        Commands::Latest => {
            let snippet = storage::list(&root)?
                .into_iter()
                .next()
                .context("No saved captures")?;
            println!(
                "{}",
                clipboard::format(Mode::Agent, &snippet.image_path(), &snippet.text)?
            );
        }
        Commands::List => {
            for snippet in storage::list(&root)? {
                let preview = snippet
                    .text
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
                let preview: String = preview.chars().take(90).collect();
                println!(
                    "{}\t{}\t{}",
                    snippet.metadata.id, snippet.metadata.created_at, preview
                );
            }
        }
        Commands::Show { id } => {
            let snippet = storage::load(&root, &id)?;
            println!(
                "{}",
                clipboard::format(Mode::Agent, &snippet.image_path(), &snippet.text)?
            );
        }
        Commands::Path { id } => println!("{}", storage::load(&root, &id)?.image_path().display()),
        Commands::Copy { id, mode } => {
            let snippet = storage::load(&root, &id)?;
            eprintln!("{}", clipboard::copy(&snippet, mode)?);
        }
        Commands::OpenFolder => open_folder(&root)?,
    }
    Ok(0)
}
fn main() {
    match run() {
        Ok(code) => {
            if code != 0 {
                std::process::exit(code);
            }
        }
        Err(error) => {
            eprintln!("snapnote: {error:#}");
            std::process::exit(1);
        }
    }
}
