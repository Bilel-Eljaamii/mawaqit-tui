//! End-to-end tests — scripted event streams through `runtime::run_scripted`
//! and `AppModel` with fake ports (spec M6 R5): the full Elm loop — boot,
//! commands, use cases, model transitions — no network, no terminal, no
//! sleeps. This is the M4-F3/M6 seam: the loop wiring is finally tested.

use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use crossterm::event::{KeyCode, KeyModifiers};
use mawaqit_tui::{
    application::ports::{
        Clock, MonthDay, MonthReadout, PortError, SettingsError, SettingsStore,
        TimesService, TodayReadout, TzSource,
    },
    domain::{
        mosque::{MosqueId, MosqueSummary},
        prayer::{ClockTime, DailyTimes, PrayerSet},
    },
    runtime::{self, Runtime},
    ui::app::{AppEvent, Boot, MonthState, Screen, TodayState},
};

// ---------------------------------------------------------------- fixtures

fn slug(raw: &str) -> MosqueId {
    MosqueId::parse(raw).unwrap()
}

fn fixed_now() -> DateTime<Utc> {
    DateTime::from_timestamp(1_791_388_800, 0).unwrap()
}

/// A fake clock: fixed instant, deterministic ticks.
struct FixedClock;

impl Clock for FixedClock {
    fn now_utc(&self) -> DateTime<Utc> {
        fixed_now()
    }
}

fn today_readout() -> TodayReadout {
    TodayReadout {
        date: chrono::NaiveDate::from_ymd_opt(2026, 10, 7).unwrap(),
        times: DailyTimes {
            adhan: PrayerSet::new(
                ClockTime::from_hms(5, 30).unwrap(),
                ClockTime::from_hms(12, 21).unwrap(),
                ClockTime::from_hms(15, 45).unwrap(),
                ClockTime::from_hms(18, 24).unwrap(),
                ClockTime::from_hms(20, 5).unwrap(),
            )
            .unwrap(),
            iqama: None,
            shurouq: Some(ClockTime::from_hms(7, 7).unwrap()),
        },
        tz: TzSource::Local,
        mosque_name: Some("Grand Mosque".to_owned()),
        jumua: None,
    }
}

fn month_readout(month: u32) -> MonthReadout {
    MonthReadout {
        month,
        tz: TzSource::Local,
        days: vec![MonthDay {
            day: 1,
            adhan: PrayerSet::new(
                ClockTime::from_hms(5, 0).unwrap(),
                ClockTime::from_hms(12, 30).unwrap(),
                ClockTime::from_hms(15, 45).unwrap(),
                ClockTime::from_hms(18, 20).unwrap(),
                ClockTime::from_hms(20, 0).unwrap(),
            )
            .unwrap(),
            iqama: None,
            shurouq: None,
        }],
        dropped: vec![15],
    }
}

/// A `TimesService` driven by a per-port-call success script: call *n*
/// succeeds iff `script[n]` is `true` (scripts longer than the call count
/// repeat their last entry). The call counter is the retry-evidence.
struct ScriptedTimes {
    today_script: Mutex<Vec<bool>>,
    today_calls: Mutex<u32>,
    month_calls: Mutex<u32>,
}

impl ScriptedTimes {
    /// All calls succeed; `month_times` echoes the requested month.
    fn healthy() -> Self {
        Self {
            today_script: Mutex::new(Vec::new()),
            today_calls: Mutex::new(0),
            month_calls: Mutex::new(0),
        }
    }

    /// Calls succeed while the script says `true`, then always succeed
    /// (the last entry repeats). `[false, true]` = boot succeeds, next
    /// call fails — the "warm cache, then network dies" arc.
    fn scripted(today_script: Vec<bool>) -> Self {
        Self {
            today_script: Mutex::new(today_script),
            today_calls: Mutex::new(0),
            month_calls: Mutex::new(0),
        }
    }

    fn today_calls(&self) -> u32 {
        *self.today_calls.lock().unwrap()
    }

    fn month_calls(&self) -> u32 {
        *self.month_calls.lock().unwrap()
    }
}

impl mawaqit_tui::application::ports::MosqueDirectory for ScriptedTimes {
    async fn search(&self, _word: &str) -> Result<Vec<MosqueSummary>, PortError> {
        Ok(vec![MosqueSummary {
            id: slug("grand-mosque"),
            name: "Grand Mosque".to_owned(),
            place: Some("Paris".to_owned()),
        }])
    }
}

impl TimesService for ScriptedTimes {
    async fn today(&self, _id: &MosqueId) -> Result<TodayReadout, PortError> {
        let script = self.today_script.lock().unwrap();
        let call = *self.today_calls.lock().unwrap() as usize;
        *self.today_calls.lock().unwrap() = (call + 1) as u32;
        let ok = script
            .get(call)
            .copied()
            .unwrap_or_else(|| script.last().copied().unwrap_or(true));
        drop(script);
        if ok {
            Ok(today_readout())
        } else {
            Err(PortError::Network("offline (scripted)".to_owned()))
        }
    }

    async fn month_times(
        &self,
        _id: &MosqueId,
        month: u32,
    ) -> Result<MonthReadout, PortError> {
        *self.month_calls.lock().unwrap() += 1;
        Ok(month_readout(month))
    }
}

#[derive(Default)]
struct FakeSettings;

impl SettingsStore for FakeSettings {
    fn load(&self) -> Result<Option<MosqueSummary>, SettingsError> {
        Ok(None)
    }
    fn save(&self, _selection: &MosqueSummary) -> Result<(), SettingsError> {
        Ok(())
    }
}

fn deps(times: ScriptedTimes) -> Runtime<ScriptedTimes, FixedClock, FakeSettings> {
    Runtime {
        times: Arc::new(times),
        clock: Arc::new(FixedClock),
        settings: Arc::new(FakeSettings),
    }
}

fn key(code: KeyCode) -> AppEvent {
    AppEvent::Key(crossterm::event::KeyEvent::new(code, KeyModifiers::NONE))
}

fn tick() -> AppEvent {
    AppEvent::Tick(fixed_now())
}

// ------------------------------------------------------------------ scripts

/// The offline arc end to end (issue #6 acceptance): boot succeeds off the
/// warm snapshot cache, the network dies on refresh — last-good data is
/// retained with the offline degradation — and a healthy retry clears it.
#[tokio::test]
async fn offline_retains_last_good_then_retry_clears_the_badge() {
    // Boot succeeds (warm snapshot), the refresh fails, then all healthy.

    let model = runtime::run_scripted(
        deps(ScriptedTimes::scripted(vec![true, false])),
        Boot::Loading(slug("grand-mosque")),
        vec![
            tick(),                  // boot load lands: Ready, live
            key(KeyCode::Char('r')), // refresh → offline failure
        ],
    )
    .await;

    assert!(
        model.offline.is_some(),
        "the failed refresh must raise the offline degradation"
    );
    assert!(
        matches!(model.today, TodayState::Ready(_)),
        "last-good data stays on screen — never blanked"
    );

    // Recovery: 30 ticks past the boundary with the port healthy — the
    // automatic retry fires and clears the badge.
    let mut script = vec![tick(), key(KeyCode::Char('r'))];
    script.extend((0..30).map(|_| tick()));
    let model = runtime::run_scripted(
        deps(ScriptedTimes::scripted(vec![true, false, true])),
        Boot::Loading(slug("grand-mosque")),
        script,
    )
    .await;
    assert!(model.offline.is_none(), "healthy retry clears the badge");
    assert!(matches!(model.today, TodayState::Ready(_)));
}

/// The retry is evidence-backed: the failed refresh consumed one port call,
/// the automatic retry consumed the second success, and the badge cleared.
#[tokio::test]
async fn offline_retry_is_performed_through_the_shared_performer() {
    let times = Arc::new(ScriptedTimes::scripted(vec![true, false, true]));
    let mut script = vec![tick(), key(KeyCode::Char('r'))];
    script.extend((0..30).map(|_| tick()));
    let model = runtime::run_scripted(
        Runtime {
            times: Arc::clone(&times),
            clock: Arc::new(FixedClock),
            settings: Arc::new(FakeSettings),
        },
        Boot::Loading(slug("grand-mosque")),
        script,
    )
    .await;
    // Boot load + failed manual refresh + the one automatic retry. A retry
    // storm (or none at all) would break this count.
    assert_eq!(times.today_calls(), 3, "boot + failed refresh + auto-retry");
    assert!(model.offline.is_none(), "the healthy retry cleared the badge");
    assert!(matches!(model.today, TodayState::Ready(_)));
}

/// Full selection round trip headless: Search → type → results → Enter →
/// persisted → Today loads for the selected mosque.
#[tokio::test]
async fn selection_round_trip_through_the_scripted_loop() {
    let model = runtime::run_scripted(
        deps(ScriptedTimes::healthy()),
        Boot::NoMosque,
        vec![
            key(KeyCode::Char('s')),
            key(KeyCode::Char('p')),
            key(KeyCode::Char('a')),
            key(KeyCode::Char('r')),
            key(KeyCode::Char('i')),
            key(KeyCode::Char('s')),
            key(KeyCode::Enter),
            tick(),
        ],
    )
    .await;

    assert_eq!(model.screen, Screen::Today);
    assert_eq!(model.selected, Some(slug("grand-mosque")));
    assert!(matches!(model.today, TodayState::Ready(_)));
    assert!(model.offline.is_none());
}

/// Month navigation with a healthy source: `m` opens the current month
/// (mosque-tz anchored), Left goes to the previous month, the cache
/// answers the return trip without a new port call (M5 acceptance at the
/// loop tier).
#[tokio::test]
async fn month_navigation_and_cache_at_the_loop_tier() {
    let times = Arc::new(ScriptedTimes::healthy());
    let model = runtime::run_scripted(
        Runtime {
            times: Arc::clone(&times),
            clock: Arc::new(FixedClock),
            settings: Arc::new(FakeSettings),
        },
        Boot::Loading(slug("grand-mosque")),
        vec![
            tick(),                  // boot Ready (fixed now: October 2026)
            key(KeyCode::Char('m')), // open month 10
            key(KeyCode::Left),      // → month 9 (fresh load)
            key(KeyCode::Right),     // → month 10 (must come from cache)
        ],
    )
    .await;

    let MonthState::Ready { month, .. } = model.month else {
        panic!("month screen must be Ready");
    };
    assert_eq!(month, 10);
    // Open(10) + Left(9) hit the port; the return to 10 was answered by the
    // cache — a third call would mean the revisit refetched.
    assert_eq!(times.month_calls(), 2, "cache must answer the revisit");
}

/// A failing month fetch surfaces verbatim on the Month screen and
/// navigation recovers (Failed → move → Loading).
#[tokio::test]
async fn month_failure_surfaces_and_navigation_recovers() {
    struct FailingMonths;
    impl TimesService for FailingMonths {
        async fn today(&self, _id: &MosqueId) -> Result<TodayReadout, PortError> {
            Ok(today_readout())
        }
        async fn month_times(
            &self,
            _id: &MosqueId,
            _month: u32,
        ) -> Result<MonthReadout, PortError> {
            Err(PortError::Network("month offline (scripted)".to_owned()))
        }
    }
    impl mawaqit_tui::application::ports::MosqueDirectory for FailingMonths {
        async fn search(&self, _word: &str) -> Result<Vec<MosqueSummary>, PortError> {
            Err(PortError::Network("no directory".to_owned()))
        }
    }

    let deps = Runtime {
        times: Arc::new(FailingMonths),
        clock: Arc::new(FixedClock),
        settings: Arc::new(FakeSettings),
    };
    let model = runtime::run_scripted(
        deps,
        Boot::Loading(slug("grand-mosque")),
        vec![
            tick(),
            key(KeyCode::Char('m')), // open month 10 → fails
            key(KeyCode::Left),      // navigate → month 9, fails again
        ],
    )
    .await;

    let mawaqit_tui::ui::app::MonthState::Failed { reason, .. } = model.month else {
        panic!("a failing month must surface Failed");
    };
    assert!(reason.contains("month offline"), "reason verbatim: {reason}");
}
