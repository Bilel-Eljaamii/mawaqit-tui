//! Durable property tests — M1 domain core.
//!
//! Everything in this file is `@tier durable` (ADR-113, CAMPAIGN.md Phase B):
//! the properties assert the public contract and must survive a domain
//! reimplementation. Oracles: each property kills mutants that violate the
//! strict-HH:MM, rollover, or slug contracts.

use mawaqit_tui::domain::{
    mosque::MosqueId,
    prayer::{ClockTime, PrayerName, PrayerSet, next_prayer},
};
use proptest::prelude::{TestCaseError, *};

fn arb_clock() -> impl Strategy<Value = ClockTime> {
    (0u8..=23, 0u8..=59).prop_map(|(h, m)| ClockTime::from_hms(h, m).unwrap())
}

fn arb_set() -> impl Strategy<Value = PrayerSet> {
    prop::collection::vec(arb_clock(), 5).prop_map(|mut v| {
        v.sort();
        PrayerSet::new(v[0], v[1], v[2], v[3], v[4]).unwrap()
    })
}

/// Slugs made of dot-separated segments: no `..`, no leading/trailing `.`.
fn arb_slug() -> impl Strategy<Value = String> {
    prop::collection::vec("[a-z0-9][a-z0-9_-]{0,20}", 1..=6)
        .prop_map(|segs| segs.join("."))
}

proptest! {
    /// Round-trip: every valid time survives display → parse unchanged. (R1)
    #[test]
    fn parse_display_roundtrip(t in arb_clock()) {
        prop_assert_eq!(ClockTime::parse_hhmm(&t.to_string()), Ok(t));
    }

    /// Canonical display is exactly the zero-padded HH:MM of (h, m). (R1)
    #[test]
    fn display_is_canonical(h in 0u8..=23, m in 0u8..=59) {
        let t = ClockTime::from_hms(h, m).unwrap();
        prop_assert_eq!(t.to_string(), format!("{h:02}:{m:02}"));
    }

    /// HOSTILE-INPUT-TOTAL: parse never panics over arbitrary strings. (R1)
    #[test]
    fn parse_is_total(s in ".*") {
        let _ = ClockTime::parse_hhmm(&s);
    }

    /// ROLLOVER-CORRECT: next_prayer selects the first strictly-after prayer,
    /// else tomorrow's Fajr with exact remaining math. (R4)
    #[test]
    fn next_prayer_matches_contract(now in arb_clock(), set in arb_set()) {
        let next = next_prayer(now, &set);
        let times = [
            (PrayerName::Fajr, set.get(PrayerName::Fajr)),
            (PrayerName::Dhuhr, set.get(PrayerName::Dhuhr)),
            (PrayerName::Asr, set.get(PrayerName::Asr)),
            (PrayerName::Maghrib, set.get(PrayerName::Maghrib)),
            (PrayerName::Isha, set.get(PrayerName::Isha)),
        ];
        match times.iter().find(|(_, at)| *at > now) {
            Some((name, at)) => {
                prop_assert!(!next.is_tomorrow);
                prop_assert_eq!(next.name, *name);
                prop_assert_eq!(next.at, *at);
                prop_assert_eq!(next.minutes_remaining, at.minutes() - now.minutes());
            }
            None => {
                let fajr = set.get(PrayerName::Fajr);
                prop_assert!(next.is_tomorrow);
                prop_assert_eq!(next.name, PrayerName::Fajr);
                prop_assert_eq!(next.minutes_remaining, 1440 - now.minutes() + fajr.minutes());
            }
        }
    }

    /// minutes_remaining stays in 1..=1440 for any valid set. (R4)
    #[test]
    fn remaining_is_bounded(now in arb_clock(), set in arb_set()) {
        let next = next_prayer(now, &set);
        prop_assert!(next.minutes_remaining >= 1 && next.minutes_remaining <= 1440);
    }

    /// Slug round-trip: accepted slugs keep their exact string. (R5)
    #[test]
    fn slug_roundtrip(s in arb_slug()) {
        let id = match MosqueId::parse(&s) {
            Ok(id) => id,
            Err(err) => return Err(TestCaseError::fail(format!("slug {s:?} rejected: {err}"))),
        };
        prop_assert_eq!(id.as_str(), s);
    }
}
