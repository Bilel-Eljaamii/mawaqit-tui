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

# Every gate, in order — must be green on the FINAL tree before any commit
verify: fmt clippy test cov
