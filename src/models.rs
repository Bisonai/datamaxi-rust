use serde::Deserialize;
use serde::Serialize;

/// Detailed information about a candle.
#[derive(Serialize, Deserialize, Debug)]
pub struct CandleDetail {
    /// The timestamp of the candle's open time.
    #[serde(rename = "d")]
    pub timestamp: String,

    /// The opening price of the asset at the beginning of the time frame.
    #[serde(rename = "o")]
    pub open: String,

    /// The highest price of the asset during the time frame.
    #[serde(rename = "h")]
    pub high: String,

    /// The lowest price of the asset during the time frame.
    #[serde(rename = "l")]
    pub low: String,

    /// The closing price of the asset at the end of the time frame.
    #[serde(rename = "c")]
    pub close: String,

    /// The total volume of trades (in the base currency) that occurred during the time frame.
    #[serde(rename = "v")]
    pub volume: String,
}

/// Response containing candle data.
#[derive(Serialize, Deserialize, Debug)]
pub struct CandleResponse {
    /// A vector containing detailed information about each candle.
    pub data: Vec<CandleDetail>,

    /// The current page number in the paginated response.
    pub page: i32,

    /// The maximum number of items per page in the response.
    pub limit: i32,

    /// The starting point of the time frame for the candle data.
    pub from: String,

    /// The ending point of the time frame for the candle data.
    pub to: String,

    /// The sorting order for the candle data (e.g., "asc" or "desc").
    pub sort: String,
}

/// Optional parameters for a candle request.
pub struct CandleOptions {
    /// The market type (e.g., spot, futures).
    pub market: Option<String>,

    /// The interval for the candle data (e.g., 1m, 1h, 1d).
    pub interval: Option<String>,

    /// The page number for the candle data.
    pub page: Option<i32>,

    /// The maximum number of items per page in the response.
    pub limit: Option<i32>,

    /// The starting date & time for the candle data.
    pub from: Option<String>,

    /// The ending date & time for the candle data.
    pub to: Option<String>,

    /// The sorting order for the candle data (e.g., "asc" or "desc").
    pub sort: Option<String>,
}

impl Default for CandleOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Provides a builder pattern for setting optional parameters for a candle request.
impl CandleOptions {
    /// Creates a new instance of `CandleOptions` with default values.
    pub fn new() -> Self {
        CandleOptions {
            market: None,
            interval: None,
            page: 1.into(),
            limit: 1000.into(),
            from: None,
            to: None,
            sort: Some("desc".into()),
        }
    }

    /// Sets the market for the candle query.
    pub fn market(mut self, market: &str) -> Self {
        self.market = Some(market.into());
        self
    }

    /// Sets the interval for the candle query.
    pub fn interval(mut self, interval: &str) -> Self {
        self.interval = Some(interval.into());
        self
    }

    /// Sets the page number for the candle query.
    pub fn page(mut self, page: i32) -> Self {
        self.page = Some(page);
        self
    }

    /// Sets the limit for the number of results returned.
    pub fn limit(mut self, limit: i32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets the starting date & time for the candle query.
    pub fn from(mut self, from: &str) -> Self {
        self.from = Some(from.into());
        self
    }

    /// Sets the ending date & time for the candle query.
    pub fn to(mut self, to: &str) -> Self {
        self.to = Some(to.into());
        self
    }

    /// Sets the sort order for the candle query (e.g., "asc" or "desc").
    pub fn sort(mut self, sort: &str) -> Self {
        self.sort = Some(sort.into());
        self
    }
}

/// Response containing details about symbols.
#[derive(Serialize, Deserialize, Debug)]
pub struct SymbolsResponse {
    /// The name of the exchange.
    #[serde(rename = "e")]
    pub exchange: String,

    /// The market type (e.g., spot, futures).
    #[serde(rename = "m")]
    pub market: String,

    /// The base asset of the trading pair.
    #[serde(rename = "b")]
    pub base: String,

    /// The quote asset of the trading pair.
    #[serde(rename = "q")]
    pub quote: String,

    /// The trading symbol (e.g., BTC-USDT).
    #[serde(rename = "s")]
    pub symbol: String,

    /// An optional unique identifier for the symbol.
    #[serde(rename = "id")]
    pub id: Option<String>,
}

/// Optional parameters for a symbols request.
pub struct SymbolsOptions {
    /// The market type (e.g., spot, futures).
    pub market: Option<String>,
}

impl Default for SymbolsOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolsOptions {
    /// Creates a new instance of `SymbolsOptions` with default values.
    pub fn new() -> Self {
        SymbolsOptions { market: None }
    }

    /// Sets the market for the symbols query.
    pub fn market(mut self, market: &str) -> Self {
        self.market = Some(market.into());
        self
    }
}
