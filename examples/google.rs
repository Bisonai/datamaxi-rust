use datamaxi::{api::Datamaxi, google::*};

fn main() {
    let api_key: &str = "API_KEY";
    let google: GoogleTrend = Datamaxi::new(api_key.to_string());

    match google.keywords() {
        Ok(answer) => println!("{:?}", answer),
        Err(e) => println!("Error: {}", e),
    }
    match google.get("Bitcoin") {
        Ok(answer) => println!("{:?}", answer),
        Err(e) => println!("Error: {}", e),
    }
}
