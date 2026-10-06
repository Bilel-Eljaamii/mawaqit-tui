# Spec: M3 Today view with live countdown (issue #3)

Builds on M1/M2. Out of scope: search screen (M4), month view (M5), retry/offline
badge (M6), help screen (M6).

## Requirements

### R1 — Second-resolution domain math (pure, integer-only)
- `DayMoment`: seconds since midnight (0..=86399); `new(u32) -> Option`,
  `from_hms(h, m, s) -> Option`, `from_naive_time(NaiveTime) -> Option` (nanos truncated).
- `ClockTime::seconds()` (minutes × 60).
- `DailyTimes::preferred() -> PrayerSet`: iqama when published, else adhan
  (next-prayer highlight rule).
- `next_prayer_at(now: DayMoment, set) -> NextPrayerAt { name, at, previous,
  is_tomorrow, seconds_remaining, seconds_into_previous, interval_seconds }`:
  same selection semantics as `next_prayer` at second resolution; invariants
  `1 ≤ seconds_remaining ≤ 86400` and `into + remaining == interval` for every
  input (rollover wrap included, degenerate all-equal sets included).

### R2 — TZ-TRUTH conversion (application)
- `day_moment_from_utc(DateTime<Utc>, TzSource) -> DayMoment`: `Mosque(tz)` via
  chrono-tz (DST-correct), `Local` via the system zone.
- `wall_now(tz, clock)` convenience over the `Clock` port.

### R3 — Readout extension
- `TodayReadout.jumua: Option<ClockTime>` from `ConfData.jumua`; parse failure
  degrades to `None` (display-only, same rule as shurouq). `jumua2` ignored.

### R4 — Elm Command/Event loop (ADR-0001 §2, now real)
- `AppEvent { Key, Tick(DateTime<Utc>), TodayLoaded(Result<TodayReadout, AppError>) }`
  and `Command { LoadToday(MosqueId) }` live in `ui::app` (model vocabulary);
  `runtime` imports them — dependency stays inward.
- `AppModel::from_boot(Boot) -> (AppModel, Vec<Command>)`; `Boot { Loading(id),
  NoMosque, Failed(msg) }`. `update` returns fresh commands; the runtime drains,
  spawns each on tokio, posts results back over an mpsc channel.
- Runtime primes one `Tick(now)` before the first draw so frame 1 is live.
- `Tick` carries the instant — no hidden clock reads anywhere.

### R5 — Boot + composition
- CLI: `--mosque <slug>` overrides settings for the session; `--help` prints usage;
  invalid usage exits 2 with stderr text (before any terminal init).
- Without CLI: settings load → `Loading` / `NoMosque` / `Failed` per result.
- main wires `MawaqitAdapter` + `TomlSettings` + `SystemClock` into the runtime.

### R6 — Today screen
- Grid: six rows (Fajr, Shurouq, Dhuhr, Asr, Maghrib, Isha) × adhan + iqama
  columns; Shurouq iqama cell is `—`; next-prayer row highlighted.
- Countdown line + `Gauge` progress for the current interval; `HH:MM:SS` format.
- Header: mosque name, readout date, tz source badge; Jumua shown when present.
- `NoMosque` explains how to configure; `Failed` shows the error verbatim.
- Tests: model transitions in ut (no terminal); render smoke via `TestBackend`.

## EVID table

| # | Claim | Status | Evidence |
|---|-------|--------|----------|
| E1 | gates green on final tree | pending | Phase E `just verify` |
| E2 | RED witnessed | pending | Phase B compile-fail capture |
| E3 | impact disclosures | **measured** | `TodayReadout` CRITICAL/exact → 2 construction sites (both updated in-lane); `AppModel`/`AppEvent` UNKNOWN → text-search resolved (all callers rewritten in-lane) |
| E4 | DST correctness | pending | ct: Paris winter/spring-transition fixtures |
| E5 | detect-changes clean + disclosed | pending | Phase F |
| E6 | live TTY countdown | pending | user visual confirmation (sandbox has no pty; M1 precedent) |
