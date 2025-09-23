use std::collections::HashMap;

use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Set {
    pub code: String,
    pub name: Option<String>,
    pub cards: Vec<Card>,
    pub collected_cards: Vec<Card>,
    pub set_percentage: String,
    pub set_count: usize,
    pub collected_count: usize,
    pub set_completion: f32,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SetsResponse {
    pub data: Vec<ScryfallSet>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScryfallSet {
    pub code: String,
    pub name: Option<String>,
    pub search_uri: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Card {
    pub name: String,
    pub set_code: String,
    pub collector_number: String,
    pub scryfall_id: Option<String>,
    pub collected: Option<bool>,
    pub image: Option<String>,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScryfallCardList {
    has_more: bool,
    next_page: Option<String>,
    data: Vec<ScryfallCard>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScryfallCard {
    pub name: String,
    pub collector_number: String,
    pub set: String,
    pub id: String,
    pub image_uris: Option<HashMap<String, String>>,
    pub card_faces: Option<Vec<ScryfallCardFace>>,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScryfallCardFace {
    pub image_uris: Option<HashMap<String, String>>,
}
#[derive(Serialize, Deserialize)]
pub struct BulkData {
    pub updated_at: String,
    pub cards: Vec<ScryfallCard>,
}