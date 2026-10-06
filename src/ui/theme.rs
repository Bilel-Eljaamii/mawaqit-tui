//! Style constants — single source for colors; M3 expands and the M6 help
//! screen consumes the keybinding table.

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
