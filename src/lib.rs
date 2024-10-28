//! # DataMaxi+ Rust SDK
//!
//! This is the official implementation of Rust SDK for [DataMaxi+](https://datamaxiplus.com/).
//! The package can be used to fetch both historical and latest data using [DataMaxi+ API](https://docs.datamaxiplus.com/).
//!
//! - [Installation](#installation)
//! - [Configuration](#configuration)
//! - [Links](#links)
//! - [Contributing](#contributing)
//! - [License](#license)
//!
//! ## Installation
//!
//! ```shell
//! [dependencies]
//! datamaxi = { git = "https://github.com/bisonai/datamaxi-rust.git" }
//! ```
//!
//! ## Configuration
//!
//! Private API endpoints are protected by an API key.
//! You can get the API key upon registering at <https://datamaxiplus.com/auth>.
//!
//!
//!| Option     | Explanation                                                                   |
//!|------------|-------------------------------------------------------------------------------|
//!| `api_key`  | Your API key                                                                  |
//!| `base_url` | If `base_url` is not provided, it defaults to `https://api.datamaxiplus.com`. |
//!
//! ## Examples
//!
//! ### CEX Candle
//!
//! ```rust
//! let api_key = "my_api_key".to_string();
//! let candle: datamaxi::cex::Candle = datamaxi::api::Datamaxi::new(api_key);
//!
//! // Fetch supported exchanges for CEX candle data
//! candle.exchanges("spot");
//!
//! // Fetch supported symbols for CEX candle data
//! let symbols_options = datamaxi::cex::SymbolsOptions::new();
//! candle.symbols("binance", symbols_options);
//!
//! // Fetch supported intervals for CEX candle data
//! candle.intervals();
//!
//! // Fetch CEX candle data
//! let candle_options = datamaxi::cex::CandleOptions::new();
//! candle.get("binance", "ETH-USDT", candle_options);
//! ```
//!
//! ### DEX
//!
//! ```rust
//! let api_key = "my_api_key".to_string();
//! let dex: datamaxi::dex::Dex = datamaxi::api::Datamaxi::new(api_key);
//!
//! // Fetch supported intervals for DEX candle data
//! dex.intervals();
//!
//! // Fetch supported exchange for DEX data
//! dex.exchanges();
//!
//! // Fetch supported chains for DEX data
//! dex.chains();
//!
//! // Fetch supported pools for DEX data
//! let pools_options = datamaxi::dex::PoolsOptions::new();
//! dex.pools(pools_options);
//!
//! // Fetch DEX candle data
//! let params = datamaxi::dex::CandleOptions::new();
//! dex.candle(
//!   "bsc_mainnet",
//!   "pancakeswap",
//!   "0xb24cd29e32FaCDDf9e73831d5cD1FFcd1e535423",
//!   params,
//! );
//!
//! // Fetch DEX trade data
//! let trade_options = datamaxi::dex::TradeOptions::new().limit(5);
//! dex.trade(
//!   "bsc_mainnet",
//!   "pancakeswap",
//!   "0xb24cd29e32FaCDDf9e73831d5cD1FFcd1e535423",
//!   trade_options
//! );
//! ```
//!
//! ## Links
//!
//! - [Official Website](https://datamaxiplus.com/)
//! - [Documentation](https://docs.datamaxiplus.com/)
//!
//! ## Contributing
//!
//! We welcome contributions!
//! If you discover a bug in this project, please feel free to open an issue to discuss the changes you would like to propose.
//!
//! ## License
//!
//![MIT License](./LICENSE)

/// API definitions and related utilities.
pub mod api;

/// CEX-related data fetcher and data structures.
///
/// This module provides functionality related to centralized exchange (CEX) data.
/// It includes data structures and methods for retrieving candle data, as well as information about supported exchanges, symbols and intervals.
///
/// # Usage
///
/// The `Candle` struct is the primary interface for interacting with the CEX data.
/// It provides methods for retrieving data with optional parameters to filter and sort the results.
///
/// ## Example
///
/// ```rust
/// let config = datamaxi::api::Config {
///     base_url: None,
///     api_key: "my_api_key".to_string(),
/// };
/// let client = datamaxi::api::Client::new(config);
/// let candle = datamaxi::cex::Candle { client: client.clone() };
///
/// // Retrieve supported exchanges
/// let exchanges = candle.exchanges("spot");
///
/// // Retrieve supported intervals
/// let intervals = candle.intervals();
///
/// // Retrieve supported Binance symbols
/// let symbols_options = datamaxi::cex::SymbolsOptions::new();
/// let symbols = candle.symbols("binance", symbols_options);
///
/// // Retrieve candle data
/// let candle_options = datamaxi::cex::CandleOptions::new().interval("1h").market("spot");
/// let candle_data = candle.get("binance", "BTC-USDT", candle_options);
/// ```
///
/// # Error Handling
///
/// All methods return a `Result` type, which should be handled appropriately to manage potential errors.
pub mod cex;

/// DEX-related data fetcher and data structures.
///
/// This module provides functionality related to decentralized exchange (DEX) data,
/// It includes data structures and methods for retrieving candle and trade data, as well as information supported chains, exchanges, pools and intervals.
///
/// # Usage
///
/// The `Candle` and `Trade` structs are the primary interfaces for interacting with the DEX data.
/// They provide methods for retrieving data with optional parameters to filter and sort the results.
///
/// ## Example
///
/// ```rust
/// let config = datamaxi::api::Config {
///     base_url: None,
///     api_key: "my_api_key".to_string(),
/// };
/// let client = datamaxi::api::Client::new(config);
/// let dex = datamaxi::dex::Dex { client: client.clone() };
///
/// // Retrieve candle data
/// let candle_options = datamaxi::dex::CandleOptions::new().interval("1h").limit(100);
/// let candle_data = dex.candle("kaia_mainnet", "dragonswap", "0x...", candle_options);
///
/// // Retrieve trade data
/// let trade_options = datamaxi::dex::TradeOptions::new().limit(50);
/// let trade_data = dex.trade("kaia_mainnet", "dragonswap", "0x...", trade_options);
///
/// // Retrieve available pools
/// let pools_options = datamaxi::dex::PoolsOptions::new().chain("kaia_mainnet");
/// let pools = dex.pools(pools_options);
/// ```
///
/// # Error Handling
///
/// All methods return a `Result` type, which should be handled appropriately to manage potential errors.
pub mod dex;

/// Data models representing API responses and optional parameters.
pub mod models;
