//! Screen dispatch on the today lifecycle (spec M3 R6).

use ratatui::{
    Frame,
    widgets::{Block, Paragraph},
};

use crate::ui::{
    app::{AppModel, TodayState},
    theme, views,
};

pub fn draw(frame: &mut Frame, model: &AppModel) {
    match &model.state {
        TodayState::Ready(readout) => views::today::render(frame, model, readout),
        TodayState::NoMosque => message(
            frame,
            "no mosque configured",
            "run `mawaqit-tui --mosque <slug>` — interactive search lands in M4",
        ),
        TodayState::Loading => message(frame, "loading today's times…", ""),
        TodayState::Failed(reason) => {
            message(frame, "could not load today's times", reason)
        }
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
