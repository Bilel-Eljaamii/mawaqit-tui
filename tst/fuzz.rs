//! Durable property tests — M1 domain core.
//!
//! Everything in this file is `@tier durable` (ADR-113, CAMPAIGN.md Phase B):
//! the properties assert the public contract and must survive a domain
//! reimplementation. Oracles: each property kills mutants that violate the
//! strict-HH:MM, rollover, or slug contracts.

use mawaqit_tui::domain::{
    mosque::MosqueId,
    prayer::{ClockTime, DayMoment, PrayerName, PrayerSet, next_prayer, next_prayer_at},
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

    /// HOSTILE-INPUT-TOTAL, mutant-killing form (M6): parse succeeds IFF
    /// the strict strict-shape predicate holds — bytes 0,1,3,4 digits,
    /// byte 2 a colon, hour ≤ 23, minute ≤ 59. Anything else — unicode
    /// digits, extra seconds, leading `+`, whitespace — must be rejected.
    #[test]
    fn parse_validity_iff_strict_shape(s in ".*") {
        let chars: Vec<char> = s.chars().collect();
        let digits = |cs: &[char]| -> Option<u8> {
            let mut value = 0u8;
            for c in cs {
                let d = c.to_digit(10)? as u8;
                value = value.checked_mul(10)?.checked_add(d)?;
            }
            Some(value)
        };
        let strict = chars.len() == 5
            && chars[2] == ':'
            && (0..5).all(|i| i == 2 || chars[i].is_ascii_digit())
            && digits(&chars[0..2]).is_some_and(|h| h <= 23)
            && digits(&chars[3..5]).is_some_and(|m| m <= 59);
        prop_assert_eq!(
            ClockTime::parse_hhmm(&s).is_ok(),
            strict,
            "parse/strict-shape disagreement on {:?}",
            s
        );
    }

    /// Slug validation rejects the hostile families outright: traversal
    /// (`..`), separators (`/`), query/fragment metacharacters (`?`, `#`),
    /// encoding (`%`), and whitespace — each guaranteed present.
    #[test]
    fn slug_rejects_hostile_families(
        body in "[a-z0-9_-]{1,10}",
        poison in proptest::sample::select(vec![
            "..", "/", "?", "#", "%20", " ",
        ]),
    ) {
        let hostile = format!("{body}{poison}{body}");
        prop_assert!(
            MosqueId::parse(&hostile).is_err(),
            "{:?} must be rejected",
            hostile
        );
    }

    /// Whitespace variants are rejected, not trimmed — and uppercase is
    /// ACCEPTED (the alphabet is `is_ascii_alphanumeric`, unlike the
    /// lowercase mawaqit.net slugs): pinning the actual contract. (R6)
    #[test]
    fn slug_whitespace_rejected_uppercase_accepted(
        body in "[a-z]{1,10}",
        pad in proptest::sample::select(vec![" ", "\t", "\n"]),
    ) {
        let upper = body.to_uppercase();
        prop_assert!(
            MosqueId::parse(&upper).is_ok(),
            "uppercase {upper:?} is within the alphabet"
        );
        let padded_start = format!("{}{}", pad, body);
        let padded_end = format!("{}{}", body, pad);
        prop_assert!(MosqueId::parse(&padded_start).is_err());
        prop_assert!(MosqueId::parse(&padded_end).is_err());
    }

    /// ROLLOVER-CORRECT at second resolution: selection matches the minute
    /// model, remaining stays bounded, and into+remaining == interval for
    /// every valid set — degenerate all-equal sets included. (spec M3 R1)
    #[test]
    fn next_prayer_at_contract(now_s in 0u32..86400u32, set in arb_set()) {
        let now = DayMoment::new(now_s).unwrap();
        let next = next_prayer_at(now, &set);
        prop_assert!(next.seconds_remaining >= 1 && next.seconds_remaining <= 86_400);
        prop_assert_eq!(
            next.seconds_into_previous + next.seconds_remaining,
            next.interval_seconds
        );
        let minute_now =
            ClockTime::from_hms((now_s / 3600) as u8, ((now_s % 3600) / 60) as u8).unwrap();
        let minute_next = next_prayer(minute_now, &set);
        prop_assert_eq!(next.name, minute_next.name);
        prop_assert_eq!(next.is_tomorrow, minute_next.is_tomorrow);
    }
}
