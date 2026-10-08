# mawaqit-tui

Prayer times in your terminal: today's schedule with a live countdown, a
searchable mosque directory, and a full month at a glance — with a best-effort
offline mode. Built on [`mawaqit-api`](https://crates.io/crates/mawaqit-api)
(keyless, no account, no API key).

Status: all six planned milestones (domain core → ports/adapters → today
countdown → search & selection → month view → offline & polish) are
implemented on the `dev` branch. Test tiers: 93 unit + 21 component +
10 property + 5 scripted end-to-end, all green under `just verify`.

## Run

```sh
cargo run                 # first boot lands on the search screen
cargo run -- --mosque <slug>
```

Type to search, ↑/↓ to move, Enter selects — the choice is saved to
`$XDG_CONFIG_HOME/mawaqit-tui/config.toml` and every later start opens
straight into today for that mosque.

### Keys

| Where | Keys | Action |
|---|---|---|
| Today | `s` | open mosque search |
| Today | `m` | month view (anchored to the mosque's timezone) |
| Today | `r` | refresh now |
| Today (offline) | auto | re-fetch every ~30 s until the network returns |
| Month | ←/→, ↑/↓ or j/k | change month, move the day cursor |
| Search | type, ↑/↓, Enter | query (debounced), navigate, select |
| Search | `q` | types a literal `q` — quit there is Ctrl-C or Esc then `q` |
| Any | `?` | help overlay (also shows the offline reason, verbatim) |
| Any | `q` / Ctrl-C | quit (`q` except while typing in search) |
| Help | `?` / Esc | close |

## Offline behaviour

Fetches go through a disk-snapshot cache
(`$XDG_CACHE_HOME/mawaqit-tui/snapshots`, maintained by `mawaqit-api`).
When the network fails *after* data was shown, the last-good times stay on
screen under an `OFFLINE` badge and refreshes retry automatically (or on
`r`). A failure with nothing to show is reported honestly instead of
faking data. Months already visited are served from an in-session cache.

## Architecture

Strict layering — `ui` → `application` → `domain`, with `infrastructure`
as the only module that imports `mawaqit_api`. The UI is an Elm-style
state machine: `AppModel::update(event) -> Vec<Command>`, commands
executed by a single `runtime::perform` shared by the live loop and a
scripted headless e2e harness. Time enters the model only as `Tick`
instants; all calendar/countdown math converts through the mosque's IANA
timezone (system-local clock as the documented fallback). Decisions are
recorded in [`docs/adr/`](docs/adr/) — architecture, ports/settings, and
the offline strategy.

## Development

```sh
just verify    # nightly rustfmt · clippy -D warnings · tests · llvm-cov · layering gate
```

`just verify` also enforces the layering rule by grep: no
`use mawaqit_api` outside `src/infrastructure/`. Tests live in `tst/`
(`ut`, `ct`, `e2e`, `fuzz` targets; `#[cfg(test)]` for in-module unit
tests). The property suite covers the strict HH:MM grammar, slug
validation, and prayer-rollover invariants; the e2e tier runs the real
event loop headless with fake ports and port-call counters.

Requires Rust ≥ 1.89 (edition 2024) and a nightly toolchain for
`rustfmt.toml`'s unstable options.

## License

MIT — see [LICENSE](LICENSE). Prayer-time data comes from the
[mawaqit.net](https://mawaqit.net) network via `mawaqit-api`.
