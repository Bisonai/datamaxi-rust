use error_chain::error_chain;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DatamaxiContentError {
    pub code: i16,
    pub msg: String,
}

error_chain! {
    errors {
        DatamaxiError(response: DatamaxiContentError)

        ValueMissingError(index: usize, name: &'static str) {
            description("invalid value"),
            display("{} at {} is missing", name, index),
        }
        BadRequest(msg: String) {
            description("Bad request")
            display("Bad request: {}", msg)
        }
        Unauthorized {
            description("Unauthorized")
            display("Unauthorized")
        }
        ServiceUnavailable {
            description("Service unavailable")
            display("Service unavailable")
        }
        InternalServerError(msg: String) {
            description("Internal server error")
            display("Internal server error: {}", msg)
        }
        UnexpectedStatusCode(status: u16) {
            description("Unexpected status code")
            display("Received unexpected status code: {}", status)
        }
     }

    foreign_links {
        ReqError(reqwest::Error);
        InvalidHeaderError(reqwest::header::InvalidHeaderValue);
        IoError(std::io::Error);
        ParseFloatError(std::num::ParseFloatError);
        UrlParserError(url::ParseError);
        Json(serde_json::Error);
        Tungstenite(tungstenite::Error);
        TimestampError(std::time::SystemTimeError);
    }
}
