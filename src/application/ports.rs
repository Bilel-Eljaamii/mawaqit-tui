//! Ports — the seams infrastructure implements (ADR-0002 §1–§3).
//!
//! RPITIT async traits (no `async-trait`, no `dyn`): futures are `Send` so the
//! runtime can spawn them; use cases and ct-tier fakes are generic over these.
//! No `mawaqit_api` type crosses inward.

use std::future::Future;

use chrono::{DateTime, NaiveDate, Utc};
use chrono_tz::Tz;

use crate::domain::{
    mosque::{MosqueId, MosqueSummary},
    prayer::{ClockTime, DailyTimes, PrayerSet},
};

/// Failure vocabulary for network-backed ports.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PortError {
    #[error("network failure: {0}")]
    Network(String),
    #[error("invalid data: {0}")]
    InvalidData(String),
}

/// Settings failures — corruption is reported, never silently swallowed
/// (ADR-0002 §5).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SettingsError {
    #[error("settings file is corrupt")]
    Corrupt,
    #[error("settings io failure: {0}")]
    Io(String),
}

/// Which timezone governs a day's times. TZ-TRUTH (ADR-0001 §3): the mosque
/// IANA timezone when published; the system-local clock otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TzSource {
    Mosque(Tz),
    Local,
}

/// One day of times, ready for display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TodayReadout {
    pub date: NaiveDate,
    pub times: DailyTimes,
    pub tz: TzSource,
    pub mosque_name: Option<String>,
    /// Friday congregational time when the mosque publishes one (M3).
    pub jumua: Option<ClockTime>,
}

/// One row of a month view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonthDay {
    pub day: u32,
    pub adhan: PrayerSet,
    pub iqama: Option<PrayerSet>,
    pub shurouq: Option<ClockTime>,
}

/// A month of rows; dropped days are surfaced verbatim (honesty, ADR-0002 §4).
/// The tz rides along (spec M5 R1): the month view anchors "today" in the
/// mosque calendar, not the viewer's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonthReadout {
    pub month: u32,
    pub tz: TzSource,
    pub days: Vec<MonthDay>,
    pub dropped: Vec<u32>,
}

/// Search over mosques. RPITIT with `Send` so the runtime can spawn the
/// futures; implementations declare plain `async fn`.
pub trait MosqueDirectory {
    fn search(
        &self,
        word: &str,
    ) -> impl Future<Output = Result<Vec<MosqueSummary>, PortError>> + Send;
}

/// Today + month times for a selected mosque.
pub trait TimesService {
    fn today(
        &self,
        id: &MosqueId,
    ) -> impl Future<Output = Result<TodayReadout, PortError>> + Send;
    fn month_times(
        &self,
        id: &MosqueId,
        month: u32,
    ) -> impl Future<Output = Result<MonthReadout, PortError>> + Send;
}

/// Persisted selection. Sync: plain file I/O, nothing to await.
pub trait SettingsStore {
    fn load(&self) -> Result<Option<MosqueSummary>, SettingsError>;
    fn save(&self, selection: &MosqueSummary) -> Result<(), SettingsError>;
}

/// The single time authority; M3 converts per `TzSource` for countdowns.
pub trait Clock {
    fn now_utc(&self) -> DateTime<Utc>;
}
