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
