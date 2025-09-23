use models::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};
pub async fn get_data(id: &str) -> Vec<Set> {
    let client = Client::new();
    let cards = get_cards(&client,id).await;
    provider::process_data(&client, cards).await
}
pub async fn get_cards(client:&Client,id: &str) -> Vec<Card> {
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
        post.page += 1;
        let res: ExportResponse = client
            .post(format!(
                "https://archidekt.com/api/collection/export/v2/{id}/"
            ))
            .json(&post)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(post.page == 1)
            .from_reader(res.content.as_bytes());
        //let mut rdr = csv::Reader::from_reader(res.content.as_bytes());
        for result in rdr.deserialize() {
            let record: ArchidektCard = result.unwrap();
            cards.push(record.into());
        }
        if !res.more_content {
            break;
        }
    }
    cards
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
    collected: Option<bool>,
}

impl From<ArchidektCard> for Card {
    fn from(value: ArchidektCard) -> Self {
        Card {
            name: value.name,
            set_code: value.set_code,
            collector_number: value.collector_number,
            scryfall_id: value.scryfall_id,
            collected: value.collected,
            image: None,
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
    page: u8,
    #[serde(rename = "pageSize")]
    page_size: u32,
}
