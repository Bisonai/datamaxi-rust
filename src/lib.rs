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
//! ```no_run
//! use datamaxi::api::Datamaxi;
//! use datamaxi::generated::{
//!     CexCandle, CexCandleExchangesMarket, CexCandleMarket, CexCandleOptions,
//!     CexCandleSymbolsOptions,
//! };
//!
//! let candle: CexCandle = Datamaxi::new("my_api_key".to_string());
//!
//! // Supported exchanges, symbols and intervals
//! let _ = candle.exchanges(CexCandleExchangesMarket::Spot);
//! let _ = candle.symbols("binance", CexCandleSymbolsOptions::new());
//! let _ = candle.intervals();
//!
//! // Fetch CEX candle data
//! let _ = candle.get(
//!     "binance",
//!     "BTC-USDT",
//!     CexCandleOptions::new().market(CexCandleMarket::Spot),
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

/// **Deprecated** hand-written CEX candle surface.
///
/// Superseded by [`crate::generated::CexCandle`], which is generated from the
/// API spec and stays in sync with it. This module is kept only for backward
/// compatibility and will be removed in a future release; new code should use
/// `generated::CexCandle`.
pub mod cex;

/// Data models representing API responses and optional parameters.
pub mod models;

/// Auto-generated typed wrappers for every public REST endpoint on
/// the data API. This module is the canonical surface for the OI /
/// Liquidation / cex-symbol surfaces; the older hand-written `cex`
/// module covers only a subset of endpoints and is kept for
/// backward compatibility.
///
/// Usage:
/// ```ignore
/// use datamaxi::api::Datamaxi;
/// use datamaxi::generated::{Liquidation, LiquidationHeatmapOptions};
///
/// let liq: Liquidation = Datamaxi::new("YOUR_API_KEY".into());
/// let opts = LiquidationHeatmapOptions::new();
/// let heatmap = liq.heatmap(opts)?;
/// ```
// `generated.rs` is code-generated (DO NOT EDIT); these lints reflect the
// upstream API's camelCase params and the generator's constructors, so they are
// suppressed at the module boundary rather than hand-edited in generated output.
#[allow(non_snake_case, unused_imports, clippy::new_without_default)]
pub mod generated;
