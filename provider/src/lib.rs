use std::collections::HashMap;

use models::{Card, ScryfallCard, Set, SetsResponse};

pub fn group_by_set(cards: &Vec<Card>) -> HashMap<String, Vec<Card>> {
    let mut grouped_cards: HashMap<String, Vec<Card>> = HashMap::new();
    for card in cards {
        grouped_cards
            .entry(card.set_code.clone())
            .or_default()
            .push(card.clone());
    }
    grouped_cards
}

#[derive(Debug)]
pub enum Error {
    Http(reqwest::Error),
    Data(String),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http(error) => write!(f, "{error}"),
            Self::Data(message) => f.write_str(message),
        }
    }
}
impl std::error::Error for Error {}
impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        Self::Http(error)
    }
}

pub async fn process_data(client: &reqwest::Client, cards: Vec<Card>) -> Result<Vec<Set>, Error> {
    let set_data: SetsResponse = client
        .get("https://api.scryfall.com/sets")
        .header("User-Agent", "archidekt-live-set-completion")
        .header("Accept", "application/json")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(process_cards(cards, scryfall::get_bulk(), set_data))
}

fn process_cards(
    cards: Vec<Card>,
    bulk_cards: Vec<ScryfallCard>,
    set_data: SetsResponse,
) -> Vec<Set> {
    // Group cards by set_code using a HashMap
    let grouped_cards = group_by_set(&cards);
    //let mut sets = HashMap::new();
    let mut sets = Vec::new();
    let mut bulk_sets: HashMap<String, Vec<ScryfallCard>> = HashMap::new();
    for card in bulk_cards {
        bulk_sets.entry(card.set.clone()).or_default().push(card);
    }
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
            a_num.total_cmp(&b_num)
        });
        let collected_count = set_cards
            .iter()
            .filter(|c| c.collected.unwrap_or(false))
            .count();
        let set_completion = if set_cards.is_empty() {
            0.0
        } else {
            collected_count as f32 / set_cards.len() as f32
        };
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
    sets.sort_by(|a, b| b.set_completion.total_cmp(&a.set_completion));
    sets
}

#[cfg(test)]
mod tests {
    use models::Card;

    #[test]
    fn unknown_set_has_finite_completion_and_sorts_after_collected_set() {
        let card = |set: &str| Card {
            name: "Test".into(),
            set_code: set.into(),
            collector_number: "1".into(),
            scryfall_id: None,
            collected: None,
            image: None,
        };
        let bulk = models::ScryfallCard {
            name: "Test".into(),
            set: "known".into(),
            collector_number: "1".into(),
            id: "id".into(),
            image_uris: None,
            card_faces: None,
        };
        let sets = super::process_cards(
            vec![card("unknown"), card("known")],
            vec![bulk],
            models::SetsResponse { data: vec![] },
        );
        assert_eq!(sets[0].code, "known");
        assert_eq!(sets[0].set_completion, 1.0);
        assert_eq!(sets[1].set_completion, 0.0);
        assert_eq!(sets[1].set_count, 0);
    }
    #[test]
    fn correct_number_of_sets() {
        let cards = vec![
            Card {
                name: "Card 1".to_string(),
                set_code: "aaa".to_string(),
                collector_number: "1".to_string(),
                scryfall_id: None,
                collected: None,
                image: None,
            },
            Card {
                name: "Card 1".to_string(),
                set_code: "bbb".to_string(),
                collector_number: "1".to_string(),
                scryfall_id: None,
                collected: None,
                image: None,
            },
        ];
        let grouped_cards = super::group_by_set(&cards);
        assert_eq!(grouped_cards.len(), 2)
    }
    #[test]
    fn correct_number_in_set() {
        let cards = vec![
            Card {
                name: "Card 1".to_string(),
                set_code: "aaa".to_string(),
                collector_number: "1".to_string(),
                scryfall_id: None,
                collected: None,
                image: None,
            },
            Card {
                name: "Card 2".to_string(),
                set_code: "aaa".to_string(),
                collector_number: "2".to_string(),
                scryfall_id: None,
                collected: None,
                image: None,
            },
            Card {
                name: "Card 1".to_string(),
                set_code: "bbb".to_string(),
                collector_number: "1".to_string(),
                scryfall_id: None,
                collected: None,
                image: None,
            },
        ];
        let grouped_cards = super::group_by_set(&cards);
        assert_eq!(grouped_cards.get("aaa").unwrap().len(), 2)
    }
}
