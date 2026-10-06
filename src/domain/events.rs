//! Domain events (ADR-0001). Minimal set for M1; expanded as milestones need.

use crate::domain::mosque::MosqueId;

/// Emitted when a mosque becomes the active selection (persisted in M4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainEvent {
    MosqueSelected { id: MosqueId },
}
