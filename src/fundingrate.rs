use crate::api::FundingRateApi;
use crate::api::API;
use crate::client::Client;
use crate::error::Result;
use crate::models::HistoricalFundingRateResponse;
use crate::models::LatestFundingRateDetail;
use crate::utils::build_request;
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct FundingRate {
    pub client: Client,
}

impl FundingRate {
    pub fn exchanges<M>(&self, market: M) -> Result<Vec<String>>
    where
        M: Into<String>,
    {
        let mut parameters: BTreeMap<String, String> = BTreeMap::new();
        parameters.insert("market".into(), market.into());
        let request = build_request(parameters);
        let symbols: Vec<String> = self.client.get(
            API::FundingRateApi(FundingRateApi::Exchanges),
            Some(request),
        )?;

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
            .get(API::FundingRateApi(FundingRateApi::Symbols), Some(request))?;

        Ok(symbols)
    }

    pub fn get_latest(&self) -> Result<Vec<LatestFundingRateDetail>> {
        let hisitorical_funding_rate: Vec<LatestFundingRateDetail> = self
            .client
            .get(API::FundingRateApi(FundingRateApi::LatestFundingRate), None)?;

        Ok(hisitorical_funding_rate)
    }

    pub fn get_historical<S, E>(
        &self,
        symbol: S,
        exchange: E,
    ) -> Result<HistoricalFundingRateResponse>
    where
        S: Into<String>,
        E: Into<String>,
    {
        let mut parameters: BTreeMap<String, String> = BTreeMap::new();
        parameters.insert("symbol".into(), symbol.into());
        parameters.insert("exchange".into(), exchange.into());
        let request = build_request(parameters);
        let hisitorical_funding_rate: HistoricalFundingRateResponse = self.client.get(
            API::FundingRateApi(FundingRateApi::HistoricalFundingRate),
            Some(request),
        )?;

        Ok(hisitorical_funding_rate)
    }
}
