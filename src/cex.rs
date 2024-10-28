use crate::api::{Client, Config, Datamaxi, Result};
pub use crate::models::{CandleOptions, SymbolsOptions};
use crate::models::{CandleResponse, SymbolsResponse};
use std::collections::HashMap;

/// Provides methods for retrieving CEX candle data and related information.
#[derive(Clone)]
pub struct Candle {
    pub client: Client,
}

impl Candle {
    /// Retrieves candle data for a specified exchange and symbol. Additional parameters can be
    /// provided to filter and sort the results. The response will contain an array of candle data
    /// objects, each representing a single candle with open, high, low, close, and volume values.
    pub fn get<E, S>(
        &self,
        exchange: E,
        symbol: S,
        options: CandleOptions,
    ) -> Result<CandleResponse>
    where
        E: Into<String>,
        S: Into<String>,
    {
        let mut parameters = HashMap::new();

        // required
        parameters.insert("exchange".to_string(), exchange.into());
        parameters.insert("symbol".to_string(), symbol.into());

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

        self.client.get("/cex/candle", Some(parameters))
    }

    /// Retrieves a list of supported exchanges for candle data. The market parameter can be used
    /// to filter the results by market.
    pub fn exchanges<M>(&self, market: M) -> Result<Vec<String>>
    where
        M: Into<String>,
    {
        let mut parameters = HashMap::new();

        // required
        parameters.insert("market".to_string(), market.into());

        self.client.get("/cex/candle/exchanges", Some(parameters))
    }

    /// Retrieves a list of supported symbols for candle data. The exchange parameter is required,
    /// and the market parameter can be used to filter the results by market.
    pub fn symbols<E>(&self, exchange: E, options: SymbolsOptions) -> Result<Vec<SymbolsResponse>>
    where
        E: Into<String>,
    {
        let mut parameters = HashMap::new();

        // required
        parameters.insert("exchange".to_string(), exchange.into());

        // optional
        if let Some(market) = options.market {
            parameters.insert("market".to_string(), market);
        }

        self.client.get("/cex/candle/symbols", Some(parameters))
    }

    /// Retrieves a list of supported candle intervals.
    pub fn intervals(&self) -> Result<Vec<String>> {
        self.client.get("/cex/candle/intervals", None)
    }
}

/// Implements the `Datamaxi` trait for `Candle`, providing methods
/// to create new instances of `Candle` with or without a custom base URL.
impl Datamaxi for Candle {
    /// Creates a new `Candle` instance with the default base URL.
    ///
    /// # Parameters
    /// - `api_key`: A `String` representing the API key used for authentication in API requests.
    ///
    /// # Returns
    /// A new `Candle` instance configured with the default base URL and the provided `api_key`.
    ///
    /// # Example
    /// ```rust
    /// use crate::datamaxi::api::Datamaxi;
    /// let candle = datamaxi::cex::Candle::new("my_api_key".to_string());
    /// ```
    fn new(api_key: String) -> Candle {
        let config = Config {
            base_url: None, // Default base URL will be used
            api_key,        // Provided API key
        };
        Candle {
            client: Client::new(config), // Create a new client with the given config
        }
    }

    /// Creates a new `Candle` instance with a custom base URL.
    ///
    /// # Parameters
    /// - `api_key`: A `String` representing the API key used for authentication in API requests.
    /// - `base_url`: A `String` representing the custom base URL for API requests.
    ///
    /// # Returns
    /// A new `Candle` instance configured with the specified `base_url` and `api_key`.
    ///
    /// # Example
    /// ```rust
    /// use crate::datamaxi::api::Datamaxi;
    /// let candle = datamaxi::cex::Candle::new_with_base_url("my_api_key".to_string(), "https://custom-api.example.com".to_string());
    /// ```
    fn new_with_base_url(api_key: String, base_url: String) -> Candle {
        let config = Config {
            base_url: Some(base_url), // Use the provided custom base URL
            api_key,                  // Provided API key
        };
        Candle {
            client: Client::new(config), // Create a new client with the given config
        }
    }
}
