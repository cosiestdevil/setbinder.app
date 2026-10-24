use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
};

use models::{Card, PrintingKind, PrintingProgress, ScryfallCard, Set, SetsResponse};

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
    let sets = cards.iter().map(|card| card.set_code.as_str()).collect();
    let bulk = scryfall::get_sets(&sets)
        .map_err(|error| Error::Data(format!("Unable to read Scryfall database: {error}")))?;
    Ok(process_cards(cards, bulk, set_data))
}

/// Natural ordering handles collector numbers such as 2, 10, 10a, and 10★
/// without floating-point parsing or integer overflow.
fn compare_collector_numbers(mut a: &str, mut b: &str) -> Ordering {
    fn chunk(value: &str) -> (&str, &str) {
        let digit = value.starts_with(|c: char| c.is_ascii_digit());
        let end = value
            .char_indices()
            .find(|(_, c)| c.is_ascii_digit() != digit)
            .map_or(value.len(), |(index, _)| index);
        value.split_at(end)
    }
    let original = (a, b);
    while !a.is_empty() && !b.is_empty() {
        let (ac, rest_a) = chunk(a);
        let (bc, rest_b) = chunk(b);
        let ad = ac.as_bytes()[0].is_ascii_digit();
        let bd = bc.as_bytes()[0].is_ascii_digit();
        let order = match (ad, bd) {
            (true, true) => {
                let an = ac.trim_start_matches('0');
                let bn = bc.trim_start_matches('0');
                an.len().cmp(&bn.len()).then_with(|| an.cmp(bn))
            }
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (false, false) => ac.to_lowercase().cmp(&bc.to_lowercase()),
        };
        if !order.is_eq() {
            return order;
        }
        a = rest_a;
        b = rest_b;
    }
    a.len()
        .cmp(&b.len())
        .then_with(|| original.0.cmp(original.1))
}

fn classify_set(mut printings: Vec<ScryfallCard>, owned: &[Card]) -> Vec<Card> {
    printings.sort_by(|a, b| {
        compare_collector_numbers(&a.collector_number, &b.collector_number)
            .then_with(|| a.id.cmp(&b.id))
    });
    // Progress counts each set/collector-number pair once, even if the source repeats it.
    printings.dedup_by(|a, b| a.collector_number == b.collector_number);
    let owned_numbers: HashSet<&str> = owned
        .iter()
        .map(|card| card.collector_number.as_str())
        .collect();
    let mut seen_oracles = HashSet::new();
    printings
        .into_iter()
        .map(|printing| {
            let kind = match printing
                .oracle_id
                .as_deref()
                .filter(|id| !id.trim().is_empty())
            {
                Some(id) if seen_oracles.insert(id.to_owned()) => PrintingKind::Base,
                Some(_) => PrintingKind::Variant,
                None => PrintingKind::Unclassified,
            };
            let mut card = Card::from(printing);
            card.printing_kind = kind;
            card.collected = Some(owned_numbers.contains(card.collector_number.as_str()));
            card
        })
        .collect()
}

fn printing_progress(cards: &[Card]) -> PrintingProgress {
    let mut progress = PrintingProgress::default();
    for card in cards {
        let (total, collected) = match card.printing_kind {
            PrintingKind::Base => (&mut progress.base_count, &mut progress.base_collected),
            PrintingKind::Variant => (&mut progress.variant_count, &mut progress.variant_collected),
            PrintingKind::Unclassified => (
                &mut progress.unclassified_count,
                &mut progress.unclassified_collected,
            ),
        };
        *total += 1;
        *collected += usize::from(card.collected.unwrap_or(false));
    }
    progress
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
        let set_cards = classify_set(bulk_sets.remove(&key).unwrap_or_default(), &cards);
        let progress = printing_progress(&set_cards);
        let set_name = set_data
            .data
            .iter()
            .find(|s| s.code == key)
            .and_then(|s| s.name.clone());
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
            progress,
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
    sets.sort_by(|a, b| {
        b.progress
            .base_completion()
            .total_cmp(&a.progress.base_completion())
            .then_with(|| a.code.cmp(&b.code))
    });
    sets
}

#[cfg(test)]
mod tests {
    use models::Card;

    fn printing(set: &str, number: &str, oracle: Option<&str>) -> models::ScryfallCard {
        models::ScryfallCard {
            name: "Same display name".into(),
            set: set.into(),
            collector_number: number.into(),
            id: format!("{set}-{number}"),
            oracle_id: oracle.map(str::to_owned),
            image_uris: None,
            card_faces: None,
        }
    }
    fn owned(set: &str, number: &str) -> Card {
        Card {
            name: "Owned".into(),
            set_code: set.into(),
            collector_number: number.into(),
            scryfall_id: None,
            collected: None,
            image: None,
            printing_kind: models::PrintingKind::Unclassified,
        }
    }
    #[test]
    fn natural_collector_order_handles_suffixes_and_large_numbers() {
        let mut numbers = vec![
            "A10",
            "10★",
            "10a",
            "10",
            "2",
            "001",
            "1",
            "A2",
            "999999999999999999999999",
        ];
        numbers.sort_by(|a, b| super::compare_collector_numbers(a, b));
        assert_eq!(
            numbers,
            [
                "001",
                "1",
                "2",
                "10",
                "10a",
                "10★",
                "999999999999999999999999",
                "A2",
                "A10"
            ]
        );
    }
    #[test]
    fn lowest_number_per_oracle_is_base_independent_of_input_order() {
        use models::PrintingKind::*;
        let cards = super::classify_set(
            vec![
                printing("tst", "10a", Some("a")),
                printing("tst", "10", Some("a")),
                printing("tst", "2", Some("a")),
                printing("tst", "3", Some("b")),
                printing("tst", "4", None),
                printing("tst", "5", None),
                printing("tst", "6", Some("")),
                printing("tst", "3", Some("b")),
            ],
            &[owned("tst", "10"), owned("tst", "10"), owned("tst", "4")],
        );
        assert_eq!(
            cards
                .iter()
                .map(|c| (c.collector_number.as_str(), c.printing_kind))
                .collect::<Vec<_>>(),
            [
                ("2", Base),
                ("3", Base),
                ("4", Unclassified),
                ("5", Unclassified),
                ("6", Unclassified),
                ("10", Variant),
                ("10a", Variant)
            ]
        );
        let progress = super::printing_progress(&cards);
        assert_eq!((progress.base_count, progress.base_collected), (2, 0));
        assert_eq!((progress.variant_count, progress.variant_collected), (2, 1));
        assert_eq!(
            (progress.unclassified_count, progress.unclassified_collected),
            (3, 1)
        );
        assert_eq!(
            progress.base_completion(),
            0.0,
            "variants must not fill the base bar"
        );
        assert_eq!(progress.variant_bonus(), 0.5);
    }
    #[test]
    fn classification_is_scoped_to_each_set() {
        let sets = super::process_cards(
            vec![owned("aaa", "2"), owned("bbb", "9")],
            vec![
                printing("aaa", "2", Some("shared")),
                printing("aaa", "9", Some("shared")),
                printing("bbb", "9", Some("shared")),
            ],
            models::SetsResponse { data: vec![] },
        );
        for set in sets {
            assert_eq!(set.progress.base_count, 1);
            assert_eq!(set.progress.base_collected, 1);
        }
    }
    #[test]
    fn variant_bonus_can_exceed_one_hundred_and_zero_base_is_safe() {
        let progress = models::PrintingProgress {
            base_count: 2,
            base_collected: 1,
            variant_count: 5,
            variant_collected: 5,
            ..Default::default()
        };
        assert_eq!(progress.base_completion(), 0.5);
        assert_eq!(progress.variant_bonus(), 2.5);
        let unknown = models::PrintingProgress::default();
        assert_eq!(unknown.base_completion(), 0.0);
        assert_eq!(unknown.variant_bonus(), 0.0);
    }
    #[test]
    fn unknown_set_has_finite_completion_and_sorts_after_collected_set() {
        let card = |set: &str| Card {
            name: "Test".into(),
            set_code: set.into(),
            collector_number: "1".into(),
            scryfall_id: None,
            collected: None,
            image: None,
            printing_kind: models::PrintingKind::Unclassified,
        };
        let bulk = models::ScryfallCard {
            name: "Test".into(),
            set: "known".into(),
            collector_number: "1".into(),
            id: "id".into(),
            oracle_id: Some("oracle-id".into()),
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
                printing_kind: models::PrintingKind::Unclassified,
            },
            Card {
                name: "Card 1".to_string(),
                set_code: "bbb".to_string(),
                collector_number: "1".to_string(),
                scryfall_id: None,
                collected: None,
                image: None,
                printing_kind: models::PrintingKind::Unclassified,
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
                printing_kind: models::PrintingKind::Unclassified,
            },
            Card {
                name: "Card 2".to_string(),
                set_code: "aaa".to_string(),
                collector_number: "2".to_string(),
                scryfall_id: None,
                collected: None,
                image: None,
                printing_kind: models::PrintingKind::Unclassified,
            },
            Card {
                name: "Card 1".to_string(),
                set_code: "bbb".to_string(),
                collector_number: "1".to_string(),
                scryfall_id: None,
                collected: None,
                image: None,
                printing_kind: models::PrintingKind::Unclassified,
            },
        ];
        let grouped_cards = super::group_by_set(&cards);
        assert_eq!(grouped_cards.get("aaa").unwrap().len(), 2)
    }
}
