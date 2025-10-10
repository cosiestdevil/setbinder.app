use std::sync::Arc;

use archidekt_provider::get_cards;
use reqwest::Client;

#[tokio::test]
async fn gets_cards(){
    let client = Arc::new(Client::new());
    let cards = get_cards(client,"758741".to_string()).await;
    assert!(cards.is_ok());
    let cards = cards.unwrap();
    assert!(cards.len() > 0)
}