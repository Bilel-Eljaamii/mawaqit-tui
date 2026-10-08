//! Unit tests — M1 domain core.
//!
//! Ephemeral example tests are tagged `// @tier ephemeral`; the durable
//! contract lives in `tst/fuzz.rs` (ADR-113 durability tiers, CAMPAIGN.md
//! Phase B). RED witnessed before GREEN: compile-fail on the empty module
//! stubs, 2026-10-06.

use chrono::{TimeZone, Utc};
use mawaqit_tui::domain::{
    mosque::{InvalidSlug, MosqueId},
    prayer::{
        ClockParseError, ClockTime, DailyTimes, PrayerName, PrayerSet, next_prayer,
    },
};

fn clock(s: &str) -> ClockTime {
    ClockTime::parse_hhmm(s).unwrap()
}

fn set(times: [&str; 5]) -> PrayerSet {
    let [f, d, a, m, i] = times;
    PrayerSet::new(clock(f), clock(d), clock(a), clock(m), clock(i)).unwrap()
}

fn reference_set() -> PrayerSet {
    set(["05:00", "12:30", "15:45", "18:20", "20:00"])
}

fn fixed_now() -> chrono::DateTime<chrono::Utc> {
    Utc.with_ymd_and_hms(2026, 10, 6, 12, 0, 0).unwrap()
}

fn readout() -> mawaqit_tui::application::ports::TodayReadout {
    use mawaqit_tui::application::ports::{TodayReadout, TzSource};
    let adhan = PrayerSet::new(
        clock("05:00"),
        clock("12:30"),
        clock("15:45"),
        clock("18:20"),
        clock("20:00"),
    )
    .unwrap();
    let iqama = PrayerSet::new(
        clock("05:20"),
        clock("12:45"),
        clock("16:00"),
        clock("18:35"),
        clock("20:15"),
    )
    .unwrap();
    TodayReadout {
        date: chrono::NaiveDate::from_ymd_opt(2026, 10, 6).unwrap(),
        times: DailyTimes { adhan, iqama: Some(iqama), shurouq: Some(clock("06:40")) },
        tz: TzSource::Mosque(chrono_tz::Tz::Europe__Paris),
        mosque_name: Some("Grande Mosquée de Paris".to_owned()),
        jumua: Some(clock("12:45")),
    }
}

// ---- R1: ClockTime::parse_hhmm --------------------------------------------

#[test]
fn parses_canonical_hhmm() {
    // @tier ephemeral
    let t = clock("05:12");
    assert_eq!((t.hour(), t.minute()), (5, 12));
    assert_eq!(t.to_string(), "05:12");
}

#[test]
fn parses_day_boundaries() {
    // @tier ephemeral
    assert_eq!(clock("00:00"), ClockTime::from_hms(0, 0).unwrap());
    assert_eq!(clock("23:59"), ClockTime::from_hms(23, 59).unwrap());
}

#[test]
fn rejects_unpadded_hour() {
    // @tier ephemeral
    assert_eq!(ClockTime::parse_hhmm("7:05"), Err(ClockParseError::Malformed));
}

#[test]
fn rejects_hour_out_of_range() {
    // @tier ephemeral
    assert_eq!(ClockTime::parse_hhmm("24:00"), Err(ClockParseError::HourRange));
}

#[test]
fn rejects_minute_out_of_range() {
    // @tier ephemeral
    assert_eq!(ClockTime::parse_hhmm("12:99"), Err(ClockParseError::MinuteRange));
    assert_eq!(ClockTime::parse_hhmm("05:60"), Err(ClockParseError::MinuteRange));
}

#[test]
fn rejects_wrong_length_and_separator() {
    // @tier ephemeral
    assert_eq!(ClockTime::parse_hhmm(""), Err(ClockParseError::Malformed));
    assert_eq!(ClockTime::parse_hhmm("2:00"), Err(ClockParseError::Malformed));
    assert_eq!(ClockTime::parse_hhmm("12:0"), Err(ClockParseError::Malformed));
    assert_eq!(ClockTime::parse_hhmm("12-00"), Err(ClockParseError::Malformed));
}

#[test]
fn rejects_non_ascii_digits() {
    // @tier ephemeral
    // Byte length 5 with `:` at the right index — still not ASCII digits.
    assert_eq!(ClockTime::parse_hhmm("٢:00"), Err(ClockParseError::NotAsciiDigits));
    assert_eq!(ClockTime::parse_hhmm("12:٢"), Err(ClockParseError::NotAsciiDigits));
    // Multi-byte digits shift the byte length.
    assert_eq!(ClockTime::parse_hhmm("1٢:00"), Err(ClockParseError::Malformed));
    assert_eq!(ClockTime::parse_hhmm("٢٤:٠٠"), Err(ClockParseError::Malformed));
}

// ---- R2: PrayerName ordering ----------------------------------------------

#[test]
fn prayer_names_are_ordered_by_time_of_day() {
    // @tier ephemeral
    let mut names = [
        PrayerName::Isha,
        PrayerName::Fajr,
        PrayerName::Asr,
        PrayerName::Dhuhr,
        PrayerName::Maghrib,
    ];
    names.sort();
    assert_eq!(names, PrayerName::ALL);
}

// ---- R3: PrayerSet / DailyTimes -------------------------------------------

#[test]
fn prayer_set_allows_equal_adjacent_times() {
    // @tier ephemeral
    let _ = set(["05:00", "05:00", "12:00", "12:00", "20:00"]);
}

#[test]
fn prayer_set_rejects_decreasing_times() {
    // @tier ephemeral
    let decreasing = PrayerSet::new(
        clock("12:00"),
        clock("05:00"),
        clock("15:00"),
        clock("18:00"),
        clock("20:00"),
    );
    assert!(decreasing.is_err());
}

#[test]
fn daily_times_keeps_iqama_optional() {
    // @tier ephemeral
    let with_iqama = DailyTimes {
        adhan: reference_set(),
        iqama: Some(set(["05:15", "12:45", "16:00", "18:35", "20:15"])),
        shurouq: Some(clock("06:40")),
    };
    assert!(with_iqama.iqama.is_some());
    assert!(with_iqama.shurouq.is_some());

    let adhan_only = DailyTimes { adhan: reference_set(), iqama: None, shurouq: None };
    assert!(adhan_only.iqama.is_none());
}

// ---- R4: next_prayer rollover ---------------------------------------------

#[test]
fn before_fajr_next_is_todays_fajr() {
    // @tier ephemeral
    let next = next_prayer(clock("04:59"), &reference_set());
    assert_eq!(next.name, PrayerName::Fajr);
    assert_eq!(next.at, clock("05:00"));
    assert_eq!(next.previous, PrayerName::Isha);
    assert_eq!(next.minutes_remaining, 1);
    assert!(!next.is_tomorrow);
}

#[test]
fn at_prayer_time_that_prayer_is_current() {
    // @tier ephemeral
    let next = next_prayer(clock("05:00"), &reference_set());
    assert_eq!(next.name, PrayerName::Dhuhr);
    assert_eq!(next.previous, PrayerName::Fajr);
    assert_eq!(next.minutes_remaining, 450);
    assert!(!next.is_tomorrow);
}

#[test]
fn one_minute_before_dhuhr() {
    // @tier ephemeral
    let next = next_prayer(clock("12:29"), &reference_set());
    assert_eq!(next.name, PrayerName::Dhuhr);
    assert_eq!(next.previous, PrayerName::Fajr);
    assert_eq!(next.minutes_remaining, 1);
}

#[test]
fn evening_next_is_isha() {
    // @tier ephemeral
    let next = next_prayer(clock("19:00"), &reference_set());
    assert_eq!(next.name, PrayerName::Isha);
    assert_eq!(next.previous, PrayerName::Maghrib);
    assert_eq!(next.minutes_remaining, 60);
}

#[test]
fn at_isha_rolls_to_next_day_fajr() {
    // @tier ephemeral
    let next = next_prayer(clock("20:00"), &reference_set());
    assert_eq!(next.name, PrayerName::Fajr);
    assert_eq!(next.previous, PrayerName::Isha);
    assert_eq!(next.minutes_remaining, 540);
    assert!(next.is_tomorrow);
}

#[test]
fn late_night_rolls_with_correct_remaining() {
    // @tier ephemeral
    let next = next_prayer(clock("23:59"), &reference_set());
    assert_eq!(next.name, PrayerName::Fajr);
    assert_eq!(next.previous, PrayerName::Isha);
    assert_eq!(next.minutes_remaining, 301);
    assert!(next.is_tomorrow);
}

#[test]
fn midnight_window_belongs_to_coming_fajr() {
    // @tier ephemeral
    let next = next_prayer(clock("00:05"), &reference_set());
    assert_eq!(next.name, PrayerName::Fajr);
    assert_eq!(next.previous, PrayerName::Isha);
    assert_eq!(next.minutes_remaining, 295);
    assert!(!next.is_tomorrow);
}

// ---- R5: MosqueId ----------------------------------------------------------

#[test]
fn accepts_plain_slugs() {
    // @tier ephemeral
    let id = MosqueId::parse("grand-mosque-paris_2.0").unwrap();
    assert_eq!(id.as_str(), "grand-mosque-paris_2.0");
}

#[test]
fn rejects_hostile_slugs() {
    // @tier ephemeral
    let hostile = [
        "",
        "../etc/passwd",
        "a?b",
        "a#b",
        "a b",
        "a/b",
        ".lead",
        "trail.",
        "a..b",
        "café",
    ];
    for bad in hostile {
        assert_eq!(
            MosqueId::parse(bad),
            Err(InvalidSlug),
            "slug {bad:?} must be rejected"
        );
    }
}

#[test]
fn rejects_overlong_slug() {
    // @tier ephemeral
    let long = "a".repeat(MosqueId::MAX_LEN + 1);
    assert_eq!(MosqueId::parse(&long), Err(InvalidSlug));
}

// ---- R6: AppModel transitions (headless) -----------------------------------

mod app_model {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use mawaqit_tui::{
        application::use_cases::AppError,
        domain::mosque::MosqueId,
        ui::app::{AppEvent, AppModel, Boot, Command, Screen, SearchState, TodayState},
    };

    use super::{fixed_now, readout};

    fn key(code: KeyCode, mods: KeyModifiers) -> AppEvent {
        AppEvent::Key(KeyEvent::new(code, mods))
    }

    #[test]
    fn boot_loading_queues_load_today() {
        // @tier ephemeral
        let id = MosqueId::parse("test-mosque").unwrap();
        let (model, commands) = AppModel::from_boot(Boot::Loading(id.clone()));
        assert_eq!(model.screen, Screen::Today);
        assert_eq!(model.today, TodayState::Loading);
        assert_eq!(commands, vec![Command::LoadToday(id)]);
    }

    /// No saved mosque boots straight onto the search screen (spec M4 R1).
    #[test]
    fn boot_no_mosque_lands_on_search_idle() {
        // @tier ephemeral
        let (model, commands) = AppModel::from_boot(Boot::NoMosque);
        assert_eq!(model.screen, Screen::Search);
        assert_eq!(model.search, SearchState::Idle);
        assert_eq!(model.query, "");
        assert!(commands.is_empty());
    }

    #[test]
    fn boot_failed_lands_on_today_with_message() {
        // @tier ephemeral
        let (model, commands) =
            AppModel::from_boot(Boot::Failed("config corrupt".into()));
        assert_eq!(model.screen, Screen::Today);
        assert_eq!(model.today, TodayState::Failed("config corrupt".into()));
        assert!(commands.is_empty());
    }

    #[test]
    fn today_loaded_ok_transitions_to_ready() {
        // @tier ephemeral
        let (mut model, _) =
            AppModel::from_boot(Boot::Loading(MosqueId::parse("test-mosque").unwrap()));
        model.update(AppEvent::TodayLoaded(Ok(readout())));
        assert_eq!(model.today, TodayState::Ready(readout()));
    }

    #[test]
    fn today_loaded_err_transitions_to_failed() {
        // @tier ephemeral
        let (mut model, _) =
            AppModel::from_boot(Boot::Loading(MosqueId::parse("test-mosque").unwrap()));
        model.update(AppEvent::TodayLoaded(Err(AppError::Port(
            mawaqit_tui::application::ports::PortError::Network("boom".into()),
        ))));
        assert_eq!(model.today, TodayState::Failed("network failure: boom".into()));
    }

    #[test]
    fn tick_records_now_for_the_view() {
        // @tier ephemeral
        let (mut model, _) = AppModel::from_boot(Boot::NoMosque);
        assert_eq!(model.last_now(), None);
        model.update(AppEvent::Tick(fixed_now()));
        assert_eq!(model.last_now(), Some(fixed_now()));
    }

    #[test]
    fn quit_still_works_when_ready() {
        // @tier ephemeral
        let (mut model, _) =
            AppModel::from_boot(Boot::Loading(MosqueId::parse("test-mosque").unwrap()));
        model.update(AppEvent::TodayLoaded(Ok(readout())));
        model.update(key(KeyCode::Char('q'), KeyModifiers::NONE));
        assert!(model.should_quit);
    }

    /// `q` quits on the Today screen; on Search it is a literal (spec M4 R2),
    /// covered by `search_screen::bare_q_types_on_search_screen`.
    #[test]
    fn q_quits_on_today() {
        // @tier ephemeral
        let (mut model, _) =
            AppModel::from_boot(Boot::Loading(MosqueId::parse("test-mosque").unwrap()));
        model.update(key(KeyCode::Char('q'), KeyModifiers::NONE));
        assert!(model.should_quit);
    }

    /// Ctrl-C quits even on the search screen, where `q` types.
    #[test]
    fn ctrl_c_quits_on_search() {
        // @tier ephemeral
        let (mut model, _) = AppModel::from_boot(Boot::NoMosque);
        model.update(key(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(model.should_quit);
    }

    #[test]
    fn unknown_keys_are_inert_on_today() {
        // @tier ephemeral
        let (mut model, _) =
            AppModel::from_boot(Boot::Loading(MosqueId::parse("test-mosque").unwrap()));
        model.update(key(KeyCode::Char('x'), KeyModifiers::NONE));
        model.update(key(KeyCode::Esc, KeyModifiers::NONE));
        // Plain `c` without CONTROL must NOT quit.
        model.update(key(KeyCode::Char('c'), KeyModifiers::NONE));
        assert!(!model.should_quit);
        assert_eq!(model.screen, Screen::Today);
    }

    #[test]
    fn shifted_and_control_q_are_inert_on_today() {
        // @tier ephemeral
        let (mut model, _) =
            AppModel::from_boot(Boot::Loading(MosqueId::parse("test-mosque").unwrap()));
        model.update(key(KeyCode::Char('Q'), KeyModifiers::SHIFT));
        model.update(key(KeyCode::Char('q'), KeyModifiers::CONTROL));
        assert!(!model.should_quit);
    }

    #[test]
    fn ticks_advance() {
        // @tier ephemeral
        let (mut model, _) = AppModel::from_boot(Boot::NoMosque);
        for _ in 0..3 {
            model.update(AppEvent::Tick(fixed_now()));
        }
        assert_eq!(model.ticks(), 3);
    }
}

mod quit_key_table {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use mawaqit_tui::ui::app::is_quit_key;

    #[test]
    fn exactly_bare_q_and_ctrl_c_quit() {
        // @tier ephemeral
        let quit = [
            (KeyCode::Char('q'), KeyModifiers::NONE),
            (KeyCode::Char('c'), KeyModifiers::CONTROL),
        ];
        let inert = [
            (KeyCode::Char('Q'), KeyModifiers::SHIFT),
            (KeyCode::Char('q'), KeyModifiers::CONTROL),
            (KeyCode::Char('c'), KeyModifiers::SHIFT),
            (KeyCode::Char('c'), KeyModifiers::NONE),
            (KeyCode::Char('x'), KeyModifiers::NONE),
            (KeyCode::Esc, KeyModifiers::NONE),
        ];
        for (code, mods) in quit {
            assert!(
                is_quit_key(KeyEvent::new(code, mods)),
                "{code:?}+{mods:?} must quit"
            );
        }
        for (code, mods) in inert {
            assert!(
                !is_quit_key(KeyEvent::new(code, mods)),
                "{code:?}+{mods:?} must be inert"
            );
        }
    }
}

mod placeholder_render {
    use mawaqit_tui::ui::app::{AppEvent, AppModel, Boot};

    use super::readout;

    /// Headless render through ratatui's TestBackend: the Ready Today screen
    /// draws without panic and carries its identity lines. (spec M1 R6 / M3 R6)
    #[test]
    fn today_screen_renders_headless() {
        // @tier ephemeral
        let backend = ratatui::backend::TestBackend::new(64, 18);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let (mut model, _) = AppModel::from_boot(Boot::Loading(
            mawaqit_tui::domain::mosque::MosqueId::parse("test-mosque").unwrap(),
        ));
        model.update(AppEvent::TodayLoaded(Ok(readout())));
        model.update(AppEvent::Tick(super::fixed_now()));
        terminal.draw(|frame| mawaqit_tui::ui::draw::draw(frame, &model)).unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(text.contains("mawaqit-tui"), "title missing: {text:?}");
        assert!(text.contains("Grande Mosquée"), "mosque name missing: {text:?}");
        assert!(text.contains("isha"), "prayer row missing: {text:?}");
        assert!(text.contains("in 02:00:00"), "countdown missing: {text:?}");
        assert!(text.contains("s · search"), "search hint missing: {text:?}");
    }

    /// Boot without a mosque lands on the search screen (spec M4 R1/R5): the
    /// query prompt and idle hint render instead of the old NoMosque notice.
    #[test]
    fn search_idle_screen_renders_headless() {
        // @tier ephemeral
        let backend = ratatui::backend::TestBackend::new(64, 12);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let (model, _) = AppModel::from_boot(Boot::NoMosque);
        terminal.draw(|frame| mawaqit_tui::ui::draw::draw(frame, &model)).unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(text.contains('>'), "query prompt missing: {text:?}");
        assert!(text.contains("type to search"), "idle hint missing: {text:?}");
    }
}

// ---- M3 R1: DayMoment + second-resolution selection -------------------------

mod day_moment {
    use mawaqit_tui::domain::prayer::DayMoment;

    #[test]
    fn constructors_valid_and_invalid() {
        // @tier ephemeral
        assert_eq!(DayMoment::new(0), DayMoment::from_hms(0, 0, 0));
        assert_eq!(DayMoment::new(86_399), DayMoment::from_hms(23, 59, 59));
        assert_eq!(DayMoment::new(86_400), None);
        assert_eq!(DayMoment::from_hms(24, 0, 0), None);
        assert_eq!(DayMoment::from_hms(0, 0, 60), None);
    }

    #[test]
    fn seconds_and_from_naive_time() {
        // @tier ephemeral
        let moment = DayMoment::from_hms(12, 30, 15).unwrap();
        assert_eq!(moment.seconds(), 45_015);
        let naive = chrono::NaiveTime::from_hms_opt(12, 30, 15).unwrap();
        assert_eq!(DayMoment::from_naive_time(naive), DayMoment::new(45_015));
        // Sub-second nanos are truncated, not rounded.
        let with_nanos = naive + chrono::TimeDelta::nanoseconds(999_999_999);
        assert_eq!(DayMoment::from_naive_time(with_nanos), DayMoment::new(45_015));
    }
}

mod next_prayer_at_seconds {
    use mawaqit_tui::domain::prayer::{DayMoment, PrayerName, next_prayer_at};

    use super::reference_set;

    fn dm(h: u32, m: u32, s: u32) -> DayMoment {
        DayMoment::from_hms(h as u8, m as u8, s as u8).unwrap()
    }

    #[test]
    fn before_fajr_interval_wraps_midnight() {
        // @tier ephemeral
        let next = next_prayer_at(dm(4, 59, 30), &reference_set());
        assert_eq!(next.name, PrayerName::Fajr);
        assert_eq!(next.previous, PrayerName::Isha);
        assert_eq!(next.seconds_remaining, 30);
        assert_eq!(next.seconds_into_previous, 32_370);
        assert_eq!(next.interval_seconds, 32_400);
        assert!(!next.is_tomorrow);
    }

    #[test]
    fn at_prayer_second_that_prayer_is_current() {
        // @tier ephemeral
        let next = next_prayer_at(dm(5, 0, 0), &reference_set());
        assert_eq!(next.name, PrayerName::Dhuhr);
        assert_eq!(next.previous, PrayerName::Fajr);
        assert_eq!(next.seconds_into_previous, 0);
        assert_eq!(next.seconds_remaining, 27_000);
        assert_eq!(next.interval_seconds, 27_000);
        assert!(!next.is_tomorrow);
    }

    #[test]
    fn one_second_before_dhuhr() {
        // @tier ephemeral
        let next = next_prayer_at(dm(12, 29, 59), &reference_set());
        assert_eq!(next.name, PrayerName::Dhuhr);
        assert_eq!(next.seconds_remaining, 1);
        assert_eq!(next.seconds_into_previous, 26_999);
    }

    #[test]
    fn at_isha_rolls_to_next_day_fajr() {
        // @tier ephemeral
        let next = next_prayer_at(dm(20, 0, 0), &reference_set());
        assert_eq!(next.name, PrayerName::Fajr);
        assert_eq!(next.previous, PrayerName::Isha);
        assert!(next.is_tomorrow);
        assert_eq!(next.seconds_remaining, 32_400);
        assert_eq!(next.seconds_into_previous, 0);
        assert_eq!(next.interval_seconds, 32_400);
    }

    #[test]
    fn late_night_second_resolution() {
        // @tier ephemeral
        let next = next_prayer_at(dm(23, 59, 59), &reference_set());
        assert_eq!(next.name, PrayerName::Fajr);
        assert!(next.is_tomorrow);
        assert_eq!(next.seconds_remaining, 18_001);
        assert_eq!(next.seconds_into_previous, 14_399);
    }
}

mod preferred_set {
    use mawaqit_tui::domain::prayer::{DailyTimes, PrayerName};

    use super::{clock, set};

    #[test]
    fn prefers_iqama_when_published_else_adhan() {
        // @tier ephemeral
        let adhan = set(["05:00", "12:30", "15:45", "18:20", "20:00"]);
        let iqama = set(["05:20", "12:45", "16:00", "18:35", "20:15"]);
        let with = DailyTimes { adhan, iqama: Some(iqama), shurouq: None };
        assert_eq!(with.preferred().get(PrayerName::Fajr), clock("05:20"));

        let without = DailyTimes { adhan, iqama: None, shurouq: None };
        assert_eq!(without.preferred().get(PrayerName::Fajr), clock("05:00"));
    }
}

// ---- M4: search screen, debounce epochs, selection flow (spec M4 R1–R3) ----

mod search_screen {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use mawaqit_tui::{
        application::{ports::PortError, use_cases::AppError},
        domain::{
            events::DomainEvent,
            mosque::{MosqueId, MosqueSummary},
        },
        ui::app::{AppEvent, AppModel, Boot, Command, Screen, SearchState, TodayState},
    };

    use super::readout;

    fn key(code: KeyCode, mods: KeyModifiers) -> AppEvent {
        AppEvent::Key(KeyEvent::new(code, mods))
    }

    fn ch(c: char) -> AppEvent {
        key(KeyCode::Char(c), KeyModifiers::NONE)
    }

    fn summaries() -> Vec<MosqueSummary> {
        vec![
            MosqueSummary {
                id: MosqueId::parse("mosquee-de-paris").unwrap(),
                name: "Grande Mosquée de Paris".into(),
                place: Some("Paris".into()),
            },
            MosqueSummary {
                id: MosqueId::parse("masjid-de-lyon").unwrap(),
                name: "Mosquée de Lyon".into(),
                place: None,
            },
        ]
    }

    fn search_model() -> AppModel {
        AppModel::from_boot(Boot::NoMosque).0
    }

    /// Type "par": three keyed edits, epochs 1..=3.
    fn type_par(model: &mut AppModel) -> Vec<Command> {
        let mut commands = Vec::new();
        for c in ['p', 'a', 'r'] {
            commands.extend(model.update(ch(c)));
        }
        commands
    }

    /// Drive the model into `Results` with two items, cursor 0.
    fn with_results() -> AppModel {
        let mut model = search_model();
        let _ = type_par(&mut model);
        model.update(AppEvent::SearchLoaded(3, Ok(summaries())));
        model
    }

    /// Drive the model into `Saving` the second summary.
    fn saving_second() -> AppModel {
        let mut model = with_results();
        model.update(key(KeyCode::Down, KeyModifiers::NONE));
        model.update(key(KeyCode::Enter, KeyModifiers::NONE));
        model
    }

    fn network_err() -> AppError {
        AppError::Port(PortError::Network("dns down".into()))
    }

    #[test]
    fn typing_appends_and_emits_epoched_commands() {
        // @tier ephemeral
        let mut model = search_model();
        let commands = type_par(&mut model);
        assert_eq!(model.query, "par");
        assert_eq!(
            commands,
            vec![
                Command::Search { epoch: 1, query: "p".into() },
                Command::Search { epoch: 2, query: "pa".into() },
                Command::Search { epoch: 3, query: "par".into() },
            ]
        );
        assert_eq!(model.search, SearchState::Querying { epoch: 3 });
    }

    #[test]
    fn backspace_pops_char_and_reemits() {
        // @tier ephemeral
        let mut model = search_model();
        let _ = type_par(&mut model);
        let commands = model.update(key(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(model.query, "pa");
        assert_eq!(commands, vec![Command::Search { epoch: 4, query: "pa".into() }]);
        // Backspace on an empty query is inert.
        let mut fresh = search_model();
        let commands = fresh.update(key(KeyCode::Backspace, KeyModifiers::NONE));
        assert!(commands.is_empty());
        assert_eq!(fresh.search, SearchState::Idle);
    }

    /// A query that only trims to empty never reaches the port (mirrors the
    /// `SearchMosques` short-circuit).
    #[test]
    fn trim_empty_query_returns_to_idle_without_command() {
        // @tier ephemeral
        let mut model = search_model();
        let commands = model.update(ch(' '));
        assert_eq!(model.query, " ");
        assert_eq!(model.search, SearchState::Idle);
        assert!(commands.is_empty());

        // Editing back to a non-trim-empty query resumes querying.
        model.update(ch('a'));
        assert_eq!(model.search, SearchState::Querying { epoch: 2 });
    }

    #[test]
    fn search_loaded_ok_matching_epoch_shows_results() {
        // @tier ephemeral
        let mut model = search_model();
        let _ = type_par(&mut model);
        model.update(AppEvent::SearchLoaded(3, Ok(summaries())));
        assert_eq!(
            model.search,
            SearchState::Results { epoch: 3, items: summaries(), cursor: 0 }
        );
    }

    /// The screen belongs to the newest query: a stale response is discarded
    /// (spec M4 R2).
    #[test]
    fn stale_search_results_are_discarded() {
        // @tier ephemeral
        let mut model = search_model();
        model.update(ch('p')); // epoch 1
        model.update(ch('a')); // epoch 2
        model.update(AppEvent::SearchLoaded(1, Ok(summaries())));
        assert_eq!(model.search, SearchState::Querying { epoch: 2 });
    }

    #[test]
    fn search_error_shows_message() {
        // @tier ephemeral
        let mut model = search_model();
        let _ = type_par(&mut model);
        model.update(AppEvent::SearchLoaded(3, Err(network_err())));
        assert_eq!(model.search, SearchState::Failed("network failure: dns down".into()));
    }

    /// A response arriving while `Idle` (e.g. after the query was cleared) is
    /// stale by definition and must not resurrect results.
    #[test]
    fn response_while_idle_is_discarded() {
        // @tier ephemeral
        let mut model = search_model();
        let _ = type_par(&mut model);
        model.update(key(KeyCode::Backspace, KeyModifiers::NONE)); // "pa"
        model.update(key(KeyCode::Backspace, KeyModifiers::NONE)); // "p"
        model.update(key(KeyCode::Backspace, KeyModifiers::NONE)); // ""
        assert_eq!(model.search, SearchState::Idle);
        model.update(AppEvent::SearchLoaded(3, Ok(summaries())));
        assert_eq!(model.search, SearchState::Idle);
    }

    #[test]
    fn cursor_navigation_clamps_at_both_ends() {
        // @tier ephemeral
        let mut model = with_results();
        for _ in 0..4 {
            model.update(key(KeyCode::Down, KeyModifiers::NONE));
        }
        assert_eq!(model.search.cursor(), Some(1));
        for _ in 0..3 {
            model.update(key(KeyCode::Up, KeyModifiers::NONE));
        }
        assert_eq!(model.search.cursor(), Some(0));
    }

    /// Enter selects the cursor row and asks for persistence; the domain
    /// event fires only once the save succeeded (spec M4 R3).
    #[test]
    fn enter_on_results_saves_cursor_row() {
        // @tier ephemeral
        let mut model = with_results();
        model.update(key(KeyCode::Down, KeyModifiers::NONE));
        let commands = model.update(key(KeyCode::Enter, KeyModifiers::NONE));
        let expected = summaries().remove(1);
        assert_eq!(commands, vec![Command::SaveSelection(expected.clone())]);
        assert_eq!(model.search, SearchState::Saving(expected));
        assert!(model.take_events().is_empty());
    }

    #[test]
    fn enter_is_inert_without_selectable_results() {
        // @tier ephemeral
        // Idle
        let mut model = search_model();
        assert!(model.update(key(KeyCode::Enter, KeyModifiers::NONE)).is_empty());
        // Querying
        let _ = type_par(&mut model);
        assert!(model.update(key(KeyCode::Enter, KeyModifiers::NONE)).is_empty());
        // Failed
        model.update(AppEvent::SearchLoaded(3, Err(network_err())));
        assert!(model.update(key(KeyCode::Enter, KeyModifiers::NONE)).is_empty());
        // Results, but empty (fresh model — the failed state above no longer
        // accepts epoch 3 responses, by design).
        let mut model = search_model();
        let _ = type_par(&mut model);
        model.update(AppEvent::SearchLoaded(3, Ok(Vec::new())));
        assert_eq!(
            model.search,
            SearchState::Results { epoch: 3, items: vec![], cursor: 0 }
        );
        assert!(model.update(key(KeyCode::Enter, KeyModifiers::NONE)).is_empty());
        // Saving (double-Enter guard)
        let mut model = saving_second();
        assert!(model.update(key(KeyCode::Enter, KeyModifiers::NONE)).is_empty());
    }

    #[test]
    fn selection_saved_ok_jumps_to_today_loading() {
        // @tier ephemeral
        let mut model = saving_second();
        let id = summaries().remove(1).id;
        let commands = model.update(AppEvent::SelectionSaved(Ok(())));
        assert_eq!(model.screen, Screen::Today);
        assert_eq!(model.today, TodayState::Loading);
        assert_eq!(commands, vec![Command::LoadToday(id.clone())]);
        assert_eq!(model.take_events(), vec![DomainEvent::MosqueSelected { id }]);
        // Events are drained, not cloned.
        assert!(model.take_events().is_empty());
    }

    /// A failed disk write shows the reason verbatim, keeps the selection,
    /// and Enter retries (spec M4 R3 — HONESTY).
    #[test]
    fn selection_saved_err_keeps_retry_path() {
        // @tier ephemeral
        let mut model = saving_second();
        let summary = summaries().remove(1);
        model.update(AppEvent::SelectionSaved(Err(AppError::Settings(
            mawaqit_tui::application::ports::SettingsError::Io("disk full".into()),
        ))));
        assert_eq!(
            model.search,
            SearchState::SaveFailed {
                summary: summary.clone(),
                reason: "settings io failure: disk full".into(),
            }
        );
        let commands = model.update(key(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(commands, vec![Command::SaveSelection(summary.clone())]);
        assert_eq!(model.search, SearchState::Saving(summary));
    }

    #[test]
    fn esc_returns_to_today_preserving_its_state() {
        // @tier ephemeral
        // From a Ready Today.
        let mut model =
            AppModel::from_boot(Boot::Loading(MosqueId::parse("m").unwrap())).0;
        model.update(AppEvent::TodayLoaded(Ok(readout())));
        model.update(ch('s'));
        assert_eq!(model.screen, Screen::Search);
        model.update(key(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(model.screen, Screen::Today);
        assert_eq!(model.today, TodayState::Ready(readout()));

        // From a boot-time Failed Today.
        let mut model = AppModel::from_boot(Boot::Failed("corrupt".into())).0;
        model.update(ch('s'));
        model.update(key(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(model.today, TodayState::Failed("corrupt".into()));
    }

    #[test]
    fn s_opens_search_from_any_today_state() {
        // @tier ephemeral
        for boot in
            [Boot::Loading(MosqueId::parse("m").unwrap()), Boot::Failed("corrupt".into())]
        {
            let (mut model, pending) = AppModel::from_boot(boot);
            let _ = pending; // LoadToday commands belong to the runtime
            model.update(ch('s'));
            assert_eq!(model.screen, Screen::Search);
        }
    }

    /// Reopening search keeps the previous query and results (spec M4 R1).
    #[test]
    fn s_reopens_preserve_previous_results() {
        // @tier ephemeral
        let mut model = with_results();
        model.update(ch('s')); // 's' while already searching types into query!
        assert_eq!(model.query, "pars");
        // Proper round-trip: back to Today, then re-open.
        let mut model = with_results();
        model.update(key(KeyCode::Esc, KeyModifiers::NONE));
        model.update(ch('s'));
        assert_eq!(model.screen, Screen::Search);
        assert_eq!(model.search.cursor(), Some(0));
        assert!(matches!(model.search, SearchState::Results { .. }));
    }

    #[test]
    fn bare_q_types_on_search_screen() {
        // @tier ephemeral
        let mut model = search_model();
        let commands = model.update(ch('q'));
        assert!(!model.should_quit);
        assert_eq!(model.query, "q");
        assert_eq!(commands, vec![Command::Search { epoch: 1, query: "q".into() }]);
    }

    #[test]
    fn control_chars_do_not_enter_the_query() {
        // @tier ephemeral
        let mut model = search_model();
        let commands = model.update(key(KeyCode::Char('p'), KeyModifiers::CONTROL));
        assert_eq!(model.query, "");
        assert!(commands.is_empty());
        assert_eq!(model.search, SearchState::Idle);
    }

    // -- render smokes (TestBackend) ---------------------------------------

    fn render(model: &AppModel) -> String {
        let backend = ratatui::backend::TestBackend::new(64, 14);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|frame| mawaqit_tui::ui::draw::draw(frame, model)).unwrap();
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol().to_owned())
            .collect()
    }

    #[test]
    fn search_results_render_names_and_count() {
        // @tier ephemeral
        let text = render(&with_results());
        assert!(text.contains("par"), "query missing: {text:?}");
        assert!(text.contains("2 found"), "result count missing: {text:?}");
        assert!(text.contains("Grande Mosquée de Paris"), "row missing: {text:?}");
        assert!(text.contains("Mosquée de Lyon"), "row missing: {text:?}");
        assert!(text.contains("Enter · select"), "footer missing: {text:?}");
    }

    #[test]
    fn empty_results_render_no_mosques_found() {
        // @tier ephemeral
        let mut model = search_model();
        let _ = type_par(&mut model);
        model.update(AppEvent::SearchLoaded(3, Ok(Vec::new())));
        let text = render(&model);
        assert!(text.contains("no mosques found"), "empty state missing: {text:?}");
    }

    #[test]
    fn querying_renders_searching_hint() {
        // @tier ephemeral
        let mut model = search_model();
        let _ = type_par(&mut model);
        let text = render(&model);
        assert!(text.contains("searching"), "hint missing: {text:?}");
    }

    #[test]
    fn save_failed_renders_reason_and_retry_hint() {
        // @tier ephemeral
        let mut model = saving_second();
        model.update(AppEvent::SelectionSaved(Err(AppError::Settings(
            mawaqit_tui::application::ports::SettingsError::Io("disk full".into()),
        ))));
        let text = render(&model);
        assert!(
            text.contains("settings io failure: disk full"),
            "verbatim reason missing: {text:?}"
        );
        assert!(text.contains("retry"), "retry hint missing: {text:?}");
    }
}

// ---- M5: month screen, in-session cache, dropped days (spec M5 R3–R5) ------

mod month_screen {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use mawaqit_tui::{
        application::{
            ports::{MonthDay, MonthReadout, PortError, TzSource},
            use_cases::AppError,
        },
        domain::mosque::{MosqueId, MosqueSummary},
        ui::app::{AppEvent, AppModel, Boot, Command, MonthState, Screen, TodayState},
    };

    use super::{fixed_now, readout, set};

    fn key(code: KeyCode, mods: KeyModifiers) -> AppEvent {
        AppEvent::Key(KeyEvent::new(code, mods))
    }

    fn ch(c: char) -> AppEvent {
        key(KeyCode::Char(c), KeyModifiers::NONE)
    }

    fn mosque_id() -> MosqueId {
        MosqueId::parse("test-mosque").unwrap()
    }

    /// Days 1..=10, all with iqama; day 15 was dropped by the source. The
    /// payload month is a parameter: the model rejects mismatched months.
    fn month_readout(month: u32) -> MonthReadout {
        let adhan = set(["05:00", "12:30", "15:45", "18:20", "20:00"]);
        let iqama = set(["05:15", "12:45", "16:00", "18:35", "20:15"]);
        MonthReadout {
            month,
            tz: TzSource::Mosque(chrono_tz::Tz::Europe__Paris),
            days: (1..=10)
                .map(|day| MonthDay { day, adhan, iqama: Some(iqama), shurouq: None })
                .collect(),
            dropped: vec![15],
        }
    }

    /// A Today screen that is Ready, ticked at the fixed instant (Paris:
    /// 2026-10-06 14:00 → month 10), with the month screen opened.
    fn opened_month() -> (AppModel, MosqueId) {
        let id = mosque_id();
        let mut model = AppModel::from_boot(Boot::Loading(id.clone())).0;
        model.update(AppEvent::TodayLoaded(Ok(readout())));
        model.update(AppEvent::Tick(fixed_now()));
        let commands = model.update(ch('m'));
        assert_eq!(
            commands,
            vec![Command::LoadMonth { id: id.clone(), month: 10 }],
            "setup must open the month screen"
        );
        (model, id)
    }

    fn ready_month() -> (AppModel, MosqueId) {
        let (mut model, id) = opened_month();
        model.update(AppEvent::MonthLoaded {
            id: id.clone(),
            month: 10,
            result: Ok(month_readout(10)),
        });
        (model, id)
    }

    #[test]
    fn m_is_inert_until_today_is_ready() {
        // @tier ephemeral
        let id = mosque_id();
        // Loading and Failed boots: no tz anchor exists yet.
        for boot in [Boot::Loading(id), Boot::Failed("corrupt".into())] {
            let (mut model, _) = AppModel::from_boot(boot);
            assert!(model.update(ch('m')).is_empty());
            assert_eq!(model.screen, Screen::Today);
        }
    }

    /// Even Ready is not enough without a tick — the anchor instant arrives
    /// only via `Tick` (no hidden clocks).
    #[test]
    fn m_without_a_tick_is_inert() {
        // @tier ephemeral
        let mut model = AppModel::from_boot(Boot::Loading(mosque_id())).0;
        model.update(AppEvent::TodayLoaded(Ok(readout())));
        assert!(model.update(ch('m')).is_empty());
    }

    #[test]
    fn m_opens_month_anchor_to_mosque_tz() {
        // @tier ephemeral
        let id = mosque_id();
        let mut model = AppModel::from_boot(Boot::Loading(id.clone())).0;
        model.update(AppEvent::TodayLoaded(Ok(readout())));
        model.update(AppEvent::Tick(fixed_now()));
        let commands = model.update(ch('m'));
        assert_eq!(commands, vec![Command::LoadMonth { id: id.clone(), month: 10 }]);
        assert_eq!(model.screen, Screen::Month);
        assert_eq!(model.month, MonthState::Loading { month: 10 });
        assert_eq!(model.selected, Some(id));
    }

    #[test]
    fn month_loaded_caches_and_cursors_today() {
        // @tier ephemeral
        let (mut model, id) = opened_month();
        model.update(AppEvent::MonthLoaded {
            id: id.clone(),
            month: 10,
            result: Ok(month_readout(10)),
        });
        assert!(matches!(model.month, MonthState::Ready { month: 10, cursor: 5, .. }));

        // Away and back: the revisit must not emit a command (cache hit).
        let commands = model.update(key(KeyCode::Left, KeyModifiers::NONE));
        assert_eq!(commands, vec![Command::LoadMonth { id: id.clone(), month: 9 }]);
        model.update(AppEvent::MonthLoaded {
            id: id.clone(),
            month: 9,
            result: Ok(month_readout(10)),
        });
        let commands = model.update(key(KeyCode::Right, KeyModifiers::NONE));
        assert!(commands.is_empty(), "cached month must not refetch: {commands:?}");
        assert!(matches!(model.month, MonthState::Ready { month: 10, .. }));
    }

    #[test]
    fn navigation_wraps_december_to_january() {
        // @tier ephemeral
        let (mut model, id) = ready_month();
        for expected in [11, 12, 1] {
            let commands = model.update(key(KeyCode::Right, KeyModifiers::NONE));
            assert_eq!(
                commands,
                vec![Command::LoadMonth { id: id.clone(), month: expected }]
            );
            model.update(AppEvent::MonthLoaded {
                id: id.clone(),
                month: expected,
                result: Ok(month_readout(expected)),
            });
        }
        // 12 → 1 wrap happened above; moving on from January must reach the
        // uncached February (December itself is cached now — see the cache
        // test for the no-refetch guarantee).
        let commands = model.update(key(KeyCode::Right, KeyModifiers::NONE));
        assert_eq!(commands, vec![Command::LoadMonth { id, month: 2 }]);
    }

    /// A response for a month the user has already navigated away from is
    /// discarded (same staleness rule as search).
    #[test]
    fn stale_month_result_discarded() {
        // @tier ephemeral
        let (mut model, id) = opened_month();
        model.update(key(KeyCode::Right, KeyModifiers::NONE)); // → Loading{11}
        model.update(AppEvent::MonthLoaded {
            id: id.clone(),
            month: 10,
            result: Ok(month_readout(10)),
        });
        assert_eq!(model.month, MonthState::Loading { month: 11 });
    }

    #[test]
    fn month_loaded_for_other_mosque_discarded() {
        // @tier ephemeral
        let (mut model, _) = opened_month();
        let stranger = MosqueId::parse("other-mosque").unwrap();
        model.update(AppEvent::MonthLoaded {
            id: stranger,
            month: 10,
            result: Ok(month_readout(10)),
        });
        assert_eq!(model.month, MonthState::Loading { month: 10 });
    }

    /// A source that quietly returns different month data must be failed
    /// loudly, never relabelled with the requested month (HONESTY).
    #[test]
    fn month_mismatch_from_source_fails_loudly() {
        // @tier ephemeral
        let (mut model, id) = opened_month();
        model.update(AppEvent::MonthLoaded {
            id: id.clone(),
            month: 10,
            result: Ok(month_readout(11)),
        });
        assert!(matches!(model.month, MonthState::Failed { month: 10, .. }));
        // Nothing was cached under the request: moving re-fetches.
        let commands = model.update(key(KeyCode::Left, KeyModifiers::NONE));
        assert_eq!(commands, vec![Command::LoadMonth { id, month: 9 }]);
    }

    #[test]
    fn month_loaded_err_shows_verbatim_and_navigates_on() {
        // @tier ephemeral
        let (mut model, id) = opened_month();
        model.update(AppEvent::MonthLoaded {
            id: id.clone(),
            month: 10,
            result: Err(AppError::Port(PortError::Network("boom".into()))),
        });
        assert_eq!(
            model.month,
            MonthState::Failed { month: 10, reason: "network failure: boom".into() }
        );
        // Navigation works from Failed: retrying is just moving again.
        let commands = model.update(key(KeyCode::Left, KeyModifiers::NONE));
        assert_eq!(commands, vec![Command::LoadMonth { id, month: 9 }]);
    }

    #[test]
    fn cursor_clamps_and_jk_work() {
        // @tier ephemeral
        let (mut model, _) = ready_month();
        for _ in 0..12 {
            model.update(key(KeyCode::Down, KeyModifiers::NONE));
        }
        assert!(matches!(model.month, MonthState::Ready { cursor: 9, .. }));
        model.update(ch('j'));
        assert!(matches!(model.month, MonthState::Ready { cursor: 9, .. }));
        model.update(key(KeyCode::Up, KeyModifiers::NONE));
        assert!(matches!(model.month, MonthState::Ready { cursor: 8, .. }));
        model.update(ch('k'));
        assert!(matches!(model.month, MonthState::Ready { cursor: 7, .. }));
        for _ in 0..8 {
            model.update(key(KeyCode::Up, KeyModifiers::NONE));
        }
        assert!(matches!(model.month, MonthState::Ready { cursor: 0, .. }));
    }

    #[test]
    fn esc_s_q_keys_on_month_screen() {
        // @tier ephemeral
        let (mut model, _) = ready_month();
        // Esc → Today, preserving its Ready state.
        model.update(key(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(model.screen, Screen::Today);
        assert!(matches!(model.today, TodayState::Ready(_)));
        // Back into the month; `s` opens Search from there.
        model.update(ch('m'));
        model.update(ch('s'));
        assert_eq!(model.screen, Screen::Search);
        // Bare `q` quits on the month screen (it is not a typing screen).
        let (mut model, _) = ready_month();
        model.update(key(KeyCode::Char('q'), KeyModifiers::NONE));
        assert!(model.should_quit);
    }

    #[test]
    fn selection_records_selected_mosque() {
        // @tier ephemeral
        let mut model =
            AppModel::from_boot(Boot::Loading(MosqueId::parse("boot-mosque").unwrap())).0;
        let summary = MosqueSummary {
            id: MosqueId::parse("new-mosque").unwrap(),
            name: "New".into(),
            place: None,
        };
        model.update(ch('s'));
        model.update(ch('x')); // epoch 1
        model.update(AppEvent::SearchLoaded(1, Ok(vec![summary.clone()])));
        model.update(key(KeyCode::Enter, KeyModifiers::NONE));
        model.update(AppEvent::SelectionSaved(Ok(())));
        assert_eq!(model.selected, Some(summary.id));
    }

    // -- render smokes (TestBackend) ---------------------------------------

    fn render(model: &AppModel) -> String {
        let backend = ratatui::backend::TestBackend::new(100, 20);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|frame| mawaqit_tui::ui::draw::draw(frame, model)).unwrap();
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol().to_owned())
            .collect()
    }

    #[test]
    fn month_renders_headers_iqama_and_dropped_marker() {
        // @tier ephemeral
        let (model, _) = ready_month();
        let text = render(&model);
        assert!(text.contains("fajr"), "adhan header missing: {text:?}");
        assert!(text.contains("i-fjr"), "iqama header missing: {text:?}");
        assert!(text.contains("October"), "month name missing: {text:?}");
        assert!(text.contains("dropped"), "dropped info missing: {text:?}");
        assert!(text.contains("Esc · today"), "footer missing: {text:?}");
    }

    /// A month without iqama must not render empty iqama columns.
    #[test]
    fn month_without_iqama_hides_iqama_columns() {
        // @tier ephemeral
        let (mut model, id) = opened_month();
        let mut readout = month_readout(10);
        for day in &mut readout.days {
            day.iqama = None;
        }
        model.update(AppEvent::MonthLoaded { id, month: 10, result: Ok(readout) });
        let text = render(&model);
        assert!(text.contains("fajr"), "adhan header missing: {text:?}");
        assert!(!text.contains("i-fjr"), "iqama columns must be hidden: {text:?}");
    }

    /// Dropped days are explicit dimmed rows, not silent gaps (HONESTY).
    #[test]
    fn dropped_days_render_between_present_days() {
        // @tier ephemeral
        let (mut model, id) = opened_month();
        let mut readout = month_readout(10);
        readout.days.truncate(2); // days 1–2 present, day 3 dropped
        readout.dropped = vec![3];
        model.update(AppEvent::MonthLoaded { id, month: 10, result: Ok(readout) });
        let text = render(&model);
        assert!(text.contains('·'), "dropped-day marker missing: {text:?}");
        assert!(text.contains("dropped: 3"), "dropped list missing: {text:?}");
    }
}

// ---- M6: offline retention, retries, help overlay (spec M6 R2–R4) ----------

mod offline_help {
    use crossterm::event::{KeyCode, KeyModifiers};
    use mawaqit_tui::{
        application::{ports::PortError, use_cases::AppError},
        domain::mosque::MosqueId,
        ui::app::{AppEvent, AppModel, Boot, Command, Screen, TodayState},
    };

    use super::{fixed_now, readout};

    fn key(code: KeyCode) -> AppEvent {
        AppEvent::Key(crossterm::event::KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn ch(c: char) -> AppEvent {
        key(KeyCode::Char(c))
    }

    fn ctrl_c() -> AppEvent {
        AppEvent::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
        ))
    }

    fn network_err() -> AppError {
        AppError::Port(PortError::Network("boom".into()))
    }

    /// A Ready Today ticked at the fixed instant, selected mosque intact.
    fn ready_today() -> AppModel {
        let mut model =
            AppModel::from_boot(Boot::Loading(MosqueId::parse("m").unwrap())).0;
        model.update(AppEvent::TodayLoaded(Ok(readout())));
        model.update(AppEvent::Tick(fixed_now()));
        model
    }

    /// Drive the model offline: a Ready Today whose refresh just failed.
    fn offline_today() -> AppModel {
        let mut model = ready_today();
        model.update(AppEvent::TodayLoaded(Err(network_err())));
        model
    }

    #[test]
    fn failed_refresh_retains_last_good_and_raises_offline() {
        // @tier ephemeral
        let model = offline_today();
        assert_eq!(model.offline.as_deref(), Some("network failure: boom"));
        // Last-good data stays on screen — never blanked.
        assert_eq!(model.today, TodayState::Ready(readout()));
    }

    /// First-boot failure with nothing retained is the honest error, not a
    /// badge over an empty screen.
    #[test]
    fn boot_failure_without_last_good_stays_failed() {
        // @tier ephemeral
        let mut model =
            AppModel::from_boot(Boot::Loading(MosqueId::parse("m").unwrap())).0;
        model.update(AppEvent::TodayLoaded(Err(network_err())));
        assert_eq!(model.offline, None);
        assert!(matches!(model.today, TodayState::Failed(_)));
    }

    #[test]
    fn successful_load_clears_the_offline_badge() {
        // @tier ephemeral
        let mut model = offline_today();
        model.update(AppEvent::TodayLoaded(Ok(readout())));
        assert_eq!(model.offline, None);
    }

    #[test]
    fn r_on_ready_refetches_without_flipping_to_loading() {
        // @tier ephemeral
        let mut model = offline_today();
        let commands = model.update(ch('r'));
        let id = MosqueId::parse("m").unwrap();
        assert_eq!(commands, vec![Command::LoadToday(id)]);
        // Retained data must not be blanked by a manual refresh.
        assert!(matches!(model.today, TodayState::Ready(_)));
    }

    #[test]
    fn r_on_failed_goes_loading() {
        // @tier ephemeral
        let mut model =
            AppModel::from_boot(Boot::Loading(MosqueId::parse("m").unwrap())).0;
        model.update(AppEvent::TodayLoaded(Err(network_err())));
        let commands = model.update(ch('r'));
        assert_eq!(commands, vec![Command::LoadToday(MosqueId::parse("m").unwrap())]);
        assert_eq!(model.today, TodayState::Loading);
    }

    /// No selection (boot-failed config): nothing to refresh.
    #[test]
    fn r_without_selection_is_inert() {
        // @tier ephemeral
        let mut model = AppModel::from_boot(Boot::Failed("corrupt".into())).0;
        assert!(model.update(ch('r')).is_empty());
    }

    #[test]
    fn auto_retry_fires_on_the_30th_tick_only() {
        // @tier ephemeral
        // offline_today() carries one priming tick already (ticks == 1).
        let mut model = offline_today();
        let id = MosqueId::parse("m").unwrap();
        let boundary = mawaqit_tui::ui::app::AppModel::OFFLINE_RETRY_TICKS;
        while model.ticks() < boundary - 1 {
            assert!(
                model.update(AppEvent::Tick(fixed_now())).is_empty(),
                "tick {} must not retry",
                model.ticks()
            );
        }
        assert_eq!(
            model.update(AppEvent::Tick(fixed_now())),
            vec![Command::LoadToday(id)]
        );
    }

    #[test]
    fn auto_retry_requires_offline_and_selection() {
        // @tier ephemeral
        // Live (not offline): the 30th tick is just a tick.
        let mut model = ready_today();
        for _ in 0..mawaqit_tui::ui::app::AppModel::OFFLINE_RETRY_TICKS {
            assert!(model.update(AppEvent::Tick(fixed_now())).is_empty());
        }
        // Offline but no selection: nothing to re-fetch.
        let mut model = AppModel::from_boot(Boot::Failed("corrupt".into())).0;
        model.offline = Some("boom".into());
        for _ in 0..mawaqit_tui::ui::app::AppModel::OFFLINE_RETRY_TICKS {
            assert!(model.update(AppEvent::Tick(fixed_now())).is_empty());
        }
    }

    // -- help overlay (spec M6 R3) ----------------------------------------

    #[test]
    fn help_toggles_from_every_screen() {
        // @tier ephemeral
        for screen in ["today", "search"] {
            let mut model = ready_today();
            if screen == "search" {
                model.update(ch('s'));
            }
            model.update(ch('?'));
            assert!(model.show_help, "? must open help on {screen}");
            model.update(key(KeyCode::Esc));
            assert!(!model.show_help, "Esc must close help on {screen}");
        }
    }

    /// While help is shown: `?`/Esc close, Ctrl-C quits, everything else is
    /// swallowed — including the screen's own keys.
    #[test]
    fn help_swallows_keys_except_close_and_quit() {
        // @tier ephemeral
        // On Search, typing must not leak into the query through the overlay.
        let mut model = ready_today();
        model.update(ch('s'));
        model.update(ch('x')); // query "x"
        model.update(ch('?')); // help up
        model.update(ch('q')); // swallowed: no quit, no close per spec
        assert!(model.show_help);
        assert!(!model.should_quit);
        model.update(ch('y')); // swallowed: query unchanged
        model.update(key(KeyCode::Esc)); // closes
        assert!(!model.show_help);
        assert_eq!(model.query, "x");

        // Today screen under help: `m`/`r`/`s` stay inert while it is up.
        let mut model = ready_today();
        model.update(ch('?'));
        assert!(model.update(ch('r')).is_empty());
        assert!(model.update(ch('s')).is_empty());
        assert_eq!(model.screen, Screen::Today);
        model.update(ch('?')); // close via ?
        assert!(!model.show_help);

        // Ctrl-C quits even with help up.
        let mut model = ready_today();
        model.update(ch('?'));
        model.update(ctrl_c());
        assert!(model.should_quit);
    }

    // -- render smokes (TestBackend) ---------------------------------------

    fn render(model: &AppModel) -> String {
        let backend = ratatui::backend::TestBackend::new(64, 14);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|frame| mawaqit_tui::ui::draw::draw(frame, model)).unwrap();
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol().to_owned())
            .collect()
    }

    #[test]
    fn offline_badge_renders_over_last_good() {
        // @tier ephemeral
        let mut model = offline_today();
        model.update(AppEvent::Tick(fixed_now()));
        let text = render(&model);
        assert!(text.contains("OFFLINE"), "badge missing: {text:?}");
        assert!(text.contains("Grande Mosquée"), "last-good data must stay: {text:?}");
    }

    #[test]
    fn skeletons_render_on_loading_screens() {
        // @tier ephemeral
        let model = AppModel::from_boot(Boot::Loading(MosqueId::parse("m").unwrap())).0;
        let text = render(&model);
        assert!(text.contains("loading today's times"), "today skeleton: {text:?}");
        assert!(text.contains('·'), "skeleton rows missing: {text:?}");
    }

    #[test]
    fn help_overlay_renders_the_keybinding_table() {
        // @tier durable — every HELP_ENTRIES row must render; the table is
        // the single source of the binding reference. Tall terminal: the
        // full 13-row table must fit (the shared render is 64×14 and would
        // clip the last rows).
        let mut model = ready_today();
        model.update(ch('?'));
        let backend = ratatui::backend::TestBackend::new(80, 30);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|frame| mawaqit_tui::ui::draw::draw(frame, &model)).unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol().to_owned())
            .collect();
        assert!(text.contains("Keys"), "header missing: {text:?}");
        assert!(text.contains("? / Esc closes"), "title missing: {text:?}");
        for entry in mawaqit_tui::ui::views::help::HELP_ENTRIES {
            assert!(text.contains(entry.keys), "keys missing: {text:?}");
            assert!(text.contains(entry.meaning), "meaning missing: {text:?}");
        }
    }
}
