use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
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

/// Default number of retries on transient failures. Zero keeps the client's
/// behavior unchanged unless retries are explicitly opted into via
/// [`ClientBuilder::max_retries`].
const DEFAULT_MAX_RETRIES: u32 = 0;

/// Default base delay for exponential backoff between retries.
const DEFAULT_RETRY_BASE_DELAY: Duration = Duration::from_millis(500);

/// Hard cap on any single backoff/`Retry-After` wait, so a huge exponent or an
/// abusive `Retry-After` header can never stall a request indefinitely.
const RETRY_MAX_DELAY: Duration = Duration::from_secs(30);

/// Retry/backoff policy shared by the async and blocking clients.
///
/// Transient conditions — request timeouts, connection errors, `429 Too Many
/// Requests`, and `5xx` server errors — are retried up to `max_retries` times
/// with exponential backoff (`base_delay * 2^attempt`, capped at
/// [`RETRY_MAX_DELAY`]). A `429` response honors its `Retry-After` header (in
/// seconds) when present. Fatal statuses (`400`/`401`/`403`/`404`, and every
/// other `4xx`) are never retried.
#[derive(Debug, Clone)]
struct RetryConfig {
    max_retries: u32,
    base_delay: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        RetryConfig {
            max_retries: DEFAULT_MAX_RETRIES,
            base_delay: DEFAULT_RETRY_BASE_DELAY,
        }
    }
}

/// Whether a response status is transient and worth retrying: `429` or any
/// `5xx`. All other statuses (including the fatal `400`/`401`/`403`/`404`) are
/// terminal.
fn is_retryable_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

/// Whether a transport-level error is transient: a timeout or a failure to
/// connect. Other errors (e.g. body decode) are terminal.
fn is_retryable_error(error: &reqwest::Error) -> bool {
    error.is_timeout() || error.is_connect()
}

/// Exponential backoff for the given zero-based `attempt`: `base * 2^attempt`,
/// saturating and capped at [`RETRY_MAX_DELAY`].
fn backoff_delay(config: &RetryConfig, attempt: u32) -> Duration {
    let factor = 1u32.checked_shl(attempt).unwrap_or(u32::MAX);
    config
        .base_delay
        .checked_mul(factor)
        .unwrap_or(RETRY_MAX_DELAY)
        .min(RETRY_MAX_DELAY)
}

/// Parse a `Retry-After` header expressed as an integer number of seconds,
/// capped at [`RETRY_MAX_DELAY`]. The HTTP-date form is not honored (returns
/// `None`, so the caller falls back to exponential backoff).
fn retry_after_delay(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    let secs = headers
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()?;
    Some(Duration::from_secs(secs).min(RETRY_MAX_DELAY))
}

/// The delay to wait before retrying a retryable response: a `429`'s
/// `Retry-After` when present, otherwise exponential backoff.
fn retry_delay_for_response(
    config: &RetryConfig,
    status: StatusCode,
    headers: &reqwest::header::HeaderMap,
    attempt: u32,
) -> Duration {
    if status == StatusCode::TOO_MANY_REQUESTS {
        if let Some(delay) = retry_after_delay(headers) {
            return delay;
        }
    }
    backoff_delay(config, attempt)
}

/// The `User-Agent` sent with every request, e.g. `datamaxi-rust/0.4.0`.
fn user_agent() -> String {
    concat!("datamaxi-rust/", env!("CARGO_PKG_VERSION")).to_string()
}

/// Truncate a server error body to at most 1000 bytes, on a UTF-8 char boundary.
fn truncate_body(mut s: String) -> String {
    if s.len() > 1000 {
        let mut end = 1000;
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        s.truncate(end);
    }
    s
}

/// Parse a `Retry-After` header into a [`Duration`].
///
/// Only the numeric delay-seconds form (RFC 9110 §10.2.3) is understood; the
/// alternative HTTP-date form yields `None` rather than panicking. A missing,
/// non-ASCII, or unparseable header also yields `None`.
fn parse_retry_after(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    let value = headers.get(reqwest::header::RETRY_AFTER)?;
    let secs = value.to_str().ok()?.trim().parse::<u64>().ok()?;
    Some(Duration::from_secs(secs))
}

/// Build the underlying async HTTP client with our defaults (timeout,
/// `User-Agent`, unbounded idle pool). Falls back to a default client if the
/// builder fails, so client construction is infallible and never panics.
fn build_inner_client(timeout: Duration) -> reqwest::Client {
    reqwest::Client::builder()
        .pool_idle_timeout(None)
        .timeout(timeout)
        .user_agent(user_agent())
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

/// The async client for interacting with the Datamaxi+ API.
///
/// This is the default surface. For a synchronous client, enable the
/// `blocking` feature and use [`blocking::Client`].
#[derive(Clone)]
pub struct Client {
    base_url: String,
    api_key: String,
    inner_client: reqwest::Client,
    retry: RetryConfig,
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
    /// Creates a new client authenticating with the given API key.
    ///
    /// Uses the production base URL and the default timeout
    /// ([`DEFAULT_TIMEOUT`]). For control over the base URL, timeout, or
    /// reading the API key from the environment, use [`ClientBuilder`]. Endpoint
    /// groups are reached via accessors, e.g. [`Client::cex_candle`].
    pub fn new(api_key: impl Into<String>) -> Self {
        Client {
            base_url: BASE_URL.to_string(),
            api_key: api_key.into(),
            inner_client: build_inner_client(DEFAULT_TIMEOUT),
            retry: RetryConfig::default(),
        }
    }

    /// Sends a GET request to the specified endpoint with optional parameters.
    ///
    /// Transient failures (timeouts, connection errors, `429`, and `5xx`) are
    /// retried per the client's [`RetryConfig`] with exponential backoff; a
    /// `429` honors its `Retry-After` header. Fatal statuses
    /// (`400`/`401`/`403`/`404`) are returned without retry.
    pub async fn get<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        parameters: Option<HashMap<String, String>>,
    ) -> Result<T> {
        let url: String = format!("{}{}", self.base_url, endpoint);
        let mut attempt: u32 = 0;

        loop {
            let mut request = self
                .inner_client
                .get(url.as_str())
                .header("X-DTMX-APIKEY", &self.api_key);

            if let Some(ref p) = parameters {
                request = request.query(p);
            }

            match request.send().await {
                Ok(response) => {
                    let status = response.status();
                    if attempt < self.retry.max_retries && is_retryable_status(status) {
                        let delay = retry_delay_for_response(
                            &self.retry,
                            status,
                            response.headers(),
                            attempt,
                        );
                        attempt += 1;
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                    return handle_response(response).await;
                }
                Err(error) => {
                    if attempt < self.retry.max_retries && is_retryable_error(&error) {
                        let delay = backoff_delay(&self.retry, attempt);
                        attempt += 1;
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                    return Err(Error::from(error));
                }
            }
        }
    }
}

/// Processes an async response from the API and returns the result.
async fn handle_response<T: DeserializeOwned>(response: reqwest::Response) -> Result<T> {
    match response.status() {
        StatusCode::OK => Ok(response.json::<T>().await?),
        StatusCode::INTERNAL_SERVER_ERROR => {
            let body = response.text().await.unwrap_or_default();
            Err(Error::InternalServerError(truncate_body(body)))
        }
        StatusCode::UNAUTHORIZED => Err(Error::Unauthorized),
        StatusCode::FORBIDDEN => Err(Error::Forbidden),
        StatusCode::NOT_FOUND => Err(Error::NotFound),
        StatusCode::TOO_MANY_REQUESTS => Err(Error::RateLimited {
            retry_after: parse_retry_after(response.headers()),
        }),
        StatusCode::BAD_REQUEST => {
            let body = response.text().await.unwrap_or_default();
            Err(Error::BadRequest(truncate_body(body)))
        }
        status => Err(Error::UnexpectedStatusCode(status.as_u16())),
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
    retry: RetryConfig,
}

impl ClientBuilder {
    /// Creates a new builder with default settings (default timeout, no retries,
    /// key read from the environment on `build`).
    pub fn new() -> Self {
        ClientBuilder {
            base_url: None,
            api_key: None,
            timeout: DEFAULT_TIMEOUT,
            retry: RetryConfig::default(),
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

    /// Sets the maximum number of retries on transient failures (timeouts,
    /// connection errors, `429`, and `5xx`). Defaults to `0` (no retries);
    /// each retry backs off exponentially from the base delay.
    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.retry.max_retries = max_retries;
        self
    }

    /// Sets the base delay for exponential retry backoff (defaults to 500ms).
    /// The nth retry waits `base_delay * 2^n`, capped at 30 seconds.
    pub fn retry_base_delay(mut self, base_delay: Duration) -> Self {
        self.retry.base_delay = base_delay;
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
            retry: self.retry,
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

    /// The API returned a `403 Forbidden` (the key is valid but lacks access to the resource).
    #[error("Forbidden")]
    Forbidden,

    /// The API returned a `404 Not Found` (the resource or endpoint does not exist).
    #[error("Not found")]
    NotFound,

    /// The API returned a `429 Too Many Requests` (rate limited).
    ///
    /// `retry_after` carries the `Retry-After` header when present and expressed
    /// as a delay in seconds; the HTTP-date form is not parsed and yields `None`.
    #[error("Rate limited")]
    RateLimited {
        /// Suggested wait before retrying, from the `Retry-After` header.
        retry_after: Option<Duration>,
    },

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

/// Synchronous client surface, enabled by the `blocking` feature.
///
/// Mirrors the async [`Client`] with the same status-to-[`Error`] mapping, for
/// scripts, notebooks, and other callers that don't run an async runtime. The
/// generated endpoint wrappers under [`crate::generated::blocking`] use this.
#[cfg(feature = "blocking")]
pub mod blocking {
    use super::{
        backoff_delay, is_retryable_error, is_retryable_status, parse_retry_after,
        retry_delay_for_response, truncate_body, user_agent, Error, Result, RetryConfig,
        API_KEY_ENV, BASE_URL, DEFAULT_TIMEOUT,
    };
    use reqwest::blocking::Response;
    use reqwest::StatusCode;
    use serde::de::DeserializeOwned;
    use std::collections::HashMap;
    use std::io::Read;
    use std::time::Duration;

    /// Build the underlying blocking HTTP client with our defaults. Falls back
    /// to a default client if the builder fails, so construction never panics.
    fn build_inner_client(timeout: Duration) -> reqwest::blocking::Client {
        reqwest::blocking::Client::builder()
            .pool_idle_timeout(None)
            .timeout(timeout)
            .user_agent(user_agent())
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new())
    }

    /// The blocking client for interacting with the Datamaxi+ API.
    #[derive(Clone)]
    pub struct Client {
        base_url: String,
        api_key: String,
        inner_client: reqwest::blocking::Client,
        retry: RetryConfig,
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
        /// Creates a new blocking `Client` authenticating with the given API
        /// key, using the production base URL and the default timeout. For more
        /// control, use [`ClientBuilder`]. Endpoint groups are reached via
        /// accessors, e.g. [`Client::cex_candle`].
        pub fn new(api_key: impl Into<String>) -> Self {
            Client {
                base_url: BASE_URL.to_string(),
                api_key: api_key.into(),
                inner_client: build_inner_client(DEFAULT_TIMEOUT),
                retry: RetryConfig::default(),
            }
        }

        /// Sends a GET request to the specified endpoint with optional parameters.
        ///
        /// Mirrors the async [`super::Client::get`] retry behavior: transient
        /// failures (timeouts, connection errors, `429`, and `5xx`) are retried
        /// per the client's retry config with exponential backoff (a `429`
        /// honors `Retry-After`); fatal statuses are returned without retry.
        /// Backoff waits use a blocking [`std::thread::sleep`].
        pub fn get<T: DeserializeOwned>(
            &self,
            endpoint: &str,
            parameters: Option<HashMap<String, String>>,
        ) -> Result<T> {
            let url: String = format!("{}{}", self.base_url, endpoint);
            let mut attempt: u32 = 0;

            loop {
                let mut request = self
                    .inner_client
                    .get(url.as_str())
                    .header("X-DTMX-APIKEY", &self.api_key);

                if let Some(ref p) = parameters {
                    request = request.query(p);
                }

                match request.send() {
                    Ok(response) => {
                        let status = response.status();
                        if attempt < self.retry.max_retries && is_retryable_status(status) {
                            let delay = retry_delay_for_response(
                                &self.retry,
                                status,
                                response.headers(),
                                attempt,
                            );
                            attempt += 1;
                            std::thread::sleep(delay);
                            continue;
                        }
                        return handle_response(response);
                    }
                    Err(error) => {
                        if attempt < self.retry.max_retries && is_retryable_error(&error) {
                            let delay = backoff_delay(&self.retry, attempt);
                            attempt += 1;
                            std::thread::sleep(delay);
                            continue;
                        }
                        return Err(Error::from(error));
                    }
                }
            }
        }
    }

    /// Processes a blocking response from the API and returns the result.
    fn handle_response<T: DeserializeOwned>(response: Response) -> Result<T> {
        match response.status() {
            StatusCode::OK => Ok(response.json::<T>()?),
            StatusCode::INTERNAL_SERVER_ERROR => {
                let mut body = String::new();
                response.take(1000).read_to_string(&mut body)?;
                Err(Error::InternalServerError(truncate_body(body)))
            }
            StatusCode::UNAUTHORIZED => Err(Error::Unauthorized),
            StatusCode::FORBIDDEN => Err(Error::Forbidden),
            StatusCode::NOT_FOUND => Err(Error::NotFound),
            StatusCode::TOO_MANY_REQUESTS => Err(Error::RateLimited {
                retry_after: parse_retry_after(response.headers()),
            }),
            StatusCode::BAD_REQUEST => {
                let mut body = String::new();
                response.take(1000).read_to_string(&mut body)?;
                Err(Error::BadRequest(truncate_body(body)))
            }
            status => Err(Error::UnexpectedStatusCode(status.as_u16())),
        }
    }

    /// Builder for a blocking [`Client`], mirroring the async [`super::ClientBuilder`].
    #[derive(Debug, Clone)]
    pub struct ClientBuilder {
        base_url: Option<String>,
        api_key: Option<String>,
        timeout: Duration,
        retry: RetryConfig,
    }

    impl ClientBuilder {
        /// Creates a new builder with default settings.
        pub fn new() -> Self {
            ClientBuilder {
                base_url: None,
                api_key: None,
                timeout: DEFAULT_TIMEOUT,
                retry: RetryConfig::default(),
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

        /// Sets the maximum number of retries on transient failures (timeouts,
        /// connection errors, `429`, and `5xx`). Defaults to `0` (no retries).
        pub fn max_retries(mut self, max_retries: u32) -> Self {
            self.retry.max_retries = max_retries;
            self
        }

        /// Sets the base delay for exponential retry backoff (defaults to
        /// 500ms). The nth retry waits `base_delay * 2^n`, capped at 30 seconds.
        pub fn retry_base_delay(mut self, base_delay: Duration) -> Self {
            self.retry.base_delay = base_delay;
            self
        }

        /// Builds the blocking [`Client`], resolving the API key from the
        /// explicit value or the `DATAMAXI_API_KEY` environment variable.
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
                retry: self.retry,
            })
        }
    }

    impl Default for ClientBuilder {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{HeaderMap, HeaderValue, RETRY_AFTER};

    #[test]
    fn retryable_statuses_are_429_and_5xx() {
        assert!(is_retryable_status(StatusCode::TOO_MANY_REQUESTS));
        assert!(is_retryable_status(StatusCode::INTERNAL_SERVER_ERROR));
        assert!(is_retryable_status(StatusCode::BAD_GATEWAY));
        assert!(is_retryable_status(StatusCode::SERVICE_UNAVAILABLE));
    }

    #[test]
    fn fatal_statuses_are_not_retryable() {
        for status in [
            StatusCode::BAD_REQUEST,
            StatusCode::UNAUTHORIZED,
            StatusCode::FORBIDDEN,
            StatusCode::NOT_FOUND,
        ] {
            assert!(!is_retryable_status(status), "{status} must not be retried");
        }
    }

    #[test]
    fn backoff_is_exponential_and_capped() {
        let config = RetryConfig {
            max_retries: 5,
            base_delay: Duration::from_millis(100),
        };
        assert_eq!(backoff_delay(&config, 0), Duration::from_millis(100));
        assert_eq!(backoff_delay(&config, 1), Duration::from_millis(200));
        assert_eq!(backoff_delay(&config, 2), Duration::from_millis(400));
        // A large attempt saturates to the hard cap rather than overflowing.
        assert_eq!(backoff_delay(&config, 1000), RETRY_MAX_DELAY);
    }

    #[test]
    fn retry_after_parses_integer_seconds_and_caps() {
        let mut headers = HeaderMap::new();
        headers.insert(RETRY_AFTER, HeaderValue::from_static("2"));
        assert_eq!(retry_after_delay(&headers), Some(Duration::from_secs(2)));

        headers.insert(RETRY_AFTER, HeaderValue::from_static("9999"));
        assert_eq!(retry_after_delay(&headers), Some(RETRY_MAX_DELAY));
    }

    #[test]
    fn retry_after_ignores_http_date_and_missing() {
        let empty = HeaderMap::new();
        assert_eq!(retry_after_delay(&empty), None);

        let mut headers = HeaderMap::new();
        headers.insert(
            RETRY_AFTER,
            HeaderValue::from_static("Wed, 21 Oct 2015 07:28:00 GMT"),
        );
        assert_eq!(retry_after_delay(&headers), None);
    }

    #[test]
    fn retry_delay_prefers_retry_after_only_for_429() {
        let config = RetryConfig {
            max_retries: 3,
            base_delay: Duration::from_millis(100),
        };
        let mut headers = HeaderMap::new();
        headers.insert(RETRY_AFTER, HeaderValue::from_static("5"));

        // 429 with Retry-After honors the header.
        assert_eq!(
            retry_delay_for_response(&config, StatusCode::TOO_MANY_REQUESTS, &headers, 0),
            Duration::from_secs(5)
        );
        // 5xx ignores Retry-After and uses backoff.
        assert_eq!(
            retry_delay_for_response(&config, StatusCode::BAD_GATEWAY, &headers, 1),
            Duration::from_millis(200)
        );
        // 429 without Retry-After falls back to backoff.
        let empty = HeaderMap::new();
        assert_eq!(
            retry_delay_for_response(&config, StatusCode::TOO_MANY_REQUESTS, &empty, 2),
            Duration::from_millis(400)
        );
    }
}
