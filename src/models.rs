use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Symbols(pub Vec<String>);
pub struct Intervals(pub Vec<String>);

#[derive(Deserialize, Debug)]
pub struct CandleResponse {
    pub data: Vec<CandleDetail>,
    pub page: i32,
    pub limit: i32,
    pub from: String,
    pub to: String,
    pub sort: String,
}

#[derive(Deserialize, Debug)]
pub struct CandleDetail {
    pub d: String,
    pub o: String,
    pub h: String,
    pub l: String,
    pub c: String,
    pub v: String,
}

#[derive(Deserialize, Debug)]
pub struct HistoricalFundingRateResponse {
    pub data: Vec<HistoricalFundingRateDetail>,
    pub page: i32,
    pub limit: i32,
    pub from: String,
    pub to: String,
    pub sort: String,
}

#[derive(Deserialize, Debug)]
pub struct HistoricalFundingRateDetail {
    pub d: String,
    pub f: String,
    pub m: String,
}

#[derive(Deserialize, Debug)]
pub struct LatestFundingRateDetail {
    pub e: String,
    pub d: i64,
    pub s: String,
    pub b: String,
    pub q: String,
    pub r: f64,
}
