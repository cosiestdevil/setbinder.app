use models::{BulkData, ScryfallCard};

pub const SUBSET_JSON: &str = include_str!(concat!(env!("OUT_DIR"), "/scryfall-cards.json"));
pub fn get_bulk() -> Vec<ScryfallCard> {
    let data = serde_json::from_str::<BulkData>(SUBSET_JSON);
    if let Ok(data) = data {
        return data.cards;
    }
    vec![]
}