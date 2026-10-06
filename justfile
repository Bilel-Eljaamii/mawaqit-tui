# ------------------------------------------------------------------------------
# mawaqit-tui — landing gates (CAMPAIGN.md Phase E)
# ------------------------------------------------------------------------------

default:
    @just --list

# Format check (rustfmt.toml uses unstable options → nightly rustfmt)
fmt:
    cargo +nightly fmt --all -- --check

# Lint, deny warnings (thresholds from clippy.toml, MSRV 1.89)
clippy:
    cargo clippy --all-targets --all-features -- -D warnings

# Full test suite — ut/ct/e2e/fuzz tiers (tst/ harness contract)
test:
    cargo test --workspace

# Coverage — module-scoped floors ratchet UP only
cov:
    cargo llvm-cov --workspace --summary-only

# Layering gate: `use mawaqit_api` only under src/infrastructure (ADR-0001 §1)
layering:
    @result=$(grep -rn "use mawaqit_api" src --include="*.rs" | grep -v "^src/infrastructure/" || true); \
    if [ -n "$result" ]; then echo "LAYERING VIOLATION:"; echo "$result"; exit 1; else echo "layering: clean"; fi

# Every gate, in order — must be green on the FINAL tree before any commit
verify: fmt clippy test cov layering
