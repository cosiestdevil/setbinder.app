use archidekt_provider::get_cards;
use reqwest::Client;

#[tokio::test]
#[ignore = "requires a public Archidekt collection and network access"]
async fn gets_cards() {
    let client = Client::new();
    let cards = get_cards(&client, "758741").await.unwrap();
    assert!(cards.len() > 0)
}
