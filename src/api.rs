use reqwest::blocking::Response;
use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::io::Read;
use std::time::Duration;
use thiserror::Error;

// Host only: the generated endpoint paths are fully qualified and already
// carry the `/api/v1` prefix, so the base URL must not repeat it (otherwise
// requests double-prefix to `/api/v1/api/v1/...`). Matches the documented
// default in the crate docs.
const BASE_URL: &str = "https://api.datamaxiplus.com";

/// Default per-request timeout, matching the Python SDK's default.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// Environment variable consulted for the API key when one is not passed explicitly.
const API_KEY_ENV: &str = "DATAMAXI_API_KEY";

/// The `User-Agent` sent with every request, e.g. `datamaxi-rust/0.4.0`.
fn user_agent() -> String {
    concat!("datamaxi-rust/", env!("CARGO_PKG_VERSION")).to_string()
}

/// Build the underlying blocking HTTP client with our defaults (timeout,
/// `User-Agent`, unbounded idle pool). Falls back to a default client if the
/// builder fails, so client construction is infallible and never panics.
fn build_inner_client(timeout: Duration) -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .pool_idle_timeout(None)
        .timeout(timeout)
        .user_agent(user_agent())
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new())
}

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

impl std::fmt::Debug for Client {
    /// Redacts the API key so it never leaks into logs or error output.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("base_url", &self.base_url)
            .field("api_key", &"<redacted>")
            .finish_non_exhaustive()
    }
}

impl Client {
    /// Creates a new instance of the `Client` struct with the provided configuration.
    ///
    /// Uses the default timeout ([`DEFAULT_TIMEOUT`]). For more control over
    /// timeout or reading the API key from the environment, use
    /// [`ClientBuilder`].
    pub fn new(config: Config) -> Self {
        Client {
            base_url: config.base_url.unwrap_or(BASE_URL.to_string()),
            api_key: config.api_key,
            inner_client: build_inner_client(DEFAULT_TIMEOUT),
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

/// Builder for a [`Client`], giving control over the API key source, base URL,
/// and request timeout.
///
/// The API key may be provided explicitly via [`api_key`](ClientBuilder::api_key)
/// or, if omitted, is read from the `DATAMAXI_API_KEY` environment variable at
/// [`build`](ClientBuilder::build) time.
///
/// # Example
/// ```no_run
/// use datamaxi::api::ClientBuilder;
/// use std::time::Duration;
///
/// // Explicit key + custom timeout.
/// let client = ClientBuilder::new()
///     .api_key("my_api_key")
///     .timeout(Duration::from_secs(30))
///     .build()
///     .expect("api key provided");
///
/// // Key taken from the DATAMAXI_API_KEY environment variable.
/// let client = ClientBuilder::new().build();
/// ```
#[derive(Debug, Clone)]
pub struct ClientBuilder {
    base_url: Option<String>,
    api_key: Option<String>,
    timeout: Duration,
}

impl ClientBuilder {
    /// Creates a new builder with default settings (default timeout, key read
    /// from the environment on `build`).
    pub fn new() -> Self {
        ClientBuilder {
            base_url: None,
            api_key: None,
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Sets the API key explicitly, overriding the `DATAMAXI_API_KEY` environment variable.
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Overrides the base URL (defaults to the production API).
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// Sets the per-request timeout (defaults to 10 seconds).
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Builds the [`Client`].
    ///
    /// Resolves the API key from the explicit value or the `DATAMAXI_API_KEY`
    /// environment variable, returning [`Error::MissingApiKey`] if neither is set.
    pub fn build(self) -> Result<Client> {
        let api_key = self
            .api_key
            .or_else(|| std::env::var(API_KEY_ENV).ok())
            .filter(|key| !key.trim().is_empty())
            .ok_or(Error::MissingApiKey)?;

        Ok(Client {
            base_url: self.base_url.unwrap_or_else(|| BASE_URL.to_string()),
            api_key,
            inner_client: build_inner_client(self.timeout),
        })
    }
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// A specialized [`Result`](std::result::Result) type for Datamaxi+ API calls.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors returned by the Datamaxi+ API client.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// No API key was provided explicitly and `DATAMAXI_API_KEY` is unset or empty.
    #[error("missing API key: pass it to ClientBuilder::api_key or set DATAMAXI_API_KEY")]
    MissingApiKey,

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
