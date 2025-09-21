use image::ImageEncoder;
use rocket::{form::Form, response::Redirect};
use rocket_dyn_templates::{Template, context};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs};
#[macro_use]
extern crate rocket;

const ARCHIDEKT_USER_ID: &str = "758741";
#[get("/")]
async fn index() -> Template {
    Template::render("index", context! {  })
}
#[post("/", data = "<url>")]
async fn process_url(url:Form<String>)->Redirect{
    regex::Regex::new(r"https://archidekt\.com/collection/v2/(\d+)/?")
        .unwrap()
        .captures(&url)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str())
        .map(|id| Redirect::to(format!("/archidekt/{id}")))
        .unwrap_or_else(|| Redirect::to("/"))
}
#[get("/archidekt/<id>")]
async fn archidekt(id:String) -> Template {
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
    let bulk_cards = get_bulk().await;
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
            card.collected = Some(cards.iter().any(|c| c.collector_number == card.collector_number));
        }
        set_cards.sort_by(|a, b| {
            let a_num = a.collector_number.parse::<f32>().unwrap_or(f32::MAX);
            let b_num = b.collector_number.parse::<f32>().unwrap_or(f32::MAX);
            a_num.partial_cmp(&b_num).unwrap()
        });
        let collected_count = set_cards.iter().filter(|c| c.collected.unwrap_or(false)).count();
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
struct ScryfallCardList {
    has_more: bool,
    next_page: Option<String>,
    data: Vec<ScryfallCard>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
struct ScryfallCard {
    name: String,
    collector_number: String,
    set: String,
    id: String,
    image_uris: Option<HashMap<String, String>>,
    card_faces: Option<Vec<ScryfallCardFace>>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
struct ScryfallCardFace {
    image_uris: Option<HashMap<String, String>>,
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
            image:None
        }
    }
}
impl From<ScryfallCard> for Card {
    fn from(value: ScryfallCard) -> Self {
        let image = value.image_uris.as_ref().and_then(|uris| uris.get("png").cloned()).or(
            value.card_faces.as_ref().and_then(|faces| {
                if !faces.is_empty() {
                    faces[0].image_uris.as_ref().and_then(|uris| uris.get("png").cloned())
                } else {
                    None
                }
            }),
        );
        
        Card {
            name: value.name,
            set_code: value.set,
            collector_number: value.collector_number,
            scryfall_id: Some(value.id),
            collected: Some(false),
            image//: image.map(|i| format!("/scryfall-thumb.webp?url={}", urlencoding::encode(&i.clone())))
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

async fn get_bulk() -> Vec<ScryfallCard> {
    let data = serde_json::from_str::<BulkData>(
        &fs::read_to_string("/var/data/scryfall-cards.json").unwrap_or("{}".to_string()),
    );
    if let Ok(data) = data {
        let age = chrono::Utc::now().signed_duration_since(
            chrono::DateTime::parse_from_rfc3339(&data.updated_at)
                .unwrap()
                .with_timezone(&chrono::Utc),
        );
        if age.num_days() < 7 {
            return data.cards;
        }
    }
    let client = reqwest::Client::new();
    let res: serde_json::Value = client
        .get("https://api.scryfall.com/bulk-data")
        .header("User-Agent", "archidekt-live-set-completion")
        .header("Accept", "application/json")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let items = res["data"].as_array().unwrap();
    for item in items {
        if item["type"] == "default_cards" {
            let updated_at = item["updated_at"].as_str().unwrap();
            let download_uri = item["download_uri"].as_str().unwrap();
            let res: Vec<ScryfallCard> = client
                .get(download_uri)
                .header("User-Agent", "archidekt-live-set-completion")
                .header("Accept", "application/json")
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            fs::write(
                "/var/data/scryfall-cards.json",
                serde_json::to_string_pretty(&BulkData {
                    updated_at: updated_at.to_owned(),
                    cards: res.clone(),
                })
                .unwrap(),
            )
            .unwrap();
            return res;
        }
    }
    vec![]
}
#[derive(Serialize, Deserialize)]
struct BulkData {
    updated_at: String,
    cards: Vec<ScryfallCard>,
}


#[get("/scryfall-thumb.webp?<url>")]
async fn scryfall_thumb(url: &str, cache: &State<CacheConfig>)
    -> Result<(ContentType, Vec<u8>), Status>
{
    // 1) Basic allowlist: only accept Scryfall image hosts.
    let parsed = Url::parse(url).map_err(|_| Status::BadRequest)?;
    let host_ok = matches!(parsed.host_str(), Some(h) if
        h.ends_with("scryfall.com") || h.ends_with("cards.scryfall.io"));
    if !host_ok { return Err(Status::BadRequest); }

    // 2) Cache key: sha256(url) + fixed size and extension.
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    let key = format!("{:x}-146x204.webp", hasher.finalize());
    let path = cache.dir.join(key);

    // 3) If cached, serve it.
    if tokio::fs::try_exists(&path).await.map_err(|_| Status::InternalServerError)? {
        let bytes = tokio::fs::read(&path).await.map_err(|_| Status::InternalServerError)?;
        return Ok((ContentType::JPEG, bytes));
    }

    // 4) Not cached: download original.
    let resp = reqwest::get(parsed.as_str()).await.map_err(|_| Status::BadGateway)?;
    if !resp.status().is_success() { return Err(Status::BadGateway); }
    let original = resp.bytes().await.map_err(|_| Status::BadGateway)?;

    // 5) Decode + resize to 146x204.
    let img = image::load_from_memory(&original)
        .map_err(|_| Status::UnsupportedMediaType)?;
    let resized = image::imageops::resize(&img, 146, 204, image::imageops::FilterType::CatmullRom);
    
    // 6) Encode as JPEG (quality 85).
    let mut out = Vec::new();
    let encoder = image::codecs::webp::WebPEncoder::new_lossless(&mut out);
    //let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 85);
    //encoder.encode_image(&resized).map_err(|_| Status::InternalServerError)?;
    encoder.write_image(&resized, 146, 204, image::ExtendedColorType::Rgba8).map_err(|_| Status::InternalServerError)?;
   // encoder.encode(buf, width, height, color_type)

    // 7) Ensure cache dir exists; write atomically.
    if let Err(_) = tokio::fs::create_dir_all(&cache.dir).await {
        // If we can't create cache dir, still serve the bytes below.
    } else {
        let tmp = path.with_extension("part");
        if tokio::fs::write(&tmp, &out).await.is_ok() {
            let _ = tokio::fs::rename(&tmp, &path).await; // best-effort
        }
    }

    Ok((ContentType::WEBP, out))
}
use std::{path::{Path, PathBuf}, io::Cursor};
use rocket::{State, http::ContentType, http::Status};
use sha2::{Digest, Sha256};
use url::Url;

#[derive(Clone)]
struct CacheConfig {
    dir: PathBuf,
}
#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(Template::fairing())
        .manage(CacheConfig { dir: PathBuf::from("/var/data") })    
        .mount("/", routes![archidekt,index,process_url,scryfall_thumb])
}
