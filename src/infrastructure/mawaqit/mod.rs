//! Mawaqit API adapter — the ONLY module allowed to import `mawaqit_api`
//! (ADR-0001 §1). Fetches live here; conversion lives in [`mapping`].

pub mod mapping;

use std::{sync::Arc, time::Duration};

use mawaqit_api::{
    ConfData, MawaqitClient, MawaqitError, MonthIqamaTimes, MonthTimes, TodayTimes,
};

use crate::{
    application::ports::{
        MonthReadout, MosqueDirectory, PortError, TimesService, TodayReadout,
    },
    domain::mosque::{MosqueId, MosqueSummary},
};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

fn net_err(err: MawaqitError) -> PortError {
    PortError::Network(err.to_string())
}

/// Adapter over the keyless client; `Clone` (the client is `Clone`).
#[derive(Debug, Clone)]
pub struct MawaqitAdapter {
    client: MawaqitClient,
}

impl Default for MawaqitAdapter {
    fn default() -> MawaqitAdapter {
        MawaqitAdapter::new()
    }
}

impl MawaqitAdapter {
    pub fn new() -> MawaqitAdapter {
        MawaqitAdapter::from_client(
            MawaqitClient::new().with_timeouts(CONNECT_TIMEOUT, REQUEST_TIMEOUT),
        )
    }

    pub fn from_client(client: MawaqitClient) -> MawaqitAdapter {
        MawaqitAdapter { client }
    }

    async fn conf(&self, id: &MosqueId) -> Result<Arc<ConfData>, PortError> {
        self.client.conf_data(id.as_str()).await.map_err(net_err)
    }

    async fn today_api(&self, id: &MosqueId) -> Result<TodayTimes, PortError> {
        self.client.today(id.as_str()).await.map_err(net_err)
    }

    async fn month_api(
        &self,
        id: &MosqueId,
        month: u32,
    ) -> Result<MonthTimes, PortError> {
        self.client.month(id.as_str(), month).await.map_err(net_err)
    }

    async fn month_iqama_api(
        &self,
        id: &MosqueId,
        month: u32,
    ) -> Result<MonthIqamaTimes, PortError> {
        self.client.month_iqama(id.as_str(), month).await.map_err(net_err)
    }
}

impl MosqueDirectory for MawaqitAdapter {
    async fn search(&self, word: &str) -> Result<Vec<MosqueSummary>, PortError> {
        let results = self.client.search_mosques(word).await.map_err(net_err)?;
        mapping::map_mosques(&results)
    }
}

impl TimesService for MawaqitAdapter {
    async fn today(&self, id: &MosqueId) -> Result<TodayReadout, PortError> {
        // conf_data is client-cached after the first call; both fetches are
        // independent, so run them concurrently like month_times does.
        let (conf, api) = tokio::join!(self.conf(id), self.today_api(id));
        let conf = conf?;
        let api = api?;
        mapping::map_today(&api, &conf)
    }

    async fn month_times(
        &self,
        id: &MosqueId,
        month: u32,
    ) -> Result<MonthReadout, PortError> {
        let (adhan, iqama) =
            tokio::join!(self.month_api(id, month), self.month_iqama_api(id, month));
        mapping::map_month(&adhan?, &iqama?)
    }
}
