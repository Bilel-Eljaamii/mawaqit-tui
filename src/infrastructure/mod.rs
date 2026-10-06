//! Infrastructure adapters — the ONLY layer allowed to import `mawaqit_api`
//! (ADR-0001 §1). Implemented in M2.

pub mod clock;
pub mod mawaqit;
pub mod settings;
pub mod snapshot;
