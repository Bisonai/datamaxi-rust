# DataMaxi+ Rust SDK

This is the official Rust SDK for [DataMaxi+](https://datamaxiplus.com/).
Fetch both historical and real-time cryptocurrency data using the [DataMaxi+ API](https://docs.datamaxiplus.com/).

**Repository**: [Bisonai/datamaxi-rust](https://github.com/Bisonai/datamaxi-rust)

## Installation

```toml
[dependencies]
datamaxi = { git = "https://github.com/bisonai/datamaxi-rust.git" }
```

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
use datamaxi::api::Datamaxi;
use datamaxi::generated::{
    CexCandle, CexCandleExchangesMarket, CexCandleMarket, CexCandleOptions,
    CexCandleSymbolsOptions,
};

let api_key = "my_api_key".to_string();
let candle: CexCandle = Datamaxi::new(api_key);

// Supported exchanges, symbols and intervals
candle.exchanges(CexCandleExchangesMarket::Spot);
candle.symbols("binance", CexCandleSymbolsOptions::new());
candle.intervals();

// Fetch candle data (exchange, symbol); market is an option
candle.get(
    "binance",
    "ETH-USDT",
    CexCandleOptions::new().market(CexCandleMarket::Spot),
);
```

Data endpoints deserialize into typed response structs generated from the API
spec: object responses into a struct (e.g. `candle.get(..)` → `CexCandleResponse`)
and list responses into a `Vec` (e.g. `candle.exchanges(..)` → `Vec<String>`,
`sym.cautions(..)` → `Vec<CexSymbolCautionsView>`).

See [`examples/`](./examples/) for runnable examples.

## Code Generation

Most of the SDK is auto-generated from the OpenAPI spec via `datamaxi-codegen`. Generated code is in `src/generated.rs` and marked with `DO NOT EDIT`. Manual edits to generated files will be overwritten.

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
