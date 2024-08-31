use datamaxi::{api::Datamaxi, fundingrate::*};

fn main() {
    let api_key: &str = "API_KEY";
    let fundingrate: FundingRate = Datamaxi::new(api_key.to_string());

    match fundingrate.exchanges("futures") {
        Ok(answer) => println!("{:?}", answer),
        Err(e) => println!("Error: {}", e),
    }
    match fundingrate.symbols("binance", "futures") {
        Ok(answer) => println!("{:?}", answer),
        Err(e) => println!("Error: {}", e),
    }
    match fundingrate.get_historical("ETH-USDT", "binance") {
        Ok(answer) => println!("{:?}", answer),
        Err(e) => println!("Error: {}", e),
    }
    match fundingrate.get_latest() {
        Ok(answer) => println!("{:?}", answer),
        Err(e) => println!("Error: {}", e),
    }
}
