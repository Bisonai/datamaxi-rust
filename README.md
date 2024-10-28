## DataMaxi+ Rust SDK

This is the official implementation of Rust SDK for [DataMaxi+](https://datamaxiplus.com/).
The package can be used to fetch both historical and latest data using [DataMaxi+ API](https://docs.datamaxiplus.com/).

- [Installation](#installation)
- [Configuration](#configuration)
- [Links](#links)
- [Contributing](#contributing)
- [License](#license)

### Installation

```shell
[dependencies]
datamaxi = { git = "https://github.com/bisonai/datamaxi-rust.git" }
```

### Configuration

Private API endpoints are protected by an API key.
You can get the API key upon registering at <https://datamaxiplus.com/auth>.


| Option     | Explanation                                                                   |
|------------|-------------------------------------------------------------------------------|
| `api_key`  | Your API key                                                                  |
| `base_url` | If `base_url` is not provided, it defaults to `https://api.datamaxiplus.com`. |

### Examples

#### CEX Candle

```rust
let api_key = "my_api_key".to_string();
let candle: datamaxi::cex::Candle = datamaxi::api::Datamaxi::new(api_key);

// Fetch supported exchanges for CEX candle data
candle.exchanges("spot");

// Fetch supported symbols for CEX candle data
let symbols_options = datamaxi::cex::SymbolsOptions::new();
candle.symbols("binance", symbols_options);

// Fetch supported intervals for CEX candle data
candle.intervals();

// Fetch CEX candle data
let candle_options = datamaxi::cex::CandleOptions::new();
candle.get("binance", "ETH-USDT", candle_options);
```

#### DEX

```rust
let api_key = "my_api_key".to_string();
let dex: datamaxi::dex::Dex = datamaxi::api::Datamaxi::new(api_key);

// Fetch supported intervals for DEX candle data
dex.intervals();

// Fetch supported exchange for DEX data
dex.exchanges();

// Fetch supported chains for DEX data
dex.chains();

// Fetch supported pools for DEX data
let pools_options = datamaxi::dex::PoolsOptions::new();
dex.pools(pools_options);

// Fetch DEX candle data
let params = datamaxi::dex::CandleOptions::new();
dex.candle(
  "bsc_mainnet",
  "pancakeswap",
  "0xb24cd29e32FaCDDf9e73831d5cD1FFcd1e535423",
  params,
);

// Fetch DEX trade data
let trade_options = datamaxi::dex::TradeOptions::new().limit(5);
dex.trade(
  "bsc_mainnet",
  "pancakeswap",
  "0xb24cd29e32FaCDDf9e73831d5cD1FFcd1e535423",
  trade_options
);
```

### Links

- [Official Website](https://datamaxiplus.com/)
- [Documentation](https://docs.datamaxiplus.com/)

### Contributing

We welcome contributions!
If you discover a bug in this project, please feel free to open an issue to discuss the changes you would like to propose.

### License

[MIT License](./LICENSE)
