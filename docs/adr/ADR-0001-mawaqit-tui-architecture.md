# ADR-0001: mawaqit-tui architecture — layering, event loop, time truth

- **Status:** Accepted
- **Date:** 2026-10-06
- **Lane:** M1 (issue #1)
- **Deciders:** Bilel Eljaamii, campaign lead agent

## Context

mawaqit-tui is a terminal prayer-times companion built on the [`mawaqit-api`](https://github.com/Bilel-Eljaamii/mawaqit-api)
crate (keyless, async, reqwest + tokio). The workspace starts greenfield with a fixed
harness (`tst/{ut,ct,e2e,fuzz}`), edition 2024, MSRV 1.89, and the CAMPAIGN.md landing
process. Later milestones (M2–M6) add use cases, adapters, views, and offline snapshots;
M1 fixes the seams they must fit into.

## Decision

1. **Clean-architecture layering, dependencies point inward:**
   `ui → application → domain`; `infrastructure` implements `application` ports.
   Only `src/infrastructure/` may import `mawaqit_api`. The domain layer stays pure:
   `std` + `chrono`/`chrono-tz` only — no tokio, no ratatui, no I/O.

2. **Elm-style Command/Event loop.** The UI never blocks. `AppModel::update(Event) -> Vec<Command>`
   is a pure state transition; the runtime (`src/runtime.rs`) executes Commands on tokio and
   feeds results back as Events (`tokio::select!` over crossterm `EventStream`, a 1 s tick,
   and the command result mpsc). The model is headless-testable.

3. **TZ-TRUTH.** The mosque timezone (from `mawaqit_api::ConfData::timezone()`) is the source
   of truth for all countdown/next-prayer math. The local clock is only a documented fallback
   when the mosque publishes no timezone. Domain functions take `now` as an argument; no
   `SystemTime`, RNG, or floats in the domain.

4. **Terminal lifecycle trio.** `runtime.rs` owns init/teardown (raw mode + alternate screen)
   with a panic hook that ALWAYS restores the terminal, even on panic. Unknown keys are inert.

5. **Dependency baseline (pinned in issue #1):** `ratatui`, `crossterm` (event-stream),
   `tokio` (rt-multi-thread, macros, time, sync), `mawaqit-api 0.5`, `chrono`, `chrono-tz`,
   `serde`, `toml`, `dirs`, `thiserror`; dev-deps `proptest`, `tempfile`.
   Any new dependency requires an ADR line.
   - **Amendment (M1):** `futures-util` (no default features, `std` only) — required for
     `StreamExt::next` over crossterm's `EventStream` in the event loop; `tokio` ships no
     `StreamExt`, and the transitive `futures-util` already in-tree (via reqwest) is reused
     rather than duplicated.

6. **Keybinding parity.** Every binding lives in one single-sourced table rendered by the
   help screen (`?`) from M6 onward; M1 fixes only `q` / Ctrl-C to quit.

7. **House invariants (enforced in review and tests):** KEYLESS-PRIVACY,
   HOSTILE-INPUT-TOTAL (external input yields `Err`/`None`, never panic — no
   `unwrap`/`expect`/unchecked indexing on external data paths), ROLLOVER-CORRECT
   (post-Isha → next-day Fajr), HONESTY, LAYERING. `#![forbid(unsafe_code)]` at the
   crate root.

## Consequences

- Domain logic is testable without a terminal or network (ut tier).
- Use cases get fake-able seams (ct tier, M2) because ports live in `application`.
- The mawaqit-api contract (strict `HH:MM`, `SNAPSHOT_MAX_AGE_DAYS`, `MonthTimes::dropped`)
  is consumed via adapters only, so a crate upgrade cannot ripple into UI code.
- Cost: slightly more indirection than a flat ratatui app; justified by the six-milestone arc.
