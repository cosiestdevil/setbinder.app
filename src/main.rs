use archidekt_live_set_completion_lib::*;
use rocket::{form::Form, http::ContentType, response::Redirect};
use rocket_dyn_templates::{Template, context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[macro_use]
extern crate rocket;

#[get("/")]
async fn index() -> Template {
    Template::render("index", context! {})
}
#[post("/", data = "<url>")]
async fn process_url(url: Form<&str>) -> Redirect {
    regex::Regex::new(r"https://archidekt\.com/collection/v2/(\d+)/?")
        .unwrap()
        .captures(&url)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str())
        .map(|id| Redirect::to(format!("/archidekt/{id}")))
        .unwrap_or_else(|| Redirect::to("/"))
}
#[get("/archidekt/<id>")]
async fn archidekt(id: &str) -> Template {
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
    let client = reqwest::Client::new();
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
    // Group cards by set_code using a HashMap
    let mut grouped_cards: HashMap<String, Vec<Card>> = HashMap::new();
    for card in &cards {
        grouped_cards
            .entry(card.set_code.clone())
            .or_default()
            .push(card.clone());
    }

    //let mut sets = HashMap::new();
    let mut sets = Vec::new();
    let bulk_cards = get_bulk();
    let mut bulk_sets: HashMap<String, Vec<ScryfallCard>> = HashMap::new();
    for card in bulk_cards {
        bulk_sets.entry(card.set.clone()).or_default().push(card);
    }
    let set_data: SetsResponse = client
        .get("https://api.scryfall.com/sets")
        .header("User-Agent", "archidekt-live-set-completion")
        .header("Accept", "application/json")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    for (key, cards) in grouped_cards {
        let set_cards = bulk_sets.get(&key).cloned().unwrap_or(Vec::new());
        let mut set_cards: Vec<Card> = set_cards.iter().map(|c| c.clone().into()).collect();
        let set_name = set_data
            .data
            .iter()
            .find(|s| s.code == key)
            .and_then(|s| s.name.clone());
        for card in &mut set_cards {
            card.collected = Some(
                cards
                    .iter()
                    .any(|c| c.collector_number == card.collector_number),
            );
        }
        set_cards.sort_by(|a, b| {
            let a_num = a.collector_number.parse::<f32>().unwrap_or(f32::MAX);
            let b_num = b.collector_number.parse::<f32>().unwrap_or(f32::MAX);
            a_num.partial_cmp(&b_num).unwrap()
        });
        let collected_count = set_cards
            .iter()
            .filter(|c| c.collected.unwrap_or(false))
            .count();
        let set_completion = collected_count as f32 / set_cards.len() as f32;
        sets.push(Set {
            code: key.clone(),
            name: set_name,
            set_completion,
            set_percentage: if set_cards.is_empty() {
                0.0.to_string()
            } else {
                format!("{:.2}%", set_completion * 100.0)
            },
            set_count: set_cards.len(),
            collected_count,
            cards: set_cards,
            collected_cards: cards,
        });
    }
    sets.sort_by(|a, b| b.set_completion.partial_cmp(&a.set_completion).unwrap());
    Template::render("sets", context! { sets: sets })
}
pub const SUBSET_JSON: &str = include_str!(concat!(env!("OUT_DIR"), "/scryfall-cards.json"));
fn get_bulk() -> Vec<ScryfallCard> {
    let data = serde_json::from_str::<BulkData>(SUBSET_JSON);
    if let Ok(data) = data {
        return data.cards;
    }
    vec![]
}
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Set {
    code: String,
    name: Option<String>,
    cards: Vec<Card>,
    collected_cards: Vec<Card>,
    set_percentage: String,
    set_count: usize,
    collected_count: usize,
    set_completion: f32,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
struct SetsResponse {
    data: Vec<ScryfallSet>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
struct ScryfallSet {
    code: String,
    name: Option<String>,
    search_uri: Option<String>,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Card {
    name: String,
    set_code: String,
    collector_number: String,
    scryfall_id: Option<String>,
    collected: Option<bool>,
    image: Option<String>,
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
impl From<ScryfallCard> for Card {
    fn from(value: ScryfallCard) -> Self {
        let image = value
            .image_uris
            .as_ref()
            .and_then(|uris| uris.get("png").cloned())
            .or(value.card_faces.as_ref().and_then(|faces| {
                if !faces.is_empty() {
                    faces[0]
                        .image_uris
                        .as_ref()
                        .and_then(|uris| uris.get("png").cloned())
                } else {
                    None
                }
            }));

        Card {
            name: value.name,
            set_code: value.set,
            collector_number: value.collector_number,
            scryfall_id: Some(value.id),
            collected: Some(false),
            image, //: image.map(|i| format!("/scryfall-thumb.webp?url={}", urlencoding::encode(&i.clone())))
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
const CSS: &str = include_str!("../static/style.css");
#[get("/style.css")]
fn style() -> (ContentType, &'static str) {
    (ContentType::CSS, CSS)
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(Template::fairing())
        .mount("/", routes![archidekt, index, process_url, style])
}
