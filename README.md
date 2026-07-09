# DataMaxi+ Rust SDK

This is the official Rust SDK for [DataMaxi+](https://datamaxiplus.com/).
Fetch both historical and real-time cryptocurrency data using the [DataMaxi+ API](https://docs.datamaxiplus.com/).

**Repository**: [Bisonai/datamaxi-rust](https://github.com/Bisonai/datamaxi-rust)

## Installation

```toml
[dependencies]
datamaxi = { git = "https://github.com/bisonai/datamaxi-rust.git" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

The client is **async** by default and needs an async runtime (e.g. `tokio`).
For a synchronous client, enable the `blocking` feature and use the mirrored
wrappers under `datamaxi::blocking`:

```toml
[dependencies]
datamaxi = { git = "https://github.com/bisonai/datamaxi-rust.git", features = ["blocking"] }
```

### Observability

Two independent, opt-in hooks:

- **`tracing` feature** — instruments each request with a span (`method`,
  `endpoint`, `attempt`, `status`) and debug events on retry (backoff delay,
  transient status/error). Off by default; adds no dependency when disabled.
  ```toml
  datamaxi = { git = "https://github.com/bisonai/datamaxi-rust.git", features = ["tracing"] }
  ```
- **Custom HTTP client** — `ClientBuilder::http_client` (and the `blocking`
  mirror) let you supply your own pre-built `reqwest::Client`, e.g. wrapped
  with `reqwest-middleware` for custom auth, metrics, or logging. Use the
  crate's re-exported `datamaxi::reqwest` to build it, so the type always
  matches without a version mismatch:
  ```rust,ignore
  let http = datamaxi::reqwest::Client::builder().build()?;
  let client = ClientBuilder::new().api_key("my_api_key").http_client(http).build()?;
  ```

### Pagination

`Client::paginate` / `blocking::Client::paginate` auto-paginate any paged
response envelope, with or without a reported `total`. The async paginator
is a plain `next_page()` cursor by default; enable the opt-in `stream`
feature to also drive it as a `futures::Stream` (adds no dependency unless
enabled):

```toml
[dependencies]
datamaxi = { git = "https://github.com/bisonai/datamaxi-rust.git", features = ["stream"] }
```

### Minimum Supported Rust Version (MSRV)

This crate requires **Rust 1.86** or newer. The MSRV is verified in CI and
may be raised in a minor version bump.

## Configuration

Private API endpoints require an API key. Register at [datamaxiplus.com/auth](https://datamaxiplus.com/auth) to get one.

| Option | Description |
|--------|-------------|
| `api_key` | Your DataMaxi+ API key |
| `base_url` | API base URL (default: `https://api.datamaxiplus.com`) |

### Environment Variable

Set `DATAMAXI_API_KEY` to avoid passing the key inline.

## Examples

### CEX Candle

```rust
use datamaxi::{
    CexCandleExchangesMarket, CexCandleMarket, CexCandleOptions, CexCandleSymbolsOptions,
    Client,
};

let client = Client::new("my_api_key");
let candle = client.cex_candle();

// Supported exchanges, symbols and intervals
candle.exchanges(CexCandleExchangesMarket::Spot).await?;
candle.symbols("binance", CexCandleSymbolsOptions::new()).await?;
candle.intervals().await?;

// Fetch candle data (exchange, symbol); market is an option
candle
    .get(
        "binance",
        "ETH-USDT",
        CexCandleOptions::new().market(CexCandleMarket::Spot),
    )
    .await?;
```

With the `blocking` feature the same calls are synchronous (no `.await`), via
`datamaxi::blocking::CexCandle` and `datamaxi::api::blocking`.

Data endpoints deserialize into typed response structs generated from the API
spec: object responses into a struct (e.g. `candle.get(..)` → `CexCandleResponse`)
and list responses into a `Vec` (e.g. `candle.exchanges(..)` → `Vec<String>`,
`sym.cautions(..)` → `Vec<CexSymbolCautionsView>`).

See [`examples/`](./examples/) for runnable examples.

## Links

- [Official Website](https://datamaxiplus.com/)
- [API Documentation](https://docs.datamaxiplus.com/)
- [Python SDK](https://github.com/bisonai/datamaxi-python)

## Contributing

We welcome contributions. If you discover a bug, please open an issue to discuss proposed changes.

`tests/live.rs` runs a small suite against the production API and is skipped
locally/in CI without a key. The `live` workflow (`.github/workflows/live.yml`)
runs it daily against production; a maintainer must provision the
`DATAMAXI_API_KEY` repo secret for it to do anything — without it the run is a
soft no-op (each test SKIPs).

## License

[MIT License](./LICENSE)
