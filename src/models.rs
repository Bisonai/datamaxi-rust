use serde::Deserialize;
use serde::Serialize;

/// Detailed information about a candle.
#[derive(Serialize, Deserialize, Debug)]
pub struct CandleDetail {
    /// The timestamp of the candle's open time.
    #[serde(rename = "d")]
    pub timestamp: i64,

    /// The opening price of the asset at the beginning of the time frame.
    #[serde(rename = "o")]
    pub open: f64,

    /// The highest price of the asset during the time frame.
    #[serde(rename = "h")]
    pub high: f64,

    /// The lowest price of the asset during the time frame.
    #[serde(rename = "l")]
    pub low: f64,

    /// The closing price of the asset at the end of the time frame.
    #[serde(rename = "c")]
    pub close: f64,

    /// The total volume of trades (in the base currency) that occurred during the time frame.
    #[serde(rename = "v")]
    pub volume: f64,
}

/// Response containing candle data.
#[derive(Serialize, Deserialize, Debug)]
pub struct CandleResponse {
    /// A vector containing detailed information about each candle.
    pub data: Vec<CandleDetail>,

    /// Requested exchange.
    pub exchange: String,

    /// Requested symbol.
    pub symbol: String,

    /// Requested market.
    pub market: String,

    /// Requested currency.
    pub currency: String,

    /// The interval for the candle data (e.g., 1m, 1h, 1d).
    pub interval: String,

    /// The sorting order for the candle data (e.g., "asc" or "desc").
    pub sort: String,
}

/// Detailed information about a trade.
#[derive(Serialize, Deserialize, Debug)]
pub struct TradeDetail {
    /// The timestamp of the trade.
    #[serde(rename = "d")]
    pub timestamp: String,

    /// The block number in which the trade was recorded.
    #[serde(rename = "b")]
    pub block_number: i64,

    /// The trading pool where the trade occurred.
    #[serde(rename = "pool")]
    pub pool: String,

    /// The trading symbol associated with the trade (e.g., BTC-USDT).
    #[serde(rename = "s")]
    pub symbol: String,

    /// The hash of the transaction related to the trade.
    #[serde(rename = "tx")]
    pub transaction_hash: String,

    /// The maker of the trade (the party who placed the order).
    #[serde(rename = "m")]
    pub maker: String,

    /// The type of trade (e.g., buy or sell).
    #[serde(rename = "t")]
    pub trade_type: String,

    /// The quantity of the base asset traded, in base unit.
    #[serde(rename = "bq")]
    pub base_quantity: String,

    /// The quantity of the quote asset traded, in base unit.
    #[serde(rename = "qq")]
    pub quote_quantity: String,

    /// The price of the trade in the quote asset's unit.
    #[serde(rename = "p")]
    pub price: String,
}

/// Response containing trade data.
#[derive(Serialize, Deserialize, Debug)]
pub struct TradeResponse {
    /// A vector containing detailed information about each trade.
    pub data: Vec<TradeDetail>,

    /// The current page number in the paginated response.
    pub page: i32,

    /// The maximum number of items per page in the response.
    pub limit: i32,

    /// The starting point of the time frame for the trade data.
    pub from: String,

    /// The ending point of the time frame for the trade data.
    pub to: String,

    /// The sorting order for the trade data (e.g., "asc" or "desc").
    pub sort: String,
}

/// Optional parameters for a candle request.
pub struct CandleOptions {
    /// The interval for the candle data (e.g., 1m, 1h, 1d).
    pub interval: Option<String>,

    /// The starting date & time for the candle data.
    pub from: Option<String>,

    /// The ending date & time for the candle data.
    pub to: Option<String>,

    /// The currency for the candle data.
    pub currency: Option<String>,
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
            interval: None,
            from: None,
            to: None,
            currency: None,
        }
    }


    /// Sets the interval for the candle query.
    pub fn interval(mut self, interval: &str) -> Self {
        self.interval = Some(interval.into());
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

/// Response containing details about pools.
#[derive(Serialize, Deserialize, Debug)]
pub struct PoolsResponse {
    /// The blockchain where the pool is located.
    #[serde(rename = "c")]
    pub chain: String,

    /// The name of the exchange where the pool is available.
    #[serde(rename = "e")]
    pub exchange: String,

    /// The base asset used in the pool.
    #[serde(rename = "b")]
    pub base: String,

    /// The quote asset used in the pool.
    #[serde(rename = "q")]
    pub quote: String,

    /// The address of the base token in the pool.
    #[serde(rename = "ba")]
    pub baset_address: String,

    /// The address of the quote token in the pool.
    #[serde(rename = "qa")]
    pub quote_address: String,

    /// The unique address of the pool.
    #[serde(rename = "pa")]
    pub pool_address: String,

    /// An optional unique identifier for the pool.
    #[serde(rename = "id")]
    pub id: Option<String>,
}

/// Optional parameters for a trade request.
pub struct TradeOptions {
    /// The page number for the trade query.
    pub page: Option<i32>,

    /// The maximum number of items per page in the response.
    pub limit: Option<i32>,

    /// The starting date for the trade query.
    pub from: Option<String>,

    /// The ending date for the trade query.
    pub to: Option<String>,

    /// The sorting order for the trade query (e.g., "asc" or "desc").
    pub sort: Option<String>,
}

impl Default for TradeOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Provides a builder pattern for setting optional parameters for a trade.
impl TradeOptions {
    /// Creates a new instance of `TradeOptions` with default values.
    pub fn new() -> Self {
        TradeOptions {
            page: 1.into(),
            limit: 1000.into(),
            from: None,
            to: None,
            sort: Some("desc".into()),
        }
    }

    /// Sets the page number for the trade query.
    pub fn page(mut self, page: i32) -> Self {
        self.page = Some(page);
        self
    }

    /// Sets the limit for the number of results returned.
    pub fn limit(mut self, limit: i32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets the starting date for the trade query.
    pub fn from(mut self, from: &str) -> Self {
        self.from = Some(from.into());
        self
    }

    /// Sets the ending date for the trade query.
    pub fn to(mut self, to: &str) -> Self {
        self.to = Some(to.into());
        self
    }

    /// Sets the sort order for the trade query (e.g., "asc" or "desc").
    pub fn sort(mut self, sort: &str) -> Self {
        self.sort = Some(sort.into());
        self
    }
}

/// Optional parameters for a pools request.
pub struct PoolsOptions {
    /// The chain for the pools query.
    pub chain: Option<String>,

    /// The exchange for the pools query.
    pub exchange: Option<String>,
}

impl Default for PoolsOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Provides a builder pattern for setting optional parameters for a pools request.
impl PoolsOptions {
    /// Creates a new instance of `PoolsOptions` with default values.
    pub fn new() -> Self {
        PoolsOptions {
            chain: None,
            exchange: None,
        }
    }

    /// Sets the chain for the pools query.
    pub fn chain(mut self, chain: &str) -> Self {
        self.chain = Some(chain.into());
        self
    }

    /// Sets the exchange for the pools query.
    pub fn exchange(mut self, exchange: &str) -> Self {
        self.exchange = Some(exchange.into());
        self
    }
}
