//! Integration tests for the auto-paginator added for issue #88
//! ([`datamaxi::api::Client::paginate`] / [`datamaxi::api::blocking::Client::paginate`]).
//!
//! These drive a real `page`/`limit`/`total`/`data` envelope
//! (`CexAnnouncementsResponse`) through a mock server and lock: multi-page
//! traversal, the `page * limit >= total` terminal condition, the empty-page
//! terminal condition, and honoring a caller-supplied starting page.

use datamaxi::api::{Client, ClientBuilder};
use datamaxi::CexAnnouncementsResponse;
use mockito::Matcher;
use std::collections::BTreeMap;

const API_KEY: &str = "test-api-key";

fn mock_client(base_url: String) -> Client {
    ClientBuilder::new()
        .api_key(API_KEY)
        .base_url(base_url)
        .build()
        .expect("mock client builds")
}

/// A page body for `CexAnnouncementsResponse`: `n` announcements, plus the
/// envelope's `page`/`limit`/`total`.
fn page_body(page: i64, limit: i64, total: i64, n: usize) -> String {
    let data: Vec<String> = (0..n)
        .map(|i| {
            format!(
                r#"{{"c":"listing","d":0,"e":"binance","s":"summary","t":"item-{page}-{i}","u":"https://example.com"}}"#
            )
        })
        .collect();
    format!(
        r#"{{"data":[{}],"limit":{limit},"page":{page},"total":{total}}}"#,
        data.join(",")
    )
}

/// Two pages of two items each, `total: 3` reached exactly on the second
/// page (`page * limit >= total`): the paginator fetches both pages, then
/// stops without a third HTTP call.
#[tokio::test]
async fn paginate_walks_multiple_pages_until_total_reached() {
    let mut server = mockito::Server::new_async().await;

    let page1 = server
        .mock("GET", "/api/v1/cex/announcements")
        .match_header("X-DTMX-APIKEY", API_KEY)
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("page".into(), "1".into()),
            Matcher::UrlEncoded("limit".into(), "2".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(page_body(1, 2, 3, 2))
        .expect(1)
        .create_async()
        .await;

    let page2 = server
        .mock("GET", "/api/v1/cex/announcements")
        .match_header("X-DTMX-APIKEY", API_KEY)
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("page".into(), "2".into()),
            Matcher::UrlEncoded("limit".into(), "2".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(page_body(2, 2, 3, 1))
        .expect(1)
        .create_async()
        .await;

    let client = mock_client(server.url());
    let mut params = BTreeMap::new();
    params.insert("limit".to_string(), "2".to_string());
    let mut pages =
        client.paginate::<CexAnnouncementsResponse>("/api/v1/cex/announcements", params);

    let first = pages.next_page().await.expect("first page ok");
    assert_eq!(first.map(|items| items.len()), Some(2));

    let second = pages.next_page().await.expect("second page ok");
    assert_eq!(second.map(|items| items.len()), Some(1));

    // Terminal: no third HTTP call should be made.
    let third = pages.next_page().await.expect("terminal call ok");
    assert!(third.is_none());

    page1.assert_async().await;
    page2.assert_async().await;
}

/// A page whose `data` comes back empty terminates the paginator even though
/// `total` hasn't been reached yet (e.g. a stale/over-reported `total`).
#[tokio::test]
async fn paginate_stops_on_empty_page_regardless_of_total() {
    let mut server = mockito::Server::new_async().await;

    let page1 = server
        .mock("GET", "/api/v1/cex/announcements")
        .match_query(Matcher::UrlEncoded("page".into(), "1".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(page_body(1, 1, 100, 1))
        .expect(1)
        .create_async()
        .await;

    let page2 = server
        .mock("GET", "/api/v1/cex/announcements")
        .match_query(Matcher::UrlEncoded("page".into(), "2".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(page_body(2, 1, 100, 0))
        .expect(1)
        .create_async()
        .await;

    let client = mock_client(server.url());
    let mut pages =
        client.paginate::<CexAnnouncementsResponse>("/api/v1/cex/announcements", BTreeMap::new());

    let first = pages.next_page().await.expect("first page ok");
    assert_eq!(first.map(|items| items.len()), Some(1));

    let second = pages.next_page().await.expect("empty page ok");
    assert!(second.is_none());

    page1.assert_async().await;
    page2.assert_async().await;
}

/// A `page` key in the seed params sets the starting page, rather than
/// always starting at `1`.
#[tokio::test]
async fn paginate_honors_starting_page_param() {
    let mut server = mockito::Server::new_async().await;

    let mock = server
        .mock("GET", "/api/v1/cex/announcements")
        .match_query(Matcher::UrlEncoded("page".into(), "3".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(page_body(3, 1, 3, 0))
        .expect(1)
        .create_async()
        .await;

    let client = mock_client(server.url());
    let mut params = BTreeMap::new();
    params.insert("page".to_string(), "3".to_string());
    let mut pages =
        client.paginate::<CexAnnouncementsResponse>("/api/v1/cex/announcements", params);

    let first = pages.next_page().await.expect("first page ok");
    assert!(first.is_none(), "empty page 3 should terminate immediately");

    mock.assert_async().await;
}

/// The blocking mirror ([`datamaxi::api::blocking::Client::paginate`])
/// implements [`Iterator`], yielding one `Result<Vec<_>>` per page and
/// stopping once `page * limit >= total`.
#[cfg(feature = "blocking")]
#[test]
fn blocking_paginate_walks_multiple_pages_until_total_reached() {
    let mut server = mockito::Server::new();

    let page1 = server
        .mock("GET", "/api/v1/cex/announcements")
        .match_query(Matcher::AllOf(vec![Matcher::UrlEncoded(
            "page".into(),
            "1".into(),
        )]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(page_body(1, 2, 3, 2))
        .expect(1)
        .create();

    let page2 = server
        .mock("GET", "/api/v1/cex/announcements")
        .match_query(Matcher::AllOf(vec![Matcher::UrlEncoded(
            "page".into(),
            "2".into(),
        )]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(page_body(2, 2, 3, 1))
        .expect(1)
        .create();

    let client = datamaxi::api::blocking::ClientBuilder::new()
        .api_key(API_KEY)
        .base_url(server.url())
        .build()
        .expect("mock blocking client builds");

    let pages: Vec<_> = client
        .paginate::<CexAnnouncementsResponse>("/api/v1/cex/announcements", BTreeMap::new())
        .collect();

    assert_eq!(pages.len(), 2, "expected exactly two pages, got {pages:?}");
    assert_eq!(pages[0].as_ref().unwrap().len(), 2);
    assert_eq!(pages[1].as_ref().unwrap().len(), 1);

    page1.assert();
    page2.assert();
}
