//! Screen dispatch (spec M3 R6, spec M4 R1/R5, spec M5 R4/R5).

use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    widgets::{Block, Paragraph},
};

use crate::ui::{
    app::{AppModel, MonthState, Screen, TodayState},
    theme, views,
};

pub fn draw(frame: &mut Frame, model: &AppModel) {
    if model.show_help {
        views::help::render(frame, model);
        return;
    }
    match model.screen {
        Screen::Today => match &model.today {
            TodayState::Ready(readout) => {
                views::today::render(frame, model, readout);
                if model.offline.is_some() {
                    draw_offline_badge(frame);
                }
            }
            TodayState::NoMosque => {
                message(frame, "no mosque selected", "press s to search")
            }
            TodayState::Loading => skeleton(frame, "loading today's times…", 6),
            TodayState::Failed(reason) => {
                message(frame, "could not load today's times", reason)
            }
        },
        Screen::Search => views::search::render(frame, model),
        Screen::Month => match &model.month {
            MonthState::Loading { .. } => skeleton(frame, "loading this month…", 12),
            MonthState::Ready { .. } => views::month::render(frame, model),
            MonthState::Failed { reason, .. } => {
                message(frame, "could not load this month", reason)
            }
        },
    }
}

/// The OFFLINE badge: top-right corner, error style — HONESTY (the data on
/// screen is last-good, and the badge says so).
fn draw_offline_badge(frame: &mut Frame) {
    let area = frame.area();
    let badge = Paragraph::new(" OFFLINE ").style(theme::error());
    let width = 9.min(area.width);
    let rect = ratatui::layout::Rect {
        x: area.width.saturating_sub(width),
        y: area.y,
        width,
        height: 1,
    };
    frame.render_widget(badge, rect);
}

/// A loading skeleton: the title plus dimmed placeholder rows — loading is
/// visibly *loading*, not a blank or finished screen (HONESTY).
fn skeleton(frame: &mut Frame, title: &str, rows: usize) {
    let block = Block::bordered()
        .title(format!("mawaqit-tui — {title}"))
        .title_style(theme::title())
        .border_style(theme::border());
    let inner = block.inner(frame.area());
    frame.render_widget(block, frame.area());

    let row = Paragraph::new("· · · · · · · · · ·").style(theme::muted());
    let heights: Vec<Constraint> = (0..rows)
        .map(|_| Constraint::Length(1))
        .chain(std::iter::once(Constraint::Min(1)))
        .collect();
    for area in Layout::vertical(heights).split(inner).iter() {
        frame.render_widget(row.clone(), *area);
    }
}

fn message(frame: &mut Frame, title: &str, detail: &str) {
    let mut body = title.to_owned();
    if !detail.is_empty() {
        body.push_str("\n\n");
        body.push_str(detail);
    }
    let block = Block::bordered()
        .title("mawaqit-tui")
        .title_style(theme::title())
        .border_style(theme::border());
    frame.render_widget(
        Paragraph::new(body).style(theme::text()).block(block),
        frame.area(),
    );
}
