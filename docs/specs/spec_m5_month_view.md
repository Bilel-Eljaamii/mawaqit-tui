# Spec: M5 Month calendar view (issue #5)

Builds on M1–M4. Out of scope: offline fallback (M6), help screen (M6),
e2e scripted loop (M6).

## Requirements

### R1 — MonthReadout carries the timezone (port change)
- `MonthReadout` gains `tz: TzSource`. The today-row highlight and the
  navigation anchor must follow the mosque timezone (TZ-TRUTH, ADR-0001 §3);
  deriving them from the system clock near month boundaries shows the wrong
  month/day.
- `map_month(adhan, iqama, conf)` sets it via the same `resolve_tz` rule as
  `map_today` (mosque IANA tz, else `Local` fallback).
- `MawaqitAdapter::month_times` joins `conf_data` (client-cached after the
  first call) with the two month fetches.

### R2 — Mosque-tz calendar date (application)
- `application::clock::date_in_utc(DateTime<Utc>, TzSource) -> NaiveDate`:
  DST-correct via chrono-tz, `Local` fallback — the date twin of
  `day_moment_from_utc`.

### R3 — Two-model additions (Elm loop, spec M4 idioms)
- `Screen::Month`; `MonthState { Loading { month }, Ready { month, readout,
  cursor }, Failed { month, reason } }` — no `NoMosque` variant: `m` is inert
  without a selection, so the screen is unreachable without one.
- `AppModel.selected: Option<MosqueId>` (set on boot `Loading(id)` and on
  `SelectionSaved(Ok)`); `AppModel.months: HashMap<(MosqueId, u32),
  MonthReadout>` — the in-session cache.
- `Command::LoadMonth { id, month }`; `AppEvent::MonthLoaded { id, month,
  result }` — applied only when it matches the current `Loading { month }`
  **and** `id == selected` (stale/misaddressed results discarded; same guard
  pattern as search epochs).

### R4 — Opening + navigation
- `m` on the Today screen (only when Today is `Ready` — the mosque tz is the
  month anchor) opens the month containing `date_in_utc(last_now, tz)`;
  inert in every other state (honest: no tz, no anchor).
- Left/Right (with wrap 1→12 / 12→1) navigate from Ready, Loading, and
  Failed alike: a cache hit renders `Ready` **immediately with no command**
  (issue acceptance: "never re-fetches a cached month within a session"); a
  miss goes `Loading` + `Command::LoadMonth`.
- Up/Down/`j`/`k` move the day cursor, clamped. A fresh `Ready` puts the
  cursor on today's row when that day is present, else 0.
- Esc → Today; `s` opens Search from Month too; `q`/Ctrl-C quit (not a
  typing screen — bare `q` quits here, unlike Search).
- `LoadMonth` executed in the runtime like `LoadToday` (spawn + mpsc).

### R5 — Month table view
- One `Table`: day column + 6 adhan columns (fajr, shur, dhuhr, asr, magh,
  isha) +, when any day publishes iqama, 5 `i-`-prefixed iqama columns
  (explained in the info line — 80-column budget: 3 + 11×6 = 69).
- Dropped days are **explicitly listed, not hidden** (issue acceptance): each
  dropped day renders as a dimmed `·` row interleaved at its day number, and
  the info line states the count and numbers verbatim.
- Row styles: cursor row highlighted (next-highlight), today's row accented,
  cursor wins on collision.
- Render smokes via `TestBackend`; transitions headless in ut.

### R6 — Acceptance mapping (issue #5)
- Cache acceptance: ut `navigation_uses_cache_without_refetch` (Ready, zero
  commands) + `stale_month_result_discarded` / `month_loaded_for_other_mosque_discarded`.
- Dropped-day acceptance: ct `month_merge_surfaces_dropped_days` (M2,
  re-run) + ut render `dropped_days_render_as_dimmed_rows` + info line.

## EVID table

| # | Claim | Status | Evidence |
|---|-------|--------|----------|
| E1 | gates green on final tree | **measured** | `just verify` exit 0: nightly fmt, clippy `-D warnings` (all-targets/all-features), ut 80 + ct 21 + fuzz 7 + e2e 2 (ignored), llvm-cov TOTAL 87.59% — new/changed files: `application/clock.rs` 100% lines, `ui/app.rs` 97.58% / fns 100%, `views/month.rs` 96.55%, `mapping.rs` 88.51%; layering clean |
| E2 | RED witnessed | **disclosed** | lane was resumed post-GREEN (WIP found at Phase E after the authoring session ended); the RED state was not captured in any record — honestly unmeasured, not claimed |
| E3 | impact disclosures | **measured** | `MonthReadout` LOW/direct-2, `map_month` LOW/direct-2 (mapping + ct, in-lane); `month_times` trait UNKNOWN → text-search: use_cases LoadMonth, adapter impl, ct FakeTimes (in-lane); `draw` UNKNOWN → runtime + ut renders (in-lane) |
| E4 | cache never refetches | **measured** | ut `month_loaded_caches_and_cursors_today` (cache hit re-renders Ready with zero commands); stale/misaddressed loads discarded (`stale_month_result_discarded`, `month_loaded_for_other_mosque_discarded`); failed month not cached (`month_mismatch_from_source_fails_loudly`) |
| E5 | dropped days explicit | **measured** | ut `month_renders_headers_iqama_and_dropped_marker` + `dropped_days_render_between_present_days` (dimmed `·` rows + `dropped: N` info line); ct `month_merge_surfaces_dropped_days` (M2 re-run) |
| E6 | detect-changes + disclosure | **measured** | 10 files / 79 symbols / 56 flows, risk critical, output complete (no partial/truncated) — disclosed in red-team report; all changed callers resolved in-lane, suite green on final tree |
| E7 | live TTY month navigation | pending | user visual confirmation (no pty; M1/M3/M4 precedent) |
| E8 | ruflo/QE MCP session | **measured** | ruflo swarm `swarm-1791316302957-k4og9l` coordination + memory ledger active this session; GitNexus via CLI fallback; agent-LLM path without provider key (disclosed) |
