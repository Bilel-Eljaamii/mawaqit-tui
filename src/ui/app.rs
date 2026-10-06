//! AppModel — Elm-style state machine (ADR-0001 §2, spec M3 R4).
//!
//! Transitions are pure: time arrives inside `Tick` (never read from a clock
//! here), side effects leave as `Command`s, results come back as events.

use chrono::{DateTime, Utc};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    application::{ports::TodayReadout, use_cases::AppError},
    domain::mosque::MosqueId,
};

/// What the runtime must do on the model's behalf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    LoadToday(MosqueId),
}

/// Events the model understands.
#[derive(Debug, Clone)]
pub enum AppEvent {
    Key(KeyEvent),
    Tick(DateTime<Utc>),
    TodayLoaded(Result<TodayReadout, AppError>),
}

/// Startup input resolved by the composition root (spec M3 R5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Boot {
    Loading(MosqueId),
    NoMosque,
    Failed(String),
}

/// Lifecycle of the today screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TodayState {
    NoMosque,
    Loading,
    Ready(TodayReadout),
    Failed(String),
}

pub struct AppModel {
    pub should_quit: bool,
    pub state: TodayState,
    ticks: u64,
    last_now: Option<DateTime<Utc>>,
}

impl AppModel {
    /// Boot state plus the commands the runtime must run first.
    pub fn from_boot(boot: Boot) -> (AppModel, Vec<Command>) {
        let (state, commands) = match boot {
            Boot::Loading(id) => (TodayState::Loading, vec![Command::LoadToday(id)]),
            Boot::NoMosque => (TodayState::NoMosque, Vec::new()),
            Boot::Failed(message) => (TodayState::Failed(message), Vec::new()),
        };
        let model = AppModel { should_quit: false, state, ticks: 0, last_now: None };
        (model, commands)
    }

    pub const fn ticks(&self) -> u64 {
        self.ticks
    }

    /// Latest `Tick` instant — the view's only time source.
    pub const fn last_now(&self) -> Option<DateTime<Utc>> {
        self.last_now
    }

    /// Transition on one event; returns fresh commands for the runtime.
    /// Total; unknown keys are inert (ADR-0001 §4).
    pub fn update(&mut self, event: AppEvent) -> Vec<Command> {
        match event {
            AppEvent::Key(key) => self.handle_key(key),
            AppEvent::Tick(now) => {
                self.ticks = self.ticks.wrapping_add(1);
                self.last_now = Some(now);
            }
            AppEvent::TodayLoaded(result) => {
                self.state = match result {
                    Ok(readout) => TodayState::Ready(readout),
                    Err(err) => TodayState::Failed(err.to_string()),
                };
            }
        }
        Vec::new()
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if is_quit_key(key) {
            self.should_quit = true;
        }
    }
}

/// Quit keys, single-sourced: bare `q`, or Ctrl-C. Unknown keys are inert.
pub const fn is_quit_key(key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('q') => key.modifiers.is_empty(),
        KeyCode::Char('c') => key.modifiers.contains(KeyModifiers::CONTROL),
        _ => false,
    }
}
