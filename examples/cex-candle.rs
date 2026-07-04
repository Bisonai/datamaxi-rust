use datamaxi::api::Datamaxi;
use datamaxi::generated::{CexCandle, CexCandleOptions, CexCandleSymbolsOptions};
use std::env;

fn main() {
    dotenvy::dotenv().ok();
    let api_key = env::var("DATAMAXI_API_KEY").expect("DATAMAXI_API_KEY not found");
    let candle: CexCandle = Datamaxi::new(api_key);

    // CEX Candle Exchanges
    match candle.exchanges("futures") {
        Ok(answer) => println!("{}", answer),
        Err(e) => println!("Error: {}", e),
    }

    // CEX Candle Symbols
    let symbols_options = CexCandleSymbolsOptions::new().exchange("binance");
    match candle.symbols(symbols_options) {
        Ok(answer) => println!("{}", answer),
        Err(e) => println!("Error: {}", e),
    }

    // CEX Candle Intervals
    match candle.intervals() {
        Ok(answer) => println!("{}", answer),
        Err(e) => println!("Error: {}", e),
    }

    // CEX Candle Data
    let candle_options = CexCandleOptions::new();
    match candle.get("binance", "spot", "ETH-USDT", candle_options) {
        Ok(answer) => println!("{}", answer),
        Err(e) => println!("Error: {}", e),
    }
}
