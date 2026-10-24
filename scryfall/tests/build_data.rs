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
    assert!(!scryfall::get_bulk().is_empty());
}
