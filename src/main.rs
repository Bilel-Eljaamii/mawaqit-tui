//! Binary entry point — composition root (ADR-0001, spec M3 R5).

use std::sync::Arc;

use mawaqit_tui::{
    application::ports::SettingsStore,
    domain::mosque::MosqueId,
    infrastructure::{
        clock::SystemClock, mawaqit::MawaqitAdapter, settings::TomlSettings,
    },
    runtime::{self, Runtime},
    ui::app::Boot,
};

const USAGE: &str = "usage: mawaqit-tui [--mosque <slug>]";

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let boot = match parse_cli() {
        Some(id) => Boot::Loading(id),
        None => match TomlSettings::default_path() {
            Some(path) => match TomlSettings::new(path).load() {
                Ok(Some(selected)) => Boot::Loading(selected.id),
                Ok(None) => Boot::NoMosque,
                Err(err) => Boot::Failed(err.to_string()),
            },
            None => Boot::Failed("no config directory on this platform".to_owned()),
        },
    };
    let deps =
        Runtime { times: Arc::new(MawaqitAdapter::new()), clock: Arc::new(SystemClock) };
    runtime::run(deps, boot).await
}

/// `--mosque <slug>` overrides the saved selection for this session; invalid
/// usage exits 2 before any terminal state is touched.
fn parse_cli() -> Option<MosqueId> {
    let mut args = std::env::args().skip(1);
    let mut mosque = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--mosque" => match args.next() {
                Some(raw) => match MosqueId::parse(&raw) {
                    Ok(id) => mosque = Some(id),
                    Err(_) => usage_exit(&format!("invalid slug {raw:?}")),
                },
                None => usage_exit("--mosque requires a value"),
            },
            "--help" | "-h" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            other => usage_exit(&format!("unknown argument {other:?}")),
        }
    }
    mosque
}

fn usage_exit(reason: &str) -> ! {
    eprintln!("{USAGE}\n{reason}");
    std::process::exit(2);
}
