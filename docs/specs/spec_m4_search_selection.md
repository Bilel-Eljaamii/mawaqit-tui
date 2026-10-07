# Spec: M4 Mosque search & saved selection (issue #4)

Builds on M1–M3. Out of scope: month view (M5), offline fallback (M6), help
screen (M6), e2e scripted loop (M6).

## Requirements

### R1 — Two-screen model
- `AppModel` gains `screen: Screen { Today, Search }`; the today field is
  renamed `state` → `today`. Boot **never** lands on `TodayState::NoMosque`
  (issue: "no saved mosque → land on Search"), but the variant is retained as
  the Esc-back destination when no selection has ever been made — its guidance
  becomes "no mosque selected · press s to search" (an Esc that lands on a
  fake "loading…" screen would violate HONESTY).
- `Boot::Failed(msg)` still lands on Today showing the config error verbatim;
  `Boot::Loading(id)` unchanged.
- Esc on Search returns to Today **whatever its state is** (Ready, Loading,
  NoMosque, or a boot-time Failed message); `s` on Today (any state) opens
  Search, keeping the previous query/results.

### R2 — Query editing + debounced search
- `SearchState { Idle, Querying { epoch }, Results { epoch, items, cursor },
  Saving(MosqueSummary), SaveFailed { summary, reason }, Failed(String) }`.
- Printable characters (no CONTROL modifiers) append to `model.query`;
  Backspace pops a char. Every edit bumps a monotonically increasing
  `search_epoch` and emits `Command::Search { epoch, query }` — except a
  trim-empty query, which goes `Idle` and emits nothing (mirrors the
  `SearchMosques` short-circuit).
- Debounce lives in the **runtime** (model stays pure): each `Search` command
  records its epoch in a shared `AtomicU64`, sleeps 300 ms, and only calls
  `SearchMosques` if it is still the latest epoch. Stale sleepers make no port
  call.
- `AppEvent::SearchLoaded(epoch, Result<Vec<MosqueSummary>, AppError>)`:
  applied only when the epoch matches the current `Querying`; stale results
  are discarded silently (the newer query owns the screen). Ok →
  `Results { cursor: 0 }` (possibly empty — the view says "no mosques
  found"); Err → `Failed` with the error verbatim.
- On the Search screen bare `q` is a **literal query character**, not quit —
  the user is typing. Quit from Search: Ctrl-C, or Esc then `q` on Today.

### R3 — Selection flow
- Up/Down move `cursor` clamped to `items.len()` (empty results: inert).
- Enter on non-empty `Results`: emit `Command::SaveSelection(summary.clone())`
  and enter `Saving(summary)`. Enter on `Idle`/`Querying`/`Failed`/empty
  `Results` is inert.
- `AppEvent::SelectionSaved(Ok(()))`: the domain event
  `DomainEvent::MosqueSelected { id }` is recorded (drained via
  `take_events()`; consumers land in M6 — disclosed as deliberately minimal),
  the app jumps to `Screen::Today` with `TodayState::Loading` and emits
  `Command::LoadToday(id)`. This is the only point where `MosqueSelected`
  fires: the event asserts a *persisted* selection, not an attempt.
- `SelectionSaved(Err(e))`: `SaveFailed { summary, reason: e.to_string() }` —
  the error shows verbatim and Enter **retries** the save with the same
  summary (results are not destroyed by a failed disk write; HONESTY).
- Settings save executes synchronously in the drain loop (sync port by
  design, ADR-0002 §5); its result is posted back as an event like any other.

### R4 — Runtime ports
- `Runtime<T, C, S> { times, clock, settings }` with
  `T: TimesService + MosqueDirectory`, `S: SettingsStore`. Single adapter
  (`MawaqitAdapter` implements both directory traits).
- main resolves the settings path once; when the platform has no config dir,
  a `NullSettings` adapter (every operation fails with an honest Io error)
  keeps the composition total — a later save surfaces verbatim in
  `SaveFailed` instead of panicking or pretending.

### R5 — Search screen
- Query line (`> {query}`), status line (idle hint / "searching…" / result
  count / "no mosques found" / error), result list `name — place` with the
  cursor row highlighted, footer keys hint.
- Today footer gains `s · search`.
- Render smoke via `TestBackend` in ut; model transitions headless in ut.

### R6 — Acceptance mapping (issue #4)
- "Restart opens the saved mosque without re-searching" — boot path M3 R5
  (settings → `Boot::Loading`), unchanged; covered by
  `ct::settings_roundtrip_preserves_selection`.
- "Search → select → persist round-trip covered by ct (fake ports)" —
  `ct::search_select_save_load_today_roundtrip` (landed M2).

## EVID table

| # | Claim | Status | Evidence |
|---|-------|--------|----------|
| E1 | gates green on final tree | **measured** | `just verify` exit 0: nightly fmt, clippy `-D warnings`, 66 ut + 19 ct + 7 fuzz, llvm-cov TOTAL 85.69% (new `ui/app.rs` 97.44% lines / 100% fns, `views/search.rs` 96.88%), layering clean |
| E2 | RED witnessed | **measured** | 2026-10-07: `cargo test --test ut` exit 101 — E0432 `Screen`/`SearchState`, E0609 `screen`/`today`/`search`/`query`, E0599 `Command::Search`/`AppEvent::SearchLoaded` |
| E3 | impact disclosures | **measured** | index refreshed (526 nodes); `AppModel`/`TodayState`/`Command`/`AppEvent`/`Boot`/`draw` upstream impact UNKNOWN/0-direct → resolved by text search: callers in main.rs, runtime.rs, draw.rs, ui/mod.rs, views/today.rs, e2e.rs, ut.rs — all rewritten in-lane; `run` CALLS main (proc_16) |
| E4 | debounce drops stale queries | **partial** | model-side stale-epoch discard: ut-measured; runtime 300 ms gate (sleep + `AtomicU64` compare): implemented but not harness-tested — `runtime::run` needs a live terminal; the M6 e2e scripted loop is the follow-up |
| E5 | ct roundtrip acceptance | **measured** | `ct::search_select_save_load_today_roundtrip` + `settings_roundtrip_preserves_selection` + `null_settings_fails_honestly` (19 ct total, zero network) |
| E6 | detect-changes clean + disclosed | **measured** | HIGH (9 files / 38 symbols / 14 flows), complete output — disclosed in red-team report: all resolved callers in-lane, suite green on final tree; index re-analyzed (624 nodes) |
| E7 | live TTY search→select→persist | pending | user visual confirmation (sandbox has no pty; M1/M3 precedent) |
| E8 | ruflo/QE MCP session | **measured** | ruflo + GitNexus MCP servers not registered in this session's tool list; GitNexus satisfied via documented CLI fallback; ruflo task ledger skipped — disclosed |
