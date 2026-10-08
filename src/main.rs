//! Binary entry point — composition root (ADR-0001, spec M3 R5, spec M4 R4).

use std::sync::Arc;

use mawaqit_tui::{
    application::ports::{Clock, MosqueDirectory, SettingsStore, TimesService},
    domain::mosque::MosqueId,
    infrastructure::{
        clock::SystemClock,
        mawaqit::MawaqitAdapter,
        settings::{NullSettings, TomlSettings},
        snapshot::cache_dir,
    },
    runtime::{self, Runtime},
    ui::app::Boot,
};

const USAGE: &str = "usage: mawaqit-tui [--mosque <slug>]";

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let cli_mosque = parse_cli();
    match TomlSettings::default_path() {
        Some(path) => {
            let settings = Arc::new(TomlSettings::new(path));
            let boot = match cli_mosque {
                Some(id) => Boot::Loading(id),
                None => match settings.load() {
                    Ok(Some(selected)) => Boot::Loading(selected.id),
                    Ok(None) => Boot::NoMosque,
                    Err(err) => Boot::Failed(err.to_string()),
                },
            };
            start(
                Arc::new(MawaqitAdapter::new(cache_dir())),
                Arc::new(SystemClock),
                settings,
                boot,
            )
            .await
        }
        None => {
            let boot = match cli_mosque {
                Some(id) => Boot::Loading(id),
                None => Boot::Failed("no config directory on this platform".to_owned()),
            };
            start(
                Arc::new(MawaqitAdapter::new(cache_dir())),
                Arc::new(SystemClock),
                Arc::new(NullSettings),
                boot,
            )
            .await
        }
    }
}

/// Monomorphized start so the with/without-config-dir compositions can share
/// the fully generic runtime (no `dyn` — ADR-0002 §1).
async fn start<T, C, S>(
    times: Arc<T>,
    clock: Arc<C>,
    settings: Arc<S>,
    boot: Boot,
) -> std::io::Result<()>
where
    T: TimesService + MosqueDirectory + Send + Sync + 'static,
    C: Clock + Send + Sync + 'static,
    S: SettingsStore + Send + Sync + 'static,
{
    runtime::run(Runtime { times, clock, settings }, boot).await
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
