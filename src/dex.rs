use crate::api::{Client, Config, Datamaxi, Result};
pub use crate::models::{CandleOptions, PoolsOptions, TradeOptions};
use crate::models::{CandleResponse, PoolsResponse, TradeResponse};
use std::collections::HashMap;

/// Provides methods for retrieving DEX candle data and related information.
#[derive(Clone)]
pub struct Dex {
    pub client: Client,
}

impl Dex {
    /// Retrieves candle data for a specified chain, exchange, and pool. Additional parameters can be
    /// provided to filter and sort the results. The response will contain an array of candle data
    /// objects, each representing a single candle with open, high, low, close, and volume values.
    pub fn candle<C, E, P>(
        &self,
        chain: C,
        exchange: E,
        pool: P,
        options: CandleOptions,
    ) -> Result<CandleResponse>
    where
        C: Into<String>,
        E: Into<String>,
        P: Into<String>,
    {
        let mut parameters = HashMap::new();

        // required
        parameters.insert("chain".to_string(), chain.into());
        parameters.insert("exchange".to_string(), exchange.into());
        parameters.insert("pool".to_string(), pool.into());

        // optional
        parameters.extend(
            [
                options
                    .market
                    .map(|market| ("market".to_string(), market.to_string())),
                options
                    .interval
                    .map(|interval| ("interval".to_string(), interval.to_string())),
                options
                    .page
                    .map(|page| ("page".to_string(), page.to_string())),
                options
                    .limit
                    .map(|limit| ("limit".to_string(), limit.to_string())),
                options
                    .from
                    .map(|from| ("from".to_string(), from.to_string())),
                options.to.map(|to| ("to".to_string(), to.to_string())),
                options
                    .sort
                    .map(|sort| ("sort".to_string(), sort.to_string())),
            ]
            .into_iter()
            .flatten(),
        );

        self.client.get("/dex/candle", Some(parameters))
    }

    /// Retrieves trade data for a specified chain, exchange, and pool. Additional parameters can be
    /// provided to filter and sort the results. The response will contain an array of trade data
    /// objects, each representing a single trade with price, amount, and timestamp values.
    pub fn trade<C, E, P>(
        &self,
        chain: C,
        exchange: E,
        pool: P,
        options: TradeOptions,
    ) -> Result<TradeResponse>
    where
        C: Into<String>,
        E: Into<String>,
        P: Into<String>,
    {
        let mut parameters = HashMap::new();

        // required
        parameters.insert("chain".to_string(), chain.into());
        parameters.insert("exchange".to_string(), exchange.into());
        parameters.insert("pool".to_string(), pool.into());

        // optional
        parameters.extend(
            [
                options
                    .page
                    .map(|page| ("page".to_string(), page.to_string())),
                options
                    .limit
                    .map(|limit| ("limit".to_string(), limit.to_string())),
                options
                    .from
                    .map(|from| ("from".to_string(), from.to_string())),
                options.to.map(|to| ("to".to_string(), to.to_string())),
                options
                    .sort
                    .map(|sort| ("sort".to_string(), sort.to_string())),
            ]
            .into_iter()
            .flatten(),
        );

        self.client.get("/dex/trade", Some(parameters))
    }

    /// Retrieves information about available pools, including details about the chain, exchange,
    /// base and quote symbols, and pool address. Optional parameters can be provided to filter the
    /// results by chain and exchange.
    pub fn pools(&self, options: PoolsOptions) -> Result<Vec<PoolsResponse>> {
        let mut parameters = HashMap::new();

        // optional
        parameters.extend(
            [
                options
                    .exchange
                    .map(|exchange| ("exchange".to_string(), exchange.to_string())),
                options
                    .chain
                    .map(|chain| ("chain".to_string(), chain.to_string())),
            ]
            .into_iter()
            .flatten(),
        );

        self.client.get("/dex/pools", Some(parameters))
    }

    /// Retrieves a list of available chains for candle data.
    pub fn chains(&self) -> Result<Vec<String>> {
        self.client.get("/dex/chains", None)
    }

    /// Retrieves a list of available exchanges for candle data.
    pub fn exchanges(&self) -> Result<Vec<String>> {
        self.client.get("/dex/exchanges", None)
    }

    /// Retrieves a list of available intervals for candle data.
    pub fn intervals(&self) -> Result<Vec<String>> {
        self.client.get("/dex/intervals", None)
    }
}

/// Implements the `Datamaxi` trait for `Dex`, providing methods
/// to create new instances of `Dex` with or without a custom base URL.
impl Datamaxi for Dex {
    /// Creates a new `Dex` instance with the default base URL.
    ///
    /// # Parameters
    /// - `api_key`: A `String` representing the API key used to authenticate requests.
    ///
    /// # Returns
    /// A new `Dex` instance configured with the default base URL and the provided `api_key`.
    ///
    /// # Example
    /// ```rust
    /// use crate::datamaxi::api::Datamaxi;
    /// let dex = datamaxi::dex::Dex::new("my_api_key".to_string());
    /// ```
    fn new(api_key: String) -> Dex {
        let config = Config {
            base_url: None, // Default base URL will be used
            api_key,        // Provided API key
        };
        Dex {
            client: Client::new(config), // Create a new client with the provided config
        }
    }

    /// Creates a new `Dex` instance with a custom base URL.
    ///
    /// # Parameters
    /// - `api_key`: A `String` representing the API key used to authenticate requests.
    /// - `base_url`: A `String` representing the custom base URL for API requests.
    ///
    /// # Returns
    /// A new `Dex` instance configured with the provided `base_url` and `api_key`.
    ///
    /// # Example
    /// ```rust
    /// use crate::datamaxi::api::Datamaxi;
    /// let dex = datamaxi::dex::Dex::new_with_base_url("my_api_key".to_string(), "https://custom-api.example.com".to_string());
    /// ```
    fn new_with_base_url(api_key: String, base_url: String) -> Dex {
        let config = Config {
            base_url: Some(base_url), // Use the provided custom base URL
            api_key,                  // Provided API key
        };
        Dex {
            client: Client::new(config), // Create a new client with the provided config
        }
    }
}
