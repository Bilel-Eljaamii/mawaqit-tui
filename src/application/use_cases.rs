//! Thin use-case orchestrators (issue #2, ADR-0002 §2). They own no I/O —
//! only input normalization, validation, and port delegation.

use crate::{
    application::ports::{
        MonthReadout, MosqueDirectory, PortError, SettingsError, SettingsStore,
        TimesService, TodayReadout,
    },
    domain::mosque::{MosqueId, MosqueSummary},
};

/// Application-level error: one vocabulary over ports and settings.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Port(#[from] PortError),
    #[error(transparent)]
    Settings(#[from] SettingsError),
}

/// Search mosques by free-text word.
pub struct SearchMosques<'a, D> {
    pub directory: &'a D,
}

impl<D: MosqueDirectory> SearchMosques<'_, D> {
    /// Whitespace-only queries short-circuit without touching the port.
    pub async fn execute(&self, word: &str) -> Result<Vec<MosqueSummary>, AppError> {
        let word = word.trim();
        if word.is_empty() {
            return Ok(Vec::new());
        }
        Ok(self.directory.search(word).await?)
    }
}

/// Load today's times for the selected mosque.
pub struct LoadToday<'a, T> {
    pub times: &'a T,
}

impl<T: TimesService> LoadToday<'_, T> {
    pub async fn execute(&self, id: &MosqueId) -> Result<TodayReadout, AppError> {
        Ok(self.times.today(id).await?)
    }
}

/// Load a month of times; the month is validated before any port call.
pub struct LoadMonth<'a, T> {
    pub times: &'a T,
}

impl<T: TimesService> LoadMonth<'_, T> {
    pub async fn execute(
        &self,
        id: &MosqueId,
        month: u32,
    ) -> Result<MonthReadout, AppError> {
        if !(1..=12).contains(&month) {
            return Err(
                PortError::InvalidData(format!("month {month} outside 1..=12")).into()
            );
        }
        Ok(self.times.month_times(id, month).await?)
    }
}

/// Persist the selected mosque.
pub struct SaveSelection<'a, S> {
    pub settings: &'a S,
}

impl<S: SettingsStore> SaveSelection<'_, S> {
    pub fn execute(&self, selection: &MosqueSummary) -> Result<(), AppError> {
        Ok(self.settings.save(selection)?)
    }
}
