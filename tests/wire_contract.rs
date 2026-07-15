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
    CexCandle, CexCandleCurrency, CexCandleExchangesMarket, CexCandleInterval, CexCandleMarket,
    CexCandleOptions, CexFeesOptions, CexSymbolCautionsMinLevel, CexSymbolCautionsOptions,
    CexSymbolLiquidationOptions, LiquidationHeatmapOptions, LiquidationHeatmapResponse,
    LiquidationHeatmapWindow, LiquidationStatsOptions, LiquidationStatsWindow,
    OpenInterestSummaryOptions, PremiumOptions, PremiumPremiumType,
};
use mockito::Matcher;

const API_KEY: &str = "test-api-key";

/// Minimal but COMPLETE `LiquidationHeatmapResponse` envelope: every required
/// field present (empty arrays are valid instances of the required `Vec`
/// fields), so this decodes under both lax (struct-level `#[serde(default)]`)
/// and strict (per-field only) generated code.
const HEATMAP_BODY: &str =
    r#"{"cells":[],"exchanges":[],"generatedAt":0,"grandTotal":0.0,"tokens":[],"window":"1h"}"#;

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
        .with_body(HEATMAP_BODY)
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
        .with_body(
            r#"{"count":1,"generatedAt":0,"longRatio":0,"longUsd":0.0,"shortUsd":0.0,"total":0.0,"venues":0,"window":"24h"}"#,
        )
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
        .with_body(
            r#"{"currency":"USD","data":[],"exchange":"binance","interval":"1h","market":"spot","symbol":"BTC-USDT"}"#,
        )
        .expect(1)
        .create_async()
        .await;

    let candle = mock_client(server.url()).cex_candle();
    let opts = CexCandleOptions::new()
        .market(CexCandleMarket::Spot)
        .interval(CexCandleInterval::_1h)
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
        .with_body(
            r#"{"currency":"USD","data":[],"exchange":"binance","interval":"1h","market":"spot","symbol":"BTC-USDT"}"#,
        )
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
    // Body deserializes into the typed `LiquidationHeatmapResponse`; a complete
    // envelope (every required field present) decodes under strict decode too.
    let res = call_with_status(200, HEATMAP_BODY).await;
    assert!(res.is_ok(), "200 should map to Ok, got {:?}", res);
}

#[tokio::test]
async fn status_400_maps_to_bad_request() {
    let err = call_with_status(400, "bad input")
        .await
        .expect_err("400 should be Err");
    assert!(
        matches!(err, Error::BadRequest { .. }),
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
        matches!(err, Error::Unauthorized { .. }),
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
        matches!(err, Error::InternalServerError { .. }),
        "500 should map to InternalServerError, got {:?}",
        err
    );
}

/// A `400` body over 1000 bytes is truncated on the wire, mirroring the
/// blocking path's `response.take(1000)`. Guards against the async path
/// buffering the full body before truncating (issue #74).
#[tokio::test]
async fn status_400_body_over_1000_bytes_is_truncated() {
    let big_body = "a".repeat(2000);
    let err = call_with_status(400, &big_body)
        .await
        .expect_err("400 should be Err");
    match err {
        Error::BadRequest { body, .. } => {
            assert!(
                body.len() <= 1000,
                "expected truncated body <= 1000 bytes, got {}",
                body.len()
            );
        }
        other => panic!("expected BadRequest, got {:?}", other),
    }
}

/// A `500` body over 1000 bytes is truncated on the wire (async parity with
/// the blocking path). Guards against the async path buffering the full body
/// before truncating (issue #74).
#[tokio::test]
async fn status_500_body_over_1000_bytes_is_truncated() {
    let big_body = "b".repeat(2000);
    let err = call_with_status(500, &big_body)
        .await
        .expect_err("500 should be Err");
    match err {
        Error::InternalServerError { body, .. } => {
            assert!(
                body.len() <= 1000,
                "expected truncated body <= 1000 bytes, got {}",
                body.len()
            );
        }
        other => panic!("expected InternalServerError, got {:?}", other),
    }
}

#[tokio::test]
async fn status_403_maps_to_forbidden() {
    let err = call_with_status(403, "nope")
        .await
        .expect_err("403 should be Err");
    assert!(
        matches!(err, Error::Forbidden { .. }),
        "403 should map to Forbidden, got {:?}",
        err
    );
}

/// A `404` maps to `NotFound` AND carries the request path that 404'd, so the
/// error is diagnosable without `tracing` (issue #116).
#[tokio::test]
async fn status_404_maps_to_not_found_with_endpoint() {
    let err = call_with_status(404, "nope")
        .await
        .expect_err("404 should be Err");
    match err {
        Error::NotFound { endpoint } => {
            assert_eq!(
                endpoint, "/api/v1/liquidation/heatmap",
                "NotFound must carry the endpoint that failed"
            );
        }
        other => panic!("expected NotFound {{ endpoint }}, got {:?}", other),
    }
}

/// An unmapped status (here `502`) still falls through to the catch-all
/// `UnexpectedStatusCode`, carrying the raw code AND the response body (the body
/// is no longer discarded — issue #116).
#[tokio::test]
async fn status_502_maps_to_unexpected_status_code() {
    let err = call_with_status(502, "bad gateway")
        .await
        .expect_err("502 should be Err");
    match err {
        Error::UnexpectedStatusCode {
            endpoint,
            status,
            body,
        } => {
            assert_eq!(status, 502);
            assert_eq!(body, "bad gateway", "the response body must be carried");
            assert_eq!(
                endpoint, "/api/v1/liquidation/heatmap",
                "the endpoint must be carried"
            );
        }
        other => panic!("expected UnexpectedStatusCode {{ 502, .. }}, got {:?}", other),
    }
}

#[tokio::test]
async fn status_429_maps_to_rate_limited_without_retry_after() {
    let err = call_with_status(429, "slow down")
        .await
        .expect_err("429 should be Err");
    assert!(
        matches!(err, Error::RateLimited { retry_after: None, .. }),
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
                retry_after: Some(d),
                ..
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
                retry_after: None,
                ..
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
        .with_body(
            r#"{"exchanges":[],"generatedAt":0,"grandTotal":0.0,"tokens":[],"totalTokens":0}"#,
        )
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
        .with_body(r#"{"data":[],"limit":1,"page":1,"sort":"x","total":0}"#)
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
        .with_body(HEATMAP_BODY)
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

/// `ClientBuilder::http_client` (issue #89) overrides the internally-built
/// `reqwest::Client`. Proven by supplying a client with a custom `User-Agent`
/// and asserting the mock server sees that value instead of the default
/// `datamaxi-rust/<ver>` one — the escape hatch for custom middleware.
#[tokio::test]
async fn client_builder_http_client_overrides_inner_client() {
    let mut server = mockito::Server::new_async().await;

    let mock = server
        .mock("GET", "/api/v1/cex/candle/exchanges")
        .match_header("X-DTMX-APIKEY", API_KEY)
        .match_header("user-agent", "my-custom-middleware-client/1.0")
        .match_query(Matcher::UrlEncoded("market".into(), "spot".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .expect(1)
        .create_async()
        .await;

    let custom = reqwest::Client::builder()
        .user_agent("my-custom-middleware-client/1.0")
        .build()
        .expect("custom reqwest client builds");

    let client = ClientBuilder::new()
        .api_key(API_KEY)
        .base_url(server.url())
        .http_client(custom)
        .build()
        .expect("client with custom http_client should build");
    let res = client
        .cex_candle()
        .exchanges(CexCandleExchangesMarket::Spot)
        .await;

    mock.assert_async().await;
    assert!(res.is_ok(), "expected Ok, got {:?}", res);
}

/// Blocking mirror of [`client_builder_http_client_overrides_inner_client`].
#[cfg(feature = "blocking")]
#[test]
fn blocking_client_builder_http_client_overrides_inner_client() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("GET", "/api/v1/cex/candle/exchanges")
        .match_header("X-DTMX-APIKEY", API_KEY)
        .match_header("user-agent", "my-custom-blocking-client/1.0")
        .match_query(Matcher::UrlEncoded("market".into(), "spot".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .expect(1)
        .create();

    let custom = reqwest::blocking::Client::builder()
        .user_agent("my-custom-blocking-client/1.0")
        .build()
        .expect("custom blocking reqwest client builds");

    let client = datamaxi::api::blocking::ClientBuilder::new()
        .api_key(API_KEY)
        .base_url(server.url())
        .http_client(custom)
        .build()
        .expect("blocking client with custom http_client should build");
    let res = client
        .cex_candle()
        .exchanges(CexCandleExchangesMarket::Spot);

    mock.assert();
    assert!(res.is_ok(), "expected Ok, got {:?}", res);
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
        .with_body(HEATMAP_BODY)
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
        .with_body(HEATMAP_BODY)
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
        .with_body(HEATMAP_BODY)
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
        matches!(res, Err(Error::UnexpectedStatusCode { status: 503, .. })),
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
        matches!(res, Err(Error::BadRequest { .. })),
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
        .with_body(HEATMAP_BODY)
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
        matches!(res, Err(Error::BadRequest { .. })),
        "expected BadRequest without retry, got {:?}",
        res
    );
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
        matches!(err, Error::Forbidden { .. }),
        "403 should map to Forbidden, got {:?}",
        err
    );
}

#[cfg(feature = "blocking")]
#[test]
fn blocking_status_404_maps_to_not_found() {
    let err = blocking_call_with_status(404, None).expect_err("404 should be Err");
    assert!(
        matches!(err, Error::NotFound { .. }),
        "404 should map to NotFound, got {:?}",
        err
    );
}

#[cfg(feature = "blocking")]
#[test]
fn blocking_status_429_maps_to_rate_limited_without_retry_after() {
    let err = blocking_call_with_status(429, None).expect_err("429 should be Err");
    assert!(
        matches!(err, Error::RateLimited { retry_after: None, .. }),
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
            Error::RateLimited { retry_after: Some(d), .. }
            if d == std::time::Duration::from_secs(42)
        ),
        "429 with Retry-After: 42 should surface Duration::from_secs(42), got {:?}",
        err
    );
}

// --- Deep response deserialization (issue #64) -----------------------------
//
// The tests above prove the OUTBOUND contract (path/query/auth). The tests
// below mock REALISTIC non-empty JSON (rather than empty `{}`/`[]`, which
// `#[serde(default)]` would trivially satisfy) and assert the response
// decodes successfully with the right shape. Per-field VALUE assertions on
// generated struct fields (renamed keys, `Option` present/absent) are now
// covered by `tests/generated_contract.rs`'s serde round-trip tests, which
// don't couple to field names and can't break on a codegen rename/retype.

/// Object response with a nested `Vec<CexCandleView>` decodes successfully
/// from realistic renamed-key (`c`/`d`/`h`/`l`/`o`/`v`) JSON.
#[tokio::test]
async fn cex_candle_decodes_renamed_ohlcv_fields() {
    let mut server = mockito::Server::new_async().await;

    let body = r#"{
        "currency": "USD",
        "exchange": "binance",
        "interval": "1h",
        "market": "spot",
        "symbol": "BTC-USDT",
        "data": [
            {"c": 100.5, "d": 1700000000, "h": 110.0, "l": 90.0, "o": 95.0, "v": 1234.5},
            {"c": 101.5, "d": 1700003600, "h": 112.0, "l": 91.0, "o": 100.5, "v": 2000.0}
        ]
    }"#;

    let _mock = server
        .mock("GET", "/api/v1/cex/candle")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create_async()
        .await;

    let resp = mock_client(server.url())
        .cex_candle()
        .get(
            "binance",
            "BTC-USDT",
            CexCandleOptions::new()
                .market(CexCandleMarket::Spot)
                .interval(CexCandleInterval::_1h)
                .currency(CexCandleCurrency::USD),
        )
        .await
        .expect("realistic candle body decodes");

    assert_eq!(resp.data.len(), 2);
}

/// Object response with renamed keys AND nullable `Option<f64>`/`Option<i64>`
/// fields PRESENT (`f`/`i`) decodes successfully.
#[tokio::test]
async fn funding_rate_latest_decodes_nullable_fields_present() {
    let mut server = mockito::Server::new_async().await;

    let body = r#"{
        "b": "BTC",
        "d": 1700000000,
        "e": "binance",
        "f": 0.0001,
        "i": 8,
        "id": "binance-btc-usdt",
        "q": "USDT",
        "s": "BTC-USDT"
    }"#;

    let _mock = server
        .mock("GET", "/api/v1/funding-rate/latest")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create_async()
        .await;

    let res = mock_client(server.url())
        .funding_rate()
        .latest("binance", "BTC-USDT")
        .await;

    assert!(
        res.is_ok(),
        "funding-rate latest body decodes, got {:?}",
        res
    );
}

/// Same shape as above but the nullable keys (`f`/`i`) are ABSENT from the
/// payload. With `#[serde(default)]` the response must still decode
/// successfully rather than erroring on the missing keys.
#[tokio::test]
async fn funding_rate_latest_decodes_nullable_fields_absent() {
    let mut server = mockito::Server::new_async().await;

    // No "f" / "i" keys at all.
    let body = r#"{
        "b": "ETH",
        "d": 1700009999,
        "e": "bybit",
        "id": "bybit-eth-usdt",
        "q": "USDT",
        "s": "ETH-USDT"
    }"#;

    let _mock = server
        .mock("GET", "/api/v1/funding-rate/latest")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create_async()
        .await;

    let res = mock_client(server.url())
        .funding_rate()
        .latest("bybit", "ETH-USDT")
        .await;

    assert!(
        res.is_ok(),
        "funding-rate latest body (nulls absent) decodes, got {:?}",
        res
    );
}

/// `Vec<View>` response: a top-level JSON array of `CexSymbolLiquidationView`
/// decodes successfully, with elements mixing `Option<f64>` USD fields
/// present (element 0) and absent (element 1).
#[tokio::test]
async fn cex_symbol_liquidation_decodes_vec_with_present_and_absent_nullables() {
    let mut server = mockito::Server::new_async().await;

    let body = r#"[
        {
            "b": "BTC", "e": "binance", "q": "USDT", "m": "futures",
            "event_count": 12,
            "long_volume": 100.0, "long_volume_usd": 4200000.0,
            "short_volume": 50.0, "short_volume_usd": 2100000.0,
            "total_volume": 150.0, "total_volume_usd": 6300000.0
        },
        {
            "b": "ETH", "e": "bybit", "q": "USDT", "m": "futures",
            "event_count": 3,
            "long_volume": 10.0,
            "short_volume": 5.0,
            "total_volume": 15.0
        }
    ]"#;

    let _mock = server
        .mock("GET", "/api/v1/cex/symbol/liquidation")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create_async()
        .await;

    let rows = mock_client(server.url())
        .cex_symbol()
        .liquidation("BTC", CexSymbolLiquidationOptions::new())
        .await
        .expect("liquidation array decodes");

    assert_eq!(rows.len(), 2);
}

/// `Vec<View>` response of `CexFeesView`, again mixing present/absent
/// `Option<f64>` fee fields, decodes successfully. Broadens coverage to the
/// `/cex/fees` endpoint (reached via the `trading_fees()` accessor).
#[tokio::test]
async fn cex_fees_decodes_vec_with_present_and_absent_nullables() {
    let mut server = mockito::Server::new_async().await;

    let body = r#"[
        {
            "base": "BTC", "quote": "USDT", "symbol": "BTC-USDT", "exchange": "binance",
            "spot_maker_fee": 0.001, "spot_take_fee": 0.001,
            "futures_maker_fee": 0.0002, "futures_taker_fee": 0.0005
        },
        {
            "base": "ETH", "quote": "USDT", "symbol": "ETH-USDT", "exchange": "binance",
            "spot_maker_fee": 0.001, "spot_take_fee": 0.001
        }
    ]"#;

    let _mock = server
        .mock("GET", "/api/v1/cex/fees")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create_async()
        .await;

    let fees = mock_client(server.url())
        .trading_fees()
        .fees(CexFeesOptions::new().exchange("binance"))
        .await
        .expect("fees array decodes");

    assert_eq!(fees.len(), 2);
}

/// Object response `ForexResponse` with the renamed keys `d`/`r`/`s` decodes
/// successfully. Small but distinct shape (flat object, no nested vec) and a
/// new endpoint (`/forex`).
#[tokio::test]
async fn forex_decodes_renamed_fields() {
    let mut server = mockito::Server::new_async().await;

    let body = r#"{"d": 1700000000, "r": 1350.25, "s": "USD/KRW"}"#;

    let _mock = server
        .mock("GET", "/api/v1/forex")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create_async()
        .await;

    let res = mock_client(server.url()).forex().get("USD/KRW").await;

    assert!(res.is_ok(), "forex body decodes, got {:?}", res);
}

/// Scalar `Vec<String>` response: the funding-rate `exchanges` list. Asserts the
/// exact decoded elements (not just non-empty), broadening breadth to another
/// endpoint (`/funding-rate/exchanges`).
#[tokio::test]
async fn funding_rate_exchanges_decodes_string_vec() {
    let mut server = mockito::Server::new_async().await;

    let _mock = server
        .mock("GET", "/api/v1/funding-rate/exchanges")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"["binance","bybit","okx"]"#)
        .create_async()
        .await;

    let exchanges = mock_client(server.url())
        .funding_rate()
        .exchanges()
        .await
        .expect("exchanges list decodes");

    assert_eq!(exchanges, vec!["binance", "bybit", "okx"]);
}

// --- tracing (issue #89) ----------------------------------------------------
//
// Proves the `tracing` feature actually emits: a minimal `Subscriber` counts
// events, and a retried request (via the blocking client, to avoid needing an
// async-aware subscriber) must produce at least the "retrying transient
// response" debug event emitted by `get_loop!`.

#[cfg(all(feature = "tracing", feature = "blocking"))]
mod tracing_smoke {
    use super::{API_KEY, HEATMAP_BODY};
    use datamaxi::LiquidationHeatmapOptions;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;
    use tracing::span::{Attributes, Id, Record};
    use tracing::{Event, Metadata, Subscriber};

    /// Counts every `event!` (e.g. `tracing::debug!`) delivered to it; ignores
    /// span lifecycle calls (a fixed dummy `Id` is fine since nothing here
    /// inspects span identity).
    struct CountingSubscriber {
        events: Arc<AtomicUsize>,
    }

    impl Subscriber for CountingSubscriber {
        fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _span: &Attributes<'_>) -> Id {
            Id::from_u64(1)
        }
        fn record(&self, _span: &Id, _values: &Record<'_>) {}
        fn record_follows_from(&self, _span: &Id, _follows: &Id) {}
        fn event(&self, _event: &Event<'_>) {
            self.events.fetch_add(1, Ordering::SeqCst);
        }
        fn enter(&self, _span: &Id) {}
        fn exit(&self, _span: &Id) {}
    }

    /// A retried `503` (followed by a `200`) must emit `get_loop!`'s "retrying
    /// transient response" debug event under the `tracing` feature.
    #[test]
    fn retry_emits_tracing_event() {
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
            .with_body(HEATMAP_BODY)
            .expect(1)
            .create();

        let client = datamaxi::api::blocking::ClientBuilder::new()
            .api_key(API_KEY)
            .base_url(server.url())
            .max_retries(1)
            .retry_base_delay(Duration::from_millis(1))
            .build()
            .expect("mock retry client builds");

        let events = Arc::new(AtomicUsize::new(0));
        let subscriber = CountingSubscriber {
            events: events.clone(),
        };

        let res = tracing::subscriber::with_default(subscriber, || {
            client
                .liquidation()
                .heatmap(LiquidationHeatmapOptions::new())
        });

        fail.assert();
        ok.assert();
        assert!(res.is_ok(), "expected Ok after retry, got {:?}", res);
        assert!(
            events.load(Ordering::SeqCst) > 0,
            "expected at least one tracing event emitted during retry"
        );
    }
}
