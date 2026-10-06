//! System clock adapter — the production `Clock` (ADR-0002 §2).

use chrono::Utc;

use crate::application::ports::Clock;

/// Reads the system UTC clock. M3 converts per `TzSource`; test doubles for
/// countdown logic live in the ct tier via the `Clock` port.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_utc(&self) -> chrono::DateTime<Utc> {
        Utc::now()
    }
}
