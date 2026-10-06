# Spec: M2 application ports, use cases, infrastructure adapters (issue #2)

Builds on M1 (spec_m1_domain_core). Out of scope: any UI change (M3), month
navigation (M5), disk snapshots (M6).

## Requirements

### R1 — Ports (application/ports.rs, ADR-0002 §2)
- `MosqueDirectory`, `TimesService`, `SettingsStore`, `Clock` as native async/sync
  traits; no `async-trait` crate; fakes are generic-implementing structs.
- Readouts carry domain VOs + `TzSource` — no API types leak inward of infrastructure.

### R2 — Use cases (application/use_cases.rs)
- `SearchMosques` (trims input; empty query short-circuits without a port call),
  `LoadToday`, `LoadMonth` (validates month 1..=12 before the port call),
  `SaveSelection` (sync). All return `AppError` (wraps `PortError`/`SettingsError`).

### R3 — Mawaqit adapter + mapping (infrastructure/mawaqit/)
- Wraps `MawaqitClient::new().with_timeouts(5 s connect, 15 s request)`.
- `search` maps `Mosque` → `MosqueSummary` (`mosque_id()` slug, `display_name()`,
  `place()`); any slug-less entry → `InvalidData` (ADR-0002 §4).
- `today` merges `today()` + `conf_data()` (tz via `raw["timezone"]`, name) →
  `TodayReadout`; tz absent/unparseable → `TzSource::Local`.
- `month_times` merges `month()` + `month_iqama()` by day number → `MonthReadout`;
  `dropped` passed through verbatim; orphan iqama days → `InvalidData`.
- Core-time parse/order failures → `InvalidData`; shurouq failure degrades to `None`.

### R4 — Settings (infrastructure/settings.rs)
- `TomlSettings` at `dirs::config_dir()/mawaqit-tui/config.toml`, format per
  ADR-0002 §5; atomic write; missing → `Ok(None)`; corrupt → `Err(Corrupt)`.

### R5 — Layering + ct tier
- `use mawaqit_api` only under `src/infrastructure/` — enforced by `just layering`
  inside `just verify`.
- `tst/ct.rs`: in-memory fakes, zero network. Durable-tagged: round-trip
  (search → select → save → load today), empty-query short-circuit, month
  validation, tz fallback, settings round-trip/corruption. Ephemeral-tagged:
  mapping fixtures (api shapes are implementation detail).

## EVID table

| # | Claim | Status | Evidence |
|---|-------|--------|----------|
| E1 | fmt/clippy/test/cov green on final tree | **measured** | `just verify` 2026-10-06: fmt clean, clippy 0 findings, tests green |
| E2 | ct tier green, zero network | **measured** | **14 passed** (3 async use-case flows, 2 settings, 9 mapping/clock) — fakes only |
| E3 | layering gate green | **measured** | `just layering`: `use mawaqit_api` only under `src/infrastructure/` |
| E4 | RED witnessed before GREEN | **measured** | `cargo test --test ct` → E0432 on all five import surfaces, 2026-10-06 |
| E5 | detect-changes clean before commits | **measured** | Phase F, scope=all, no partial/truncated |
| E6 | impact run on touched symbols | **measured** | `DailyTimes` upstream → LOW, 1 caller (ut), exact epistemic |
| E7 | ledger rulings applied | **measured** | domain untouched by thiserror; M1 floor carried forward |
| E8 | coverage floor recorded | **measured** | Residual misses beyond M1's: `MawaqitAdapter` network impls (untestable without network — fake-server e2e owned by M6). Floor ratchets UP only. |
| E9 | durable-first tiers tagged | **measured** | ct: roundtrip/empty-query/month-validation/tz-fallback/settings ×2 + clock `@tier durable`; mapping fixtures `@tier ephemeral` |
