//! mawaqit-tui — terminal prayer-times companion on `mawaqit-api`.
//!
//! Layering (ADR-0001): `ui → application → domain`; only `infrastructure`
//! imports `mawaqit_api`. The domain stays pure: `std` + `chrono`/`chrono-tz`.

pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod runtime;
pub mod ui;
