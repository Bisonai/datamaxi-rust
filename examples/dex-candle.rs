use std::env;

fn main() {
    dotenv::dotenv().ok();
    let api_key = env::var("DATAMAXI_API_KEY").expect("DATAMAXI_API_KEY not found");
    let candle: datamaxi::dex::Candle = datamaxi::api::Datamaxi::new(api_key);

    // DEX Candle Intervals
    // match candle.intervals() {
    //     Ok(answer) => println!("{:?}", answer),
    //     Err(e) => println!("Error: {}", e),
    // }

    // // DEX Candle Exchanges
    // match candle.exchanges() {
    //     Ok(answer) => println!("{:?}", answer),
    //     Err(e) => println!("Error: {}", e),
    // }

    // // DEX Candle Chains
    // match candle.chains() {
    //     Ok(answer) => println!("{:?}", answer),
    //     Err(e) => println!("Error: {}", e),
    // }

    // DEX Candle Pools
    // let pools_options = datamaxi::dex::PoolsOptions::new();
    // let pools_response = candle.pools(pools_options);
    // match pools_response {
    //     Ok(answer) => match serde_json::to_string(&answer) {
    //         Ok(json) => println!("{}", json),
    //         Err(e) => println!("Error: {}", e),
    //     },
    //     Err(e) => println!("Error: {}", e),
    // }

    // DEX Candle Data
    let params = datamaxi::dex::CandleOptions::new();
    let candle_response = candle.get(
        "bsc_mainnet",
        "pancakeswap",
        "0xb24cd29e32FaCDDf9e73831d5cD1FFcd1e535423",
        params,
    );
    match candle_response {
        Ok(answer) => match serde_json::to_string(&answer) {
            Ok(json) => println!("{}", json),
            Err(e) => println!("Error: {}", e),
        },
        Err(e) => println!("Error: {}", e),
    }
}
