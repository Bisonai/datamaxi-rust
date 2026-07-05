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
//! The client is async by default (requires a runtime such as `tokio`). For a
//! synchronous client, enable the `blocking` feature and use the mirrored
//! wrappers under `datamaxi::generated::blocking` with `datamaxi::api::blocking`.
//!
//! ```no_run
//! use datamaxi::api::Datamaxi;
//! use datamaxi::generated::{
//!     CexCandle, CexCandleExchangesMarket, CexCandleMarket, CexCandleOptions,
//!     CexCandleSymbolsOptions,
//! };
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let candle: CexCandle = Datamaxi::new("my_api_key".to_string());
//!
//! // Supported exchanges, symbols and intervals
//! let _ = candle.exchanges(CexCandleExchangesMarket::Spot).await?;
//! let _ = candle.symbols("binance", CexCandleSymbolsOptions::new()).await?;
//! let _ = candle.intervals().await?;
//!
//! // Fetch CEX candle data
//! let _ = candle
//!     .get(
//!         "binance",
//!         "BTC-USDT",
//!         CexCandleOptions::new().market(CexCandleMarket::Spot),
//!     )
//!     .await?;
//! # Ok(())
//! # }
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

/// Auto-generated typed wrappers for every public REST endpoint on the data
/// API. This is the canonical surface for every endpoint group (CEX candle,
/// OI, Liquidation, cex-symbol, …).
///
/// Usage:
/// ```ignore
/// use datamaxi::api::Datamaxi;
/// use datamaxi::generated::{Liquidation, LiquidationHeatmapOptions};
///
/// let liq: Liquidation = Datamaxi::new("YOUR_API_KEY".into());
/// let opts = LiquidationHeatmapOptions::new();
/// let heatmap = liq.heatmap(opts).await?;
/// ```
// `generated.rs` is code-generated (DO NOT EDIT); these lints reflect the
// generator's unconditional imports and its `new()`-only option constructors,
// so they are suppressed at the module boundary rather than hand-edited in
// generated output.
#[allow(unused_imports, clippy::new_without_default)]
pub mod generated;
