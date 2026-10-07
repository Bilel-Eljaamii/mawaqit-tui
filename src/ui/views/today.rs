//! Today screen (spec M3 R6): prayer grid, iqama-preferring next highlight,
//! live countdown + progress gauge.

use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::Style,
    widgets::{Block, Cell, Gauge, Paragraph, Row, Table},
};

use crate::{
    application::{
        clock::day_moment_from_utc,
        ports::{TodayReadout, TzSource},
    },
    domain::prayer::{ClockTime, DayMoment, NextPrayerAt, PrayerName, next_prayer_at},
    ui::{app::AppModel, theme},
};

pub fn render(frame: &mut Frame, model: &AppModel, readout: &TodayReadout) {
    let area = frame.area();
    let outer = Block::bordered()
        .title(outer_title(readout))
        .title_style(theme::title())
        .border_style(theme::border());
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let [info, grid, gauge_area, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(7),
        Constraint::Length(2),
        Constraint::Min(1),
    ])
    .areas(inner);

    frame.render_widget(info_line(readout), info);

    // TZ-TRUTH: the wall clock comes from the tick instant converted in the
    // mosque timezone; the selection follows the iqama-preferring set.
    let preferred = readout.times.preferred();
    let now = model.last_now().map(|instant| day_moment_from_utc(instant, readout.tz));
    let next = now.map(|moment| next_prayer_at(moment, &preferred));

    frame.render_widget(prayer_table(readout, now, next.as_ref().map(|n| n.name)), grid);
    if let Some(next) = next {
        frame.render_widget(countdown_gauge(next), gauge_area);
    }
    frame.render_widget(keys_hint(), footer);
}

fn outer_title(readout: &TodayReadout) -> String {
    match &readout.mosque_name {
        Some(name) => format!("mawaqit-tui — {name}"),
        None => "mawaqit-tui".to_owned(),
    }
}

fn info_line(readout: &TodayReadout) -> Paragraph<'static> {
    let tz_badge = match readout.tz {
        TzSource::Mosque(tz) => tz.to_string(),
        TzSource::Local => "local clock".to_owned(),
    };
    let mut line = format!("{} · {}", readout.date, tz_badge);
    if let Some(jumua) = readout.jumua {
        line.push_str(&format!(" · Jumua {jumua}"));
    }
    Paragraph::new(line).style(theme::text())
}

struct GridRow {
    label: String,
    adhan: Option<ClockTime>,
    iqama: Option<ClockTime>,
    /// `None` for the display-only Shurouq line.
    name: Option<PrayerName>,
}

fn prayer_table(
    readout: &TodayReadout,
    now: Option<DayMoment>,
    next: Option<PrayerName>,
) -> Table<'static> {
    let times = &readout.times;
    let mut rows: Vec<GridRow> = Vec::with_capacity(6);
    for name in PrayerName::ALL {
        rows.push(GridRow {
            label: name.slug().to_owned(),
            adhan: Some(times.adhan.get(name)),
            iqama: times.iqama.map(|set| set.get(name)),
            name: Some(name),
        });
    }
    // Shurouq sits between Fajr and Dhuhr, display-only.
    rows.insert(
        1,
        GridRow {
            label: "shurouq".to_owned(),
            adhan: times.shurouq,
            iqama: None,
            name: None,
        },
    );

    let body = rows.into_iter().map(|row| {
        let style = row_style(&row, now, next);
        Row::new(vec![
            Cell::from(row.label),
            Cell::from(fmt_opt(row.adhan)),
            Cell::from(fmt_opt(row.iqama)),
        ])
        .style(style)
    });

    Table::new(
        body,
        [Constraint::Length(10), Constraint::Length(8), Constraint::Length(8)],
    )
    .header(Row::new(["Prayer", "Adhan", "Iqama"]).style(theme::title()))
}

fn row_style(row: &GridRow, now: Option<DayMoment>, next: Option<PrayerName>) -> Style {
    if row.name.is_some() && row.name == next {
        return theme::next_highlight();
    }
    // The effective time is the iqama when present, else the adhan; shurouq
    // dims by its own time. Anything at-or-before `now` has passed.
    let effective = row.iqama.or(row.adhan);
    let passed = now
        .map(|moment| moment.seconds())
        .zip(effective)
        .is_some_and(|(now_s, at)| at.seconds() <= now_s);
    if passed { theme::passed() } else { theme::text() }
}

fn fmt_opt(time: Option<ClockTime>) -> String {
    time.map_or("—".to_owned(), |t| t.to_string())
}

fn countdown_gauge(next: NextPrayerAt) -> Gauge<'static> {
    let ratio = if next.interval_seconds > 0 {
        (f64::from(next.seconds_into_previous) / f64::from(next.interval_seconds))
            .clamp(0.0, 1.0)
    } else {
        0.0
    };
    Gauge::default()
        .ratio(ratio)
        .label(format!("{} in {}", next.name.slug(), format_hms(next.seconds_remaining)))
        .gauge_style(theme::gauge())
}

fn format_hms(total: u32) -> String {
    format!("{:02}:{:02}:{:02}", total / 3600, (total % 3600) / 60, total % 60)
}

fn keys_hint() -> Paragraph<'static> {
    Paragraph::new("q · quit    s · search    Ctrl-C · quit").style(theme::muted())
}
