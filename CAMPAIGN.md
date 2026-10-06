# Ultimate implementation campaign — issue #xx (mawaqit)

Plan and execute the COMPLETE landing campaign for GitHub issue **#xx** in **Bilel-Eljaamii/mawaqit-tui** (`/home/bilel/Workspace/mawaqit-tui`). Full arc, no shortcuts: **recon → impact → ADR/spec → worktree → TDD RED→GREEN → red team → QE honest review → fix ALL findings → every gate green → land on LOCAL dev.** Ground every decision in LOCAL ground truth — cite local file paths first; the sibling checkouts `~/Workspace/mawaqit-api` and `~/Workspace/mawaqit-desktop` are LOCAL GitNexus-indexed sources of crate truth. docs.rs/crates.io only as a last-resort supplement (via `web_reader`). Never push to any remote (standing Bilel rule).

## 0. Context loading — before any tool that writes

1. `gh issue view #xx --repo Bilel-Eljaamii/mawaqit-tui` + the milestone issues it references (#1–#6 of the DDD campaign; gh broken → memory-ledger fallback; retry gh for the evidence comment).
2. Memory ledger — `memory_search` (ruflo MCP), namespace `mawaqit-tui`: prior lanes, HELD/BLOCKED constraints (X←Y), standing rulings, scope authority. State cleared/uncleared blockers explicitly. Invalidate superseded rulings with `memory_delete`.
3. Local ground truth, in order of authority:
   - `AGENTS.md` — GitNexus MUST/NEVER rules (impact before edits, detect-changes before commits, `UNKNOWN ≠ unused`).
   - `tst/` harness contract — `ut.rs` (domain unit), `ct.rs` (use cases, fake ports, no network), `e2e.rs` (scripted full loop, headless), `fuzz.rs` (proptest). Every tier is one integration-test target + a module dir.
   - `rustfmt.toml` (edition 2024, max_width 90, unstable opts → nightly fmt), `clippy.toml` (MSRV 1.89, cognitive-complexity 20, too-many-args 8).
   - `docs/specs/**`, `docs/adr/**` (bootstrap the skeleton if this is the first landed lane). Pin exact clause numbers for every standard claim.
   - Sibling crate contracts: `~/Workspace/mawaqit-api/src/{client,models,disk,error,time}.rs` — strict `HH:MM` (ADR-0010), `SNAPSHOT_MAX_AGE_DAYS` staleness + year-boundary refusal (F27), `MonthTimes::dropped` honesty, keyless/privacy-first ethos.
4. Current state: what exists, what's deferred, sibling-lane patterns to reuse (mawaqit-desktop conventions for ADR/naming style).

## 1. Phase A — Impact, architecture, worktree

- GitNexus `impact(upstream)` on EVERY symbol to touch (`--repo mawaqit-tui`; cross-repo contract checks against `--repo mawaqit-api`). UNKNOWN ≠ unused — text-search resolve. Never edit before impact; warn HIGH/CRITICAL; treat `partial:`/`truncated:` as unresolved.
- Worktree: no `justfile` yet → the first lane bootstraps one (`just worktree-start/stop`, `just verify` = fmt + clippy + test + coverage), else plain `git worktree add ../mawaqit-tui-<lane> -b feat/<lane>`; symlink `.gitnexus/`; `cargo check --workspace` warm; confirm `cargo +nightly fmt` availability (rustfmt.toml uses unstable opts).
- Draft ADR + `docs/specs/spec_<feat>.md` with EVID table (measured vs pending). Check `docs/adr/` latest number for collisions (first-landed keeps; renumber). An ADR line is REQUIRED for: any new dependency, the timezone strategy, the persistence format, any public module reshaping.
- Flip source spec §Deferred entries to "LANDED → new spec".

## 2. Phase B — TDD RED (witness the failure)

- **Invoke the `qe-test-generation` skill for the whole tier** (agent fallback: `qe-test-architect` — spawn likely fails on this account, so run its method inline). Generate tests **durable-first** (ADR-113):
  - **Durable (always, ≥1 per public target, tagged `// @tier durable`):** invariants, contracts, proptest properties at the module boundary — a `mosque-tz` rewrite or storage swap must not invalidate them. Apply the language-swap heuristic: if reimplementing the module in another language breaks the test, it sits at the wrong boundary — push it up.
  - **Ephemeral (red-green loop, tagged `// @tier ephemeral`):** happy-path examples and error paths; delete freely. A mock-call/interaction assertion may NEVER be a target's only test.
  - **Oracles:** every generated test must kill mutants — happy-path-only tests that kill zero mutants are rejected before they're committed.
- ut/ct/e2e tiers per the `tst/` contract; fuzz tier = proptest in `tst/fuzz.rs` (the natural home of the durable tier). Hostile suite hygiene: no `#[ignore]`, no silently-empty suites — every new pure fn and every exported AppModel transition gets a test; RED = compile-fail or assertion-fail, witnessed before GREEN.
- Branch matrices from the model: iqama present/absent, mosque-tz present/absent (fallback path), online/offline/degraded, first-run vs saved-mosque, snapshot fresh/stale/wrong-year.
- KATs from LOCAL standard clauses: `HH:MM` golden vectors (hostile: `7:5`, `24:00`, `99:99`, unicode digits); rollover vectors (post-Isha → next-day Fajr; 00:05 inside tomorrow's Fajr window); iqama-offset resolution ("+600"-style); `SNAPSHOT_MAX_AGE_DAYS` + December-snapshot-never-answers-January pins.
- Determinism pins: same inputs ⇒ same Model state/rendered bytes; no RNG/clock in domain (inject `Clock`).

## 3. Phase C — GREEN

- `#![forbid(unsafe_code)]` at the crate root. Invariants (house pillars for mawaqit):
  - **KEYLESS-PRIVACY** — no API key, no account, no telemetry/PII ever leaves the machine; logs clean.
  - **HOSTILE-INPUT-TOTAL** — hostile page JSON, settings, slugs, snapshots yield `Err`/`None`, never panic; no `unwrap`/`expect`/unchecked indexing on external data.
  - **TZ-TRUTH** — the mosque timezone (confData) is the countdown source of truth; local clock only as a documented fallback.
  - **ROLLOVER-CORRECT** — day and year boundaries honored end to end.
  - **HONESTY** — offline ≠ faked live data, loading ≠ done, dropped days surfaced, measured ≠ pending, failures counted not swallowed.
  - **LAYERING** — ui → application → domain; only `infrastructure` imports `mawaqit_api`; domain stays pure (std + chrono/chrono-tz).
- Compose crate surfaces — NEVER duplicate mawaqit-api logic in the app (no re-parsing pages, no parallel time math where the crate already provides it). House shape: deterministic tokio-free *Core (domain + AppModel, fully headless-testable) + tokio host (`runtime.rs`: crossterm EventStream + 1s tick + command mpsc via `tokio::select!`) + terminal start/stop/status trio (init/teardown + panic hook that ALWAYS restores) + Elm Command/Event loop + bounded channels + no global mutable state.
- Deterministic models: integer math on `NaiveTime`/`Duration`; wide math before narrowing; no floats/RNG/system-clock reads in domain.
- Keybinding parity: every binding lives in ONE single-sourced table rendered by the help screen (`?`); unknown keys are inert, never crash; parseable ⇒ documented.
- No new Rust deps without an ADR line (baseline set pinned in issue #1). README/spec fiction sweep before gates.

## 4. Phase D — Red team + QE (fix ALL findings)

- Inline lenses (subagent spawning fails on this account — use inline passes; roster agents below are fallback-only): hostile time-string/slug/snapshot fuzz, guard bypass (corrupt TOML, hostile slugs `../ ? #`, truncated snapshots), panic sweep, privacy audit (no key/PII/telemetry; inspect snapshot dir contents), TZ/DST sweep (DST transitions, year boundary), honesty audit, spec/ADR/code coherence, README fiction, layering-violation sweep (grep `mawaqit_api` outside `src/infrastructure/`).
- **Mandatory lens — the `brutal-honesty-review` skill**, run inline over the final tree before the red-team report is written. Three passes, one per mode:
  - **Linus (technical):** layering violations, blocking I/O on the event loop, unwinding on external data, wrong-side time math. "You're re-parsing HH:MM in the UI layer — the crate already owns that contract."
  - **Ramsay (standards):** test-suite quality per the rubric — scenario coverage, edge cases, stability. "12 ut tests and 11 assert the happy path. Where's rollover? Where's the hostile-input tier?"
  - **Bach (BS detection):** completion theater and evidence honesty — "comprehensive tests generated" with stubs, EVID rows claiming measured that are pending, README claims no binary supports. Red flags: cargo-cult invariants, certification theater in specs.
  - **Enforcement:** minimum **3 weighted findings** (CRITICAL=3, HIGH=2, MEDIUM=1, LOW=0.5) per review — if fewer, escalate to deeper analysis and say exactly why with evidence. Use the criticism structure (What's Broken → Why It's Wrong → What Correct Looks Like → How to Fix → Why This Matters). Attack the work, never the worker; every finding lands with an actionable fix.
- `docs/reports/red-team-<date>-<lane>.md`: severity table + disposition per finding (brutal-honesty findings merged in, weighted). Fix EVERYTHING before gates.

## 5. Phase E — Gates

- `cargo +nightly fmt --all -- --check` green on the FINAL tree (fmt stage mutates → commit drift as style; clippy drift waves → `#[expect]` + comment, never weaken asserts).
- `cargo clippy --all-targets --all-features -- -D warnings` (thresholds from `clippy.toml`).
- `cargo test --workspace` — ut/ct/e2e/fuzz all green; fuzz cases bounded and deterministic.
- Coverage `cargo llvm-cov --workspace`: module-scoped 0-missed on new/changed fns; floors ratchet UP only (this replaces Amperion's vitest floors — pure-Rust repo).
- Mutation: `#[mutants::skip]` BANNED; proptest properties are the mutation-resistant floor; register new surfaces for cargo-mutants once CI exists (CI-only, Bilel rule — never local cargo-mutants).
- GitNexus `detect_changes --scope all` before commit; `partial:`/`truncated:` = not clean — re-run.

## 6. Phase F — Land on local dev

- Conventional commits; PATHSPEC commits only (shared index with live lanes; never `git add -A`).
- dev moves mid-lane: merge dev into lane → resolve (prefer the other lane's resolution on identical drift) → re-verify → repeat; re-check ADR-number collisions per merge.
- Land by `--ff-only`; dirty main checkout with live lane WIP → `git update-ref refs/heads/dev <target>` (never stash others' WIP).
- Backup branch BEFORE worktree cleanup (from the MAIN checkout; Bash cwd dies → ruflo terminal rescue).
- Memory-ledger update + `gh issue comment #xx` evidence when gh works; close the issue only when every acceptance box is ticked; CI-dependent EVID stays "registered, pending scheduler".

### 6b. Parallel lanes — `v3-swarm-coordination` pattern (only when spawning works)

Default is ONE inline sequential lane (spawning fails on this account). If a session allows subagents (e.g., a different account/runtime), use the `v3-swarm-coordination` skill's hierarchical-mesh pattern, scaled down from its 15-agent template to mawaqit's milestone graph:

```
                CAMPAIGN LEAD (queen)
                        │
     ┌──────────────────┼──────────────────┐
 DOMAIN LANE         ADAPTER LANE        UI LANE
 (#1 domain core)    (#2 ports/adapters) (#3 Today, #4 search,
     │                   │               #5 month)
     └───────────────────┴────────────────┘
                         │
                    QE GATE LANE
        (#6 offline+polish; red-team, brutal-honesty,
         gates, detect-changes — cross-cutting, all phases)
```

- **Dependency edges = issue dependencies**, enforced exactly: #1 → #2 → {#3, #4, #5} → #6 (#4 also needs #3). Deadlock check before dispatch: a lane with no ready work means a dependency violation, never a guess.
- **Lane isolation:** each lane = its own worktree + PATHSPEC commits (shared index rules from Phase F still apply); QE gate lane reviews merged lane tips, never WIP.
- **Communication:** the memory ledger (namespace `mawaqit-tui`) is the inter-lane bus — HELD/BLOCKED rulings written by a lane are read by all; no cross-lane edits to files another lane owns.
- **Spawn failure fallback (standing rule):** first spawn error → collapse to inline sequential lanes in dependency order (#1 → #2 → #3 → #4 → #5 → #6) and do NOT retry spawning for the rest of the campaign.

## Capability roster (use, don't guess)

**GitNexus MCP** (repo param picks the index: `mawaqit-tui`, `mawaqit-api`, `mawaqit-desktop`):
`impact` (blast radius), `context` (symbol 360°), `query` (concepts/flows), `trace` (A→B path), `check` (import cycles), `detect_changes` (pre-commit), `explain` / `pdg_query` (taint/control flow when indexed with `--pdg`), `cypher` (structural queries), `rename` (graph-aware renames), `list_repos`.
CLI fallback: `node .gitnexus/run.cjs impact|detect-changes …`.

**ruflo MCP:**
Ledger: `memory_store` / `memory_search` / `memory_search_unified` / `memory_list` / `memory_delete` (namespace `mawaqit-tui`).
Routing & learning: `hooks_route`, `hooks_model-route`, `hooks_pre-task`, `hooks_post-task`, `hooks_intelligence_pattern-search` / `hooks_intelligence_pattern-store` (cross-session patterns), `agentdb_pattern-search`.
Safety: `aidefence_scan` / `aidefence_is_safe` on any untrusted pasted page content before it reaches the model (privacy pillar).
Health: `system_health`.

**web_reader MCP:** docs.rs/crates.io reads ONLY when local ground truth cannot answer; cite as supplementary, never primary.

**Skills** (invoke when the phase matches):
- GitNexus workflow: `gitnexus-impact-analysis`, `gitnexus-exploring`, `gitnexus-refactoring`, `gitnexus-debugging`, `gitnexus-review`, `gitnexus-taint-analysis`, `gitnexus-pdg-query`, `gitnexus-cli`, `gitnexus-plan`, `gitnexus-work`, `gitnexus-lfg`, `gitnexus-guide`.
- TDD/method: `strict-tdd`, `tdd-london-chicago`, `qe-tdd-london-chicago`, `qe-test-generation` (durable-first tiers, Phase B driver), `sparc-methodology`, `qe-sparc-methodology`, `xp-practices`, `shift-left-testing`, `pair-programming`.
- QE gates: `validation-pipeline`, `verification-quality`, `brutal-honesty-review` (mandatory Phase D lens — Linus/Ramsay/Bach, min-3-findings), `sherlock-review`, `pr-review`, `code-review-quality`, `mutation-testing`, `risk-based-testing`, `regression-testing`, `test-design-techniques`, `exploratory-testing-advanced`, `freeze-tests`, `no-skip`, `coverage-guard`, `test-failure-investigator`, `debug-loop`, `qe-iterative-loop`, `stream-chain`, `chaos-engineering-resilience`, `observability-testing-patterns`.
- Craft: `refactoring-patterns`, `technical-writing`, `v3-ddd-architecture`, `v3-core-implementation`, `v3-performance-optimization`, `v3-swarm-coordination` (parallel-lane mesh, §6b), `ui-ux-pro-max` (TUI theming/layout), `ruflo`.

**Agents** (fallback — spawning has failed on this account; prefer inline lenses; if a spawn errors, do NOT retry, go inline):
`system-architect` (ADR drafts), `qe-test-architect` (Phase B test design — durable-first synthesis, run its method inline), `qe-tdd-specialist` (RED/GREEN discipline), `qe-property-tester` (proptest design), `qe-mutation-tester`, `qe-devils-advocate` (red-team lens), `qe-code-reviewer` (brutal-honesty Linus/Ramsay passes), `qe-security-scanner` / `security-auditor` (privacy audit), `qe-coverage-specialist`, `qe-regression-analyzer`, `qe-risk-assessor`, `qe-root-cause-analyzer`, `qe-flaky-hunter`, `performance-engineer` / `perf-analyzer`, `reviewer`, `researcher`, `reasoningbank-learner`, `memory-coordinator`.

## Standing rulings

NEVER push. `#[mutants::skip]` banned. KEYLESS-PRIVACY / HOSTILE-INPUT-TOTAL / TZ-TRUTH / ROLLOVER-CORRECT / HONESTY / LAYERING invariants. Honesty: broken input = described result, offline ≠ faked, loading ≠ done, measured ≠ pending, rejections counted, dropped days surfaced. `AGENTS.md` + `docs/specs|adr` + the `tst/` contract + `~/Workspace/mawaqit-api/src` + issues #1–#6 are the only standards sources. Never edit before impact. Inline lenses over subagents.
