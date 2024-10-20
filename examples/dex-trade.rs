use std::env;

fn main() {
    dotenv::dotenv().ok();
    let api_key = env::var("DATAMAXI_API_KEY").expect("DATAMAXI_API_KEY not found");
    let trade: datamaxi::dex::Trade = datamaxi::api::Datamaxi::new(api_key);

    // DEX Trade Exchanges
    match trade.exchanges() {
        Ok(answer) => println!("{:?}", answer),
        Err(e) => println!("Error: {}", e),
    }

    // DEX Trade Chains
    match trade.chains() {
        Ok(answer) => println!("{:?}", answer),
        Err(e) => println!("Error: {}", e),
    }

    // DEX Trade Pools
    let pools_options = datamaxi::dex::PoolsOptions::new();
    let pools_response = trade.pools(pools_options);
    match pools_response {
        Ok(answer) => match serde_json::to_string(&answer) {
            Ok(json) => println!("{}", json),
            Err(e) => println!("Error: {}", e),
        },
        Err(e) => println!("Error: {}", e),
    }

    // DEX Trade Data
    let trade_options = datamaxi::dex::TradeOptions::new().limit(5);
    let trade_response = trade.get(
        "bsc_mainnet",
        "pancakeswap",
        "0xb24cd29e32FaCDDf9e73831d5cD1FFcd1e535423",
        trade_options,
    );
    match trade_response {
        Ok(answer) => match serde_json::to_string(&answer) {
            Ok(json) => println!("{}", json),
            Err(e) => println!("Error: {}", e),
        },
        Err(e) => println!("Error: {}", e),
    }
}
