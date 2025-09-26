use archidekt_provider::get_cards;
use reqwest::Client;

#[tokio::test]
async fn gets_cards(){
    let client = Client::new();
    let cards = get_cards(&client,"758741").await;
    assert!(cards.len() > 0)
}