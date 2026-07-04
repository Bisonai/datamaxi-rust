//! Live integration tests against the real DataMaxi+ production API.
//!
//! These exercise the full stack over the wire: request construction, auth
//! header, real HTTP, and `handle_response` decoding of a genuine 200 body.
//! Unlike `wire_contract.rs` (offline `mockito`), they catch drift between the
//! SDK and the live API — e.g. a renamed field, a moved path, or a wire-key
//! regression that only manifests against production.
//!
//! Gated on an API key in the environment (`DTMX_API_KEY`, or
//! `DATAMAXI_API_KEY`). When absent, each test prints a SKIP line and returns
//! Ok, so `cargo test` stays green offline and in CI (which has no key). Run
//! locally with the key present to exercise them:
//!
//! ```shell
//! DTMX_API_KEY=... cargo test --test live
//! ```

use datamaxi::api::Datamaxi;
use datamaxi::generated::{
    CexCandle, CexCandleExchangesMarket, CexCandleMarket, CexCandleOptions, Liquidation,
    LiquidationHeatmapOptions, LiquidationHeatmapWindow,
};

/// Resolve the API key from the environment, preferring `DTMX_API_KEY` and
/// falling back to `DATAMAXI_API_KEY`. Empty values are treated as absent.
fn api_key() -> Option<String> {
    ["DTMX_API_KEY", "DATAMAXI_API_KEY"]
        .into_iter()
        .filter_map(|k| std::env::var(k).ok())
        .find(|v| !v.trim().is_empty())
}

/// Fetch the key or print a SKIP line and bail out of the test as a pass.
macro_rules! key_or_skip {
    ($test:literal) => {
        match api_key() {
            Some(k) => k,
            None => {
                eprintln!("SKIP {}: set DTMX_API_KEY to run live tests", $test);
                return;
            }
        }
    };
}

/// `/cex/candle/exchanges` returns a non-empty list including a well-known
/// exchange. Locks the path, auth, and top-level array shape against prod.
#[test]
fn live_cex_candle_exchanges() {
    let key = key_or_skip!("live_cex_candle_exchanges");
    let candle: CexCandle = Datamaxi::new(key);

    let v = candle
        .exchanges(CexCandleExchangesMarket::Spot)
        .expect("live /cex/candle/exchanges request failed");

    let arr = v.as_array().expect("expected a JSON array of exchanges");
    assert!(!arr.is_empty(), "exchange list should not be empty");
    assert!(
        arr.iter().any(|e| e.as_str() == Some("binance")),
        "expected 'binance' in exchange list, got {v}"
    );
}

/// `/cex/candle` returns an object carrying a `data` array. Locks the primary
/// candle endpoint and its response envelope against prod.
#[test]
fn live_cex_candle_get() {
    let key = key_or_skip!("live_cex_candle_get");
    let candle: CexCandle = Datamaxi::new(key);

    let opts = CexCandleOptions::new()
        .market(CexCandleMarket::Spot)
        .interval("1h");
    let v = candle
        .get("binance", "BTC-USDT", opts)
        .expect("live /cex/candle request failed");

    let data = v
        .get("data")
        .and_then(|d| d.as_array())
        .expect("expected a `data` array in the candle response");
    assert!(!data.is_empty(), "candle `data` should not be empty");
}

/// `/liquidation/heatmap` returns an object with a `tokens` array. Also
/// exercises the `top_n` snake_case wire key over the real API (the PR #8
/// regression surface).
#[test]
fn live_liquidation_heatmap() {
    let key = key_or_skip!("live_liquidation_heatmap");
    let liq: Liquidation = Datamaxi::new(key);

    let opts = LiquidationHeatmapOptions::new()
        .window(LiquidationHeatmapWindow::_1h)
        .top_n(3);
    let v = liq
        .heatmap(opts)
        .expect("live /liquidation/heatmap request failed");

    assert!(
        v.get("tokens").and_then(|t| t.as_array()).is_some(),
        "expected a `tokens` array in the heatmap response, got {v}"
    );
}
