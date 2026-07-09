//! ## Observability
//!
//! Two independent, additive, opt-in hooks:
//!
//! - **`tracing` feature** — instruments [`Client::get`] / [`blocking::Client::get`]
//!   with a span (`method`, `endpoint`, `attempt`, `status`) and debug events on
//!   each retry (backoff delay, transient status/error). Off by default and
//!   compiles away entirely (no `tracing` dependency pulled in) when disabled.
//!   The API key is never recorded — [`Client`]'s `Debug` impl already redacts
//!   it, and no span/event field ever carries it.
//! - **Custom HTTP client** — [`ClientBuilder::http_client`] /
//!   [`blocking::ClientBuilder::http_client`] let callers supply their own
//!   pre-built `reqwest::Client`, e.g. wrapped with `reqwest-middleware` for
//!   custom auth, metrics, or logging middleware. When omitted, the client
//!   falls back to the built-in defaults (`User-Agent`, unbounded idle pool,
//!   the configured timeout).
//!
//! ## Pagination
//!
//! [`Client::paginate`] / [`blocking::Client::paginate`] auto-paginate any
//! [`Paginated`] response envelope, whether or not it reports a `total` (see
//! [`Paginated::total`]). The async [`Paginator`] is a plain `next_page()`
//! cursor by default; enabling the opt-in **`stream` feature** additionally
//! implements
//! [`Stream`](https://docs.rs/futures-core/latest/futures_core/stream/trait.Stream.html)
//! for it, so it composes with `futures`/`StreamExt` combinators and
//! `.next().await`. Off by default and compiles away entirely (no
//! `futures-core` dependency pulled in) when disabled. The blocking
//! [`blocking::Paginator`] already implements [`Iterator`] unconditionally.

use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;
use std::marker::PhantomData;
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

/// Hard cap, in bytes, on how much of a `400`/`500` error body is read and
/// surfaced via [`Error::BadRequest`] / [`Error::InternalServerError`].
/// Shared by the async streaming reader ([`read_body_capped`]), the blocking
/// `Read`-based reader, and [`truncate_body`], so the cap can never drift
/// between call sites.
const MAX_ERROR_BODY_BYTES: usize = 1000;

/// Retry/backoff policy shared by the async and blocking clients.
///
/// Transient conditions — request timeouts, connection errors, `429 Too Many
/// Requests`, and `5xx` server errors — are retried up to `max_retries` times
/// with exponential backoff (`base_delay * 2^attempt`, capped at
/// [`RETRY_MAX_DELAY`]) plus full jitter (see [`apply_jitter`]), so many
/// clients retrying at once don't thundering-herd the server in lockstep. A
/// `429` response honors its `Retry-After` header (in seconds) when present —
/// that's an explicit server-suggested delay, so it is used as-is, without
/// jitter. Fatal statuses (`400`/`401`/`403`/`404`, and every other `4xx`)
/// are never retried.
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
///
/// This is the pure upper bound only — no jitter — so it stays exactly
/// testable. Retry call sites should use [`jittered_backoff_delay`], which
/// applies full jitter on top of this value.
fn backoff_delay(config: &RetryConfig, attempt: u32) -> Duration {
    let factor = 1u32.checked_shl(attempt).unwrap_or(u32::MAX);
    config
        .base_delay
        .checked_mul(factor)
        .unwrap_or(RETRY_MAX_DELAY)
        .min(RETRY_MAX_DELAY)
}

/// Full-jitter [`backoff_delay`]: a uniformly random duration in
/// `[0, backoff_delay(config, attempt)]`. Chosen over equal-jitter because it
/// spreads retries over the widest possible range, which minimizes
/// thundering-herd risk the most (see the "Exponential Backoff and Jitter"
/// AWS Architecture Blog post, which found full jitter performs best of the
/// strategies it benchmarks). Still bounded by [`RETRY_MAX_DELAY`], since the
/// underlying `backoff_delay` upper bound already is.
fn jittered_backoff_delay(config: &RetryConfig, attempt: u32) -> Duration {
    apply_jitter(backoff_delay(config, attempt))
}

/// Picks a uniformly random duration in `[0, upper]` using the process-wide
/// `fastrand` source. `upper` is expected to already be capped (see
/// [`backoff_delay`]), so the result is too.
fn apply_jitter(upper: Duration) -> Duration {
    jitter_in_range(upper, fastrand::u64)
}

/// Same computation as [`apply_jitter`], with the random-number source
/// injected so tests can assert exact, deterministic output instead of
/// depending on real randomness.
fn jitter_in_range(
    upper: Duration,
    random_u64: impl FnOnce(std::ops::RangeInclusive<u64>) -> u64,
) -> Duration {
    let upper_millis = u64::try_from(upper.as_millis()).unwrap_or(u64::MAX);
    if upper_millis == 0 {
        return Duration::ZERO;
    }
    Duration::from_millis(random_u64(0..=upper_millis))
}

/// Parse a `Retry-After` header into a [`Duration`], expressed as an integer
/// number of seconds. Only the numeric delay-seconds form (RFC 9110 §10.2.3)
/// is understood; the alternative HTTP-date form yields `None` rather than
/// panicking, as does a missing, non-ASCII, or unparseable header.
///
/// The returned value is the raw parsed duration, uncapped. Internal retry
/// sleeps must apply their own [`RETRY_MAX_DELAY`] cap at the call site (see
/// [`retry_delay_for_response`]); the value surfaced via
/// [`Error::RateLimited`] is deliberately left uncapped so callers see the
/// server's actual suggestion.
fn parse_retry_after(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    let secs = headers
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()?;
    Some(Duration::from_secs(secs))
}

/// The delay to wait before retrying a retryable response: a `429`'s
/// `Retry-After` when present (capped at [`RETRY_MAX_DELAY`], not jittered —
/// it's an explicit server-suggested delay), otherwise jittered exponential
/// backoff (see [`jittered_backoff_delay`]).
fn retry_delay_for_response(
    config: &RetryConfig,
    status: StatusCode,
    headers: &reqwest::header::HeaderMap,
    attempt: u32,
) -> Duration {
    if status == StatusCode::TOO_MANY_REQUESTS {
        if let Some(delay) = parse_retry_after(headers) {
            return delay.min(RETRY_MAX_DELAY);
        }
    }
    jittered_backoff_delay(config, attempt)
}

/// The `User-Agent` sent with every request, e.g. `datamaxi-rust/0.4.0`.
fn user_agent() -> String {
    concat!("datamaxi-rust/", env!("CARGO_PKG_VERSION")).to_string()
}

/// Truncate a server error body to at most [`MAX_ERROR_BODY_BYTES`] bytes, on
/// a UTF-8 char boundary.
fn truncate_body(mut s: String) -> String {
    if s.len() > MAX_ERROR_BODY_BYTES {
        let mut end = MAX_ERROR_BODY_BYTES;
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        s.truncate(end);
    }
    s
}

/// Maps a terminal status — anything other than `200 OK`, `400`, and `500`
/// (which need per-flavor body handling) — to the corresponding [`Error`].
/// Returns `None` for those three statuses, leaving them to the caller.
/// Shared by the async and blocking `handle_response`.
fn map_error_status(status: StatusCode, headers: &reqwest::header::HeaderMap) -> Option<Error> {
    match status {
        StatusCode::UNAUTHORIZED => Some(Error::Unauthorized),
        StatusCode::FORBIDDEN => Some(Error::Forbidden),
        StatusCode::NOT_FOUND => Some(Error::NotFound),
        StatusCode::TOO_MANY_REQUESTS => Some(Error::RateLimited {
            retry_after: parse_retry_after(headers),
        }),
        _ => None,
    }
}

/// Shared mutable state behind [`ClientBuilder`] and
/// [`blocking::ClientBuilder`]: the four knobs (API key, base URL, timeout,
/// retry policy) plus the logic to resolve them at `build()` time. Each
/// flavor's builder is a thin wrapper that forwards its setters here and
/// supplies its own `build_inner_client` to construct the right `Client`.
#[derive(Debug, Clone)]
struct BuilderState {
    base_url: Option<String>,
    api_key: Option<String>,
    timeout: Duration,
    retry: RetryConfig,
}

/// The pieces a flavor's `build()` needs, once [`BuilderState::resolve`] has
/// applied the API key / base URL defaults.
struct ResolvedBuilder {
    api_key: String,
    base_url: String,
    timeout: Duration,
    retry: RetryConfig,
}

impl BuilderState {
    fn new() -> Self {
        BuilderState {
            base_url: None,
            api_key: None,
            timeout: DEFAULT_TIMEOUT,
            retry: RetryConfig::default(),
        }
    }

    fn api_key(&mut self, api_key: impl Into<String>) {
        self.api_key = Some(api_key.into());
    }

    fn base_url(&mut self, base_url: impl Into<String>) {
        self.base_url = Some(base_url.into());
    }

    fn timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    fn max_retries(&mut self, max_retries: u32) {
        self.retry.max_retries = max_retries;
    }

    fn retry_base_delay(&mut self, base_delay: Duration) {
        self.retry.base_delay = base_delay;
    }

    /// Resolves the API key from the explicit value or the `DATAMAXI_API_KEY`
    /// environment variable, returning [`Error::MissingApiKey`] if neither is
    /// set, and the base URL from the explicit value or [`BASE_URL`].
    fn resolve(self) -> Result<ResolvedBuilder> {
        let api_key = self
            .api_key
            .or_else(|| std::env::var(API_KEY_ENV).ok())
            .filter(|key| !key.trim().is_empty())
            .ok_or(Error::MissingApiKey)?;
        let base_url = self.base_url.unwrap_or_else(|| BASE_URL.to_string());

        Ok(ResolvedBuilder {
            api_key,
            base_url,
            timeout: self.timeout,
            retry: self.retry,
        })
    }
}

/// Generates the retry loop shared by [`Client::get`] and
/// [`blocking::Client::get`]. The two flavors are identical except for
/// whether `send`, the backoff sleep, and `handle_response` are awaited: pass
/// `await` as the trailing argument for the async flavor, and omit it for the
/// blocking flavor.
macro_rules! get_loop {
    ($self:expr, $endpoint:expr, $parameters:expr, $handle_response:path, $sleep:path $(, $aw:ident)?) => {{
        let url: String = format!("{}{}", $self.base_url, $endpoint);
        let mut attempt: u32 = 0;

        loop {
            #[cfg(feature = "tracing")]
            tracing::Span::current().record("attempt", attempt as u64);

            let mut request = $self
                .inner_client
                .get(url.as_str())
                .header("X-DTMX-APIKEY", &$self.api_key);

            if let Some(ref p) = $parameters {
                request = request.query(p);
            }

            match request.send()$(.$aw)? {
                Ok(response) => {
                    let status = response.status();
                    #[cfg(feature = "tracing")]
                    tracing::Span::current().record("status", status.as_u16() as u64);

                    if attempt < $self.retry.max_retries && is_retryable_status(status) {
                        let delay = retry_delay_for_response(
                            &$self.retry,
                            status,
                            response.headers(),
                            attempt,
                        );
                        #[cfg(feature = "tracing")]
                        tracing::debug!(
                            target: "datamaxi::retry",
                            attempt = attempt as u64,
                            status = status.as_u16() as u64,
                            delay_ms = delay.as_millis() as u64,
                            "retrying transient response"
                        );
                        attempt += 1;
                        $sleep(delay)$(.$aw)?;
                        continue;
                    }
                    return $handle_response(response)$(.$aw)?;
                }
                Err(error) => {
                    if attempt < $self.retry.max_retries && is_retryable_error(&error) {
                        let delay = jittered_backoff_delay(&$self.retry, attempt);
                        #[cfg(feature = "tracing")]
                        tracing::debug!(
                            target: "datamaxi::retry",
                            attempt = attempt as u64,
                            delay_ms = delay.as_millis() as u64,
                            error = %error,
                            "retrying after transport error"
                        );
                        attempt += 1;
                        $sleep(delay)$(.$aw)?;
                        continue;
                    }
                    #[cfg(feature = "tracing")]
                    tracing::warn!(target: "datamaxi::retry", error = %error, "request failed");
                    return Err(Error::from(error));
                }
            }
        }
    }};
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
    ///
    /// With the `tracing` feature enabled, each call is wrapped in a span
    /// carrying `method`, `endpoint`, `attempt`, and the resolved `status`;
    /// retries additionally emit a debug event with the backoff delay. The
    /// API key is never recorded.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "datamaxi.get",
            skip(self, parameters),
            fields(method = "GET", attempt = tracing::field::Empty, status = tracing::field::Empty)
        )
    )]
    pub async fn get<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        parameters: Option<BTreeMap<String, String>>,
    ) -> Result<T> {
        get_loop!(
            self,
            endpoint,
            parameters,
            handle_response,
            tokio::time::sleep,
            await
        )
    }

    /// Returns an auto-paginator over a paged endpoint (see [`Paginated`]).
    ///
    /// `params` seeds the query string for every page (e.g. `limit`, `sort`,
    /// filters); a `page` key in `params` sets the starting page (defaults to
    /// `1`) and is overwritten on each subsequent request as the paginator
    /// advances. Call [`Paginator::next_page`] in a loop to walk forward.
    pub fn paginate<T>(
        &self,
        endpoint: impl Into<String>,
        params: BTreeMap<String, String>,
    ) -> Paginator<T>
    where
        T: Paginated + DeserializeOwned,
    {
        Paginator::new(self.clone(), endpoint, params)
    }
}

/// Implemented by paged response envelopes — the `page`/`limit`/`data` shape
/// shared by several Datamaxi+ endpoints, with or without a `total` (e.g.
/// `CexAnnouncementsResponse` reports `total`; `FundingRateHistoryResponse`
/// does not) — so [`Client::paginate`] / [`blocking::Client::paginate`] can
/// drive a generic auto-paginator over them without codegen needing to emit a
/// bespoke helper per endpoint.
///
/// Envelopes without a `total` (see [`Paginated::total`]) auto-paginate the
/// same way, just terminating only on an empty page rather than also on
/// `page * limit >= total`.
pub trait Paginated {
    /// The item type yielded per page (the envelope's `data` element type).
    type Item;

    /// The 1-based page number this response represents.
    fn page(&self) -> i64;

    /// The page size requested/echoed back by the server.
    fn limit(&self) -> i64;

    /// The total number of items across all pages, if the envelope reports one.
    fn total(&self) -> Option<i64>;

    /// Consumes the response, yielding this page's items.
    fn into_items(self) -> Vec<Self::Item>;
}

/// Implements [`Paginated`] for a `page`/`limit`/`total`/`data` response
/// envelope, mapping its fields directly (`data` -> items).
///
/// A second arm, `impl_paginated!($response, $item, no_total)`, covers the
/// `page`/`limit`/`data` shape without a `total` field: [`Paginated::total`]
/// returns `None`, so [`consume_page`] terminates only on an empty page.
macro_rules! impl_paginated {
    ($response:ty, $item:ty) => {
        impl Paginated for $response {
            type Item = $item;

            fn page(&self) -> i64 {
                self.page
            }

            fn limit(&self) -> i64 {
                self.limit
            }

            fn total(&self) -> Option<i64> {
                Some(self.total)
            }

            fn into_items(self) -> Vec<Self::Item> {
                self.data
            }
        }
    };
    ($response:ty, $item:ty, no_total) => {
        impl Paginated for $response {
            type Item = $item;

            fn page(&self) -> i64 {
                self.page
            }

            fn limit(&self) -> i64 {
                self.limit
            }

            fn total(&self) -> Option<i64> {
                None
            }

            fn into_items(self) -> Vec<Self::Item> {
                self.data
            }
        }
    };
}

impl_paginated!(
    crate::generated::CexAnnouncementsResponse,
    crate::generated::CexAnnouncementsView
);
impl_paginated!(
    crate::generated::CexTokenUpdatesResponse,
    crate::generated::CexTokenUpdatesView
);
impl_paginated!(
    crate::generated::OpenInterestOverviewResponse,
    crate::generated::OpenInterestOverviewView
);
impl_paginated!(
    crate::generated::PremiumResponse,
    crate::generated::PremiumView
);
impl_paginated!(
    crate::generated::TelegramChannelsResponse,
    crate::generated::TelegramChannelsView
);
impl_paginated!(
    crate::generated::TelegramMessagesResponse,
    crate::generated::TelegramMessagesView
);
impl_paginated!(
    crate::generated::FundingRateHistoryResponse,
    crate::generated::FundingRateHistoryView,
    no_total
);

/// Async auto-paginator returned by [`Client::paginate`].
///
/// `next_page` is a plain cursor, with no dependency on `futures`:
///
/// ```no_run
/// use datamaxi::api::ClientBuilder;
/// use datamaxi::CexAnnouncementsResponse;
/// use std::collections::BTreeMap;
///
/// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
/// let client = ClientBuilder::new().api_key("my_api_key").build()?;
/// let mut pages = client
///     .paginate::<CexAnnouncementsResponse>("/api/v1/cex/announcements", BTreeMap::new());
///
/// while let Some(items) = pages.next_page().await? {
///     for item in items {
///         println!("{}", item.title);
///     }
/// }
/// # Ok(())
/// # }
/// ```
///
/// With the opt-in `stream` feature, [`Paginator`] also implements
/// [`Stream`](https://docs.rs/futures-core/latest/futures_core/stream/trait.Stream.html)
/// (the trait re-exported as `futures::Stream` by the
/// [`futures`](https://docs.rs/futures) crate), yielding one
/// `Result<Vec<T::Item>>` per page, so it composes with `futures`/`StreamExt`
/// combinators and `.next().await`:
///
/// ```no_run
/// # #[cfg(feature = "stream")]
/// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
/// use datamaxi::api::ClientBuilder;
/// use datamaxi::CexAnnouncementsResponse;
/// use futures::StreamExt;
/// use std::collections::BTreeMap;
///
/// let client = ClientBuilder::new().api_key("my_api_key").build()?;
/// let mut pages = client
///     .paginate::<CexAnnouncementsResponse>("/api/v1/cex/announcements", BTreeMap::new());
///
/// while let Some(page) = pages.next().await {
///     for item in page? {
///         println!("{}", item.title);
///     }
/// }
/// # Ok(())
/// # }
/// ```
pub struct Paginator<T: Paginated> {
    client: Client,
    endpoint: String,
    params: BTreeMap<String, String>,
    next_page: i64,
    done: bool,
    _marker: PhantomData<T>,
    /// The in-flight `next_page()` future backing [`futures_core::Stream::poll_next`],
    /// present only under the `stream` feature. Built from owned clones of
    /// `client`/`endpoint`/`params` (rather than borrowing `self`) so it can
    /// be stored across `poll_next` calls without a self-referential struct.
    #[cfg(feature = "stream")]
    pending: Option<std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>>>,
}

impl<T> Paginator<T>
where
    T: Paginated + DeserializeOwned,
{
    fn new(client: Client, endpoint: impl Into<String>, params: BTreeMap<String, String>) -> Self {
        let next_page = starting_page(&params);
        Paginator {
            client,
            endpoint: endpoint.into(),
            params,
            next_page,
            done: false,
            _marker: PhantomData,
            #[cfg(feature = "stream")]
            pending: None,
        }
    }

    /// Fetches and returns the next page's items, or `Ok(None)` once the
    /// server has no more data: an empty page, or (when the envelope reports
    /// a `total`) a page reaching `page * limit >= total`. Once exhausted,
    /// further calls keep returning `Ok(None)` rather than re-fetching.
    pub async fn next_page(&mut self) -> Result<Option<Vec<T::Item>>> {
        if self.done {
            return Ok(None);
        }

        let mut params = self.params.clone();
        params.insert("page".to_string(), self.next_page.to_string());

        let response: T = self.client.get(&self.endpoint, Some(params)).await?;
        Ok(self.consume_response(response))
    }

    /// Shared bookkeeping for a fetched page: advances `next_page`, marks
    /// [`Paginator::done`] on an empty page or on reaching `total`, and
    /// extracts the items. Kept free of `async`/blocking specifics so both
    /// flavors share the exact same terminal-condition logic.
    fn consume_response(&mut self, response: T) -> Option<Vec<T::Item>> {
        consume_page(&mut self.next_page, &mut self.done, response)
    }
}

/// [`futures_core::Stream`] over [`Paginator`]'s pages, gated by the `stream`
/// feature so the `futures-core` dependency stays opt-in. Yields one
/// `Result<Vec<T::Item>>` per page and, like [`Iterator`] for
/// [`blocking::Paginator`], stops (yields `None`) once the server reports no
/// more data, and after the first `Err`.
///
/// Each poll drives an owned future built from clones of `client`/`endpoint`/
/// `params` (see [`Paginator::pending`]) rather than borrowing `self`, so the
/// future can be stored across `poll_next` calls without a self-referential
/// struct; on `Poll::Ready`, the same [`consume_page`] bookkeeping used by
/// `next_page` updates `next_page`/`done`.
#[cfg(feature = "stream")]
impl<T> futures_core::Stream for Paginator<T>
where
    T: Paginated + DeserializeOwned + Send + Unpin + 'static,
{
    type Item = Result<Vec<T::Item>>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use std::task::Poll;

        let this = self.get_mut();

        if this.done {
            return Poll::Ready(None);
        }

        if this.pending.is_none() {
            let client = this.client.clone();
            let endpoint = this.endpoint.clone();
            let mut params = this.params.clone();
            params.insert("page".to_string(), this.next_page.to_string());
            this.pending = Some(Box::pin(async move {
                client.get::<T>(&endpoint, Some(params)).await
            }));
        }

        let pending = this
            .pending
            .as_mut()
            .expect("pending future was just set above");
        match pending.as_mut().poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(result) => {
                this.pending = None;
                match result {
                    Ok(response) => Poll::Ready(
                        consume_page(&mut this.next_page, &mut this.done, response).map(Ok),
                    ),
                    Err(error) => {
                        // Stop the stream after an error rather than retrying
                        // the same page forever, mirroring the blocking
                        // `Iterator` impl.
                        this.done = true;
                        Poll::Ready(Some(Err(error)))
                    }
                }
            }
        }
    }
}

/// The starting page for a paginator: the caller-supplied `page` param when
/// present and positive, otherwise `1`.
fn starting_page(params: &BTreeMap<String, String>) -> i64 {
    params
        .get("page")
        .and_then(|p| p.parse::<i64>().ok())
        .filter(|&p| p > 0)
        .unwrap_or(1)
}

/// Shared terminal-condition logic for both the async and blocking
/// paginators: given a fetched `response`, updates `next_page`/`done` and
/// returns `Some(items)`, or `None` once there is nothing left to yield.
fn consume_page<T: Paginated>(
    next_page: &mut i64,
    done: &mut bool,
    response: T,
) -> Option<Vec<T::Item>> {
    let page = response.page();
    let limit = response.limit();
    let total = response.total();
    let items = response.into_items();

    if items.is_empty() {
        *done = true;
        return None;
    }

    *next_page = page + 1;
    if let Some(total) = total {
        if limit > 0 && page.saturating_mul(limit) >= total {
            *done = true;
        }
    }

    Some(items)
}

/// Reads at most [`MAX_ERROR_BODY_BYTES`] of an async response body, streaming
/// chunk by chunk rather than buffering the whole body. Mirrors the blocking
/// path's `response.take(MAX_ERROR_BODY_BYTES).read_to_string(&mut body)`.
/// Invalid UTF-8 in the truncated bytes is replaced lossily.
async fn read_body_capped(mut response: reqwest::Response) -> String {
    let mut buf: Vec<u8> = Vec::new();
    while buf.len() < MAX_ERROR_BODY_BYTES {
        match response.chunk().await {
            Ok(Some(chunk)) => {
                // Take only up to the remaining budget so a single oversized
                // chunk can't push `buf` past the cap — a byte-exact bound
                // matching the blocking path's `response.take(MAX_ERROR_BODY_BYTES)`.
                let take = (MAX_ERROR_BODY_BYTES - buf.len()).min(chunk.len());
                buf.extend_from_slice(&chunk[..take]);
            }
            Ok(None) => break,
            Err(_) => break,
        }
    }
    truncate_body(String::from_utf8_lossy(&buf).into_owned())
}

/// Processes an async response from the API and returns the result.
async fn handle_response<T: DeserializeOwned>(response: reqwest::Response) -> Result<T> {
    match response.status() {
        StatusCode::OK => Ok(response.json::<T>().await?),
        StatusCode::INTERNAL_SERVER_ERROR => {
            Err(Error::InternalServerError(read_body_capped(response).await))
        }
        StatusCode::BAD_REQUEST => Err(Error::BadRequest(read_body_capped(response).await)),
        status => match map_error_status(status, response.headers()) {
            Some(err) => Err(err),
            None => Err(Error::UnexpectedStatusCode(status.as_u16())),
        },
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
    state: BuilderState,
    http_client: Option<reqwest::Client>,
}

impl ClientBuilder {
    /// Creates a new builder with default settings (default timeout, no retries,
    /// key read from the environment on `build`).
    pub fn new() -> Self {
        ClientBuilder {
            state: BuilderState::new(),
            http_client: None,
        }
    }

    /// Sets the API key explicitly, overriding the `DATAMAXI_API_KEY` environment variable.
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.state.api_key(api_key);
        self
    }

    /// Overrides the base URL (defaults to the production API).
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.state.base_url(base_url);
        self
    }

    /// Sets the per-request timeout (defaults to 10 seconds).
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.state.timeout(timeout);
        self
    }

    /// Sets the maximum number of retries on transient failures (timeouts,
    /// connection errors, `429`, and `5xx`). Defaults to `0` (no retries);
    /// each retry backs off exponentially from the base delay.
    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.state.max_retries(max_retries);
        self
    }

    /// Sets the base delay for exponential retry backoff (defaults to 500ms).
    /// The nth retry waits `base_delay * 2^n`, capped at 30 seconds.
    pub fn retry_base_delay(mut self, base_delay: Duration) -> Self {
        self.state.retry_base_delay(base_delay);
        self
    }

    /// Overrides the internally-built `datamaxi::reqwest::Client` with a
    /// caller-supplied one — the escape hatch for custom middleware,
    /// timeouts, proxies, or instrumentation (e.g. a `reqwest-middleware`
    /// client wrapped down to its inner `reqwest::Client`, or one built with
    /// `reqwest_tracing`). Use the crate's re-exported [`crate::reqwest`] to
    /// build it, so the type always matches without a version mismatch.
    ///
    /// When set, [`ClientBuilder::timeout`] is ignored for HTTP-level
    /// settings (the caller's client is used as-is); [`build`](Self::build)
    /// no longer applies the built-in `User-Agent` / pool defaults.
    pub fn http_client(mut self, client: reqwest::Client) -> Self {
        self.http_client = Some(client);
        self
    }

    /// Builds the [`Client`].
    ///
    /// Resolves the API key from the explicit value or the `DATAMAXI_API_KEY`
    /// environment variable, returning [`Error::MissingApiKey`] if neither is set.
    pub fn build(self) -> Result<Client> {
        let resolved = self.state.resolve()?;
        let inner_client = self
            .http_client
            .unwrap_or_else(|| build_inner_client(resolved.timeout));

        Ok(Client {
            base_url: resolved.base_url,
            api_key: resolved.api_key,
            inner_client,
            retry: resolved.retry,
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
    /// `retry_after` carries the raw `Retry-After` header value (in seconds)
    /// when present; the HTTP-date form is not parsed and yields `None`. This
    /// is the server's actual suggestion and is **not** clamped to
    /// [`RETRY_MAX_DELAY`] (that cap only bounds the client's internal retry
    /// sleeps).
    #[error("Rate limited")]
    RateLimited {
        /// Suggested wait before retrying, from the `Retry-After` header
        /// (raw, uncapped).
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
        consume_page, is_retryable_error, is_retryable_status, jittered_backoff_delay,
        map_error_status, retry_delay_for_response, starting_page, truncate_body, user_agent,
        BuilderState, Error, Paginated, Result, RetryConfig, BASE_URL, DEFAULT_TIMEOUT,
        MAX_ERROR_BODY_BYTES,
    };
    use reqwest::blocking::Response;
    use reqwest::StatusCode;
    use serde::de::DeserializeOwned;
    use std::collections::BTreeMap;
    use std::io::Read;
    use std::marker::PhantomData;
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
        ///
        /// With the `tracing` feature enabled, each call is wrapped in a span
        /// carrying `method`, `endpoint`, `attempt`, and the resolved
        /// `status`; retries additionally emit a debug event with the
        /// backoff delay. The API key is never recorded.
        #[cfg_attr(
            feature = "tracing",
            tracing::instrument(
                name = "datamaxi.get",
                skip(self, parameters),
                fields(method = "GET", attempt = tracing::field::Empty, status = tracing::field::Empty)
            )
        )]
        pub fn get<T: DeserializeOwned>(
            &self,
            endpoint: &str,
            parameters: Option<BTreeMap<String, String>>,
        ) -> Result<T> {
            get_loop!(
                self,
                endpoint,
                parameters,
                handle_response,
                std::thread::sleep
            )
        }

        /// Returns an auto-paginator over a paged endpoint (see
        /// [`super::Paginated`]). Mirrors the async [`super::Client::paginate`];
        /// see its docs for how `params` and the starting page work.
        pub fn paginate<T>(
            &self,
            endpoint: impl Into<String>,
            params: BTreeMap<String, String>,
        ) -> Paginator<T>
        where
            T: Paginated + DeserializeOwned,
        {
            Paginator::new(self.clone(), endpoint, params)
        }
    }

    /// Blocking auto-paginator returned by [`Client::paginate`], implementing
    /// [`Iterator`] over pages of items (one `Result<Vec<T::Item>>` per page).
    /// Iteration stops (yields `None`) once the server reports no more data,
    /// and after the first `Err`, so a caller can `?`-propagate mid-loop
    /// without risking a retry of the same failing page.
    ///
    /// ```no_run
    /// use datamaxi::api::blocking::ClientBuilder;
    /// use datamaxi::CexAnnouncementsResponse;
    /// use std::collections::BTreeMap;
    ///
    /// # fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ClientBuilder::new().api_key("my_api_key").build()?;
    /// for page in
    ///     client.paginate::<CexAnnouncementsResponse>("/api/v1/cex/announcements", BTreeMap::new())
    /// {
    ///     for item in page? {
    ///         println!("{}", item.title);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub struct Paginator<T: Paginated> {
        client: Client,
        endpoint: String,
        params: BTreeMap<String, String>,
        next_page: i64,
        done: bool,
        _marker: PhantomData<T>,
    }

    impl<T> Paginator<T>
    where
        T: Paginated + DeserializeOwned,
    {
        fn new(
            client: Client,
            endpoint: impl Into<String>,
            params: BTreeMap<String, String>,
        ) -> Self {
            let next_page = starting_page(&params);
            Paginator {
                client,
                endpoint: endpoint.into(),
                params,
                next_page,
                done: false,
                _marker: PhantomData,
            }
        }
    }

    impl<T> Iterator for Paginator<T>
    where
        T: Paginated + DeserializeOwned,
    {
        type Item = Result<Vec<T::Item>>;

        fn next(&mut self) -> Option<Self::Item> {
            if self.done {
                return None;
            }

            let mut params = self.params.clone();
            params.insert("page".to_string(), self.next_page.to_string());

            match self.client.get::<T>(&self.endpoint, Some(params)) {
                Ok(response) => consume_page(&mut self.next_page, &mut self.done, response).map(Ok),
                Err(error) => {
                    // Stop iterating after an error rather than retrying the
                    // same page forever.
                    self.done = true;
                    Some(Err(error))
                }
            }
        }
    }

    /// Reads at most [`MAX_ERROR_BODY_BYTES`] of a blocking response body,
    /// truncated on a UTF-8 char boundary. The blocking counterpart to the
    /// async [`super::read_body_capped`]; shared by the `400` and `500` arms of
    /// [`handle_response`] so the cap and truncation stay in one place.
    fn read_body_capped(response: Response) -> std::io::Result<String> {
        let mut body = String::new();
        response
            .take(MAX_ERROR_BODY_BYTES as u64)
            .read_to_string(&mut body)?;
        Ok(truncate_body(body))
    }

    /// Processes a blocking response from the API and returns the result.
    fn handle_response<T: DeserializeOwned>(response: Response) -> Result<T> {
        match response.status() {
            StatusCode::OK => Ok(response.json::<T>()?),
            StatusCode::INTERNAL_SERVER_ERROR => {
                Err(Error::InternalServerError(read_body_capped(response)?))
            }
            StatusCode::BAD_REQUEST => Err(Error::BadRequest(read_body_capped(response)?)),
            status => match map_error_status(status, response.headers()) {
                Some(err) => Err(err),
                None => Err(Error::UnexpectedStatusCode(status.as_u16())),
            },
        }
    }

    /// Builder for a blocking [`Client`], mirroring the async [`super::ClientBuilder`].
    #[derive(Debug, Clone)]
    pub struct ClientBuilder {
        state: BuilderState,
        http_client: Option<reqwest::blocking::Client>,
    }

    impl ClientBuilder {
        /// Creates a new builder with default settings.
        pub fn new() -> Self {
            ClientBuilder {
                state: BuilderState::new(),
                http_client: None,
            }
        }

        /// Sets the API key explicitly, overriding the `DATAMAXI_API_KEY` environment variable.
        pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
            self.state.api_key(api_key);
            self
        }

        /// Overrides the base URL (defaults to the production API).
        pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
            self.state.base_url(base_url);
            self
        }

        /// Sets the per-request timeout (defaults to 10 seconds).
        pub fn timeout(mut self, timeout: Duration) -> Self {
            self.state.timeout(timeout);
            self
        }

        /// Sets the maximum number of retries on transient failures (timeouts,
        /// connection errors, `429`, and `5xx`). Defaults to `0` (no retries).
        pub fn max_retries(mut self, max_retries: u32) -> Self {
            self.state.max_retries(max_retries);
            self
        }

        /// Sets the base delay for exponential retry backoff (defaults to
        /// 500ms). The nth retry waits `base_delay * 2^n`, capped at 30 seconds.
        pub fn retry_base_delay(mut self, base_delay: Duration) -> Self {
            self.state.retry_base_delay(base_delay);
            self
        }

        /// Overrides the internally-built `datamaxi::reqwest::blocking::Client`
        /// with a caller-supplied one. Mirrors
        /// [`super::ClientBuilder::http_client`] for the blocking flavor; use
        /// the crate's re-exported [`crate::reqwest`] to build it, so the
        /// type always matches without a version mismatch.
        pub fn http_client(mut self, client: reqwest::blocking::Client) -> Self {
            self.http_client = Some(client);
            self
        }

        /// Builds the blocking [`Client`], resolving the API key from the
        /// explicit value or the `DATAMAXI_API_KEY` environment variable.
        pub fn build(self) -> Result<Client> {
            let resolved = self.state.resolve()?;
            let inner_client = self
                .http_client
                .unwrap_or_else(|| build_inner_client(resolved.timeout));

            Ok(Client {
                base_url: resolved.base_url,
                api_key: resolved.api_key,
                inner_client,
                retry: resolved.retry,
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
    fn jitter_in_range_is_exact_and_deterministic_with_injected_source() {
        let upper = Duration::from_millis(400);

        // A stand-in "random" source that always returns the range's upper
        // bound picks exactly `upper`.
        assert_eq!(jitter_in_range(upper, |range| *range.end()), upper);
        // ...and one that always returns the range's lower bound picks zero.
        assert_eq!(
            jitter_in_range(upper, |range| *range.start()),
            Duration::ZERO
        );

        // A zero upper bound short-circuits to zero without even consulting
        // the random source.
        assert_eq!(
            jitter_in_range(Duration::ZERO, |_| panic!("must not be called")),
            Duration::ZERO
        );
    }

    #[test]
    fn apply_jitter_stays_within_zero_and_upper() {
        // Property check over the real `fastrand` source: the result must
        // always land in `[0, upper]`, regardless of the actual random draw,
        // so this stays deterministic (always passes) without seeding.
        for millis in [0, 1, 100, 30_000, 45_000] {
            let upper = Duration::from_millis(millis).min(RETRY_MAX_DELAY);
            for _ in 0..200 {
                let jittered = apply_jitter(upper);
                assert!(
                    jittered <= upper,
                    "jittered delay {jittered:?} exceeded upper bound {upper:?}"
                );
            }
        }
    }

    #[test]
    fn jittered_backoff_delay_stays_within_zero_and_backoff_delay() {
        let config = RetryConfig {
            max_retries: 5,
            base_delay: Duration::from_millis(100),
        };
        for attempt in [0, 1, 2, 3, 1000] {
            let upper = backoff_delay(&config, attempt);
            for _ in 0..200 {
                let jittered = jittered_backoff_delay(&config, attempt);
                assert!(jittered <= upper);
                assert!(jittered <= RETRY_MAX_DELAY);
            }
        }
    }

    #[test]
    fn parse_retry_after_parses_integer_seconds_uncapped() {
        let mut headers = HeaderMap::new();
        headers.insert(RETRY_AFTER, HeaderValue::from_static("2"));
        assert_eq!(parse_retry_after(&headers), Some(Duration::from_secs(2)));

        // The parser itself never caps; that's the call site's job.
        headers.insert(RETRY_AFTER, HeaderValue::from_static("9999"));
        assert_eq!(parse_retry_after(&headers), Some(Duration::from_secs(9999)));
    }

    #[test]
    fn parse_retry_after_ignores_http_date_and_missing() {
        let empty = HeaderMap::new();
        assert_eq!(parse_retry_after(&empty), None);

        let mut headers = HeaderMap::new();
        headers.insert(
            RETRY_AFTER,
            HeaderValue::from_static("Wed, 21 Oct 2015 07:28:00 GMT"),
        );
        assert_eq!(parse_retry_after(&headers), None);
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
        // 5xx ignores Retry-After and uses jittered backoff, bounded by the
        // unjittered upper bound (see `backoff_is_exponential_and_capped`).
        assert!(
            retry_delay_for_response(&config, StatusCode::BAD_GATEWAY, &headers, 1)
                <= Duration::from_millis(200)
        );
        // 429 without Retry-After falls back to jittered backoff, same bound.
        let empty = HeaderMap::new();
        assert!(
            retry_delay_for_response(&config, StatusCode::TOO_MANY_REQUESTS, &empty, 2)
                <= Duration::from_millis(400)
        );
    }

    #[test]
    fn retry_delay_for_response_caps_large_retry_after() {
        // The internal retry-loop call site must still cap a large
        // Retry-After, even though the shared parser itself is uncapped.
        let config = RetryConfig {
            max_retries: 3,
            base_delay: Duration::from_millis(100),
        };
        let mut headers = HeaderMap::new();
        headers.insert(RETRY_AFTER, HeaderValue::from_static("9999"));
        assert_eq!(
            retry_delay_for_response(&config, StatusCode::TOO_MANY_REQUESTS, &headers, 0),
            RETRY_MAX_DELAY
        );
    }

    // --- Pagination (issue #88) --------------------------------------------

    /// A minimal `page`/`limit`/`total`/`data` envelope for exercising
    /// [`consume_page`] / [`starting_page`] without a real generated response
    /// type.
    #[derive(Debug)]
    struct DummyPage {
        page: i64,
        limit: i64,
        total: Option<i64>,
        data: Vec<i32>,
    }

    impl Paginated for DummyPage {
        type Item = i32;

        fn page(&self) -> i64 {
            self.page
        }

        fn limit(&self) -> i64 {
            self.limit
        }

        fn total(&self) -> Option<i64> {
            self.total
        }

        fn into_items(self) -> Vec<i32> {
            self.data
        }
    }

    #[test]
    fn starting_page_defaults_to_one_when_absent_or_invalid() {
        assert_eq!(starting_page(&BTreeMap::new()), 1);

        let mut params = BTreeMap::new();
        params.insert("page".to_string(), "not-a-number".to_string());
        assert_eq!(starting_page(&params), 1);

        params.insert("page".to_string(), "0".to_string());
        assert_eq!(starting_page(&params), 1);

        params.insert("page".to_string(), "-1".to_string());
        assert_eq!(starting_page(&params), 1);
    }

    #[test]
    fn starting_page_honors_explicit_positive_page() {
        let mut params = BTreeMap::new();
        params.insert("page".to_string(), "5".to_string());
        assert_eq!(starting_page(&params), 5);
    }

    #[test]
    fn consume_page_continues_while_below_total() {
        let mut next_page = 1;
        let mut done = false;
        let page = DummyPage {
            page: 1,
            limit: 2,
            total: Some(5),
            data: vec![1, 2],
        };

        let items = consume_page(&mut next_page, &mut done, page);

        assert_eq!(items, Some(vec![1, 2]));
        assert_eq!(next_page, 2);
        assert!(!done);
    }

    #[test]
    fn consume_page_terminates_when_total_reached() {
        let mut next_page = 2;
        let mut done = false;
        // page * limit == 4 >= total (3): last page.
        let page = DummyPage {
            page: 2,
            limit: 2,
            total: Some(3),
            data: vec![3],
        };

        let items = consume_page(&mut next_page, &mut done, page);

        assert_eq!(items, Some(vec![3]));
        assert!(done, "reaching total must mark the paginator done");
    }

    #[test]
    fn consume_page_terminates_on_empty_data_regardless_of_total() {
        let mut next_page = 2;
        let mut done = false;
        // total (100) not yet reached, but an empty page still terminates.
        let page = DummyPage {
            page: 2,
            limit: 1,
            total: Some(100),
            data: Vec::<i32>::new(),
        };

        let items = consume_page(&mut next_page, &mut done, page);

        assert_eq!(items, None);
        assert!(done);
    }

    #[test]
    fn consume_page_without_total_relies_on_empty_page() {
        let mut next_page = 1;
        let mut done = false;
        let page = DummyPage {
            page: 1,
            limit: 10,
            total: None,
            data: vec![1],
        };

        let items = consume_page(&mut next_page, &mut done, page);

        assert_eq!(items, Some(vec![1]));
        assert!(
            !done,
            "without a reported total, only an empty page should terminate"
        );
    }
}
