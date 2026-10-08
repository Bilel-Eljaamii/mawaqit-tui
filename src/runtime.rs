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
                other => {
                    // Network commands run through the shared performer so
                    // the live loop and the e2e scripted loop cannot drift
                    // (spec M6 R5).
                    let times = Arc::clone(&deps.times);
                    let clock = Arc::clone(&deps.clock);
                    let settings = Arc::clone(&deps.settings);
                    let tx = event_tx.clone();
                    tokio::spawn(async move {
                        let mut ctx = CommandCtx {
                            times: times.as_ref(),
                            clock: clock.as_ref(),
                            settings: settings.as_ref(),
                        };
                        let event = perform(&mut ctx, other).await;
                        let _ = tx.send(event).await;
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

/// The single command→event match, shared by the live runtime and the e2e
/// scripted loop (spec M6 R5): whichever path runs a command, the same use
/// case executes against the same ports. Debounce stays in the live loop —
/// it is a *timing* concern, not a command semantic.
pub struct CommandCtx<
    'a,
    T: TimesService + MosqueDirectory + ?Sized,
    C: Clock + ?Sized,
    S: SettingsStore + ?Sized,
> {
    pub times: &'a T,
    pub clock: &'a C,
    pub settings: &'a S,
}

pub async fn perform<T, C, S>(
    ctx: &mut CommandCtx<'_, T, C, S>,
    command: Command,
) -> AppEvent
where
    T: TimesService + MosqueDirectory + Send + Sync,
    C: Clock + Send + Sync,
    S: SettingsStore + Send + Sync,
{
    match command {
        Command::LoadToday(id) => {
            AppEvent::TodayLoaded(LoadToday { times: ctx.times }.execute(&id).await)
        }
        Command::Search { epoch, query } => AppEvent::SearchLoaded(
            epoch,
            SearchMosques { directory: ctx.times }.execute(&query).await,
        ),
        Command::SaveSelection(summary) => AppEvent::SelectionSaved(
            SaveSelection { settings: ctx.settings }.execute(&summary),
        ),
        Command::LoadMonth { id, month } => AppEvent::MonthLoaded {
            id: id.clone(),
            month,
            result: LoadMonth { times: ctx.times }.execute(&id, month).await,
        },
    }
}

/// The e2e seam (spec M6 R5): boot → apply each scripted event → drain and
/// perform every emitted command inline → feed results back until quiet →
/// next event. Same model, same use cases, same command semantics as the
/// live loop; no terminal, no network, no sleeps.
pub async fn run_scripted<T, C, S>(
    deps: Runtime<T, C, S>,
    boot: Boot,
    script: Vec<AppEvent>,
) -> AppModel
where
    T: TimesService + MosqueDirectory + Send + Sync + 'static,
    C: Clock + Send + Sync + 'static,
    S: SettingsStore + Send + Sync + 'static,
{
    let (mut model, mut commands) = AppModel::from_boot(boot);
    model.update(AppEvent::Tick(deps.clock.now_utc()));

    for scripted in script {
        // Drain the command queue to quiescence before the next scripted
        // event, feeding resulting events straight back into the model.
        loop {
            let mut resulting = Vec::new();
            for command in commands.drain(..) {
                let mut ctx = CommandCtx {
                    times: deps.times.as_ref(),
                    clock: deps.clock.as_ref(),
                    settings: deps.settings.as_ref(),
                };
                resulting.push(perform(&mut ctx, command).await);
            }
            if resulting.is_empty() {
                break;
            }
            for event in resulting {
                for command in model.update(event) {
                    commands.push(command);
                }
            }
        }
        for command in model.update(scripted) {
            commands.push(command);
        }
    }

    // Final quiescence: the last scripted event may have left commands.
    loop {
        let mut resulting = Vec::new();
        for command in commands.drain(..) {
            let mut ctx = CommandCtx {
                times: deps.times.as_ref(),
                clock: deps.clock.as_ref(),
                settings: deps.settings.as_ref(),
            };
            resulting.push(perform(&mut ctx, command).await);
        }
        if resulting.is_empty() {
            break;
        }
        for event in resulting {
            for command in model.update(event) {
                commands.push(command);
            }
        }
    }
    model
}
