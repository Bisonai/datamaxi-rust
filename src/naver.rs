use crate::api::NaverTrendApi;
use crate::api::API;
use crate::client::Client;
use crate::error::Result;
use crate::utils::build_request;
use std::collections::BTreeMap;

pub struct NaverTrend {
    pub client: Client,
}

impl NaverTrend {
    pub fn symbols(&self) -> Result<Vec<String>> {
        let symbols: Vec<String> = self
            .client
            .get(API::NaverTrendApi(NaverTrendApi::Symbols), None)?;

        Ok(symbols)
    }

    pub fn get<S>(&self, symbol: S) -> Result<Vec<Vec<String>>>
    where
        S: Into<String>,
    {
        let mut parameters: BTreeMap<String, String> = BTreeMap::new();
        parameters.insert("symbol".into(), symbol.into());
        let request = build_request(parameters);
        let forex_data: Vec<Vec<String>> = self
            .client
            .get(API::NaverTrendApi(NaverTrendApi::Trend), Some(request))?;

        Ok(forex_data)
    }
}
