//! AppModel — Elm-style state machine (ADR-0001 §2, spec M3 R4, spec M4 R1–R3).
//!
//! Transitions are pure: time arrives inside `Tick` (never read from a clock
//! here), side effects leave as `Command`s, results come back as events.

use chrono::{DateTime, Utc};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    application::{ports::TodayReadout, use_cases::AppError},
    domain::{
        events::DomainEvent,
        mosque::{MosqueId, MosqueSummary},
    },
};

/// What the runtime must do on the model's behalf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    LoadToday(MosqueId),
    /// Debounced by the runtime: only the newest epoch is executed.
    Search {
        epoch: u64,
        query: String,
    },
    SaveSelection(MosqueSummary),
}

/// Events the model understands.
#[derive(Debug, Clone)]
pub enum AppEvent {
    Key(KeyEvent),
    Tick(DateTime<Utc>),
    TodayLoaded(Result<TodayReadout, AppError>),
    /// Search results for query `epoch`; stale epochs are discarded by the
    /// model (spec M4 R2).
    SearchLoaded(u64, Result<Vec<MosqueSummary>, AppError>),
    SelectionSaved(Result<(), AppError>),
}

/// Startup input resolved by the composition root (spec M3 R5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Boot {
    Loading(MosqueId),
    NoMosque,
    Failed(String),
}

/// Which screen is front.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Today,
    Search,
}

/// Lifecycle of the today screen. `NoMosque` is never a boot state (boot
/// lands on Search) — it is where Esc goes back to before any selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TodayState {
    NoMosque,
    Loading,
    Ready(TodayReadout),
    Failed(String),
}

/// Lifecycle of the search screen (spec M4 R2–R3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchState {
    Idle,
    Querying {
        epoch: u64,
    },
    Results {
        epoch: u64,
        items: Vec<MosqueSummary>,
        cursor: usize,
    },
    /// A selection is being persisted; Enter is inert until it resolves.
    Saving(MosqueSummary),
    /// Persistence failed; the reason is verbatim and Enter retries.
    SaveFailed {
        summary: MosqueSummary,
        reason: String,
    },
    /// The directory failed; the reason is verbatim.
    Failed(String),
}

impl SearchState {
    /// Cursor row while a result list is on screen.
    pub const fn cursor(&self) -> Option<usize> {
        match self {
            SearchState::Results { cursor, .. } => Some(*cursor),
            _ => None,
        }
    }
}

pub struct AppModel {
    pub should_quit: bool,
    pub screen: Screen,
    pub today: TodayState,
    pub search: SearchState,
    /// Raw query text; trimmed before it reaches the port.
    pub query: String,
    /// Monotonic query generation; rises on every edit (wrapping).
    search_epoch: u64,
    /// Domain events drained by `take_events` (consumers land in M6).
    events: Vec<DomainEvent>,
    ticks: u64,
    last_now: Option<DateTime<Utc>>,
}

impl AppModel {
    /// Boot state plus the commands the runtime must run first.
    pub fn from_boot(boot: Boot) -> (AppModel, Vec<Command>) {
        let (screen, today, search, commands) = match boot {
            Boot::Loading(id) => (
                Screen::Today,
                TodayState::Loading,
                SearchState::Idle,
                vec![Command::LoadToday(id)],
            ),
            Boot::NoMosque => {
                (Screen::Search, TodayState::NoMosque, SearchState::Idle, Vec::new())
            }
            Boot::Failed(message) => (
                Screen::Today,
                TodayState::Failed(message),
                SearchState::Idle,
                Vec::new(),
            ),
        };
        let model = AppModel {
            should_quit: false,
            screen,
            today,
            search,
            query: String::new(),
            search_epoch: 0,
            events: Vec::new(),
            ticks: 0,
            last_now: None,
        };
        (model, commands)
    }

    pub const fn ticks(&self) -> u64 {
        self.ticks
    }

    /// Latest `Tick` instant — the view's only time source.
    pub const fn last_now(&self) -> Option<DateTime<Utc>> {
        self.last_now
    }

    /// Drain recorded domain events (spec M4 R3).
    pub fn take_events(&mut self) -> Vec<DomainEvent> {
        std::mem::take(&mut self.events)
    }

    /// Transition on one event; returns fresh commands for the runtime.
    /// Total; unknown keys are inert (ADR-0001 §4).
    pub fn update(&mut self, event: AppEvent) -> Vec<Command> {
        match event {
            AppEvent::Key(key) => self.handle_key(key),
            AppEvent::Tick(now) => {
                self.ticks = self.ticks.wrapping_add(1);
                self.last_now = Some(now);
                Vec::new()
            }
            AppEvent::TodayLoaded(result) => {
                self.today = match result {
                    Ok(readout) => TodayState::Ready(readout),
                    Err(err) => TodayState::Failed(err.to_string()),
                };
                Vec::new()
            }
            AppEvent::SearchLoaded(epoch, result) => self.on_search_loaded(epoch, result),
            AppEvent::SelectionSaved(result) => self.on_selection_saved(result),
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Vec<Command> {
        match self.screen {
            Screen::Search => self.handle_search_key(key),
            Screen::Today => self.handle_today_key(key),
        }
    }

    fn handle_today_key(&mut self, key: KeyEvent) -> Vec<Command> {
        if is_quit_key(key) {
            self.should_quit = true;
        } else if is_bare(key, KeyCode::Char('s')) {
            self.screen = Screen::Search;
        }
        Vec::new()
    }

    /// On Search, bare `q` is a literal query character; only Ctrl-C quits
    /// here (spec M4 R2).
    fn handle_search_key(&mut self, key: KeyEvent) -> Vec<Command> {
        if is_force_quit_key(key) {
            self.should_quit = true;
            return Vec::new();
        }
        match key.code {
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.query.push(c);
                self.query_changed()
            }
            KeyCode::Backspace => {
                self.query.pop();
                self.query_changed()
            }
            KeyCode::Esc => {
                self.screen = Screen::Today;
                Vec::new()
            }
            KeyCode::Down => {
                self.move_cursor(1);
                Vec::new()
            }
            KeyCode::Up => {
                self.move_cursor(-1);
                Vec::new()
            }
            KeyCode::Enter => self.select_cursor(),
            _ => Vec::new(),
        }
    }

    /// Every edit starts a new query generation; a query that trims to empty
    /// goes idle without touching the port (mirrors `SearchMosques`).
    fn query_changed(&mut self) -> Vec<Command> {
        self.search_epoch = self.search_epoch.wrapping_add(1);
        if self.query.trim().is_empty() {
            self.search = SearchState::Idle;
            Vec::new()
        } else {
            let epoch = self.search_epoch;
            self.search = SearchState::Querying { epoch };
            vec![Command::Search { epoch, query: self.query.clone() }]
        }
    }

    fn move_cursor(&mut self, delta: isize) {
        if let SearchState::Results { items, cursor, .. } = &mut self.search {
            if items.is_empty() {
                return;
            }
            let len = items.len();
            let next = *cursor as isize + delta;
            *cursor = next.clamp(0, len as isize - 1) as usize;
        }
    }

    fn select_cursor(&mut self) -> Vec<Command> {
        let summary = match &self.search {
            SearchState::Results { items, cursor, .. } if !items.is_empty() => {
                items[*cursor].clone()
            }
            SearchState::SaveFailed { summary, .. } => summary.clone(),
            _ => return Vec::new(),
        };
        self.search = SearchState::Saving(summary.clone());
        vec![Command::SaveSelection(summary)]
    }

    /// Only the newest query owns the screen; stale responses are discarded.
    fn on_search_loaded(
        &mut self,
        epoch: u64,
        result: Result<Vec<MosqueSummary>, AppError>,
    ) -> Vec<Command> {
        if matches!(self.search, SearchState::Querying { epoch: current } if current == epoch)
        {
            self.search = match result {
                Ok(items) => SearchState::Results { epoch, items, cursor: 0 },
                Err(err) => SearchState::Failed(err.to_string()),
            };
        }
        Vec::new()
    }

    /// Persistence resolved: success records `MosqueSelected` and jumps to a
    /// fresh Today; failure keeps the selection for an Enter retry.
    fn on_selection_saved(&mut self, result: Result<(), AppError>) -> Vec<Command> {
        let SearchState::Saving(summary) = self.search.clone() else {
            return Vec::new();
        };
        match result {
            Ok(()) => {
                self.events.push(DomainEvent::MosqueSelected { id: summary.id.clone() });
                self.screen = Screen::Today;
                self.today = TodayState::Loading;
                vec![Command::LoadToday(summary.id)]
            }
            Err(err) => {
                self.search =
                    SearchState::SaveFailed { summary, reason: err.to_string() };
                Vec::new()
            }
        }
    }
}

/// Quit keys, single-sourced: bare `q`, or Ctrl-C. Unknown keys are inert.
/// On the Search screen only Ctrl-C applies (see `handle_search_key`).
pub const fn is_quit_key(key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('q') => key.modifiers.is_empty(),
        _ => is_force_quit_key(key),
    }
}

/// Ctrl-C quits on every screen — the one shared source for it.
pub const fn is_force_quit_key(key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('c') => key.modifiers.contains(KeyModifiers::CONTROL),
        _ => false,
    }
}

/// The key with no modifiers at all.
fn is_bare(key: KeyEvent, code: KeyCode) -> bool {
    key.modifiers.is_empty() && key.code == code
}
