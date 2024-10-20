use std::env;

fn main() {
    dotenv::dotenv().ok();
    let api_key = env::var("DATAMAXI_API_KEY").expect("DATAMAXI_API_KEY not found");
    let candle: datamaxi::cex::Candle = datamaxi::api::Datamaxi::new(api_key);

    // CEX Candle Exchanges
    match candle.exchanges("futures") {
        Ok(answer) => println!("{:?}", answer),
        Err(e) => println!("Error: {}", e),
    }

    // CEX Candle Symbols
    let symbols_options = datamaxi::cex::SymbolsOptions::new();
    let symbols_response = candle.symbols("binance", symbols_options);
    match symbols_response {
        Ok(answer) => match serde_json::to_string(&answer) {
            Ok(json) => println!("{}", json),
            Err(e) => println!("Error: {}", e),
        },
        Err(e) => println!("Error: {}", e),
    }

    // CEX Candle Intervals
    match candle.intervals() {
        Ok(answer) => println!("{:?}", answer),
        Err(e) => println!("Error: {}", e),
    }

    // CEX Candle Data
    let candle_options = datamaxi::cex::CandleOptions::new();
    let candle_response = candle.get("binance", "ETH-USDT", candle_options);
    match candle_response {
        Ok(answer) => match serde_json::to_string(&answer) {
            Ok(json) => println!("{}", json),
            Err(e) => println!("Error: {}", e),
        },
        Err(e) => println!("Error: {}", e),
    }
}
