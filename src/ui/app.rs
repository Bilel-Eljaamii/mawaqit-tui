//! AppModel — pure state machine (ADR-0001 §2). Headless-testable: transitions
//! depend only on the events fed in, never on a terminal or the clock.

use crossterm::event::KeyEvent;

use crate::runtime::{AppEvent, is_quit_key};

/// Application state. Fields grow in M3+ (screen enum, times, errors); the
/// shape of `update` does not.
pub struct AppModel {
    pub should_quit: bool,
    ticks: u64,
}

impl AppModel {
    pub fn new() -> AppModel {
        AppModel { should_quit: false, ticks: 0 }
    }

    /// Ticks elapsed since start; display-only, wrapping at u64 bounds.
    pub const fn ticks(&self) -> u64 {
        self.ticks
    }

    /// Transition on one event. Total; unknown keys are inert (ADR-0001 §4).
    pub fn update(&mut self, event: AppEvent) {
        match event {
            AppEvent::Key(key) => self.handle_key(key),
            AppEvent::Tick => self.ticks = self.ticks.wrapping_add(1),
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if is_quit_key(key) {
            self.should_quit = true;
        }
    }
}

impl Default for AppModel {
    fn default() -> AppModel {
        AppModel::new()
    }
}
