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
        domain::mosque::MosqueId,
        ui::app::{AppEvent, AppModel, Boot, Command},
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
        assert!(matches!(model.state, mawaqit_tui::ui::app::TodayState::Loading));
        assert_eq!(commands, vec![Command::LoadToday(id)]);
    }

    #[test]
    fn boot_no_mosque_has_no_commands() {
        // @tier ephemeral
        let (model, commands) = AppModel::from_boot(Boot::NoMosque);
        assert!(matches!(model.state, mawaqit_tui::ui::app::TodayState::NoMosque));
        assert!(commands.is_empty());
    }

    #[test]
    fn boot_failed_preserves_message() {
        // @tier ephemeral
        let (model, commands) =
            AppModel::from_boot(Boot::Failed("config corrupt".into()));
        assert!(matches!(
            model.state,
            mawaqit_tui::ui::app::TodayState::Failed(ref msg) if msg == "config corrupt"
        ));
        assert!(commands.is_empty());
    }

    #[test]
    fn today_loaded_ok_transitions_to_ready() {
        // @tier ephemeral
        let (mut model, _) =
            AppModel::from_boot(Boot::Loading(MosqueId::parse("test-mosque").unwrap()));
        model.update(AppEvent::TodayLoaded(Ok(readout())));
        assert!(matches!(model.state, mawaqit_tui::ui::app::TodayState::Ready(_)));
    }

    #[test]
    fn today_loaded_err_transitions_to_failed() {
        // @tier ephemeral
        let (mut model, _) =
            AppModel::from_boot(Boot::Loading(MosqueId::parse("test-mosque").unwrap()));
        model.update(AppEvent::TodayLoaded(Err(
            mawaqit_tui::application::use_cases::AppError::Port(
                mawaqit_tui::application::ports::PortError::Network("boom".into()),
            ),
        )));
        assert!(matches!(model.state, mawaqit_tui::ui::app::TodayState::Failed(_)));
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

    #[test]
    fn q_quits() {
        // @tier ephemeral
        let (model, _) = AppModel::from_boot(Boot::NoMosque);
        let mut model = model;
        model.update(key(KeyCode::Char('q'), KeyModifiers::NONE));
        assert!(model.should_quit);
    }

    #[test]
    fn ctrl_c_quits() {
        // @tier ephemeral
        let (model, _) = AppModel::from_boot(Boot::NoMosque);
        let mut model = model;
        model.update(key(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(model.should_quit);
    }

    #[test]
    fn unknown_keys_are_inert() {
        // @tier ephemeral
        let (model, _) = AppModel::from_boot(Boot::NoMosque);
        let mut model = model;
        model.update(key(KeyCode::Char('x'), KeyModifiers::NONE));
        model.update(key(KeyCode::Esc, KeyModifiers::NONE));
        // Plain `c` without CONTROL must NOT quit.
        model.update(key(KeyCode::Char('c'), KeyModifiers::NONE));
        assert!(!model.should_quit);
    }

    #[test]
    fn shifted_and_control_q_are_inert() {
        // @tier ephemeral
        let (model, _) = AppModel::from_boot(Boot::NoMosque);
        let mut model = model;
        model.update(key(KeyCode::Char('Q'), KeyModifiers::SHIFT));
        model.update(key(KeyCode::Char('q'), KeyModifiers::CONTROL));
        assert!(!model.should_quit);
    }

    #[test]
    fn ticks_advance() {
        // @tier ephemeral
        let (model, _) = AppModel::from_boot(Boot::NoMosque);
        let mut model = model;
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
    }

    /// The NoMosque boot state renders its guidance instead of panicking.
    #[test]
    fn no_mosque_screen_renders_headless() {
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
        assert!(text.contains("no mosque"), "guidance missing: {text:?}");
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
