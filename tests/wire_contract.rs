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
async fn status_403_maps_to_forbidden() {
    let err = call_with_status(403, "nope")
        .await
        .expect_err("403 should be Err");
    assert!(
        matches!(err, Error::Forbidden),
        "403 should map to Forbidden, got {:?}",
        err
    );
}

#[tokio::test]
async fn status_404_maps_to_not_found() {
    let err = call_with_status(404, "nope")
        .await
        .expect_err("404 should be Err");
    assert!(
        matches!(err, Error::NotFound),
        "404 should map to NotFound, got {:?}",
        err
    );
}

/// An unmapped status (here `502`) still falls through to the catch-all
/// `UnexpectedStatusCode`, carrying the raw code.
#[tokio::test]
async fn status_502_maps_to_unexpected_status_code() {
    let err = call_with_status(502, "bad gateway")
        .await
        .expect_err("502 should be Err");
    assert!(
        matches!(err, Error::UnexpectedStatusCode(502)),
        "502 should map to UnexpectedStatusCode(502), got {:?}",
        err
    );
}

#[tokio::test]
async fn status_429_maps_to_rate_limited_without_retry_after() {
    let err = call_with_status(429, "slow down")
        .await
        .expect_err("429 should be Err");
    assert!(
        matches!(
            err,
            Error::RateLimited {
                retry_after: None
            }
        ),
        "429 without Retry-After should map to RateLimited {{ retry_after: None }}, got {:?}",
        err
    );
}

#[tokio::test]
async fn status_429_surfaces_retry_after_seconds() {
    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("GET", "/api/v1/liquidation/heatmap")
        .with_status(429)
        .with_header("Retry-After", "42")
        .with_body("slow down")
        .create_async()
        .await;

    let liq = mock_client(server.url()).liquidation();
    let err = liq
        .heatmap(LiquidationHeatmapOptions::new())
        .await
        .expect_err("429 should be Err");
    assert!(
        matches!(
            err,
            Error::RateLimited {
                retry_after: Some(d)
            } if d == std::time::Duration::from_secs(42)
        ),
        "429 with Retry-After: 42 should surface Duration::from_secs(42), got {:?}",
        err
    );
}

/// A `Retry-After` HTTP-date (rather than delay-seconds) is not parsed and must
/// yield `None` without panicking, while still mapping to `RateLimited`.
#[tokio::test]
async fn status_429_http_date_retry_after_yields_none() {
    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("GET", "/api/v1/liquidation/heatmap")
        .with_status(429)
        .with_header("Retry-After", "Wed, 21 Oct 2015 07:28:00 GMT")
        .with_body("slow down")
        .create_async()
        .await;

    let liq = mock_client(server.url()).liquidation();
    let err = liq
        .heatmap(LiquidationHeatmapOptions::new())
        .await
        .expect_err("429 should be Err");
    assert!(
        matches!(
            err,
            Error::RateLimited {
                retry_after: None
            }
        ),
        "429 with HTTP-date Retry-After should map to RateLimited {{ retry_after: None }}, got {:?}",
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

/// Blocking mirror maps 403/404/429 to the same dedicated variants as async,
/// including surfacing `Retry-After` on 429.
#[cfg(feature = "blocking")]
fn blocking_call_with_status(
    status: usize,
    retry_after: Option<&str>,
) -> datamaxi::api::Result<LiquidationHeatmapResponse> {
    let mut server = mockito::Server::new();
    let mut m = server
        .mock("GET", "/api/v1/liquidation/heatmap")
        .with_status(status)
        .with_body("body");
    if let Some(ra) = retry_after {
        m = m.with_header("Retry-After", ra);
    }
    let _mock = m.create();

    let liq = mock_blocking_client(server.url()).liquidation();
    liq.heatmap(LiquidationHeatmapOptions::new())
}

#[cfg(feature = "blocking")]
#[test]
fn blocking_status_403_maps_to_forbidden() {
    let err = blocking_call_with_status(403, None).expect_err("403 should be Err");
    assert!(
        matches!(err, Error::Forbidden),
        "403 should map to Forbidden, got {:?}",
        err
    );
}

#[cfg(feature = "blocking")]
#[test]
fn blocking_status_404_maps_to_not_found() {
    let err = blocking_call_with_status(404, None).expect_err("404 should be Err");
    assert!(
        matches!(err, Error::NotFound),
        "404 should map to NotFound, got {:?}",
        err
    );
}

#[cfg(feature = "blocking")]
#[test]
fn blocking_status_429_maps_to_rate_limited_without_retry_after() {
    let err = blocking_call_with_status(429, None).expect_err("429 should be Err");
    assert!(
        matches!(err, Error::RateLimited { retry_after: None }),
        "429 without Retry-After should map to RateLimited {{ retry_after: None }}, got {:?}",
        err
    );
}

#[cfg(feature = "blocking")]
#[test]
fn blocking_status_429_surfaces_retry_after_seconds() {
    let err = blocking_call_with_status(429, Some("42")).expect_err("429 should be Err");
    assert!(
        matches!(
            err,
            Error::RateLimited { retry_after: Some(d) }
            if d == std::time::Duration::from_secs(42)
        ),
        "429 with Retry-After: 42 should surface Duration::from_secs(42), got {:?}",
        err
    );
}
