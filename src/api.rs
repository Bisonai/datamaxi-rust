use crate::candle::Candle;
use crate::client::Client;
use crate::forex::Forex;
use crate::fundingrate::FundingRate;
use crate::google::GoogleTrend;
use crate::naver::NaverTrend;

pub enum API {
    CandleApi(CandleApi),
    FundingRateApi(FundingRateApi),
    ForexApi(ForexApi),
    NaverTrendApi(NaverTrendApi),
    GoogleTrendApi(GoogleTrendApi),
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

pub enum ForexApi {
    Symbols,
    Forex,
}

pub enum NaverTrendApi {
    Symbols,
    Trend,
}

pub enum GoogleTrendApi {
    Keywords,
    Trend,
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
            API::ForexApi(route) => match route {
                ForexApi::Symbols => "/forex/symbols",
                ForexApi::Forex => "/forex",
            },
            API::NaverTrendApi(route) => match route {
                NaverTrendApi::Symbols => "/naver/symbols",
                NaverTrendApi::Trend => "/naver/trend",
            },
            API::GoogleTrendApi(route) => match route {
                GoogleTrendApi::Keywords => "/google/keywords",
                GoogleTrendApi::Trend => "/google/trend",
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

impl Datamaxi for Forex {
    fn new(api_key: String) -> Forex {
        Forex {
            client: Client::new(api_key),
        }
    }
}

impl Datamaxi for NaverTrend {
    fn new(api_key: String) -> NaverTrend {
        NaverTrend {
            client: Client::new(api_key),
        }
    }
}

impl Datamaxi for GoogleTrend {
    fn new(api_key: String) -> GoogleTrend {
        GoogleTrend {
            client: Client::new(api_key),
        }
    }
}
