use models::{Card, PrintingKind, Rarity, Set};
use reqwest::Client;
use serde::Deserialize;
use std::{collections::HashSet, time::Duration};

const PAGE_SIZE: usize = 1000;
const MAX_ENTRIES: usize = 100_000;

pub fn valid_id(id: &str) -> bool {
    id.len() == 22
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
}
fn client() -> Result<Client, reqwest::Error> {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("SetBinder/0.1 (+https://setbinder.app)")
        .redirect(reqwest::redirect::Policy::none())
        .build()
}
pub async fn get_data(id: &str) -> Result<Vec<Set>, provider::Error> {
    let client = client()?;
    provider::process_data(&client, get_cards(&client, id).await?).await
}
pub async fn get_set_data(id: &str, code: &str) -> Result<Option<Set>, provider::Error> {
    let client = client()?;
    let mut cards = get_cards(&client, id).await?;
    cards.retain(|card| card.set_code == code);
    if cards.is_empty() {
        return Ok(None);
    }
    Ok(provider::process_data(&client, cards)
        .await?
        .into_iter()
        .next())
}
pub async fn get_cards(client: &Client, id: &str) -> Result<Vec<Card>, provider::Error> {
    if !valid_id(id) {
        return Err(provider::Error::Data(
            "Invalid Moxfield collection ID".into(),
        ));
    }
    get_cards_from_url(
        client,
        &format!("https://api2.moxfield.com/v1/collections/search/{id}"),
    )
    .await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Page {
    page_number: usize,
    page_size: usize,
    total_results: usize,
    total_pages: usize,
    data: Vec<Entry>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    id: String,
    quantity: u64,
    game: String,
    is_proxy: bool,
    card: Printing,
}
#[derive(Deserialize)]
struct Printing {
    name: String,
    set: String,
    cn: String,
    scryfall_id: Option<String>,
}
fn data_error(message: &str) -> provider::Error {
    provider::Error::Data(message.into())
}

async fn get_cards_from_url(client: &Client, url: &str) -> Result<Vec<Card>, provider::Error> {
    let mut cards = Vec::new();
    let mut ids = HashSet::new();
    let mut printings = HashSet::new();
    let mut expected = None;
    let mut page_number = 1;
    loop {
        let page: Page = client
            .get(url)
            .query(&[("pageNumber", page_number), ("pageSize", PAGE_SIZE)])
            .header("Accept", "application/json")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        if page.total_results > MAX_ENTRIES
            || page.page_size == 0
            || page.page_size > PAGE_SIZE
            || page.page_number != page_number
        {
            return Err(data_error(
                "Invalid or oversized Moxfield collection pagination",
            ));
        }
        let pages = page.total_results.div_ceil(page.page_size).max(1);
        if pages > 100
            || (page.total_pages != pages && !(page.total_results == 0 && page.total_pages == 0))
        {
            return Err(data_error("Invalid Moxfield collection page count"));
        }
        let metadata = (page.total_results, page.page_size, pages);
        if expected.is_some_and(|value| value != metadata) {
            return Err(data_error(
                "Moxfield collection changed while loading; please retry",
            ));
        }
        expected = Some(metadata);
        let expected_rows = page
            .total_results
            .saturating_sub((page_number - 1) * page.page_size)
            .min(page.page_size);
        if page.data.len() != expected_rows {
            return Err(data_error(
                "Incomplete Moxfield collection page; please retry",
            ));
        }
        for entry in page.data {
            if entry.id.is_empty() || !ids.insert(entry.id) {
                return Err(data_error(
                    "Repeated Moxfield collection entry; please retry",
                ));
            }
            // Set Binder tracks owned physical printings, regardless of finish or quantity.
            if entry.quantity == 0 || entry.game != "paper" || entry.is_proxy {
                continue;
            }
            let printing = entry.card;
            let code = printing.set.to_ascii_lowercase();
            if code.is_empty()
                || code.len() > 16
                || !code.bytes().all(|c| c.is_ascii_alphanumeric())
                || printing.cn.trim().is_empty()
                || printing.name.trim().is_empty()
            {
                return Err(data_error(
                    "Moxfield card is missing a valid printing identity",
                ));
            }
            if !printings.insert((code.clone(), printing.cn.clone())) {
                continue;
            }
            cards.push(Card {
                name: printing.name,
                set_code: code,
                collector_number: printing.cn,
                scryfall_id: printing.scryfall_id,
                collected: None,
                image: None,
                printing_kind: PrintingKind::Unclassified,
                rarity: Rarity::Unknown,
            });
        }
        if page_number == pages {
            break;
        }
        page_number += 1;
    }
    Ok(cards)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    fn entry(id: &str, number: &str) -> Value {
        json!({"id":id,"quantity":2,"game":"paper","isProxy":false,
            "card":{"name":"Front // Back","set":"ECL","cn":number,"scryfall_id":"printing-id"}})
    }
    fn page(number: usize, total: usize, data: Vec<Value>) -> Value {
        json!({"pageNumber":number,"pageSize":2,"totalResults":total,"totalPages":total.div_ceil(2),"data":data})
    }
    fn server(responses: Vec<(u16, String)>) -> (String, std::thread::JoinHandle<()>) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let thread = std::thread::spawn(move || {
            for (index, (status, body)) in responses.into_iter().enumerate() {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut request = Vec::new();
                let mut byte = [0];
                while !request.ends_with(b"\r\n\r\n") {
                    socket.read_exact(&mut byte).unwrap();
                    request.push(byte[0]);
                }
                let request = String::from_utf8(request).unwrap();
                assert!(
                    request.starts_with(&format!("GET /?pageNumber={}&pageSize=1000 ", index + 1))
                );
                assert!(
                    request
                        .to_ascii_lowercase()
                        .contains("accept: application/json")
                );
                write!(socket,"HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
            }
        });
        (url, thread)
    }
    async fn fetch(responses: Vec<(u16, String)>) -> Result<Vec<Card>, provider::Error> {
        let (url, thread) = server(responses);
        let result = get_cards_from_url(&client().unwrap(), &url).await;
        thread.join().unwrap();
        result
    }
    #[tokio::test]
    async fn paginates_and_deduplicates_exact_printings() {
        let cards = fetch(vec![
            (
                200,
                page(1, 3, vec![entry("a", "347"), entry("b", "347")]).to_string(),
            ),
            (200, page(2, 3, vec![entry("c", "10a")]).to_string()),
        ])
        .await
        .unwrap();
        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].set_code, "ecl");
        assert_eq!(cards[0].name, "Front // Back");
        assert_eq!(cards[0].scryfall_id.as_deref(), Some("printing-id"));
        assert_eq!(cards[1].collector_number, "10a");
    }
    #[tokio::test]
    async fn excludes_digital_proxy_and_zero_quantity_entries() {
        let mut entries = vec![
            entry("a", "1"),
            entry("b", "2"),
            entry("c", "3"),
            entry("d", "4"),
        ];
        entries[0]["game"] = "arena".into();
        entries[1]["game"] = "mtgo".into();
        entries[2]["isProxy"] = true.into();
        entries[3]["quantity"] = 0.into();
        let mut body = page(1, 4, entries);
        body["pageSize"] = 4.into();
        body["totalPages"] = 1.into();
        assert!(
            fetch(vec![(200, body.to_string())])
                .await
                .unwrap()
                .is_empty()
        );
    }
    #[tokio::test]
    async fn accepts_empty_collections() {
        for total_pages in [0, 1] {
            let mut body = page(1, 0, vec![]);
            body["totalPages"] = total_pages.into();
            assert!(
                fetch(vec![(200, body.to_string())])
                    .await
                    .unwrap()
                    .is_empty()
            );
        }
    }
    #[tokio::test]
    async fn rejects_partial_repeated_changed_and_malformed_pages() {
        let first = page(1, 3, vec![entry("a", "1"), entry("b", "2")]);
        for second in [
            page(2, 3, vec![]),
            page(2, 3, vec![entry("a", "3")]),
            page(2, 4, vec![entry("c", "3"), entry("d", "4")]),
            page(1, 3, vec![entry("c", "3")]),
        ] {
            assert!(
                fetch(vec![(200, first.to_string()), (200, second.to_string())])
                    .await
                    .is_err()
            );
        }
        for body in [
            json!({}),
            page(1, 3, vec![]),
            page(1, 100001, vec![]),
            page(1, 1, vec![json!({"id":"bad"})]),
        ] {
            assert!(fetch(vec![(200, body.to_string())]).await.is_err());
        }
        let mut bad = entry("a", " ");
        bad["card"]["set"] = "../bad".into();
        assert!(
            fetch(vec![(200, page(1, 1, vec![bad]).to_string())])
                .await
                .is_err()
        );
    }
    #[tokio::test]
    async fn upstream_errors_are_not_empty_collections() {
        for status in [401, 403, 404, 429, 500] {
            let error = fetch(vec![(status, "{}".into())]).await.unwrap_err();
            assert!(
                matches!(error,provider::Error::Http(error) if error.status().unwrap().as_u16()==status)
            );
        }
        assert!(
            fetch(vec![(200, "<html>challenge</html>".into())])
                .await
                .is_err()
        );
    }
    #[tokio::test]
    async fn invalid_ids_fail_before_http() {
        for id in ["", "../bad", "short", "RD3924ci602tUJxRTOfSJ/"] {
            assert!(!valid_id(id));
            assert!(get_cards(&client().unwrap(), id).await.is_err());
        }
        assert!(valid_id("RD3924ci602tUJxRTOfSJw"));
        assert!(valid_id("RD3924ci602tUJxRTOfS_w"));
    }
    #[tokio::test]
    #[ignore = "requires MOXFIELD_TEST_COLLECTION and live Moxfield access"]
    async fn public_collection_smoke() {
        let id = std::env::var("MOXFIELD_TEST_COLLECTION").expect("set MOXFIELD_TEST_COLLECTION");
        let cards = get_cards(&client().unwrap(), &id).await.unwrap();
        assert!(!cards.is_empty());
        assert!(
            cards
                .iter()
                .all(|c| !c.set_code.is_empty() && !c.collector_number.is_empty())
        );
    }
}
