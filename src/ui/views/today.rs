//! Today screen placeholder (spec M1 R6). Real adhan/iqama data, countdown,
//! and progress land in M3 (issue #3).

use ratatui::{
    Frame,
    widgets::{Block, Paragraph},
};

use crate::ui::{app::AppModel, theme};

pub fn render(frame: &mut Frame, model: &AppModel) {
    let body = format!(
        "mawaqit-tui — M1 scaffold\n\n\
         placeholder Today screen (adhan/iqama data lands in M3)\n\n\
         ticks: {}\n\n\
         q · quit     Ctrl-C · quit",
        model.ticks(),
    );
    let block = Block::bordered()
        .title("mawaqit-tui")
        .title_style(theme::title())
        .border_style(theme::border());
    frame.render_widget(
        Paragraph::new(body).style(theme::text()).block(block),
        frame.area(),
    );
}
