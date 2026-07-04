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
let api_key = "my_api_key".to_string();
let candle: datamaxi::cex::Candle = datamaxi::api::Datamaxi::new(api_key);

// Supported exchanges and symbols
candle.exchanges("spot");
let symbols_options = datamaxi::cex::SymbolsOptions::new();
candle.symbols("binance", symbols_options);
candle.intervals();

// Fetch candle data
let candle_options = datamaxi::cex::CandleOptions::new();
candle.get("binance", "ETH-USDT", candle_options);
```

See [`examples/`](./examples/) for runnable examples.

## Code Generation

Most of the SDK is auto-generated from the OpenAPI spec via `datamaxi-codegen`. Generated code is in `src/generated.rs` and marked with `DO NOT EDIT`. Manual edits to generated files will be overwritten.

## Links

- [Official Website](https://datamaxiplus.com/)
- [API Documentation](https://docs.datamaxiplus.com/)
- [Python SDK](https://github.com/bisonai/datamaxi-python)

## Contributing

We welcome contributions. If you discover a bug, please open an issue to discuss proposed changes.

## License

[MIT License](./LICENSE)
