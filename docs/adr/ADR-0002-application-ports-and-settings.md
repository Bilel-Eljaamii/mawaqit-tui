# ADR-0002: application ports, adapters, and settings format

- **Status:** Accepted
- **Date:** 2026-10-06
- **Lane:** M2 (issue #2)
- **Amends:** ADR-0001 (§1 layering, concrete port shapes)

## Context

M2 introduces the application layer (ports + use cases) and the first infrastructure
adapters. The seams fixed here are consumed by every later milestone (M3 reads
`TodayReadout`, M4 persists selections, M5 reads `MonthReadout`), and the settings
file format becomes a user-visible compatibility surface on day one.

## Decision

1. **Ports are native async traits** (`async fn` in traits, Rust ≥ 1.75). No
   `async-trait` crate, no `dyn` dispatch — use cases and fakes are generic over the
   port, which keeps ct-tier fakes zero-cost and the layering honest.

2. **Port set (application/ports.rs):**
   - `MosqueDirectory::search(&self, word: &str) -> Result<Vec<MosqueSummary>, PortError>`
   - `TimesService::{today, month_times}` returning **readouts** (application-owned
     types composed of domain VOs): `TodayReadout { date, times: DailyTimes, tz: TzSource, mosque_name }`,
     `MonthReadout { month, days: Vec<MonthDay>, dropped: Vec<u32> }`,
     `MonthDay { day, adhan: PrayerSet, iqama: Option<PrayerSet>, shurouq: Option<ClockTime> }`
   - `SettingsStore::{load, save}` (sync — file I/O, no await inside) over
     `MosqueSummary` as the selection payload
   - `Clock::now_utc() -> DateTime<Utc>` — the single time authority; M3 converts to
     mosque tz for countdowns (TZ-TRUTH)

3. **TzSource is honest, not faked:** `enum TzSource { Mosque(chrono_tz::Tz), Local }`.
   The mosque IANA tz (from `ConfData::timezone()`, already plausibility-filtered by
   the crate) is the source of truth; when absent or unparseable the adapter returns
   `Local` — the system clock is the documented fallback, never a fabricated `UTC`.

4. **Mapping strictness (infrastructure/mawaqit/mapping.rs):**
   - Core prayer times (adhan, iqama): hostile/non-`HH:MM` or order-violating input →
     `PortError::InvalidData` (fail-visible; the crate's contract says this cannot
     happen for a usable day — if it does, we want it loud).
   - `shurouq`: display-only; parse failure degrades to `None` (documented, counted
     nowhere today — revisit if M3 surfaces it).
   - Search results missing a slug → `InvalidData` naming the count (strict; silently
     dropping results would be dishonest).
   - Month merge: iqama days without an adhan day → `InvalidData`; `dropped` is passed
     through verbatim (honesty: surfaced, never hidden).

5. **Settings format:** TOML at `dirs::config_dir()/mawaqit-tui/config.toml`:
   `selected_mosque = { slug, name, place }`. Writes are atomic (sibling `.tmp` file +
   `fs::rename`), parent dirs created on demand. Missing file → `Ok(None)`;
   corrupt/unparsable → `Err(SettingsError::Corrupt)` — callers decide (the UI later
   shows a badge; the app never bricks on a bad config).

6. **Layering is a gate, not a convention:** `just layering` greps for `use mawaqit_api`
   outside `src/infrastructure/` and fails the run. Chained into `just verify`.

## Consequences

- M3 gets countdown inputs without touching the API (readouts carry tz + domain VOs).
- ct tier runs with in-memory fakes only — no network in component tests, ever.
- `serde`/`toml`/`dirs` become live dependencies (baseline, issue #1); `thiserror`
  re-enters in application/infrastructure only — the M1 domain-purity ruling stands.
