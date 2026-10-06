//! Mosque identity value objects (spec M1 R5).

use core::{fmt, fmt::Display};

/// Validated mosque slug. Rules: non-empty, ≤ [`MosqueId::MAX_LEN`] bytes,
/// ASCII alphanumeric plus `- _ .`, no `..`, no leading/trailing `.`.
///
/// The app-side rule mirrors the spirit of `mawaqit_api`'s slug validation;
/// the M2 adapter re-validates against the crate so a divergence between the
/// two cannot slip a hostile slug through.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MosqueId(String);

/// Rejection reason for [`MosqueId::parse`] — one opaque unit for all rule
/// violations; the input is hostile by definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidSlug;

impl fmt::Display for InvalidSlug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid mosque slug")
    }
}

impl std::error::Error for InvalidSlug {}

impl MosqueId {
    pub const MAX_LEN: usize = 256;

    /// Total over arbitrary input; hostile slugs are rejected, never sanitized
    /// silently (HOSTILE-INPUT-TOTAL).
    pub fn parse(raw: &str) -> Result<MosqueId, InvalidSlug> {
        if raw.is_empty() || raw.len() > Self::MAX_LEN {
            return Err(InvalidSlug);
        }
        if raw.starts_with('.') || raw.ends_with('.') || raw.contains("..") {
            return Err(InvalidSlug);
        }
        if !raw
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
        {
            return Err(InvalidSlug);
        }
        Ok(MosqueId(raw.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for MosqueId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Search-result summary handed to the UI; the crate's `Mosque` is mapped
/// onto this in the M2 adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MosqueSummary {
    pub id: MosqueId,
    pub name: String,
    pub place: Option<String>,
}
