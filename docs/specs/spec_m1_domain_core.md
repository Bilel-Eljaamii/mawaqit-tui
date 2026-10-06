# Spec: M1 domain core (issue #1)

Scope of the M1 lane only: project scaffold, DDD module tree, terminal event-loop
skeleton, and the pure domain core for prayer times. Everything M2+ is explicitly out.

## Requirements

### R1 — ClockTime value object

- Strict `HH:MM` parser mirroring the mawaqit-api ADR-0010 contract.
- Accepts exactly 5 bytes, ASCII digits, `:` separator, hour `00–23`, minute `00–59`.
- Hostile inputs rejected with classified errors: `Malformed`, `NotAsciiDigits`,
  `HourRange`, `MinuteRange` — never a panic.
- `Display` renders zero-padded canonical form; canonical output re-parses to itself.

### R2 — PrayerName

- Enum `Fajr, Dhuhr, Asr, Maghrib, Isha`, ordered (`Ord`) by time of day.
- `Shurouq` is display-only (M3) and never a next-prayer candidate.

### R3 — PrayerSet / DailyTimes

- `PrayerSet`: five adhan times, non-decreasing through the day (validated, else error).
- `DailyTimes`: adhan set + optional iqama set + optional shurouq.

### R4 — next_prayer (ROLLOVER-CORRECT)

- `next_prayer(now, set) -> NextPrayer { name, at, previous, minutes_remaining, is_tomorrow }`.
- First prayer with `at > now` in day order; `previous` = prayer at-or-before `now`
  (before Fajr → `Isha`). At `now == at`, that prayer is current and the next one follows.
- Post-Isha (including `now == isha`): rolls to next-day Fajr, `is_tomorrow = true`,
  `minutes_remaining = 1440 − now + fajr`. Total for every input.
- Integer math only. Progress-ratio display is deferred to M3.

### R5 — MosqueId / MosqueSummary

- `MosqueId` slug newtype: non-empty, ≤ 256 chars, ASCII alnum plus `- _ .`,
  no `..` substring, no leading/trailing `.`. Hostile slugs (`../`, `?`, `#`) rejected.
- `MosqueSummary { id, name, place: Option<String> }`.

### R6 — Event loop skeleton

- Terminal init/teardown with panic-hook restore; 1 s tick; `q` / Ctrl-C quit;
  all other keys inert. Placeholder Today screen renders without panic.

### R7 — Harness

- `tst/ut.rs`: parser, rollover, ordering, slug tests (ephemeral examples tagged
  `// @tier ephemeral`). `tst/fuzz.rs`: durable proptest properties tagged
  `// @tier durable` (round-trip, rollover totality, ordering invariants).
- ct/e2e targets exist with explicit deferred-scope notes (not silently empty).

## EVID table

| # | Claim | Status | Evidence |
| --- | ------- | -------- | ---------- |
| E1 | `cargo +nightly fmt --all -- --check` green | measured | `just fmt` (Phase E) |
| E2 | `cargo clippy --all-targets -- -D warnings` green | measured | `just clippy` (Phase E) |
| E3 | ut parser/rollover/slug suites green | measured | `just test` (Phase E) |
| E4 | fuzz properties green (bounded) | **measured** | 6 passed × 256 cases, `just test` |
| E5 | coverage floor recorded | **measured** | llvm-cov: regions 84.48% / lines 83.63% total. Residual misses: `main.rs` (binary entry) and `runtime.rs` terminal I/O (`run`, guard, panic hook) — headless-untestable, owned by the M6 e2e tier. Floor ratchets UP only. |
| E6 | placeholder screen renders, `q`/Ctrl-C quit | **pending** | render asserted headless via `TestBackend` (ut); live-TTY behavior awaits user visual confirmation |
| E7 | nightly rustfmt available | **measured** | `nightly-x86_64-unknown-linux-gnu` in toolchain list |
| E8 | MSRV 1.89 satisfied | **measured** | stable 1.99.0 active; 1.89.0 toolchain present |
| E9 | memory ledger empty at lane start | **measured** | `memory_search` namespace `mawaqit-tui` → 0 results |
| E10 | GitNexus index refreshed post-scaffold | **measured** | `analyze --index-only` post-scaffold, Phase E |
| E11 | RED witnessed before GREEN | **measured** | `cargo test --test ut` → E0432 unresolved imports on empty stubs, 2026-10-06 |
| E12 | `futures-util` dependency justified | **measured** | ADR-0001 §5 amendment (EventStream `StreamExt`) |
