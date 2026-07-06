//! Request-level wire-contract tests.
//!
//! These lock the outbound HTTP contract (path + query-string keys + auth
//! header) that the typed builders produce, and exercise `handle_response`'s
//! status-code mapping. They use `mockito` to stand up a local server and a
//! `Client` pointed at it (via the `mock_client` helper, i.e. `ClientBuilder`
//! + `base_url`), then reach each endpoint group through a root-client accessor.
//!
//! Regression guard: PR #8 shipped a wrong snake_case query key. The builder
//! methods are camelCase (`topN`, `minVolumeUsd`) but the wire keys MUST be
//! snake_case (`top_n`, `min_volume_usd`). The query-key assertions below fail
//! if a future codegen regen reintroduces that bug.

use datamaxi::api::{Client, ClientBuilder, Error};
use datamaxi::{
    CexCandle, CexCandleCurrency, CexCandleExchangesMarket, CexCandleMarket, CexCandleOptions,
    CexSymbolCautionsMinLevel, CexSymbolCautionsOptions, LiquidationHeatmapOptions,
    LiquidationHeatmapResponse, LiquidationHeatmapWindow, LiquidationStatsOptions,
    LiquidationStatsWindow, OpenInterestSummaryOptions, PremiumOptions, PremiumPremiumType,
};
use mockito::Matcher;

const API_KEY: &str = "test-api-key";

/// Build an async client pointed at the mock server (explicit key + base URL),
/// exercising the new root-client construction + accessor path the SDK exposes.
fn mock_client(base_url: String) -> Client {
    ClientBuilder::new()
        .api_key(API_KEY)
        .base_url(base_url)
        .build()
        .expect("mock client builds")
}

/// Blocking mirror of [`mock_client`], over `crate::api::blocking::Client`.
#[cfg(feature = "blocking")]
fn mock_blocking_client(base_url: String) -> datamaxi::api::blocking::Client {
    datamaxi::api::blocking::ClientBuilder::new()
        .api_key(API_KEY)
        .base_url(base_url)
        .build()
        .expect("mock blocking client builds")
}

/// `top_n` wire key (regression guard for PR #8) + `window`, plus path and the
/// `X-DTMX-APIKEY` auth header.
#[tokio::test]
async fn liquidation_heatmap_sends_top_n_window_and_apikey() {
    let mut server = mockito::Server::new_async().await;

    let mock = server
        .mock("GET", "/api/v1/liquidation/heatmap")
        .match_header("X-DTMX-APIKEY", API_KEY)
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("top_n".into(), "10".into()),
            Matcher::UrlEncoded("window".into(), "1h".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("{}")
        .expect(1)
        .create_async()
        .await;

    let liq = mock_client(server.url()).liquidation();
    let opts = LiquidationHeatmapOptions::new()
        .window(LiquidationHeatmapWindow::_1h)
        .top_n(10);
    let res = liq.heatmap(opts).await;

    mock.assert_async().await;
    assert!(res.is_ok(), "expected Ok, got {:?}", res);
}

/// `min_volume_usd` wire key (regression guard for PR #8) on the stats endpoint.
#[tokio::test]
async fn liquidation_stats_sends_min_volume_usd() {
    let mut server = mockito::Server::new_async().await;

    let mock = server
        .mock("GET", "/api/v1/liquidation/stats")
        .match_header("X-DTMX-APIKEY", API_KEY)
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("min_volume_usd".into(), "5".into()),
            Matcher::UrlEncoded("window".into(), "24h".into()),
        ]))
        .with_status(200)
        .with_body("{}")
        .expect(1)
        .create_async()
        .await;

    let liq = mock_client(server.url()).liquidation();
    let opts = LiquidationStatsOptions::new()
        .window(LiquidationStatsWindow::_24h)
        .min_volume_usd(5.0);
    let res = liq.stats(opts).await;

    mock.assert_async().await;
    assert!(res.is_ok(), "expected Ok, got {:?}", res);
}

/// CEX candle: required path params + several optional query keys/values.
#[tokio::test]
async fn cex_candle_sends_required_and_optional_keys() {
    let mut server = mockito::Server::new_async().await;

    let mock = server
        .mock("GET", "/api/v1/cex/candle")
        .match_header("X-DTMX-APIKEY", API_KEY)
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("exchange".into(), "binance".into()),
            Matcher::UrlEncoded("market".into(), "spot".into()),
            Matcher::UrlEncoded("symbol".into(), "BTC-USDT".into()),
            Matcher::UrlEncoded("interval".into(), "1h".into()),
            Matcher::UrlEncoded("currency".into(), "USD".into()),
        ]))
        .with_status(200)
        .with_body("{}")
        .expect(1)
        .create_async()
        .await;

    let candle = mock_client(server.url()).cex_candle();
    let opts = CexCandleOptions::new()
        .market(CexCandleMarket::Spot)
        .interval("1h")
        .currency(CexCandleCurrency::USD);
    let res = candle.get("binance", "BTC-USDT", opts).await;

    mock.assert_async().await;
    assert!(res.is_ok(), "expected Ok, got {:?}", res);
}

/// Query-param values containing reserved/non-ASCII characters MUST be
/// percent-encoded on the wire (issue #10). `Matcher::UrlEncoded` decodes the
/// incoming query before comparing, so a match proves the raw value was encoded
/// and round-trips. A pre-fix hand-rolled `k=v` join would emit the raw `&`,
/// `=`, space and `\u{e9}`, corrupting the query string and failing this match.
#[tokio::test]
async fn query_params_are_percent_encoded() {
    let mut server = mockito::Server::new_async().await;

    // Contains `&`, `=`, a space and a non-ASCII char.
    let raw_symbol = "A&B=C D\u{e9}";

    let mock = server
        .mock("GET", "/api/v1/cex/candle")
        .match_header("X-DTMX-APIKEY", API_KEY)
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("exchange".into(), "binance".into()),
            Matcher::UrlEncoded("market".into(), "spot".into()),
            Matcher::UrlEncoded("symbol".into(), raw_symbol.into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("{}")
        .expect(1)
        .create_async()
        .await;

    let candle = mock_client(server.url()).cex_candle();
    let res = candle
        .get(
            "binance",
            raw_symbol,
            CexCandleOptions::new().market(CexCandleMarket::Spot),
        )
        .await;

    mock.assert_async().await;
    assert!(res.is_ok(), "expected Ok, got {:?}", res);
}

// --- handle_response status mapping ---------------------------------------

async fn call_with_status(
    status: usize,
    body: &str,
) -> datamaxi::api::Result<LiquidationHeatmapResponse> {
    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("GET", "/api/v1/liquidation/heatmap")
        .with_status(status)
        .with_body(body)
        .create_async()
        .await;

    let liq = mock_client(server.url()).liquidation();
    liq.heatmap(LiquidationHeatmapOptions::new()).await
}

#[tokio::test]
async fn status_200_maps_to_ok() {
    // Body deserializes into the typed `LiquidationHeatmapResponse`; struct-level
    // serde default lets an empty `{}` payload decode with zero-valued fields.
    let res = call_with_status(200, "{}").await;
    assert!(res.is_ok(), "200 should map to Ok, got {:?}", res);
}

#[tokio::test]
async fn status_400_maps_to_bad_request() {
    let err = call_with_status(400, "bad input")
        .await
        .expect_err("400 should be Err");
    assert!(
        matches!(err, Error::BadRequest(_)),
        "400 should map to BadRequest, got {:?}",
        err
    );
}

#[tokio::test]
async fn status_401_maps_to_unauthorized() {
    let err = call_with_status(401, "")
        .await
        .expect_err("401 should be Err");
    assert!(
        matches!(err, Error::Unauthorized),
        "401 should map to Unauthorized, got {:?}",
        err
    );
}

#[tokio::test]
async fn status_500_maps_to_internal_server_error() {
    let err = call_with_status(500, "boom")
        .await
        .expect_err("500 should be Err");
    assert!(
        matches!(err, Error::InternalServerError(_)),
        "500 should map to InternalServerError, got {:?}",
        err
    );
}

#[tokio::test]
async fn status_404_maps_to_unexpected_status_code() {
    let err = call_with_status(404, "nope")
        .await
        .expect_err("404 should be Err");
    assert!(
        matches!(err, Error::UnexpectedStatusCode(404)),
        "404 should map to UnexpectedStatusCode(404), got {:?}",
        err
    );
}

// --- Per-endpoint snake_case wire-key guards (issue #16) -------------------
//
// These extend the heatmap/stats guards to more endpoints most at risk of a
// camelCase-builder → snake_case-wire-key regression on a codegen regen (the
// PR #8 failure mode). Each also locks the request path (the generated paths
// carry the `/api/v1` prefix exactly once — no duplication with `base_url`,
// which is the bare mock-server host).

/// `/api/v1/open-interest/summary`: the `top_n` builder must serialize to the
/// `top_n` wire key — the same camelCase→snake_case case PR #8 got wrong on
/// liquidation.
#[tokio::test]
async fn open_interest_summary_sends_top_n() {
    let mut server = mockito::Server::new_async().await;

    let mock = server
        .mock("GET", "/api/v1/open-interest/summary")
        .match_header("X-DTMX-APIKEY", API_KEY)
        .match_query(Matcher::UrlEncoded("top_n".into(), "5".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("{}")
        .expect(1)
        .create_async()
        .await;

    let oi = mock_client(server.url()).open_interest();
    let res = oi.summary(OpenInterestSummaryOptions::new().top_n(5)).await;

    mock.assert_async().await;
    assert!(res.is_ok(), "expected Ok, got {:?}", res);
}

/// `/cex/symbol/cautions`: multi-word snake_case keys `min_level` and
/// `active_only` (bool) round-trip on the wire.
#[tokio::test]
async fn cex_symbol_cautions_sends_snake_keys() {
    let mut server = mockito::Server::new_async().await;

    let mock = server
        .mock("GET", "/api/v1/cex/symbol/cautions")
        .match_header("X-DTMX-APIKEY", API_KEY)
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("exchange".into(), "binance".into()),
            Matcher::UrlEncoded("min_level".into(), "danger".into()),
            Matcher::UrlEncoded("active_only".into(), "true".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        // `/cex/symbol/cautions` returns a top-level array (Vec<..>).
        .with_body("[]")
        .expect(1)
        .create_async()
        .await;

    let sym = mock_client(server.url()).cex_symbol();
    let opts = CexSymbolCautionsOptions::new()
        .exchange("binance")
        .min_level(CexSymbolCautionsMinLevel::Danger)
        .active_only(true);
    let res = sym.cautions(opts).await;

    mock.assert_async().await;
    assert!(res.is_ok(), "expected Ok, got {:?}", res);
}

/// `/premium`: several multi-word snake_case keys (`source_exchange`,
/// `target_exchange`, `premium_type`) and the `/api/v1`-prefixed path.
#[tokio::test]
async fn premium_sends_snake_keys() {
    let mut server = mockito::Server::new_async().await;

    let mock = server
        .mock("GET", "/api/v1/premium")
        .match_header("X-DTMX-APIKEY", API_KEY)
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("source_exchange".into(), "binance".into()),
            Matcher::UrlEncoded("target_exchange".into(), "upbit".into()),
            Matcher::UrlEncoded("premium_type".into(), "spot-futures".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("{}")
        .expect(1)
        .create_async()
        .await;

    let premium = mock_client(server.url()).premium();
    let opts = PremiumOptions::new()
        .source_exchange("binance")
        .target_exchange("upbit")
        .premium_type(PremiumPremiumType::SpotFutures);
    let res = premium.get(opts).await;

    mock.assert_async().await;
    assert!(res.is_ok(), "expected Ok, got {:?}", res);
}

// --- Root client accessors -------------------------------------------------

/// The root `Client` exposes each endpoint group via a generated accessor
/// (`client.cex_candle()`, `client.liquidation()`, …), replacing the old
/// `Datamaxi::new` per-endpoint constructor. This drives requests through TWO
/// different accessors on the SAME client, proving one client (its auth + base
/// URL) backs every endpoint group and each accessor yields a working handle.
#[tokio::test]
async fn root_client_accessors_share_one_client() {
    let mut server = mockito::Server::new_async().await;

    let heatmap = server
        .mock("GET", "/api/v1/liquidation/heatmap")
        .match_header("X-DTMX-APIKEY", API_KEY)
        .match_query(Matcher::UrlEncoded("window".into(), "1h".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("{}")
        .expect(1)
        .create_async()
        .await;
    let exchanges = server
        .mock("GET", "/api/v1/cex/candle/exchanges")
        .match_header("X-DTMX-APIKEY", API_KEY)
        .match_query(Matcher::UrlEncoded("market".into(), "spot".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .expect(1)
        .create_async()
        .await;

    // One client, two endpoint groups reached via accessors.
    let client = mock_client(server.url());
    let liq = client
        .liquidation()
        .heatmap(LiquidationHeatmapOptions::new().window(LiquidationHeatmapWindow::_1h))
        .await;
    let ex = client
        .cex_candle()
        .exchanges(CexCandleExchangesMarket::Spot)
        .await;

    heatmap.assert_async().await;
    exchanges.assert_async().await;
    assert!(
        liq.is_ok(),
        "liquidation via accessor: expected Ok, got {:?}",
        liq
    );
    assert!(
        ex.is_ok(),
        "cex_candle via accessor: expected Ok, got {:?}",
        ex
    );
}

// --- ClientBuilder ---------------------------------------------------------

/// A `Client` built via `ClientBuilder` sends the `datamaxi-rust/<ver>`
/// User-Agent and the auth header, and routes requests through the same path.
/// Also exercises `from_client`, which wraps a pre-built client.
#[tokio::test]
async fn client_builder_sets_user_agent_and_auth() {
    let mut server = mockito::Server::new_async().await;

    let mock = server
        .mock("GET", "/api/v1/cex/candle/exchanges")
        .match_header("X-DTMX-APIKEY", API_KEY)
        .match_header(
            "user-agent",
            Matcher::Regex(r"^datamaxi-rust/\d+\.\d+\.\d+".to_string()),
        )
        .match_query(Matcher::UrlEncoded("market".into(), "spot".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        // `/cex/candle/exchanges` returns a top-level array (Vec<String>).
        .with_body("[]")
        .expect(1)
        .create_async()
        .await;

    let client = ClientBuilder::new()
        .api_key(API_KEY)
        .base_url(server.url())
        .build()
        .expect("explicit api key should build");
    let candle = CexCandle::from_client(client);
    let res = candle.exchanges(CexCandleExchangesMarket::Spot).await;

    mock.assert_async().await;
    assert!(res.is_ok(), "expected Ok, got {:?}", res);
}

/// With no explicit key and `DATAMAXI_API_KEY` unset, `build` fails with
/// `MissingApiKey` rather than silently constructing an unauthenticated client.
#[test]
fn client_builder_without_key_errors() {
    std::env::remove_var("DATAMAXI_API_KEY");
    let err = ClientBuilder::new()
        .build()
        .expect_err("no key should fail to build");
    assert!(
        matches!(err, Error::MissingApiKey),
        "expected MissingApiKey, got {:?}",
        err
    );
}

// --- Blocking feature smoke tests ------------------------------------------

/// The `blocking` mirror exposes the same endpoints synchronously and routes
/// through `crate::api::blocking::Client`.
#[cfg(feature = "blocking")]
#[test]
fn blocking_cex_candle_exchanges_smoke() {
    let mut server = mockito::Server::new();
    let mock = server
        .mock("GET", "/api/v1/cex/candle/exchanges")
        .match_header("X-DTMX-APIKEY", API_KEY)
        .match_query(Matcher::UrlEncoded("market".into(), "spot".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .expect(1)
        .create();

    let candle = mock_blocking_client(server.url()).cex_candle();
    let res = candle.exchanges(CexCandleExchangesMarket::Spot);

    mock.assert();
    assert!(res.is_ok(), "expected Ok, got {:?}", res);
}

/// Blocking mirror decodes a typed object response and maps status errors the
/// same way as the async surface.
#[cfg(feature = "blocking")]
#[test]
fn blocking_liquidation_heatmap_smoke() {
    let mut server = mockito::Server::new();
    let mock = server
        .mock("GET", "/api/v1/liquidation/heatmap")
        .match_header("X-DTMX-APIKEY", API_KEY)
        .match_query(Matcher::UrlEncoded("window".into(), "1h".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("{}")
        .expect(1)
        .create();

    let liq = mock_blocking_client(server.url()).liquidation();
    let res = liq.heatmap(LiquidationHeatmapOptions::new().window(LiquidationHeatmapWindow::_1h));

    mock.assert();
    assert!(res.is_ok(), "expected Ok, got {:?}", res);
}

// --- Retry / backoff (issue #66) ------------------------------------------
//
// These drive the retry loop through the public `get` (via the liquidation
// accessor). A tiny `retry_base_delay` keeps the exponential backoff sleeps
// sub-millisecond so the suite stays fast while still exercising the real code
// path. Transient statuses (429, 5xx) retry; fatal statuses (400/401/403/404)
// return immediately.

/// Build an async client with retries enabled and a negligible backoff delay.
fn mock_retry_client(base_url: String, max_retries: u32) -> Client {
    ClientBuilder::new()
        .api_key(API_KEY)
        .base_url(base_url)
        .max_retries(max_retries)
        .retry_base_delay(std::time::Duration::from_millis(1))
        .build()
        .expect("mock retry client builds")
}

/// A transient `503` on the first attempt is retried and the subsequent `200`
/// succeeds. Two same-path mocks are consumed in order: the first (503) once,
/// then the second (200).
#[tokio::test]
async fn retry_then_succeed_on_5xx() {
    let mut server = mockito::Server::new_async().await;
    let fail = server
        .mock("GET", "/api/v1/liquidation/heatmap")
        .with_status(503)
        .with_body("unavailable")
        .expect(1)
        .create_async()
        .await;
    let ok = server
        .mock("GET", "/api/v1/liquidation/heatmap")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("{}")
        .expect(1)
        .create_async()
        .await;

    let res = mock_retry_client(server.url(), 2)
        .liquidation()
        .heatmap(LiquidationHeatmapOptions::new())
        .await;

    fail.assert_async().await;
    ok.assert_async().await;
    assert!(res.is_ok(), "expected Ok after retry, got {:?}", res);
}

/// A `429` with `Retry-After` is retried (honoring the header path) and then
/// succeeds. `Retry-After: 0` keeps the test instant.
#[tokio::test]
async fn retry_then_succeed_on_429_with_retry_after() {
    let mut server = mockito::Server::new_async().await;
    let throttled = server
        .mock("GET", "/api/v1/liquidation/heatmap")
        .with_status(429)
        .with_header("Retry-After", "0")
        .with_body("slow down")
        .expect(1)
        .create_async()
        .await;
    let ok = server
        .mock("GET", "/api/v1/liquidation/heatmap")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("{}")
        .expect(1)
        .create_async()
        .await;

    let res = mock_retry_client(server.url(), 3)
        .liquidation()
        .heatmap(LiquidationHeatmapOptions::new())
        .await;

    throttled.assert_async().await;
    ok.assert_async().await;
    assert!(res.is_ok(), "expected Ok after 429 retry, got {:?}", res);
}

/// When every attempt is transiently failing, the client exhausts its retries
/// (initial + `max_retries` = 3 total requests) and returns the last error.
#[tokio::test]
async fn retry_exhaustion_returns_error() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("GET", "/api/v1/liquidation/heatmap")
        .with_status(503)
        .with_body("unavailable")
        .expect(3)
        .create_async()
        .await;

    let res = mock_retry_client(server.url(), 2)
        .liquidation()
        .heatmap(LiquidationHeatmapOptions::new())
        .await;

    mock.assert_async().await; // exactly 3 attempts
    assert!(
        matches!(res, Err(Error::UnexpectedStatusCode(503))),
        "expected exhausted retries to surface the 503, got {:?}",
        res
    );
}

/// A fatal `400` is never retried even with retries enabled: exactly one
/// request, and the error surfaces immediately.
#[tokio::test]
async fn no_retry_on_400() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("GET", "/api/v1/liquidation/heatmap")
        .with_status(400)
        .with_body("bad input")
        .expect(1)
        .create_async()
        .await;

    let res = mock_retry_client(server.url(), 5)
        .liquidation()
        .heatmap(LiquidationHeatmapOptions::new())
        .await;

    mock.assert_async().await; // exactly one attempt, no retries
    assert!(
        matches!(res, Err(Error::BadRequest(_))),
        "expected BadRequest without retry, got {:?}",
        res
    );
}

/// Blocking mirror: a transient `503` is retried and the following `200`
/// succeeds, proving the blocking `get` shares the async retry semantics.
#[cfg(feature = "blocking")]
#[test]
fn blocking_retry_then_succeed_on_5xx() {
    let mut server = mockito::Server::new();
    let fail = server
        .mock("GET", "/api/v1/liquidation/heatmap")
        .with_status(503)
        .with_body("unavailable")
        .expect(1)
        .create();
    let ok = server
        .mock("GET", "/api/v1/liquidation/heatmap")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("{}")
        .expect(1)
        .create();

    let client = datamaxi::api::blocking::ClientBuilder::new()
        .api_key(API_KEY)
        .base_url(server.url())
        .max_retries(2)
        .retry_base_delay(std::time::Duration::from_millis(1))
        .build()
        .expect("blocking retry client builds");
    let res = client
        .liquidation()
        .heatmap(LiquidationHeatmapOptions::new());

    fail.assert();
    ok.assert();
    assert!(
        res.is_ok(),
        "expected Ok after blocking retry, got {:?}",
        res
    );
}

/// Blocking mirror: a fatal `400` is not retried (exactly one request).
#[cfg(feature = "blocking")]
#[test]
fn blocking_no_retry_on_400() {
    let mut server = mockito::Server::new();
    let mock = server
        .mock("GET", "/api/v1/liquidation/heatmap")
        .with_status(400)
        .with_body("bad input")
        .expect(1)
        .create();

    let client = datamaxi::api::blocking::ClientBuilder::new()
        .api_key(API_KEY)
        .base_url(server.url())
        .max_retries(5)
        .retry_base_delay(std::time::Duration::from_millis(1))
        .build()
        .expect("blocking retry client builds");
    let res = client
        .liquidation()
        .heatmap(LiquidationHeatmapOptions::new());

    mock.assert();
    assert!(
        matches!(res, Err(Error::BadRequest(_))),
        "expected BadRequest without retry, got {:?}",
        res
    );
}
