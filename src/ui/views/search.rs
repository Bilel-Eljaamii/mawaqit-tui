//! Search screen (spec M4 R5): query line, status, result list, keys hint.

use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::Style,
    widgets::{Block, List, ListItem, Paragraph},
};

use crate::{
    domain::mosque::MosqueSummary,
    ui::{
        app::{AppModel, SearchState},
        theme,
    },
};

pub fn render(frame: &mut Frame, model: &AppModel) {
    let area = frame.area();
    let outer = Block::bordered()
        .title("mawaqit-tui — search")
        .title_style(theme::title())
        .border_style(theme::border());
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let [query, status, list, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .areas(inner);

    frame.render_widget(query_line(model), query);
    frame.render_widget(status_line(&model.search), status);
    frame.render_widget(result_list(&model.search), list);
    frame.render_widget(keys_hint(&model.search), footer);
}

fn query_line(model: &AppModel) -> Paragraph<'static> {
    Paragraph::new(format!("> {}", model.query)).style(theme::accent())
}

fn status_line(state: &SearchState) -> Paragraph<'static> {
    let (text, style): (String, Style) = match state {
        SearchState::Idle => ("type to search".to_owned(), theme::muted()),
        SearchState::Querying { .. } => ("searching…".to_owned(), theme::muted()),
        SearchState::Results { items, .. } if items.is_empty() => {
            ("no mosques found".to_owned(), theme::muted())
        }
        SearchState::Results { items, .. } => {
            (format!("{} found", items.len()), theme::text())
        }
        SearchState::Saving(_) => ("saving selection…".to_owned(), theme::muted()),
        // Errors are shown verbatim (HONESTY).
        SearchState::SaveFailed { reason, .. } => (reason.clone(), theme::error()),
        SearchState::Failed(reason) => (reason.clone(), theme::error()),
    };
    Paragraph::new(text).style(style)
}

fn result_list(state: &SearchState) -> List<'static> {
    let SearchState::Results { items, cursor, .. } = state else {
        return List::new(Vec::<ListItem>::new());
    };
    let rows = items
        .iter()
        .enumerate()
        .map(|(index, summary)| {
            ListItem::new(row_text(summary)).style(row_style(index == *cursor))
        })
        .collect::<Vec<_>>();
    List::new(rows)
}

fn row_text(summary: &MosqueSummary) -> String {
    match &summary.place {
        Some(place) => format!("{} — {place}", summary.name),
        None => summary.name.clone(),
    }
}

fn row_style(selected: bool) -> Style {
    if selected { theme::next_highlight() } else { theme::text() }
}

fn keys_hint(state: &SearchState) -> Paragraph<'static> {
    let enter = match state {
        SearchState::SaveFailed { .. } => "Enter · retry",
        _ => "Enter · select",
    };
    Paragraph::new(format!(
        "type · query    ↑/↓ · navigate    {enter}    Esc · back    Ctrl-C · quit"
    ))
    .style(theme::muted())
}
