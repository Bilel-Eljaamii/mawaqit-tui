//! Terminal lifecycle + the Elm command/event loop (ADR-0001 §2, spec M3 R4,
//! spec M4 R2–R4).
//!
//! The loop drains fresh commands, spawns each on tokio, draws, then selects
//! over terminal input, the 1 s tick, and command results arriving on mpsc.
//! Time enters the model only through `Tick` instants — never read directly.

use std::{
    io,
    io::stdout,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use crossterm::{
    event::{Event as CtEvent, EventStream},
    execute,
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
    },
};
use futures_util::StreamExt;
use ratatui::{Terminal, backend::CrosstermBackend};
use tokio::sync::mpsc;

use crate::{
    application::{
        ports::{Clock, MosqueDirectory, SettingsStore, TimesService},
        use_cases::{LoadMonth, LoadToday, SaveSelection, SearchMosques},
    },
    ui::app::{AppEvent, AppModel, Boot, Command},
};

/// Quiet window before a query actually hits the directory (spec M4 R2).
const SEARCH_DEBOUNCE: Duration = Duration::from_millis(300);

/// Owns raw-mode + alternate-screen state; restores the terminal on drop,
/// including unwinding through a panic (panic hook installed alongside).
pub struct TerminalGuard;

impl TerminalGuard {
    /// Enter raw mode and the alternate screen.
    pub fn new() -> io::Result<TerminalGuard> {
        enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen)?;
        Ok(TerminalGuard)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
    }
}

/// Restore the terminal before the default panic hook runs, so a panic never
/// leaves the user's shell broken.
pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
        default_hook(info);
    }));
}

/// Ports the command executor runs against. One adapter serves both
/// directory traits (spec M4 R4).
pub struct Runtime<T: TimesService + MosqueDirectory, C: Clock, S: SettingsStore> {
    pub times: Arc<T>,
    pub clock: Arc<C>,
    pub settings: Arc<S>,
}

/// Event loop: `q` / Ctrl-C quit (screen-dependent, see the model); every
/// other key is inert. An ended or failed input stream degrades to a clean
/// exit.
pub async fn run<T, C, S>(deps: Runtime<T, C, S>, boot: Boot) -> io::Result<()>
where
    T: TimesService + MosqueDirectory + Send + Sync + 'static,
    C: Clock + Send + Sync + 'static,
    S: SettingsStore + Send + Sync + 'static,
{
    install_panic_hook();
    let _guard = TerminalGuard::new()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let (mut model, mut commands) = AppModel::from_boot(boot);
    // Frame 1 is live: prime the clock before entering the loop.
    model.update(AppEvent::Tick(deps.clock.now_utc()));

    let (event_tx, mut event_rx) = mpsc::channel::<AppEvent>(32);
    let mut events = EventStream::new();
    let mut tick = tokio::time::interval(Duration::from_secs(1));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    // Newest query generation; debounce sleepers compare against it.
    let latest_search = Arc::new(AtomicU64::new(0));

    loop {
        for command in commands.drain(..) {
            match command {
                Command::LoadToday(id) => {
                    let times = Arc::clone(&deps.times);
                    let tx = event_tx.clone();
                    tokio::spawn(async move {
                        let result =
                            LoadToday { times: times.as_ref() }.execute(&id).await;
                        let _ = tx.send(AppEvent::TodayLoaded(result)).await;
                    });
                }
                Command::Search { epoch, query } => {
                    latest_search.store(epoch, Ordering::Release);
                    let directory = Arc::clone(&deps.times);
                    let latest = Arc::clone(&latest_search);
                    let tx = event_tx.clone();
                    tokio::spawn(async move {
                        tokio::time::sleep(SEARCH_DEBOUNCE).await;
                        // A newer keystroke owns the screen: this query is
                        // dropped without touching the port (spec M4 R2).
                        if latest.load(Ordering::Acquire) != epoch {
                            return;
                        }
                        let result = SearchMosques { directory: directory.as_ref() }
                            .execute(&query)
                            .await;
                        let _ = tx.send(AppEvent::SearchLoaded(epoch, result)).await;
                    });
                }
                Command::SaveSelection(summary) => {
                    // Sync port by design (ADR-0002 §5): a tiny atomic TOML
                    // write inline, result posted back like any other event.
                    let result = SaveSelection { settings: deps.settings.as_ref() }
                        .execute(&summary);
                    let _ = event_tx.send(AppEvent::SelectionSaved(result)).await;
                }
                Command::LoadMonth { id, month } => {
                    let times = Arc::clone(&deps.times);
                    let tx = event_tx.clone();
                    tokio::spawn(async move {
                        let result =
                            LoadMonth { times: times.as_ref() }.execute(&id, month).await;
                        let _ =
                            tx.send(AppEvent::MonthLoaded { id, month, result }).await;
                    });
                }
            }
        }

        terminal.draw(|frame| crate::ui::draw::draw(frame, &model))?;
        tokio::select! {
            maybe_event = events.next() => {
                match maybe_event {
                    Some(Ok(CtEvent::Key(key))) => {
                        commands = model.update(AppEvent::Key(key));
                        if model.should_quit {
                            break;
                        }
                    }
                    // Resize and other events redraw on the next loop pass.
                    Some(Ok(_)) => {}
                    // The input stream ended or failed; degrade to a clean exit.
                    _ => break,
                }
            }
            _ = tick.tick() => {
                commands = model.update(AppEvent::Tick(deps.clock.now_utc()));
            }
            maybe_app = event_rx.recv() => {
                match maybe_app {
                    Some(event) => commands = model.update(event),
                    None => break,
                }
            }
        }
        if model.should_quit {
            break;
        }
    }
    Ok(())
}
