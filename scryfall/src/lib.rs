use models::{BulkData, ScryfallCard};

//pub const SUBSET_JSON: &str = include_str!(concat!(env!("OUT_DIR"), "/scryfall-cards.json"));
pub const SUBSET_BIN:&[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/scryfall-cards.bin"));
pub fn get_bulk() -> Vec<ScryfallCard> {
    let data:Result<(BulkData,usize),_> = bincode::serde::borrow_decode_from_slice(SUBSET_BIN, bincode::config::standard()); //serde_json::from_str::<BulkData>(SUBSET_JSON);
    if let Ok(data) = data {
        return data.0.cards;
    }
    vec![]
}