//! Wire-shape → domain/VO mapping (ADR-0002 §4). Pure functions: the adapter
//! fetches, these convert. Core times fail loud (`InvalidData`); shurouq
//! degrades to `None`; dropped days pass through verbatim.

use chrono_tz::Tz;
use mawaqit_api::{
    ConfData, DailyIqamaTimes, DailyPrayerTimes, MonthIqamaTimes, MonthTimes, Mosque,
    TodayTimes,
};

use crate::{
    application::ports::{MonthDay, MonthReadout, PortError, TodayReadout, TzSource},
    domain::{
        mosque::{MosqueId, MosqueSummary},
        prayer::{ClockTime, DailyTimes, PrayerSet},
    },
};

fn parse_time(value: &str, field: &'static str) -> Result<ClockTime, PortError> {
    ClockTime::parse_hhmm(value)
        .map_err(|err| PortError::InvalidData(format!("{field}: {err}")))
}

/// Six adhan times → (prayer set, optional shurouq). Shurouq is display-only,
/// so a parse failure there degrades to `None` instead of failing the day.
fn parse_prayer_times(
    times: &DailyPrayerTimes,
) -> Result<(PrayerSet, Option<ClockTime>), PortError> {
    let set = PrayerSet::new(
        parse_time(&times.fajr, "fajr")?,
        parse_time(&times.dhuhr, "dhuhr")?,
        parse_time(&times.asr, "asr")?,
        parse_time(&times.maghrib, "maghrib")?,
        parse_time(&times.isha, "isha")?,
    )
    .map_err(|err| PortError::InvalidData(format!("adhan order: {err}")))?;
    let shurouq = ClockTime::parse_hhmm(&times.shurouq).ok();
    Ok((set, shurouq))
}

fn parse_iqama(times: &DailyIqamaTimes) -> Result<PrayerSet, PortError> {
    PrayerSet::new(
        parse_time(&times.fajr, "iqama.fajr")?,
        parse_time(&times.dhuhr, "iqama.dhuhr")?,
        parse_time(&times.asr, "iqama.asr")?,
        parse_time(&times.maghrib, "iqama.maghrib")?,
        parse_time(&times.isha, "iqama.isha")?,
    )
    .map_err(|err| PortError::InvalidData(format!("iqama order: {err}")))
}

/// Mosque-published IANA timezone, else `Local` (TZ-TRUTH fallback,
/// ADR-0002 §3). The crate already plausibility-filters the raw string;
/// a shape-valid string that misses the tz database still falls back.
fn resolve_tz(conf: &ConfData) -> TzSource {
    match conf.timezone() {
        Some(raw) => raw.parse::<Tz>().map_or(TzSource::Local, TzSource::Mosque),
        None => TzSource::Local,
    }
}

/// `TodayTimes` + `ConfData` → `TodayReadout`.
pub fn map_today(api: &TodayTimes, conf: &ConfData) -> Result<TodayReadout, PortError> {
    let (adhan, shurouq) = parse_prayer_times(&api.adhan)?;
    let iqama = match &api.iqama {
        Some(times) => Some(parse_iqama(times)?),
        None => None,
    };
    // Display-only like shurouq: a parse failure degrades, it does not fail
    // the day. `jumua2` (second Friday slot) is not surfaced in M3.
    let jumua = conf.jumua.as_deref().and_then(|raw| ClockTime::parse_hhmm(raw).ok());
    Ok(TodayReadout {
        date: api.date,
        times: DailyTimes { adhan, iqama, shurouq },
        tz: resolve_tz(conf),
        mosque_name: conf.name.clone(),
        jumua,
    })
}

/// Merge adhan + iqama months by day number.
pub fn map_month(
    adhan: &MonthTimes,
    iqama: &MonthIqamaTimes,
) -> Result<MonthReadout, PortError> {
    let mut days = Vec::with_capacity(adhan.days.len());
    for day in &adhan.days {
        let (set, shurouq) = parse_prayer_times(&day.times)?;
        days.push(MonthDay { day: day.day, adhan: set, iqama: None, shurouq });
    }
    for iq in &iqama.days {
        let Some(row) = days.iter_mut().find(|row| row.day == iq.day) else {
            return Err(PortError::InvalidData(format!(
                "iqama day {} has no adhan day",
                iq.day
            )));
        };
        row.iqama = Some(parse_iqama(&iq.times)?);
    }
    Ok(MonthReadout { month: adhan.month, days, dropped: adhan.dropped.clone() })
}

/// Search results → summaries. A slug-less result fails the whole page:
/// silently shrinking results would be dishonest (ADR-0002 §4).
pub fn map_mosques(api: &[Mosque]) -> Result<Vec<MosqueSummary>, PortError> {
    let mut out = Vec::with_capacity(api.len());
    for mosque in api {
        let Some(raw_slug) = mosque.mosque_id() else {
            return Err(PortError::InvalidData(
                "search result without a slug".to_owned(),
            ));
        };
        let id = MosqueId::parse(raw_slug)
            .map_err(|err| PortError::InvalidData(format!("slug {raw_slug:?}: {err}")))?;
        out.push(MosqueSummary {
            id,
            name: mosque.display_name().to_owned(),
            place: mosque.place(),
        });
    }
    Ok(out)
}
