# Spec: M6 Offline fallback & polish (issue #6)

Builds on M1–M5. [ADR-0003](../adr/0003-offline-strategy-and-dependencies.md)
is the decision record; this spec is the behavior contract.

## Requirements

### R1 — Warm snapshots (adapter + main)
- `MawaqitAdapter::new(snapshot_dir: PathBuf)` constructs the client with
  `with_disk_cache` (crate 0.6.0: 40-day TTL + year rule + sanitize-on-load).
- `main` resolves `dirs::cache_dir()/mawaqit-tui/snapshots` (temp fallback);
  `infrastructure::snapshot::cache_dir()` is the total resolver.
- mawaqit-api pin rises to `0.6.0`.

### R2 — Offline retention + badge + retries (model)
- `AppModel.offline: Option<String>` — `Some(reason)` when a Today fetch
  fails *and* last-good data exists. The last-good readout stays on
  screen (never blanked); draw renders the OFFLINE badge.
- Success clears `offline`. A failure with **no** last-good data stays
  `Failed(reason)` (first-boot offline without snapshots: honest error).
- Retries, model-emitted: `r` on Today (manual) and every 30th tick while
  offline (deterministic; ~30 s at the 1 Hz tick). Both emit
  `Command::LoadToday` only when a mosque is selected.
- Month screen: navigation already retries; failed months are not cached.

### R3 — Help overlay (`?`)
- `AppModel.show_help: bool`; `?` toggles it over any screen/state;
  while shown, keys are inert except `?`/`Esc` (close) and Ctrl-C (quit).
- `views/help.rs` renders the single-sourced keybinding table
  (`HELP_ENTRIES`), grouped per screen; the table is the one source the
  tests assert against.

### R4 — Banners + skeletons
- Today `Loading` renders a skeleton (dimmed placeholder rows), not a
  bare message. Search `Querying` keeps its pulse line. Month `Loading`
  renders a skeleton. Failed states keep verbatim reasons (existing).
- Offline badge renders on the Today outer title when `offline.is_some()`.

### R5 — Scripted e2e loop
- `runtime::perform(deps, command) -> AppEvent`: the one command→event
  match, shared by the live runtime (spawn + mpsc) and the e2e seam.
- `runtime::run_scripted(deps, boot, script: Vec<AppEvent>) -> AppModel`:
  boot → loop { drain commands via `perform`, feed results back, apply
  the next scripted event } — full Elm loop, fake ports, no network, no
  terminal, no sleeps.
- e2e scripts: offline boot with warm cache (fail → badge+last-good →
  retry succeeds → badge clears); selection round-trip; month
  navigation with a failing month.

### R6 — Fuzz additions
- HH:MM parse over arbitrary strings succeeds **iff** the strict shape
  (mutant-killing, beyond the existing totality property).
- Slug validation rejects hostile families (`..`, leading/trailing `.`,
  empty, uppercase) and accepts generated valid slugs.

## EVID table

| # | Claim | Status | Evidence |
|---|-------|--------|----------|
| E1 | gates green on final tree | **measured** | `just verify` exit 0: nightly fmt, clippy `-D warnings`, 93 ut + 21 ct + 10 fuzz + 5 e2e, llvm-cov 88.67% lines total, layering clean |
| E2 | offline boot + warm cache renders data + badge | **measured** | e2e `offline_retains_last_good_then_retry_clears_the_badge` + ut `failed_refresh_retains_last_good_and_raises_offline` + render smoke `offline_badge_renders_over_last_good` (badge AND last-good both asserted) |
| E3 | retry policy (r + 30th tick) | **measured** | ut `r_on_ready_refetches_without_flipping_to_loading` / `r_on_failed_goes_loading` / `r_without_selection_is_inert` / `auto_retry_fires_on_the_30th_tick_only` / `auto_retry_requires_offline_and_selection`; loop-tier evidence: e2e pins `today_calls == 3` (boot + failed refresh + one auto-retry) |
| E4 | help overlay reachable everywhere, keys inert | **measured** | ut `help_toggles_from_every_screen` + `help_swallows_keys_except_close_and_quit` (incl. Ctrl-C-quit fix F2) + render smoke `help_overlay_renders_the_keybinding_table` |
| E5 | skeletons + offline badge render | **measured** | ut render smokes `skeletons_render_on_loading_screens` (Today), month skeleton via `draw.rs` same helper; badge smoke above |
| E6 | e2e + fuzz suites green | **measured** | 5 e2e (offline arc, retry evidence, selection round-trip, month nav + cache counters, failing-month recovery); 10 fuzz incl. `parse_validity_iff_strict_shape` (mutant-killing iff) + hostile slug families; regressions file committed |
| E7 | scripted loop drives the real use cases | **measured** | `runtime::perform` is the single command→event match shared by the live loop and `run_scripted` (ADR-0003 §5); e2e fakes count port calls — no network, no terminal, no sleeps |
| E8 | review provenance | **measured** | implementation arrived as parallel-session WIP; reviewed before landing — 6 findings (3 HIGH: sham e2e assertions, Ctrl-C-while-help, missing ut tier), all fixed pre-commit; `docs/reports/red-team-2026-10-08-m6.md` |
| E9 | "first commits pushed to origin" (issue box) | pending | standing Bilel rule: the agent never pushes — Bilel pushes himself |
