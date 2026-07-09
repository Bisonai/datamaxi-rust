//! Regression test for issue #113: response structs must be constructible
//! by downstream consumers.
//!
//! Files under `tests/` compile as a separate crate linking `datamaxi` as an
//! external dependency, so `#[non_exhaustive]` applies here exactly as it
//! would to a real downstream user -- both the struct-literal and the
//! `..Default::default()` forms below fail to compile if it's present.

use datamaxi::{CexAnnouncementsView, CexCandleView};

#[test]
fn response_structs_are_constructible_by_downstream_crates() {
    // Struct literal -- impossible while `#[non_exhaustive]` is present.
    let candle = CexCandleView {
        close: 1.5,
        timestamp: 1704067200,
        high: 2.0,
        low: 0.5,
        open: 1.0,
        volume: 10.0,
    };
    assert_eq!(candle.close, 1.5);

    // Functional update syntax -- also blocked by `#[non_exhaustive]`.
    let ann = CexAnnouncementsView {
        title: "t".into(),
        ..Default::default()
    };
    assert_eq!(ann.title, "t");
}
