//! Prayer-time value objects and next-prayer selection (spec M1 R1–R4).
//!
//! Pure domain: integer math only, no clock reads, no floats (ADR-0001 §3).

use core::{fmt, fmt::Display};

/// Five daily prayers in day order. Shurouq is display-only and lives on
/// [`DailyTimes`], never here (spec M1 R2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PrayerName {
    Fajr,
    Dhuhr,
    Asr,
    Maghrib,
    Isha,
}

impl PrayerName {
    /// All prayers in day order.
    pub const ALL: [PrayerName; 5] = [
        PrayerName::Fajr,
        PrayerName::Dhuhr,
        PrayerName::Asr,
        PrayerName::Maghrib,
        PrayerName::Isha,
    ];

    /// Stable machine name, matching the mawaqit field naming.
    pub const fn slug(self) -> &'static str {
        match self {
            PrayerName::Fajr => "fajr",
            PrayerName::Dhuhr => "dhuhr",
            PrayerName::Asr => "asr",
            PrayerName::Maghrib => "maghrib",
            PrayerName::Isha => "isha",
        }
    }
}

/// A wall-clock time at minute granularity, `00:00..=23:59`.
///
/// Strict `HH:MM` contract, mirroring mawaqit-api ADR-0010: ASCII digits
/// only, zero-padded, no alternate spellings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClockTime {
    hour: u8,
    minute: u8,
}

/// Classified parse failures — hostile input yields an error, never a panic
/// (HOSTILE-INPUT-TOTAL).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockParseError {
    Malformed,
    NotAsciiDigits,
    HourRange,
    MinuteRange,
}

impl fmt::Display for ClockParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            ClockParseError::Malformed => "time must be exactly 5 characters HH:MM",
            ClockParseError::NotAsciiDigits => "only ASCII digits are accepted",
            ClockParseError::HourRange => "hour must be 00-23",
            ClockParseError::MinuteRange => "minute must be 00-59",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for ClockParseError {}

impl ClockTime {
    /// Infallible constructor; `None` outside `00:00..=23:59`.
    pub const fn from_hms(hour: u8, minute: u8) -> Option<ClockTime> {
        if hour <= 23 && minute <= 59 { Some(ClockTime { hour, minute }) } else { None }
    }

    /// Strict `HH:MM` parser. Total over arbitrary input.
    pub fn parse_hhmm(s: &str) -> Result<ClockTime, ClockParseError> {
        let bytes = s.as_bytes();
        if bytes.len() != 5 || bytes[2] != b':' {
            return Err(ClockParseError::Malformed);
        }
        for (i, &byte) in bytes.iter().enumerate() {
            if i != 2 && !byte.is_ascii_digit() {
                return Err(ClockParseError::NotAsciiDigits);
            }
        }
        let hour = (bytes[0] - b'0') * 10 + (bytes[1] - b'0');
        let minute = (bytes[3] - b'0') * 10 + (bytes[4] - b'0');
        if hour > 23 {
            return Err(ClockParseError::HourRange);
        }
        if minute > 59 {
            return Err(ClockParseError::MinuteRange);
        }
        Ok(ClockTime { hour, minute })
    }

    pub const fn hour(self) -> u8 {
        self.hour
    }

    pub const fn minute(self) -> u8 {
        self.minute
    }

    /// Minutes since midnight.
    pub const fn minutes(self) -> u32 {
        self.hour as u32 * 60 + self.minute as u32
    }
}

impl Display for ClockTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}", self.hour, self.minute)
    }
}

/// Five adhan times, non-decreasing through the day (spec M1 R3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrayerSet {
    fajr: ClockTime,
    dhuhr: ClockTime,
    asr: ClockTime,
    maghrib: ClockTime,
    isha: ClockTime,
}

/// The five times must be non-decreasing through the day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderingViolation;

impl fmt::Display for OrderingViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("prayer times must be non-decreasing through the day")
    }
}

impl std::error::Error for OrderingViolation {}

impl PrayerSet {
    pub fn new(
        fajr: ClockTime,
        dhuhr: ClockTime,
        asr: ClockTime,
        maghrib: ClockTime,
        isha: ClockTime,
    ) -> Result<PrayerSet, OrderingViolation> {
        if !(fajr <= dhuhr && dhuhr <= asr && asr <= maghrib && maghrib <= isha) {
            return Err(OrderingViolation);
        }
        Ok(PrayerSet { fajr, dhuhr, asr, maghrib, isha })
    }

    pub const fn get(self, name: PrayerName) -> ClockTime {
        match name {
            PrayerName::Fajr => self.fajr,
            PrayerName::Dhuhr => self.dhuhr,
            PrayerName::Asr => self.asr,
            PrayerName::Maghrib => self.maghrib,
            PrayerName::Isha => self.isha,
        }
    }
}

/// One day of times: the adhan set, optional iqama set, optional shurouq
/// (display-only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DailyTimes {
    pub adhan: PrayerSet,
    pub iqama: Option<PrayerSet>,
    pub shurouq: Option<ClockTime>,
}

/// The next prayer and where `now` sits in the day (spec M1 R4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NextPrayer {
    pub name: PrayerName,
    pub at: ClockTime,
    /// Prayer whose interval `now` falls in; before Fajr this is `Isha`
    /// (yesterday's tail), at `now == at` the prayer just began.
    pub previous: PrayerName,
    pub minutes_remaining: u32,
    /// True when the next prayer is tomorrow's Fajr (post-Isha).
    pub is_tomorrow: bool,
}

const MINUTES_PER_DAY: u32 = 1440;

/// First prayer strictly after `now`; post-Isha (including `now == isha`)
/// rolls over to tomorrow's Fajr. Total over all inputs; integer math only
/// (ROLLOVER-CORRECT, spec M1 R4).
pub const fn next_prayer(now: ClockTime, set: &PrayerSet) -> NextPrayer {
    let mut previous = PrayerName::Isha;
    let mut i = 0;
    while i < PrayerName::ALL.len() {
        let name = PrayerName::ALL[i];
        let at = set.get(name);
        if at.minutes() > now.minutes() {
            return NextPrayer {
                name,
                at,
                previous,
                minutes_remaining: at.minutes() - now.minutes(),
                is_tomorrow: false,
            };
        }
        previous = name;
        i += 1;
    }
    let fajr = set.get(PrayerName::Fajr);
    NextPrayer {
        name: PrayerName::Fajr,
        at: fajr,
        previous: PrayerName::Isha,
        minutes_remaining: MINUTES_PER_DAY - now.minutes() + fajr.minutes(),
        is_tomorrow: true,
    }
}
