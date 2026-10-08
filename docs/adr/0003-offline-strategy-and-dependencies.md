# ADR-0003: Offline strategy, snapshot persistence, and dependency additions

- **Status:** Accepted
- **Date:** 2026-10-07
- **Decides:** How mawaqit-tui works without network; where snapshots live;
  which dependencies M6 adds.
- **Related:** [ADR-0001](0001-mawaqit-tui-architecture.md) (layering, TZ-TRUTH),
  [ADR-0002](0002-application-ports-and-settings.md) (ports, settings),
  sibling `mawaqit-api` ADR-0005 (snapshot layer) / ADR-0012 (proxy).

## Context

M6 (issue #6) requires working without network and "feeling finished":
an offline boot with a warm snapshot cache must render full data with a
badge, retries must be bounded and user-controllable, and the e2e tier
must run the full Elm loop headless with fake ports.

The heavy lifting already lives in `mawaqit-api` 0.6.0: `with_disk_cache`
writes/reads per-mosque snapshot envelopes with atomic temp-file writes,
a 40-day freshness window, a same-calendar-year rule, and shared
sanitization on load (its F23/F27). Duplicating any of that in the app
would violate the no-duplication rule (CAMPAIGN.md §3).

## Decision

1. **Snapshots are owned by the crate.** The adapter constructs the
   client with `with_disk_cache(dir)`; `dir` is
   `$cache_dir/mawaqit-tui/snapshots` (`dirs::cache_dir()`, falling back
   to `temp_dir()` when the platform has none — never the config dir,
   which is for the user's *selection*, not derived cache data).
   `MawaqitAdapter::new(cache_dir)` takes it explicitly; tests pass a
   per-test temp dir.
2. **Offline retention is a model concern, not an adapter one.** The
   adapter reports `PortError::Network` verbatim; the `AppModel` keeps
   the last-good `TodayReadout` on failure and raises `offline: Some`
   (the badge). HONESTY: the badge names the degraded state; the data
   shown is labeled by the badge, never faked as live.
3. **Retry policy is deterministic and user-controllable:** the model
   emits `LoadToday` again on `r` (manual refresh, Today screen) and on
   every 30th tick while offline (~30 s at the 1 Hz tick). No exp
   backoff state machine — the 1 Hz loop and the key are the policy,
   and both are model-tested.
4. **Help is an overlay flag, not a screen** in the Screen enum: `?`
   toggles `show_help` over whatever screen is active, so the help
   reference is reachable from every screen without a navigation
   history stack.
5. **The e2e tier drives the real loop headless:** the command-execution
   match is extracted into `runtime::perform` (shared by the live
   runtime and the e2e seam), and `runtime::run_scripted` feeds a
   scripted event stream through boot → commands → use cases → model
   with fake ports. No network, no terminal, no sleeps.
6. **Dependencies:** no new crates in M6. `dirs` (7.0, M1) already
   provides the cache dir; snapshots come from the existing
   `mawaqit-api` dependency. (The `builtin-tor` feature shipped in
   mawaqit-api 0.6.0 stays opt-in and is *not* enabled by the TUI.)

## Consequences

- The mawaqit-api pin rises to `0.6.0` (the promotions release the TUI
  contract builds on: `TodayTimes`, `next_event`, disk TTL).
- `src/infrastructure/snapshot.rs` (a RED-phase stub since M1) is
  implemented as the cache-dir resolver it was reserved for.
- Offline behavior is fully model-tested; the e2e tier proves the loop
  wiring (boot → failure → badge → retry → success) headless.
