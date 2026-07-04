//! Request-level wire-contract tests.
//!
//! These lock the outbound HTTP contract (path + query-string keys + auth
//! header) that the typed builders produce, and exercise `handle_response`'s
//! status-code mapping. They use `mockito` to stand up a local server and a
//! client pointed at it via `new_with_base_url`.
//!
//! Regression guard: PR #8 shipped a wrong snake_case query key. The builder
//! methods are camelCase (`topN`, `minVolumeUsd`) but the wire keys MUST be
//! snake_case (`top_n`, `min_volume_usd`). The query-key assertions below fail
//! if a future codegen regen reintroduces that bug.

use datamaxi::api::{ClientBuilder, Datamaxi, Error};
use datamaxi::generated::{
    CexCandle, CexCandleOptions, Liquidation, LiquidationHeatmapOptions, LiquidationStatsOptions,
};
use mockito::Matcher;

const API_KEY: &str = "test-api-key";

/// `top_n` wire key (regression guard for PR #8) + `window`, plus path and the
/// `X-DTMX-APIKEY` auth header.
#[test]
fn liquidation_heatmap_sends_top_n_window_and_apikey() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("GET", "/liquidation/heatmap")
        .match_header("X-DTMX-APIKEY", API_KEY)
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("top_n".into(), "10".into()),
            Matcher::UrlEncoded("window".into(), "1h".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("{}")
        .expect(1)
        .create();

    let liq: Liquidation = Datamaxi::new_with_base_url(API_KEY.to_string(), server.url());
    let opts = LiquidationHeatmapOptions::new().window("1h").topN(10);
    let res = liq.heatmap(opts);

    mock.assert();
    assert!(res.is_ok(), "expected Ok, got {:?}", res);
}

/// `min_volume_usd` wire key (regression guard for PR #8) on the stats endpoint.
#[test]
fn liquidation_stats_sends_min_volume_usd() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("GET", "/liquidation/stats")
        .match_header("X-DTMX-APIKEY", API_KEY)
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("min_volume_usd".into(), "5".into()),
            Matcher::UrlEncoded("window".into(), "24h".into()),
        ]))
        .with_status(200)
        .with_body("{}")
        .expect(1)
        .create();

    let liq: Liquidation = Datamaxi::new_with_base_url(API_KEY.to_string(), server.url());
    let opts = LiquidationStatsOptions::new()
        .window("24h")
        .minVolumeUsd(5.0);
    let res = liq.stats(opts);

    mock.assert();
    assert!(res.is_ok(), "expected Ok, got {:?}", res);
}

/// CEX candle: required path params + several optional query keys/values.
#[test]
fn cex_candle_sends_required_and_optional_keys() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("GET", "/cex/candle")
        .match_header("X-DTMX-APIKEY", API_KEY)
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("exchange".into(), "binance".into()),
            Matcher::UrlEncoded("market".into(), "spot".into()),
            Matcher::UrlEncoded("symbol".into(), "BTC-USDT".into()),
            Matcher::UrlEncoded("interval".into(), "1h".into()),
            Matcher::UrlEncoded("currency".into(), "USD".into()),
        ]))
        .with_status(200)
        .with_body("[]")
        .expect(1)
        .create();

    let candle: CexCandle = Datamaxi::new_with_base_url(API_KEY.to_string(), server.url());
    let opts = CexCandleOptions::new().interval("1h").currency("USD");
    let res = candle.get("binance", "spot", "BTC-USDT", opts);

    mock.assert();
    assert!(res.is_ok(), "expected Ok, got {:?}", res);
}

/// Query-param values containing reserved/non-ASCII characters MUST be
/// percent-encoded on the wire (issue #10). `Matcher::UrlEncoded` decodes the
/// incoming query before comparing, so a match proves the raw value was encoded
/// and round-trips. A pre-fix hand-rolled `k=v` join would emit the raw `&`,
/// `=`, space and `\u{e9}`, corrupting the query string and failing this match.
#[test]
fn query_params_are_percent_encoded() {
    let mut server = mockito::Server::new();

    // Contains `&`, `=`, a space and a non-ASCII char.
    let raw_symbol = "A&B=C D\u{e9}";

    let mock = server
        .mock("GET", "/cex/candle")
        .match_header("X-DTMX-APIKEY", API_KEY)
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("exchange".into(), "binance".into()),
            Matcher::UrlEncoded("market".into(), "spot".into()),
            Matcher::UrlEncoded("symbol".into(), raw_symbol.into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .expect(1)
        .create();

    let candle: CexCandle = Datamaxi::new_with_base_url(API_KEY.to_string(), server.url());
    let res = candle.get("binance", "spot", raw_symbol, CexCandleOptions::new());

    mock.assert();
    assert!(res.is_ok(), "expected Ok, got {:?}", res);
}

// --- handle_response status mapping ---------------------------------------

fn call_with_status(status: usize, body: &str) -> datamaxi::api::Result<serde_json::Value> {
    let mut server = mockito::Server::new();
    let _mock = server
        .mock("GET", "/liquidation/heatmap")
        .with_status(status)
        .with_body(body)
        .create();

    let liq: Liquidation = Datamaxi::new_with_base_url(API_KEY.to_string(), server.url());
    liq.heatmap(LiquidationHeatmapOptions::new())
}

#[test]
fn status_200_maps_to_ok() {
    let res = call_with_status(200, "{\"ok\":true}");
    assert!(res.is_ok(), "200 should map to Ok, got {:?}", res);
}

#[test]
fn status_400_maps_to_bad_request() {
    let err = call_with_status(400, "bad input").expect_err("400 should be Err");
    assert!(
        matches!(err, Error::BadRequest(_)),
        "400 should map to BadRequest, got {:?}",
        err
    );
}

#[test]
fn status_401_maps_to_unauthorized() {
    let err = call_with_status(401, "").expect_err("401 should be Err");
    assert!(
        matches!(err, Error::Unauthorized),
        "401 should map to Unauthorized, got {:?}",
        err
    );
}

#[test]
fn status_500_maps_to_internal_server_error() {
    let err = call_with_status(500, "boom").expect_err("500 should be Err");
    assert!(
        matches!(err, Error::InternalServerError(_)),
        "500 should map to InternalServerError, got {:?}",
        err
    );
}

#[test]
fn status_404_maps_to_unexpected_status_code() {
    let err = call_with_status(404, "nope").expect_err("404 should be Err");
    assert!(
        matches!(err, Error::UnexpectedStatusCode(404)),
        "404 should map to UnexpectedStatusCode(404), got {:?}",
        err
    );
}

// --- ClientBuilder ---------------------------------------------------------

/// A `Client` built via `ClientBuilder` sends the `datamaxi-rust/<ver>`
/// User-Agent and the auth header, and routes requests through the same path.
#[test]
fn client_builder_sets_user_agent_and_auth() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("GET", "/cex/candle/exchanges")
        .match_header("X-DTMX-APIKEY", API_KEY)
        .match_header(
            "user-agent",
            Matcher::Regex(r"^datamaxi-rust/\d+\.\d+\.\d+".to_string()),
        )
        .match_query(Matcher::UrlEncoded("market".into(), "spot".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .expect(1)
        .create();

    let client = ClientBuilder::new()
        .api_key(API_KEY)
        .base_url(server.url())
        .build()
        .expect("explicit api key should build");
    let candle = CexCandle { client };
    let res = candle.exchanges("spot");

    mock.assert();
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
