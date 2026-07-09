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
//! ### Minimum Supported Rust Version (MSRV)
//!
//! This crate requires **Rust 1.86** or newer. The MSRV is verified in CI and
//! may be raised in a minor version bump.
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
//! wrappers under `datamaxi::blocking` with `datamaxi::api::blocking`.
//!
//! ```no_run
//! use datamaxi::{
//!     CexCandleExchangesMarket, CexCandleMarket, CexCandleOptions, CexCandleSymbolsOptions,
//!     Client,
//! };
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let client = Client::new("my_api_key");
//! let candle = client.cex_candle();
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

// `generated.rs` is code-generated (DO NOT EDIT). Its contents are re-exported
// at the crate root (below), so callers write `datamaxi::CexCandle` rather than
// through this module path. Hidden from the docs but kept `pub` for backward
// compatibility. The lint allows reflect the generator's unconditional imports
// and its `new()`-only option constructors.
#[doc(hidden)]
#[allow(unused_imports, clippy::new_without_default)]
pub mod generated;

/// Typed wrappers for every REST endpoint on the data API — the canonical
/// surface (CEX candle, OI, Liquidation, cex-symbol, …). Endpoint groups are
/// reached through accessors on the root [`Client`]. Async by default; with the
/// `blocking` feature, a parallel [`blocking`] module offers synchronous
/// equivalents.
///
/// ```ignore
/// use datamaxi::{Client, LiquidationHeatmapOptions};
///
/// let client = Client::new("YOUR_API_KEY");
/// let heatmap = client
///     .liquidation()
///     .heatmap(LiquidationHeatmapOptions::new())
///     .await?;
/// ```
pub use generated::*;

/// The root client and its builder are re-exported at the crate root so callers
/// write `datamaxi::Client` / `datamaxi::ClientBuilder`. Endpoint groups hang
/// off the client via generated accessors, e.g. `client.cex_candle()`.
pub use api::{Client, ClientBuilder};

/// Re-exported so callers can name the exact `reqwest::Client` /
/// `reqwest::blocking::Client` type expected by
/// [`ClientBuilder::http_client`] / `blocking::ClientBuilder::http_client`
/// (and the `reqwest::Error` wrapped by [`api::Error::Http`]) without adding
/// `reqwest` to their own `Cargo.toml` and risking a version mismatch with
/// this crate's dependency.
pub use reqwest;
