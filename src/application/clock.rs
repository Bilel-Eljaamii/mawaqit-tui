//! TZ-TRUTH wall-clock conversion (spec M3 R2).

use chrono::{DateTime, Local, Utc};

use crate::{application::ports::TzSource, domain::prayer::DayMoment};

/// Convert a UTC instant into the day-moment of the governing timezone.
/// `Mosque(tz)` is DST-correct via chrono-tz; `Local` uses the system zone
/// (the documented TZ-TRUTH fallback, never a fabricated zone).
pub fn day_moment_from_utc(instant: DateTime<Utc>, tz: TzSource) -> DayMoment {
    match tz {
        TzSource::Mosque(tz) => {
            let naive = instant.with_timezone(&tz).time();
            // Invariant: chrono's num_seconds_from_midnight() < 86_400, so the
            // constructor cannot fail here (internal invariant, not input).
            DayMoment::from_naive_time(naive).expect("chrono wall time fits in a day")
        }
        TzSource::Local => {
            let naive = instant.with_timezone(&Local).time();
            DayMoment::from_naive_time(naive).expect("chrono wall time fits in a day")
        }
    }
}
