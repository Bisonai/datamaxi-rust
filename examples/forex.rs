use datamaxi::{api::Datamaxi, forex::*};

fn main() {
    let api_key: &str = "API_KEY";
    let forex: Forex = Datamaxi::new(api_key.to_string());

    match forex.symbols() {
        Ok(answer) => println!("{:?}", answer),
        Err(e) => println!("Error: {}", e),
    }
    match forex.get() {
        Ok(answer) => println!("{:?}", answer),
        Err(e) => println!("Error: {}", e),
    }
}
