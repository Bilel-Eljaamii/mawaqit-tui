//! Screen dispatch. M1 has one screen; M3 adds tabs over the same entry.

use ratatui::Frame;

use crate::ui::{app::AppModel, views};

pub fn draw(frame: &mut Frame, model: &AppModel) {
    views::today::render(frame, model);
}
