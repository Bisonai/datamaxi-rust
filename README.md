# DataMaxi+ Rust Client

This is the official implementation of Rust client for DataMaxi+ API.
The package can be used to fetch both historical and latest data using [DataMaxi+ API](https://docs.datamaxiplus.com/).

- [Installation](#installation)
- [Quickstart](#quickstart)
- [Links](#links)
- [Contributing](#contributing)

## Installation

```shell
[dependencies]
binance = { git = "https://github.com/bisonai/datamaxi-rust.git" }
```

## Quickstart

### Candle

```rust
use datamaxi:: {api::Datamaxi, candle::*};

fn main() {
    let api_key: &str = "API_KEY";
    let candle: Candle = Datamaxi::new(api_key.to_string());

    match candle.exchanges("futures") {
        Ok(answer) => println!("{:?}", answer),
        Err(e) => println!("Error: {}", e),
    }
    match candle.symbols("binance", "futures") {
        Ok(answer) => println!("{:?}", answer),
        Err(e) => println!("Error: {}", e),
    }
    match candle.intervals("binance", "futures") {
        Ok(answer) => println!("{:?}", answer),
        Err(e) => println!("Error: {}", e),
    }
    match candle.get("ETH-USDT", "binance", "spot", "1d") {
        Ok(answer) => println!("{:?}", answer),
        Err(e) => println!("Error: {}", e),
    }
}

```

## Links

- [DataMaxi+](https://datamaxiplus.com/)
- [DataMaxi+ API Documentation](https://docs.datamaxiplus.com/)

## Contributing

We welcome contributions!
