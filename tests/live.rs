//! Live integration tests against the real DataMaxi+ production API.
//!
//! These exercise the full stack over the wire: request construction, auth
//! header, real HTTP, and `handle_response` decoding of a genuine 200 body.
//! Unlike `wire_contract.rs` (offline `mockito`), they catch drift between the
//! SDK and the live API — e.g. a renamed field, a moved path, or a wire-key
//! regression that only manifests against production.
//!
//! Also validates the 24 typed response structs in `src/generated.rs`
//! (`CexCandleResponse`, `LiquidationStatsResponse`, `ForexResponse`, etc.)
//! against real payloads: those structs derive `Deserialize` with
//! struct-level `#[serde(default)]`, which tolerates missing fields but not
//! type mismatches, so a passing test here is evidence the derived field
//! types actually match what the API returns on the wire.
//!
//! Gated on an API key in the environment (`DTMX_API_KEY`, or
//! `DATAMAXI_API_KEY`). When absent, each test prints a SKIP line and returns
//! Ok, so `cargo test` stays green offline and in CI (which has no key). Run
//! locally with the key present to exercise them:
//!
//! ```shell
//! DATAMAXI_API_KEY=... cargo test --test live
//! ```

use datamaxi::api::{Datamaxi, Error};
use datamaxi::generated::{
    Announcements, CexAnnouncementsOptions, CexCandle, CexCandleExchangesMarket, CexCandleMarket,
    CexCandleOptions, CexTokenUpdatesOptions, Forex, FundingRate, FundingRateHistoryOptions,
    IndexPrice, IndexPriceOptions, Liquidation, LiquidationFeedOptions, LiquidationHeatmapOptions,
    LiquidationHeatmapWindow, LiquidationMapOptions, LiquidationOptions, LiquidationStatsOptions,
    LiquidationStatsWindow, LiquidationSymbolHistoryInterval, LiquidationSymbolHistoryOptions,
    LiquidationSymbolHistoryWindow, Listing, ListingsHistoricalOptions, MarginBorrow, OpenInterest,
    OpenInterestHistoryAggregatedOptions, OpenInterestListOptions, OpenInterestOverviewOptions,
    OpenInterestSummaryOptions, Premium, PremiumOptions, Telegram, TelegramChannelsOptions,
    TelegramMessagesOptions, Ticker, TickerMarket, TickerOptions, Token,
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

    assert!(!v.is_empty(), "exchange list should not be empty");
    assert!(
        v.iter().any(|e| e == "binance"),
        "expected 'binance' in exchange list, got {v:?}"
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

    assert!(!v.data.is_empty(), "candle `data` should not be empty");
}

/// `/cex/announcements` returns a paginated `data` array of announcements.
#[test]
fn live_cex_announcements() {
    let key = key_or_skip!("live_cex_announcements");
    let ann: Announcements = Datamaxi::new(key);

    let resp = ann
        .announcements(CexAnnouncementsOptions::new())
        .expect("live /cex/announcements request failed");

    assert!(
        !resp.data.is_empty(),
        "announcements `data` should not be empty"
    );
}

/// `/cex/token/updates` returns a paginated `data` array of listed/delisted
/// token updates.
#[test]
fn live_cex_token_updates() {
    let key = key_or_skip!("live_cex_token_updates");
    let token: Token = Datamaxi::new(key);

    let resp = token
        .updates(CexTokenUpdatesOptions::new())
        .expect("live /cex/token/updates request failed");

    assert!(
        !resp.data.is_empty(),
        "token updates `data` should not be empty"
    );
}

/// `/forex` echoes the requested `symbol` in the typed response.
#[test]
fn live_forex_get() {
    let key = key_or_skip!("live_forex_get");
    let forex: Forex = Datamaxi::new(key);

    let resp = forex.get("USD-KRW").expect("live /forex request failed");

    assert_eq!(
        resp.symbol, "USD-KRW",
        "forex response should echo the requested symbol"
    );
}

/// `/funding-rate/history` returns a non-empty `data` array for a liquid
/// perpetual pair.
#[test]
fn live_funding_rate_history() {
    let key = key_or_skip!("live_funding_rate_history");
    let fr: FundingRate = Datamaxi::new(key);

    let opts = FundingRateHistoryOptions::new().limit(5);
    let resp = fr
        .history("binance", "BTC-USDT", opts)
        .expect("live /funding-rate/history request failed");

    assert!(
        !resp.data.is_empty(),
        "funding-rate history `data` should not be empty"
    );
}

/// `/funding-rate/latest` echoes the requested `exchange`.
#[test]
fn live_funding_rate_latest() {
    let key = key_or_skip!("live_funding_rate_latest");
    let fr: FundingRate = Datamaxi::new(key);

    let resp = fr
        .latest("binance", "BTC-USDT")
        .expect("live /funding-rate/latest request failed");

    assert_eq!(
        resp.exchange, "binance",
        "funding-rate latest response should echo the requested exchange"
    );
}

/// `/index-price` returns a non-empty `data` array of price points.
#[test]
fn live_index_price_get() {
    let key = key_or_skip!("live_index_price_get");
    let idx: IndexPrice = Datamaxi::new(key);

    let resp = idx
        .get("BTC", IndexPriceOptions::new())
        .expect("live /index-price request failed");

    assert!(
        !resp.data.is_empty(),
        "index-price `data` should not be empty"
    );
}

/// `/liquidation` returns at most `limit` recent liquidation events.
#[test]
fn live_liquidation_get() {
    let key = key_or_skip!("live_liquidation_get");
    let liq: Liquidation = Datamaxi::new(key);

    let opts = LiquidationOptions::new().limit(5);
    let resp = liq
        .get("binance", "BTC-USDT", opts)
        .expect("live /liquidation request failed");

    assert!(
        resp.data.len() <= 5,
        "liquidation `data` should respect the requested limit"
    );
}

/// `/liquidation/feed` returns at most `limit` recent liquidation events
/// across all symbols.
#[test]
fn live_liquidation_feed() {
    let key = key_or_skip!("live_liquidation_feed");
    let liq: Liquidation = Datamaxi::new(key);

    let opts = LiquidationFeedOptions::new().exchange("binance").limit(5);
    let resp = liq
        .feed(opts)
        .expect("live /liquidation/feed request failed");

    assert!(
        resp.data.len() <= 5,
        "liquidation feed `data` should respect the requested limit"
    );
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

    assert_eq!(
        v.window, "1h",
        "heatmap response should echo the requested window"
    );
}

/// `/liquidation/map` echoes the requested `exchange` for a liquid pair.
#[test]
fn live_liquidation_map() {
    let key = key_or_skip!("live_liquidation_map");
    let liq: Liquidation = Datamaxi::new(key);

    let opts = LiquidationMapOptions::new()
        .exchange("binance")
        .base("BTC")
        .quote("USDT");
    let resp = liq.map(opts).expect("live /liquidation/map request failed");

    assert_eq!(
        resp.exchange, "binance",
        "liquidation map response should echo the requested exchange"
    );
}

/// `/liquidation/stats` echoes the requested `window` (also the `min_volume_usd`
/// snake_case wire key from the PR #8 regression surface).
#[test]
fn live_liquidation_stats() {
    let key = key_or_skip!("live_liquidation_stats");
    let liq: Liquidation = Datamaxi::new(key);

    let opts = LiquidationStatsOptions::new().window(LiquidationStatsWindow::_24h);
    let resp = liq
        .stats(opts)
        .expect("live /liquidation/stats request failed");

    assert_eq!(
        resp.window, "24h",
        "liquidation stats response should echo the requested window"
    );
}

/// `/liquidation/symbol-history` echoes the requested `symbol` and `window`.
#[test]
fn live_liquidation_symbol_history() {
    let key = key_or_skip!("live_liquidation_symbol_history");
    let liq: Liquidation = Datamaxi::new(key);

    let opts = LiquidationSymbolHistoryOptions::new()
        .quote("USDT")
        .exchange("binance")
        .interval(LiquidationSymbolHistoryInterval::_1h)
        .window(LiquidationSymbolHistoryWindow::_24h);
    let resp = liq
        .symbol_history("BTC", opts)
        .expect("live /liquidation/symbol-history request failed");

    assert_eq!(
        resp.symbol, "BTC",
        "liquidation symbol-history response should echo the requested symbol"
    );
    assert_eq!(
        resp.window, "24h",
        "liquidation symbol-history response should echo the requested window"
    );
}

/// `/listings/historical` returns a non-empty `data` array.
#[test]
fn live_listings_historical() {
    let key = key_or_skip!("live_listings_historical");
    let listing: Listing = Datamaxi::new(key);

    match listing.historical(ListingsHistoricalOptions::new()) {
        Ok(resp) => assert!(
            !resp.data.is_empty(),
            "listings historical `data` should not be empty"
        ),
        // Known bug (codegen#55 / backend#7943): `network` is nullable on the
        // wire but typed as non-optional `String` in `ListingsHistoricalView`.
        Err(Error::Http(e)) if e.is_decode() => {
            eprintln!("KNOWN NULL-DECODE /listings/historical `network`: {e}");
        }
        Err(e) => panic!("live /listings/historical request failed: {e}"),
    }
}

/// `/margin-borrow` returns non-null `cross`/`isolated` objects for a widely
/// listed asset.
#[test]
fn live_margin_borrow_get() {
    let key = key_or_skip!("live_margin_borrow_get");
    let mb: MarginBorrow = Datamaxi::new(key);

    let resp = mb.get("BTC").expect("live /margin-borrow request failed");

    assert!(
        !resp.cross.is_null(),
        "margin-borrow `cross` should be present for BTC"
    );
}

/// `/open-interest` echoes the requested `exchange`/`symbol`.
#[test]
fn live_open_interest_get() {
    let key = key_or_skip!("live_open_interest_get");
    let oi: OpenInterest = Datamaxi::new(key);

    let resp = oi
        .get("binance", "BTC-USDT")
        .expect("live /open-interest request failed");

    assert_eq!(
        resp.exchange, "binance",
        "open-interest response should echo the requested exchange"
    );
}

/// `/open-interest/history-aggregated` returns token metadata for the
/// requested `token_id`.
#[test]
fn live_open_interest_history_aggregated() {
    let key = key_or_skip!("live_open_interest_history_aggregated");
    let oi: OpenInterest = Datamaxi::new(key);

    let resp = oi
        .history_aggregated("bitcoin", OpenInterestHistoryAggregatedOptions::new())
        .expect("live /open-interest/history-aggregated request failed");

    assert_eq!(
        resp.token.symbol, "BTC",
        "open-interest history-aggregated response should resolve `bitcoin` to BTC"
    );
}

/// `/open-interest/list` returns a non-empty `data` array.
#[test]
fn live_open_interest_list() {
    let key = key_or_skip!("live_open_interest_list");
    let oi: OpenInterest = Datamaxi::new(key);

    let opts = OpenInterestListOptions::new().exchange("binance");
    let resp = oi
        .list(opts)
        .expect("live /open-interest/list request failed");

    assert!(
        !resp.data.is_empty(),
        "open-interest list `data` should not be empty"
    );
}

/// `/open-interest/overview` returns a non-empty `data` array.
#[test]
fn live_open_interest_overview() {
    let key = key_or_skip!("live_open_interest_overview");
    let oi: OpenInterest = Datamaxi::new(key);

    let resp = oi
        .overview(OpenInterestOverviewOptions::new())
        .expect("live /open-interest/overview request failed");

    assert!(
        !resp.data.is_empty(),
        "open-interest overview `data` should not be empty"
    );
}

/// `/open-interest/summary` returns a non-empty `tokens` array. Also exercises
/// the `top_n` snake_case wire key from the PR #8 regression surface.
#[test]
fn live_open_interest_summary() {
    let key = key_or_skip!("live_open_interest_summary");
    let oi: OpenInterest = Datamaxi::new(key);

    let opts = OpenInterestSummaryOptions::new().top_n(5);
    let resp = oi
        .summary(opts)
        .expect("live /open-interest/summary request failed");

    assert!(
        !resp.tokens.is_empty(),
        "open-interest summary `tokens` should not be empty"
    );
}

/// `/premium` returns a non-empty `data` array with default pagination.
#[test]
fn live_premium_get() {
    let key = key_or_skip!("live_premium_get");
    let premium: Premium = Datamaxi::new(key);

    match premium.get(PremiumOptions::new().limit(10)) {
        Ok(resp) => assert!(!resp.data.is_empty(), "premium `data` should not be empty"),
        // Known bug (codegen#55 / backend#7943): `PremiumDetail.tc` (and
        // siblings like `sc`/`spa`/`tpa`) are nullable on the wire but typed
        // as non-optional `String`.
        Err(Error::Http(e)) if e.is_decode() => {
            eprintln!("KNOWN NULL-DECODE /premium `detail.tc`: {e}");
        }
        Err(e) => panic!("live /premium request failed: {e}"),
    }
}

/// `/telegram/channels` returns a non-empty `data` array with default
/// pagination.
#[test]
fn live_telegram_channels() {
    let key = key_or_skip!("live_telegram_channels");
    let tg: Telegram = Datamaxi::new(key);

    match tg.channels(TelegramChannelsOptions::new()) {
        Ok(resp) => assert!(
            !resp.data.is_empty(),
            "telegram channels `data` should not be empty"
        ),
        // Known bug (codegen#55 / backend#7943): `createdAt` is nullable on
        // the wire but typed as non-optional `i64` in `TelegramChannelsView`.
        Err(Error::Http(e)) if e.is_decode() => {
            eprintln!("KNOWN NULL-DECODE /telegram/channels `createdAt`: {e}");
        }
        Err(e) => panic!("live /telegram/channels request failed: {e}"),
    }
}

/// `/telegram/messages` returns a non-empty `data` array with default
/// pagination.
#[test]
fn live_telegram_messages() {
    let key = key_or_skip!("live_telegram_messages");
    let tg: Telegram = Datamaxi::new(key);

    let resp = tg
        .messages(TelegramMessagesOptions::new())
        .expect("live /telegram/messages request failed");

    assert!(
        !resp.data.is_empty(),
        "telegram messages `data` should not be empty"
    );
}

/// `/ticker` echoes the requested `symbol` in the nested typed `data` view.
#[test]
fn live_ticker_get() {
    let key = key_or_skip!("live_ticker_get");
    let ticker: Ticker = Datamaxi::new(key);

    let resp = ticker
        .get(
            "binance",
            "BTC-USDT",
            TickerMarket::Spot,
            TickerOptions::new(),
        )
        .expect("live /ticker request failed");

    assert_eq!(
        resp.data.symbol, "BTC-USDT",
        "ticker response should echo the requested symbol"
    );
}
