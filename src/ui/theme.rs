//! Style constants — single source for colors; the M6 help screen consumes
//! the keybinding table.

use ratatui::style::{Color, Style};

pub fn title() -> Style {
    Style::new().fg(Color::Cyan).bold()
}

pub fn text() -> Style {
    Style::new()
}

pub fn border() -> Style {
    Style::new().fg(Color::Gray)
}

/// The next prayer's row: high-contrast filled highlight.
pub fn next_highlight() -> Style {
    Style::new().fg(Color::Black).bg(Color::Cyan).bold()
}

/// Rows whose time has already passed today.
pub fn passed() -> Style {
    Style::new().fg(Color::DarkGray)
}

/// Low-emphasis chrome (footer hints, secondary text).
pub fn muted() -> Style {
    Style::new().fg(Color::DarkGray)
}

pub fn error() -> Style {
    Style::new().fg(Color::Red).bold()
}

pub fn gauge() -> Style {
    Style::new().fg(Color::Green)
}

pub fn accent() -> Style {
    Style::new().fg(Color::Yellow)
}
