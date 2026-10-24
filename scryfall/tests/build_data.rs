#[path = "../build_data.rs"]
mod build_data;

const CARD: &str = r#"{"name":"Test","collector_number":"1","set":"tst","id":"test-id"}"#;

#[test]
fn accepts_legacy_and_current_bulk_downloads() {
    for (field, jsonl) in [("download_uri", false), ("jsonl_download_uri", true)] {
        let input = format!(
            r#"{{"data":[{{"type":"default_cards","updated_at":"today","{field}":"https://example.com/cards"}}]}}"#
        );
        let list: build_data::BulkList = serde_json::from_str(&input).unwrap();
        assert_eq!(
            list.data[0].download().unwrap(),
            ("https://example.com/cards", jsonl)
        );
        assert_eq!(list.data[0].r#type, "default_cards");
        assert_eq!(list.data[0].updated_at, "today");
    }
}
#[test]
fn missing_download_is_an_error() {
    let list: build_data::BulkList =
        serde_json::from_str(r#"{"data":[{"type":"default_cards","updated_at":"today"}]}"#)
            .unwrap();
    assert!(list.data[0].download().is_err());
}
#[test]
fn reads_json_array_and_json_lines() {
    let array = format!("[{CARD}]");
    let lines = format!("{CARD}\n\n{CARD}\n");
    assert_eq!(
        build_data::read_cards(array.as_bytes(), false)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        build_data::read_cards(lines.as_bytes(), true)
            .unwrap()
            .len(),
        2
    );
    assert!(build_data::read_cards(b"invalid".as_slice(), true).is_err());
}
#[test]
fn local_file_errors_are_returned() {
    assert!(
        build_data::local_bulk(std::path::Path::new("/nonexistent/setbinder-bulk.json")).is_err()
    );
}
#[test]
fn embedded_database_decodes() {
    let cards = scryfall::get_bulk();
    assert!(!cards.is_empty());
    assert!(
        cards.iter().any(|card| card.oracle_id.is_some()),
        "bulk serialization must retain Oracle IDs"
    );
}

#[test]
fn oracle_identity_is_preserved_and_optional() {
    let input =
        r#"[{"name":"Card","collector_number":"2a","set":"tst","id":"id","oracle_id":"oracle"}]"#;
    let cards = build_data::read_cards(input.as_bytes(), false).unwrap();
    assert_eq!(cards[0].oracle_id.as_deref(), Some("oracle"));
    let cards = build_data::read_cards(CARD.as_bytes(), true).unwrap();
    assert!(cards[0].oracle_id.is_none());
}

#[test]
fn selected_sets_match_bulk_and_empty_selection_retains_nothing() {
    use std::collections::HashSet;
    let bulk = scryfall::get_bulk();
    let code = bulk[0].set.as_str();
    let selected = scryfall::get_sets(&HashSet::from([code])).unwrap();
    let expected: Vec<_> = bulk.iter().filter(|card| card.set == code).collect();
    assert!(!selected.is_empty());
    assert_eq!(
        serde_json::to_value(&selected).unwrap(),
        serde_json::to_value(expected).unwrap()
    );
    assert!(scryfall::get_sets(&HashSet::new()).unwrap().is_empty());
}
