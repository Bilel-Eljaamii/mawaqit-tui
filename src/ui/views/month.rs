//! Month calendar (spec M5 R5): day rows × adhan(+iqama) columns, dropped
//! days explicit, cursor + today-row highlights.

use chrono::Datelike;
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::Style,
    widgets::{Block, Cell, Paragraph, Row, Table},
};

use crate::{
    application::{
        clock::date_in_utc,
        ports::{MonthDay, MonthReadout},
    },
    domain::prayer::{ClockTime, PrayerName},
    ui::{
        app::{AppModel, MonthState},
        theme,
    },
};

const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

const ADHAN_HEADERS: [&str; 6] = ["fajr", "shur", "dhuhr", "asr", "magh", "isha"];
/// `i-` marks iqama; the info line explains it. 6-wide cells keep an
/// 80-terminal inside its border (3 + 11×6 = 69).
const IQAMA_HEADERS: [&str; 5] = ["i-fjr", "i-dhu", "i-asr", "i-mag", "i-ish"];

pub fn render(frame: &mut Frame, model: &AppModel) {
    let MonthState::Ready { month, readout, cursor } = &model.month else {
        return;
    };
    let area = frame.area();
    // `month` is model-side and always 1..=12; the lookup guard keeps a
    // broken invariant from becoming a panic (HOSTILE-INPUT-TOTAL spirit).
    let month_name =
        MONTH_NAMES.get(month.wrapping_sub(1) as usize).copied().unwrap_or("?");
    let outer = Block::bordered()
        .title(format!("mawaqit-tui — {month_name}"))
        .title_style(theme::title())
        .border_style(theme::border());
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let [info, table_area, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .areas(inner);

    frame.render_widget(info_line(readout), info);
    frame.render_widget(
        month_table(readout, *cursor, today_day(model, readout)),
        table_area,
    );
    frame.render_widget(keys_hint(), footer);
}

fn info_line(readout: &MonthReadout) -> Paragraph<'static> {
    let mut line = format!("{} days", readout.days.len());
    if readout.days.iter().any(|day| day.iqama.is_some()) {
        line.push_str(" · i- prefix = iqama");
    }
    if !readout.dropped.is_empty() {
        // Explicit, not hidden (HONESTY; issue #5 acceptance).
        let list = readout
            .dropped
            .iter()
            .map(|day| day.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        line.push_str(&format!(" · dropped: {list}"));
    }
    Paragraph::new(line).style(theme::text())
}

/// Today's day-of-month when the viewed month is the mosque-tz current month.
fn today_day(model: &AppModel, readout: &MonthReadout) -> Option<u32> {
    model
        .last_now()
        .map(|now| date_in_utc(now, readout.tz))
        .filter(|date| date.month() == readout.month)
        .map(|date| date.day())
}

fn month_table(
    readout: &MonthReadout,
    cursor: usize,
    today: Option<u32>,
) -> Table<'static> {
    let has_iqama = readout.days.iter().any(|day| day.iqama.is_some());
    let cursor_day = readout.days.get(cursor).map(|day| day.day);
    let time_columns = if has_iqama { 11 } else { 6 };

    let mut headers = vec!["day"];
    headers.extend(ADHAN_HEADERS);
    if has_iqama {
        headers.extend(IQAMA_HEADERS);
    }
    let mut widths = vec![Constraint::Length(3)];
    widths.extend(std::iter::repeat_n(Constraint::Length(6), time_columns));

    let last_published = readout.days.iter().map(|day| day.day).max().unwrap_or(0);
    let last_dropped = readout.dropped.iter().copied().max().unwrap_or(0);

    let mut rows = Vec::new();
    for day in 1..=last_published.max(last_dropped) {
        match readout.days.iter().find(|row| row.day == day) {
            Some(row) => {
                let style = row_style(Some(day), cursor_day, today);
                rows.push(present_row(row, has_iqama).style(style));
            }
            // Declared dropped: a dimmed placeholder row (explicit, subtle).
            None if readout.dropped.contains(&day) => {
                let cells = std::iter::once(Cell::from(day.to_string()))
                    .chain(std::iter::repeat_n(Cell::from("·"), time_columns))
                    .collect::<Vec<_>>();
                rows.push(Row::new(cells).style(theme::passed()));
            }
            // Neither published nor declared: not ours to invent.
            None => {}
        }
    }

    Table::new(rows, widths).header(
        Row::new(headers.iter().map(|header| Cell::from(*header))).style(theme::title()),
    )
}

fn present_row(row: &MonthDay, has_iqama: bool) -> Row<'static> {
    let mut cells = vec![
        Cell::from(row.day.to_string()),
        Cell::from(row.adhan.get(PrayerName::Fajr).to_string()),
        Cell::from(fmt_opt(row.shurouq)),
        Cell::from(row.adhan.get(PrayerName::Dhuhr).to_string()),
        Cell::from(row.adhan.get(PrayerName::Asr).to_string()),
        Cell::from(row.adhan.get(PrayerName::Maghrib).to_string()),
        Cell::from(row.adhan.get(PrayerName::Isha).to_string()),
    ];
    if has_iqama {
        match &row.iqama {
            Some(iqama) => {
                for name in PrayerName::ALL {
                    cells.push(Cell::from(iqama.get(name).to_string()));
                }
            }
            None => cells.extend(std::iter::repeat_n(Cell::from("—"), 5)),
        }
    }
    Row::new(cells)
}

fn row_style(day: Option<u32>, cursor_day: Option<u32>, today: Option<u32>) -> Style {
    if day == cursor_day {
        theme::next_highlight()
    } else if day == today {
        theme::accent()
    } else {
        theme::text()
    }
}

fn fmt_opt(time: Option<ClockTime>) -> String {
    time.map_or("—".to_owned(), |t| t.to_string())
}

fn keys_hint() -> Paragraph<'static> {
    Paragraph::new("←/→ · month    ↑/↓ · day    s · search    Esc · today    q · quit")
        .style(theme::muted())
}
