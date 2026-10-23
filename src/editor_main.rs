mod gui;
use clap::Parser;
use std::path::PathBuf;
#[derive(Parser)]
struct Args {
    #[arg(long)]
    config: PathBuf,
    #[arg(long)]
    capture_file: Option<PathBuf>,
    #[arg(long, default_value = "capture")]
    page: String,
    #[arg(long)]
    status: Option<String>,
    #[arg(long)]
    capture_completed_ms: Option<u128>,
    #[arg(long)]
    anchor: Option<String>,
}
fn main() {
    let args = Args::parse();
    let loaded = snapnote::config::Config::load(&args.config);
    match gui::run(
        args.config,
        loaded,
        gui::Options {
            capture_file: args.capture_file,
            page: args.page,
            status: args.status,
            capture_completed_ms: args.capture_completed_ms,
            anchor: args
                .anchor
                .and_then(|text| serde_json::from_str(&text).ok()),
        },
    ) {
        Ok(code) => {
            if code != 0 {
                std::process::exit(code);
            }
        }
        Err(error) => {
            eprintln!("snapnote-editor: {error:#}");
            std::process::exit(1);
        }
    }
}
