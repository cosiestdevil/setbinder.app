use models::ScryfallCard;
use std::collections::HashSet;
use std::io::Cursor;

pub const SUBSET_BIN: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/scryfall-cards.bin"));

/// Decode one card at a time, retaining only sets present in the collection.
/// The database stays in read-only executable storage instead of being expanded
/// into a second, fully allocated copy on every request.
pub fn get_sets(sets: &HashSet<&str>) -> Result<Vec<ScryfallCard>, bincode::error::DecodeError> {
    decode_cards(SUBSET_BIN, |card| sets.contains(card.set.as_str()))
}

pub fn get_bulk() -> Vec<ScryfallCard> {
    decode_cards(SUBSET_BIN, |_| true).expect("embedded Scryfall database must decode")
}

fn decode_cards(
    bytes: &[u8],
    retain: impl Fn(&ScryfallCard) -> bool,
) -> Result<Vec<ScryfallCard>, bincode::error::DecodeError> {
    let mut reader = Cursor::new(bytes);
    let config = bincode::config::standard();
    // BulkData is encoded as updated_at followed by a length-prefixed card vector.
    let _: String = bincode::serde::decode_from_std_read(&mut reader, config)?;
    let count: u64 = bincode::decode_from_std_read(&mut reader, config)?;
    let mut cards = Vec::new();
    for _ in 0..count {
        let card: ScryfallCard = bincode::serde::decode_from_std_read(&mut reader, config)?;
        if retain(&card) {
            cards.push(card);
        }
    }
    Ok(cards)
}
