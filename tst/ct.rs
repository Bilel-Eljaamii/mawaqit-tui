//! Component tests — use cases and adapters against in-memory fakes.
//! Zero network (issue #2 acceptance).
//!
//! Durable tests assert contracts that survive a reimplementation (ADR-113);
//! mapping-fixture tests are `@tier ephemeral` (API shapes are crate detail).

use std::sync::Mutex;

use chrono::NaiveDate;
use mawaqit_api::{ConfData, MonthIqamaTimes, MonthTimes, Mosque, TodayTimes};
use mawaqit_tui::{
    application::{
        ports::{
            MonthDay, MonthReadout, MosqueDirectory, PortError, SettingsError,
            SettingsStore, TimesService, TodayReadout, TzSource,
        },
        use_cases::{AppError, LoadMonth, LoadToday, SaveSelection, SearchMosques},
    },
    domain::{
        mosque::{MosqueId, MosqueSummary},
        prayer::{ClockTime, DailyTimes, PrayerName, PrayerSet},
    },
    infrastructure::{
        mawaqit::mapping::{map_month, map_mosques, map_today},
        settings::TomlSettings,
    },
};

// ---- fixtures --------------------------------------------------------------

fn t(s: &str) -> ClockTime {
    ClockTime::parse_hhmm(s).unwrap()
}

fn slug(s: &str) -> MosqueId {
    MosqueId::parse(s).unwrap()
}

fn summary(slug_str: &str, name: &str, place: Option<&str>) -> MosqueSummary {
    MosqueSummary {
        id: slug(slug_str),
        name: name.to_owned(),
        place: place.map(str::to_owned),
    }
}

fn today_fixture() -> TodayReadout {
    let adhan =
        PrayerSet::new(t("05:00"), t("12:30"), t("15:45"), t("18:20"), t("20:00"))
            .unwrap();
    let iqama =
        PrayerSet::new(t("05:20"), t("12:45"), t("16:00"), t("18:35"), t("20:15"))
            .unwrap();
    TodayReadout {
        date: NaiveDate::from_ymd_opt(2026, 10, 6).unwrap(),
        times: DailyTimes { adhan, iqama: Some(iqama), shurouq: Some(t("06:40")) },
        tz: TzSource::Mosque(chrono_tz::Tz::Europe__Paris),
        mosque_name: Some("Grande Mosquée de Paris".to_owned()),
        jumua: Some(t("12:45")),
    }
}

fn month_fixture() -> MonthReadout {
    MonthReadout {
        month: 10,
        tz: TzSource::Mosque(chrono_tz::Tz::Europe__Paris),
        days: vec![MonthDay {
            day: 1,
            adhan: PrayerSet::new(
                t("05:00"),
                t("12:30"),
                t("15:45"),
                t("18:20"),
                t("20:00"),
            )
            .unwrap(),
            iqama: None,
            shurouq: None,
        }],
        dropped: vec![15],
    }
}

// ---- fakes -----------------------------------------------------------------

#[derive(Default)]
struct FakeDirectory {
    results: Vec<MosqueSummary>,
    queries: Mutex<Vec<String>>,
}

impl MosqueDirectory for FakeDirectory {
    async fn search(&self, word: &str) -> Result<Vec<MosqueSummary>, PortError> {
        self.queries.lock().unwrap().push(word.to_owned());
        Ok(self.results.clone())
    }
}

struct FakeTimes {
    today: TodayReadout,
    month: MonthReadout,
    asked: Mutex<Vec<String>>,
}

impl TimesService for FakeTimes {
    async fn today(&self, id: &MosqueId) -> Result<TodayReadout, PortError> {
        self.asked.lock().unwrap().push(id.as_str().to_owned());
        Ok(self.today.clone())
    }

    async fn month_times(
        &self,
        id: &MosqueId,
        month: u32,
    ) -> Result<MonthReadout, PortError> {
        self.asked.lock().unwrap().push(format!("{}#{month}", id.as_str()));
        Ok(self.month.clone())
    }
}

#[derive(Default)]
struct FakeSettings {
    saved: Mutex<Option<MosqueSummary>>,
}

impl SettingsStore for FakeSettings {
    fn load(&self) -> Result<Option<MosqueSummary>, SettingsError> {
        Ok(self.saved.lock().unwrap().clone())
    }

    fn save(&self, selection: &MosqueSummary) -> Result<(), SettingsError> {
        *self.saved.lock().unwrap() = Some(selection.clone());
        Ok(())
    }
}

// ---- use cases (durable contracts) ------------------------------------------

#[tokio::test]
async fn search_select_save_load_today_roundtrip() {
    // @tier durable
    let directory = FakeDirectory {
        results: vec![summary(
            "grand-mosque-paris",
            "Grande Mosquée de Paris",
            Some("Paris"),
        )],
        ..Default::default()
    };
    let found =
        SearchMosques { directory: &directory }.execute("  paris  ").await.unwrap();
    assert_eq!(found.len(), 1);
    // Input is trimmed before it reaches the port.
    assert_eq!(
        directory.queries.lock().unwrap().last().map(String::as_str),
        Some("paris")
    );

    let settings = FakeSettings::default();
    SaveSelection { settings: &settings }.execute(&found[0]).unwrap();

    let times = FakeTimes {
        today: today_fixture(),
        month: month_fixture(),
        asked: Mutex::default(),
    };
    let selected = settings.load().unwrap().unwrap();
    let readout = LoadToday { times: &times }.execute(&selected.id).await.unwrap();
    assert_eq!(readout.tz, TzSource::Mosque(chrono_tz::Tz::Europe__Paris));
    assert_eq!(*times.asked.lock().unwrap(), vec![selected.id.as_str().to_owned()]);
}

#[tokio::test]
async fn empty_query_short_circuits_without_port_call() {
    // @tier durable
    let directory = FakeDirectory::default();
    let empty = SearchMosques { directory: &directory }.execute("   ").await.unwrap();
    assert!(empty.is_empty());
    assert!(directory.queries.lock().unwrap().is_empty());
}

#[tokio::test]
async fn load_month_rejects_out_of_range_before_port() {
    // @tier durable
    let times = FakeTimes {
        today: today_fixture(),
        month: month_fixture(),
        asked: Mutex::default(),
    };
    let use_case = LoadMonth { times: &times };
    for bad in [0, 13] {
        let err = use_case.execute(&slug("grand-mosque-paris"), bad).await.unwrap_err();
        assert!(
            matches!(err, AppError::Port(PortError::InvalidData(_))),
            "month {bad} must be rejected as invalid data"
        );
    }
    assert!(times.asked.lock().unwrap().is_empty());
}

// ---- settings adapter (durable contracts) -----------------------------------

#[test]
fn settings_roundtrip_preserves_selection() {
    // @tier durable
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let store = TomlSettings::new(&path);
    assert_eq!(store.load().unwrap(), None);

    let selection =
        summary("grand-mosque-paris", "Grande Mosquée de Paris", Some("Paris"));
    store.save(&selection).unwrap();
    store.save(&selection).unwrap(); // second atomic write over the first
    assert_eq!(store.load().unwrap(), Some(selection));
}

#[test]
fn corrupt_settings_reported_not_swallowed() {
    // @tier durable
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, b"\xff\xfe not toml at all {{{").unwrap();
    let store = TomlSettings::new(&path);
    assert!(matches!(store.load(), Err(SettingsError::Corrupt)));
}

/// Platforms without a config dir get an honest failure, never a silent
/// pretend-save (spec M4 R4).
#[test]
fn null_settings_fails_honestly() {
    // @tier durable
    use mawaqit_tui::infrastructure::settings::NullSettings;
    let store = NullSettings;
    assert!(matches!(store.load(), Err(SettingsError::Io(_))));
    assert!(matches!(
        store.save(&summary("any-slug", "Any", None)),
        Err(SettingsError::Io(_))
    ));
}

// ---- mapping fixtures (ephemeral — crate wire shapes) -----------------------

fn api_prayer_times(dhuhr: &str) -> mawaqit_api::DailyPrayerTimes {
    mawaqit_api::DailyPrayerTimes {
        fajr: "05:00".into(),
        shurouq: "06:40".into(),
        dhuhr: dhuhr.into(),
        asr: "15:45".into(),
        maghrib: "18:20".into(),
        isha: "20:00".into(),
    }
}

fn api_iqama() -> mawaqit_api::DailyIqamaTimes {
    mawaqit_api::DailyIqamaTimes {
        fajr: "05:20".into(),
        dhuhr: "12:45".into(),
        asr: "16:00".into(),
        maghrib: "18:35".into(),
        isha: "20:15".into(),
    }
}

fn api_today(iqama: Option<mawaqit_api::DailyIqamaTimes>) -> TodayTimes {
    TodayTimes {
        date: NaiveDate::from_ymd_opt(2026, 10, 6).unwrap(),
        adhan: api_prayer_times("12:30"),
        iqama,
        iqama_at: None,
    }
}

fn conf_fixture(tz: Option<&str>, name: Option<&str>) -> ConfData {
    conf_fixture_with_jumua(tz, name, None)
}

fn conf_fixture_with_jumua(
    tz: Option<&str>,
    name: Option<&str>,
    jumua: Option<&str>,
) -> ConfData {
    serde_json::from_value(serde_json::json!({
        "timezone": tz,
        "name": name,
        "jumua": jumua,
    }))
    .unwrap()
}

fn mosque_fixture(slug: Option<&str>) -> Mosque {
    serde_json::from_value(serde_json::json!({
        "slug": slug,
        "name": "Mosquée de fixture",
        "locality": "Villefranche",
    }))
    .unwrap()
}

fn month_day(day: u32) -> mawaqit_api::DayTimes {
    mawaqit_api::DayTimes { day, times: api_prayer_times("12:30") }
}

fn month_iqama_day(day: u32) -> mawaqit_api::DayIqamaTimes {
    mawaqit_api::DayIqamaTimes { day, times: api_iqama() }
}

#[test]
fn today_maps_iqama_mosque_name_and_tz() {
    // @tier ephemeral
    let conf = conf_fixture_with_jumua(
        Some("Europe/Paris"),
        Some("Grande Mosquée de Paris"),
        Some("12:45"),
    );
    let readout = map_today(&api_today(Some(api_iqama())), &conf).unwrap();
    assert_eq!(readout.tz, TzSource::Mosque(chrono_tz::Tz::Europe__Paris));
    assert_eq!(readout.mosque_name.as_deref(), Some("Grande Mosquée de Paris"));
    assert_eq!(readout.jumua, Some(t("12:45")));
    let iqama = readout.times.iqama.unwrap();
    assert_eq!(iqama.get(PrayerName::Fajr), t("05:20"));
    assert_eq!(readout.times.shurouq, Some(t("06:40")));
}

#[test]
fn today_tz_falls_back_to_local_when_absent() {
    // @tier durable
    let conf = conf_fixture(None, None);
    let readout = map_today(&api_today(None), &conf).unwrap();
    assert_eq!(readout.tz, TzSource::Local);
}

#[test]
fn hostile_core_time_is_invalid_data() {
    // @tier ephemeral
    let mut hostile = api_today(None);
    hostile.adhan.dhuhr = "12:xx".into();
    let conf = conf_fixture(Some("Europe/Paris"), None);
    assert!(matches!(map_today(&hostile, &conf), Err(PortError::InvalidData(_))));
}

#[test]
fn hostile_shurouq_degrades_to_none() {
    // @tier ephemeral
    let mut degraded = api_today(None);
    degraded.adhan.shurouq = "sinrise".into();
    let conf = conf_fixture(None, None);
    let readout = map_today(&degraded, &conf).unwrap();
    assert_eq!(readout.times.shurouq, None);
}

#[test]
fn month_merge_surfaces_dropped_days() {
    // @tier ephemeral
    let adhan_month = MonthTimes {
        month: 10,
        days: vec![month_day(1), month_day(2)],
        dropped: vec![15],
    };
    let iqama_month =
        MonthIqamaTimes { month: 10, days: vec![month_iqama_day(1)], dropped: vec![] };
    let conf = conf_fixture(Some("Europe/Paris"), None);
    let readout = map_month(&adhan_month, &iqama_month, &conf).unwrap();
    assert_eq!(readout.dropped, vec![15]);
    assert_eq!(readout.days.len(), 2);
    assert!(readout.days[0].iqama.is_some());
    assert!(readout.days[1].iqama.is_none());
}

#[test]
fn orphan_iqama_day_is_invalid_data() {
    // @tier ephemeral
    let adhan_month = MonthTimes { month: 10, days: vec![month_day(1)], dropped: vec![] };
    let iqama_month = MonthIqamaTimes {
        month: 10,
        days: vec![month_iqama_day(1), month_iqama_day(9)],
        dropped: vec![],
    };
    let conf = conf_fixture(Some("Europe/Paris"), None);
    assert!(matches!(
        map_month(&adhan_month, &iqama_month, &conf),
        Err(PortError::InvalidData(_))
    ));
}

/// The month view anchors "today" in the mosque timezone (TZ-TRUTH), so the
/// readout must carry the same tz rule as the today view.
#[test]
fn month_maps_tz_from_conf() {
    // @tier durable
    let adhan_month = MonthTimes { month: 10, days: vec![month_day(1)], dropped: vec![] };
    let iqama_month = MonthIqamaTimes { month: 10, days: vec![], dropped: vec![] };

    let paris = conf_fixture(Some("Europe/Paris"), None);
    let readout = map_month(&adhan_month, &iqama_month, &paris).unwrap();
    assert_eq!(readout.tz, TzSource::Mosque(chrono_tz::Tz::Europe__Paris));

    let absent = conf_fixture(None, None);
    let fallback = map_month(&adhan_month, &iqama_month, &absent).unwrap();
    assert_eq!(fallback.tz, TzSource::Local);
}

#[test]
fn slugless_search_results_are_invalid_data() {
    // @tier ephemeral
    let results = vec![mosque_fixture(None), mosque_fixture(Some("ok-slug"))];
    assert!(matches!(map_mosques(&results), Err(PortError::InvalidData(_))));
}

#[test]
fn search_results_map_slug_name_place() {
    // @tier ephemeral
    let results = vec![mosque_fixture(Some("mosquee-de-fixture"))];
    let mapped = map_mosques(&results).unwrap();
    assert_eq!(mapped.len(), 1);
    assert_eq!(mapped[0].id, slug("mosquee-de-fixture"));
    assert_eq!(mapped[0].name, "Mosquée de fixture");
    assert_eq!(mapped[0].place.as_deref(), Some("Villefranche"));
}

mod system_clock {
    use chrono::Utc;
    use mawaqit_tui::{application::ports::Clock, infrastructure::clock::SystemClock};

    #[test]
    fn now_utc_is_bracketed_by_wall_clock() {
        // @tier durable
        let before = Utc::now();
        let now = SystemClock.now_utc();
        let after = Utc::now();
        assert!(before <= now && now <= after);
    }
}

mod wall_clock {
    use chrono::{TimeZone, Utc};
    use chrono_tz::Tz;
    use mawaqit_tui::{
        application::{clock::day_moment_from_utc, ports::TzSource},
        domain::prayer::DayMoment,
    };

    fn dm(h: u8, m: u8, s: u8) -> DayMoment {
        DayMoment::from_hms(h, m, s).unwrap()
    }

    #[test]
    fn mosque_tz_winter_and_summer_offsets() {
        // @tier durable
        // Summer (CEST, +02:00)
        let summer = Utc.with_ymd_and_hms(2026, 10, 6, 10, 30, 0).unwrap();
        assert_eq!(
            day_moment_from_utc(summer, TzSource::Mosque(Tz::Europe__Paris)),
            dm(12, 30, 0)
        );
        // Winter (CET, +01:00)
        let winter = Utc.with_ymd_and_hms(2026, 1, 15, 10, 30, 0).unwrap();
        assert_eq!(
            day_moment_from_utc(winter, TzSource::Mosque(Tz::Europe__Paris)),
            dm(11, 30, 0)
        );
    }

    #[test]
    fn dst_spring_transition_is_correct() {
        // @tier durable
        // 2026-03-29: Paris jumps CET→CEST at 01:00 UTC, so 01:30 UTC is
        // +02:00.
        let during = Utc.with_ymd_and_hms(2026, 3, 29, 1, 30, 0).unwrap();
        assert_eq!(
            day_moment_from_utc(during, TzSource::Mosque(Tz::Europe__Paris)),
            dm(3, 30, 0)
        );
    }

    #[test]
    fn local_fallback_uses_system_zone() {
        // @tier durable
        let instant = Utc.with_ymd_and_hms(2026, 10, 6, 10, 30, 0).unwrap();
        let expected =
            DayMoment::from_naive_time(instant.with_timezone(&chrono::Local).time());
        assert_eq!(day_moment_from_utc(instant, TzSource::Local), expected.unwrap());
    }

    /// `date_in_utc` is the date twin of `day_moment_from_utc` (spec M5 R2):
    /// the month anchor must follow the mosque calendar, not the agent's UTC
    /// wall clock.
    #[test]
    fn date_in_mosque_tz_winter_summer_and_dst() {
        // @tier durable
        use mawaqit_tui::application::clock::date_in_utc;
        let paris = TzSource::Mosque(Tz::Europe__Paris);
        // Summer (CEST): 2026-10-06 22:30 UTC is already Oct 7 in Paris.
        let late = Utc.with_ymd_and_hms(2026, 10, 6, 22, 30, 0).unwrap();
        assert_eq!(date_in_utc(late, paris).to_string(), "2026-10-07");
        // Winter (CET): 2026-01-15 10:30 UTC is still Jan 15 in Paris.
        let winter = Utc.with_ymd_and_hms(2026, 1, 15, 10, 30, 0).unwrap();
        assert_eq!(date_in_utc(winter, paris).to_string(), "2026-01-15");
        // DST spring transition: 2026-03-29 01:30 UTC is 03:30 CEST.
        let during = Utc.with_ymd_and_hms(2026, 3, 29, 1, 30, 0).unwrap();
        assert_eq!(date_in_utc(during, paris).to_string(), "2026-03-29");
        // Year boundary: 2026-12-31 23:30 UTC is 2027-01-01 00:30 in Paris.
        let new_year = Utc.with_ymd_and_hms(2026, 12, 31, 23, 30, 0).unwrap();
        assert_eq!(date_in_utc(new_year, paris).to_string(), "2027-01-01");
        // Local fallback mirrors the system zone.
        let instant = Utc.with_ymd_and_hms(2026, 10, 6, 10, 30, 0).unwrap();
        assert_eq!(
            date_in_utc(instant, TzSource::Local),
            instant.with_timezone(&chrono::Local).date_naive()
        );
    }
}

#[test]
fn hostile_jumua_degrades_to_none() {
    // @tier ephemeral
    let conf = conf_fixture_with_jumua(None, None, Some("noon"));
    let readout = map_today(&api_today(None), &conf).unwrap();
    assert_eq!(readout.jumua, None);
}
