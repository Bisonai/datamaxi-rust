use crate::candle::Candle;
use crate::client::Client;
use crate::fundingrate::FundingRate;
pub enum API {
    CandleApi(CandleApi),
    FundingRateApi(FundingRateApi),
}

pub enum CandleApi {
    Exchanges,
    Symbols,
    Intervals,
    CandleDetails,
}

pub enum FundingRateApi {
    Exchanges,
    Symbols,
    HistoricalFundingRate,
    LatestFundingRate,
}

impl From<API> for String {
    fn from(item: API) -> Self {
        String::from(match item {
            API::CandleApi(route) => match route {
                CandleApi::CandleDetails => "/candle",
                CandleApi::Exchanges => "/candle/exchanges",
                CandleApi::Symbols => "/candle/symbols",
                CandleApi::Intervals => "/candle/intervals",
            },
            API::FundingRateApi(route) => match route {
                FundingRateApi::Exchanges => "/funding-rate/exchanges",
                FundingRateApi::Symbols => "/funding-rate/symbols",
                FundingRateApi::HistoricalFundingRate => "/funding-rate",
                FundingRateApi::LatestFundingRate => "/funding-rate/latest",
            },
        })
    }
}

pub trait Datamaxi {
    fn new(api_key: String) -> Self;
}

impl Datamaxi for Candle {
    fn new(api_key: String) -> Candle {
        Candle {
            client: Client::new(api_key),
        }
    }
}

impl Datamaxi for FundingRate {
    fn new(api_key: String) -> FundingRate {
        FundingRate {
            client: Client::new(api_key),
        }
    }
}
