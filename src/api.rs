use reqwest::blocking::Response;
use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::io::Read;
use thiserror::Error;

const BASE_URL: &str = "https://api.datamaxiplus.com/api/v1";

/// A trait that defines the required methods for interacting with the Datamaxi+ API.
pub trait Datamaxi {
    /// Creates a new instance of the implementing type using the provided API key.
    fn new(api_key: String) -> Self;

    /// Creates a new instance of the implementing type using the provided API key and base URL.
    fn new_with_base_url(api_key: String, base_url: String) -> Self;
}

/// The configuration for the Datamaxi+ API client.
pub struct Config {
    /// The base URL for the API.
    pub base_url: Option<String>,

    /// The API key used for authentication.
    pub api_key: String,
}

/// The client for interacting with the Datamaxi+ API.
#[derive(Clone)]
pub struct Client {
    base_url: String,
    api_key: String,
    inner_client: reqwest::blocking::Client,
}

impl Client {
    /// Creates a new instance of the `Client` struct with the provided configuration.
    pub fn new(config: Config) -> Self {
        Client {
            base_url: config.base_url.unwrap_or(BASE_URL.to_string()),
            api_key: config.api_key,
            inner_client: reqwest::blocking::Client::builder()
                .pool_idle_timeout(None)
                .build()
                .unwrap(),
        }
    }

    /// Sends a GET request to the specified endpoint with optional parameters.
    pub fn get<T: DeserializeOwned>(
        &self,
        endpoint: &'static str,
        parameters: Option<HashMap<String, String>>,
    ) -> Result<T> {
        let url: String = format!("{}{}", self.base_url, endpoint);

        let mut request = self
            .inner_client
            .get(url.as_str())
            .header("X-DTMX-APIKEY", &self.api_key);

        if let Some(p) = parameters {
            request = request.query(&p);
        }

        let response = request.send()?;

        self.handle_response(response)
    }

    /// Processes the response from the API and returns the result.
    fn handle_response<T: DeserializeOwned>(&self, response: Response) -> Result<T> {
        match response.status() {
            StatusCode::OK => Ok(response.json::<T>()?),
            StatusCode::INTERNAL_SERVER_ERROR => {
                let mut response_text = String::new();
                response.take(1000).read_to_string(&mut response_text)?;
                Err(Error::InternalServerError(response_text))
            }
            StatusCode::UNAUTHORIZED => Err(Error::Unauthorized),
            StatusCode::BAD_REQUEST => {
                let mut response_text = String::new();
                response.take(1000).read_to_string(&mut response_text)?;
                Err(Error::BadRequest(response_text))
            }
            status => Err(Error::UnexpectedStatusCode(status.as_u16())),
        }
    }
}

/// A specialized [`Result`](std::result::Result) type for Datamaxi+ API calls.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors returned by the Datamaxi+ API client.
#[derive(Debug, Error)]
pub enum Error {
    /// The API returned a `400 Bad Request`; the payload carries the server message.
    #[error("Bad request: {0}")]
    BadRequest(String),

    /// The API returned a `401 Unauthorized` (missing or invalid API key).
    #[error("Unauthorized")]
    Unauthorized,

    /// The API returned a `500 Internal Server Error`; the payload carries the server message.
    #[error("Internal server error: {0}")]
    InternalServerError(String),

    /// The API returned a status code the client does not specifically handle.
    #[error("Received unexpected status code: {0}")]
    UnexpectedStatusCode(u16),

    /// The underlying HTTP request failed, or the response body could not be decoded.
    #[error(transparent)]
    Http(#[from] reqwest::Error),

    /// Reading the response body failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
