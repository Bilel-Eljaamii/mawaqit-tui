//! Terminal lifecycle and the event loop skeleton (spec M1 R6, ADR-0001 §2/§4).
//!
//! The loop is the only place that awaits real I/O: crossterm `EventStream`,
//! a 1 s tick, and (from M2 on) command results. Everything else is a pure
//! state transition in the AppModel.

use std::{io, io::stdout, time::Duration};

use crossterm::{
    event::{Event as CtEvent, EventStream, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
    },
};
use futures_util::StreamExt;
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::ui::app::AppModel;

/// Events the AppModel understands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEvent {
    Key(KeyEvent),
    Tick,
}

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

/// Event loop: draw, wait for the next input event or tick, transition the
/// model, repeat. `q` / Ctrl-C quit (ADR-0001 §6); every other key is inert.
pub async fn run(mut model: AppModel) -> io::Result<()> {
    install_panic_hook();
    let _guard = TerminalGuard::new()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut events = EventStream::new();
    let mut tick = tokio::time::interval(Duration::from_secs(1));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        terminal.draw(|frame| crate::ui::draw::draw(frame, &model))?;
        tokio::select! {
            maybe_event = events.next() => {
                match maybe_event {
                    Some(Ok(CtEvent::Key(key))) => {
                        model.update(AppEvent::Key(key));
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
            _ = tick.tick() => model.update(AppEvent::Tick),
        }
    }
    Ok(())
}

/// Quit keys, single-sourced for the runtime and tests: `q` without
/// modifiers, or Ctrl-C. Unknown keys are inert.
pub const fn is_quit_key(key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('q') => key.modifiers.is_empty(),
        KeyCode::Char('c') => key.modifiers.contains(KeyModifiers::CONTROL),
        _ => false,
    }
}
