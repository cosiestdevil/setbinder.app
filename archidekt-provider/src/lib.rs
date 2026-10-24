use models::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};
pub async fn get_data(id: &str) -> Result<Vec<Set>, provider::Error> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    let cards = get_cards(&client, id).await?;
    provider::process_data(&client, cards).await
}
pub async fn get_cards(client: &Client, id: &str) -> Result<Vec<Card>, provider::Error> {
    get_cards_from_url(
        client,
        &format!("https://archidekt.com/api/collection/export/v2/{id}/"),
    )
    .await
}

async fn get_cards_from_url(client: &Client, url: &str) -> Result<Vec<Card>, provider::Error> {
    let mut post = ExportRequest {
        fields: vec![
            "card__oracleCard__name".to_string(),
            "card__edition__editioncode".to_string(),
            "card__collectorNumber".to_string(),
            "card__uid".to_string(),
        ],
        game: 1,
        page: 0,
        page_size: 2500,
    };
    let mut cards: Vec<Card> = vec![];

    loop {
        post.page = post
            .page
            .checked_add(1)
            .ok_or_else(|| provider::Error::Data("Too many export pages".into()))?;
        let res: ExportResponse = client
            .post(url)
            .json(&post)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        cards.extend(parse_page(&res.content, post.page == 1)?);
        if !res.more_content {
            break;
        }
    }
    Ok(cards)
}
fn parse_page(content: &str, has_headers: bool) -> Result<Vec<Card>, provider::Error> {
    csv::ReaderBuilder::new()
        .has_headers(has_headers)
        .from_reader(content.as_bytes())
        .deserialize::<ArchidektCard>()
        .map(|record| {
            record
                .map(Card::from)
                .map_err(|error| provider::Error::Data(error.to_string()))
        })
        .collect()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ArchidektCard {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Edition Code")]
    set_code: String,
    #[serde(rename = "Collector Number")]
    collector_number: String,
    #[serde(rename = "Scryfall ID")]
    scryfall_id: Option<String>,
}

impl From<ArchidektCard> for Card {
    fn from(value: ArchidektCard) -> Self {
        Card {
            name: value.name,
            set_code: value.set_code,
            collector_number: value.collector_number,
            scryfall_id: value.scryfall_id,
            collected: None,
            image: None,
            printing_kind: PrintingKind::Unclassified,
        }
    }
}
#[derive(Serialize, Deserialize)]
struct ExportResponse {
    content: String,
    #[serde(rename = "totalRows")]
    total_rows: u32,
    #[serde(rename = "moreContent")]
    more_content: bool,
}
#[derive(Serialize)]
struct ExportRequest {
    fields: Vec<String>,
    game: u8,
    page: u32,
    #[serde(rename = "pageSize")]
    page_size: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    // A local HTTP server keeps error and pagination tests independent of live services.
    fn server(responses: Vec<(&'static str, String)>) -> String {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            for (status, body) in responses {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                    .unwrap();
                let mut request = Vec::new();
                let mut byte = [0];
                while !request.ends_with(b"\r\n\r\n") {
                    socket.read_exact(&mut byte).unwrap();
                    request.push(byte[0]);
                }
                let headers = String::from_utf8_lossy(&request);
                let length: usize = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse().unwrap())
                    })
                    .unwrap_or(0);
                socket.read_exact(&mut vec![0; length]).unwrap();
                write!(socket, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            }
        });
        url
    }
    #[tokio::test]
    async fn upstream_status_and_malformed_response_return_errors() {
        let client = Client::new();
        for (status, body) in [
            ("404 Not Found", "{}"),
            ("429 Too Many Requests", "{}"),
            ("200 OK", "invalid"),
        ] {
            let url = server(vec![(status, body.into())]);
            let error = get_cards_from_url(&client, &url).await.unwrap_err();
            match error {
                provider::Error::Http(error) if status != "200 OK" => {
                    assert_eq!(error.status().unwrap().as_u16().to_string(), &status[..3])
                }
                provider::Error::Http(error) => assert!(error.is_decode()),
                other => panic!("unexpected error: {other}"),
            }
        }
    }
    #[tokio::test]
    async fn follows_export_pages() {
        let content = |csv: &str, more: bool| {
            // CSV has no embedded quotes in this fixture.
            format!(
                r#"{{"content":"{}","totalRows":2501,"moreContent":{more}}}"#,
                csv.replace('\n', "\\n")
            )
        };
        let url = server(vec![
            (
                "200 OK",
                content(
                    "Name,Edition Code,Collector Number,Scryfall ID\nCard One,tst,1,id-one\n",
                    true,
                ),
            ),
            ("200 OK", content("Card Two,tst,2,id-two\n", false)),
        ]);
        let cards = get_cards_from_url(&Client::new(), &url).await.unwrap();
        assert_eq!(cards.len(), 2);
        assert_eq!(cards[1].name, "Card Two");
    }
    #[test]
    fn parses_header_and_continuation_pages() {
        let first = "Name,Edition Code,Collector Number,Scryfall ID\nCard One,tst,1,id-one\n";
        let next = "Card Two,tst,2,id-two\nCard Three,tst,3,\n";
        let mut cards = parse_page(first, true).unwrap();
        cards.extend(parse_page(next, false).unwrap());
        assert_eq!(cards.len(), 3);
        assert_eq!(cards[1].collector_number, "2");
        assert_eq!(cards[2].scryfall_id, None);
    }
    #[test]
    fn malformed_csv_returns_error() {
        assert!(parse_page("Card,tst\n", false).is_err());
    }
}
