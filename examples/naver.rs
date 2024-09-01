use datamaxi::{api::Datamaxi, naver::*};

fn main() {
    let api_key: &str = "API_KEY";
    let naver: NaverTrend = Datamaxi::new(api_key.to_string());

    match naver.symbols() {
        Ok(answer) => println!("{:?}", answer),
        Err(e) => println!("Error: {}", e),
    }
    match naver.get("AAVE") {
        Ok(answer) => println!("{:?}", answer),
        Err(e) => println!("Error: {}", e),
    }
}
