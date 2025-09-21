use std::collections::HashMap;

use serde::{Deserialize, Serialize};

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
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScryfallCardFace {
    pub image_uris: Option<HashMap<String, String>>,
}
#[derive(Serialize, Deserialize)]
pub struct BulkData {
    pub updated_at: String,
    pub cards: Vec<ScryfallCard>,
}