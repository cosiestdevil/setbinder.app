use std::sync::Arc;

use futures::{StreamExt, stream,TryStreamExt};
use models::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};
pub async fn get_data(id: String) -> Result<Vec<Set>, Box<dyn std::error::Error>> {
    let client = Arc::new(Client::new());
    let cards = get_cards(client.clone(), id).await;
    let cards = provider::process_data(client.clone(), cards?).await;
    Ok(cards)
}
pub async fn get_cards(client: Arc<Client>, id: String) -> Result<Vec<Card>,Box<dyn std::error::Error>> {
    let post = ExportRequest {
        fields: vec![
            "card__oracleCard__name".to_string(),
            "card__edition__editioncode".to_string(),
            "card__collectorNumber".to_string(),
            "card__uid".to_string(),
        ],
        game: 1,
        page: 1,
        page_size: 2500,
    };
    let page = get_page(client.clone(), post.clone(), id.clone()).await?;
    let total = page.1;
    let cards: Vec<Card> = page.0;
    let page_count = (total as f32 / post.page_size as f32).ceil() as u8;
    if page_count <= 1 {
        return Ok(cards);
    }
    let cards = stream::iter(1..page_count)
        .map(|it| {
            let mut post = post.clone();
            post.page = it;
            (client.clone(), post, id.clone())
        })
        .map(async move |it| {
            let (client, post, id) = it;
            get_page(client, post, id).await
        })
        .buffer_unordered(5)
        .try_fold(cards, |mut acc,(mut cards,_)| async move{
            
            acc.append(&mut cards);
            Ok(acc)
        }).await;
    
    cards
}
async fn get_page(
    client: Arc<Client>,
    post: ExportRequest,
    id: String,
) -> Result<(Vec<Card>, usize), Box<dyn std::error::Error>> {
    let mut cards: Vec<Card> = vec![];
    let res: ExportResponse = client
        .post(format!(
            "https://archidekt.com/api/collection/export/v2/{id}/"
        ))
        .json(&post)
        .send()
        .await?
        .json()
        .await?;
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(post.page == 1)
        .from_reader(res.content.as_bytes());
    //let mut rdr = csv::Reader::from_reader(res.content.as_bytes());
    for result in rdr.deserialize() {
        let record: ArchidektCard = result?;
        cards.push(record.into());
    }
    return Ok((cards, res.total_rows as usize));
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
            collected: Some(true),
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
#[derive(Serialize, Clone)]
struct ExportRequest {
    fields: Vec<String>,
    game: u8,
    page: u8,
    #[serde(rename = "pageSize")]
    page_size: u32,
}
