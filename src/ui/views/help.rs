//! Help overlay (spec M6 R3): the single-sourced keybinding reference.
//! [`HELP_ENTRIES`] is the one table describing every binding; the ut tier
//! asserts the overlay renders it and that per-screen contexts show their
//! rows.

use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::Style,
    widgets::{Block, Cell, Paragraph, Row, Table},
};

use crate::ui::{app::AppModel, theme};

/// One row of the keybinding reference: (keys, context, meaning).
pub struct HelpEntry {
    pub keys: &'static str,
    pub context: &'static str,
    pub meaning: &'static str,
}

/// The single source of truth for every binding (CAMPAIGN.md §3: one
/// table, rendered here, asserted by tests).
pub const HELP_ENTRIES: &[HelpEntry] = &[
    HelpEntry {
        keys: "q / Ctrl-C",
        context: "everywhere",
        meaning: "quit (Search: Ctrl-C only — q types)",
    },
    HelpEntry { keys: "?", context: "everywhere", meaning: "toggle this help" },
    HelpEntry { keys: "s", context: "Today, Month", meaning: "open mosque search" },
    HelpEntry { keys: "m", context: "Today (ready)", meaning: "open the month view" },
    HelpEntry { keys: "r", context: "Today", meaning: "refresh now (manual)" },
    HelpEntry {
        keys: "auto",
        context: "Today (offline)",
        meaning: "auto-refresh every ~30 s",
    },
    HelpEntry {
        keys: "Left / Right",
        context: "Month",
        meaning: "previous / next month",
    },
    HelpEntry {
        keys: "Up / Down / j / k",
        context: "Month",
        meaning: "move the day cursor",
    },
    HelpEntry { keys: "Up / Down", context: "Search", meaning: "move the result cursor" },
    HelpEntry {
        keys: "Enter",
        context: "Search",
        meaning: "select the highlighted mosque",
    },
    HelpEntry {
        keys: "Backspace",
        context: "Search",
        meaning: "delete a query character",
    },
    HelpEntry { keys: "Esc", context: "Month, Search, help", meaning: "back / close" },
];

pub fn render(frame: &mut Frame, model: &AppModel) {
    let area = frame.area();
    let block = Block::bordered()
        .title("help — ? / Esc closes · Ctrl-C quits")
        .title_style(theme::title())
        .border_style(theme::border());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = HELP_ENTRIES.iter().map(|e| {
        Row::new(vec![Cell::from(e.keys), Cell::from(e.context), Cell::from(e.meaning)])
            .style(Style::new())
    });

    let header = Row::new(["Keys", "Where", "What it does"]).style(theme::title());
    let widths = [Constraint::Length(16), Constraint::Length(16), Constraint::Min(20)];

    let [table_area, note] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(inner);

    frame.render_widget(
        Table::new(rows, widths).header(header).block(Block::new()),
        table_area,
    );
    frame.render_widget(
        Paragraph::new(format!(
            "screen: {:?} · offline: {}",
            model.screen,
            if model.offline.is_some() { "yes" } else { "no" }
        ))
        .style(theme::muted()),
        note,
    );
}
