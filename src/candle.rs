use crate::api::CandleApi;
use crate::api::API;
use crate::client::Client;
use crate::error::Result;
use crate::models::CandleResponse;
use crate::utils::build_request;
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct Candle {
    pub client: Client,
}

impl Candle {
    pub fn exchanges<M>(&self, market: M) -> Result<Vec<String>>
    where
        M: Into<String>,
    {
        let mut parameters: BTreeMap<String, String> = BTreeMap::new();
        parameters.insert("market".into(), market.into());
        let request = build_request(parameters);
        let symbols: Vec<String> = self
            .client
            .get(API::CandleApi(CandleApi::Exchanges), Some(request))?;

        Ok(symbols)
    }

    pub fn symbols<E, M>(&self, exchange: E, market: M) -> Result<Vec<String>>
    where
        E: Into<String>,
        M: Into<String>,
    {
        let mut parameters: BTreeMap<String, String> = BTreeMap::new();
        parameters.insert("exchange".into(), exchange.into());
        parameters.insert("market".into(), market.into());
        let request = build_request(parameters);
        let symbols: Vec<String> = self
            .client
            .get(API::CandleApi(CandleApi::Symbols), Some(request))?;

        Ok(symbols)
    }

    pub fn intervals<E, M>(&self, exchange: E, market: M) -> Result<Vec<String>>
    where
        E: Into<String>,
        M: Into<String>,
    {
        let mut parameters: BTreeMap<String, String> = BTreeMap::new();
        parameters.insert("exchange".into(), exchange.into());
        parameters.insert("market".into(), market.into());
        let request = build_request(parameters);
        let symbols: Vec<String> = self
            .client
            .get(API::CandleApi(CandleApi::Intervals), Some(request))?;

        Ok(symbols)
    }
    pub fn get<S, E, M, I>(
        &self,
        symbol: S,
        exchange: E,
        market: M,
        interval: I,
    ) -> Result<CandleResponse>
    where
        S: Into<String>,
        E: Into<String>,
        M: Into<String>,
        I: Into<String>,
    {
        let mut parameters: BTreeMap<String, String> = BTreeMap::new();
        parameters.insert("symbol".into(), symbol.into());
        parameters.insert("exchange".into(), exchange.into());
        parameters.insert("market".into(), market.into());
        parameters.insert("interval".into(), interval.into());
        let request = build_request(parameters);
        let candle: CandleResponse = self
            .client
            .get(API::CandleApi(CandleApi::CandleDetails), Some(request))?;

        Ok(candle)
    }
}
