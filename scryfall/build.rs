use models::BulkData;
use std::{env, fs, io, path::PathBuf, time::Duration};
mod build_data;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=build_data.rs");
    println!("cargo:rerun-if-env-changed=SCRYFALL_BULK_FILE");
    let out_path = PathBuf::from(env::var("OUT_DIR")?).join("scryfall-cards.bin");
    // A pinned BulkData JSON file makes offline and reproducible builds possible.
    // Never silently substitute an empty database when downloading fails.
    let bulk = if let Some(path) = env::var_os("SCRYFALL_BULK_FILE") {
        let path = PathBuf::from(path);
        println!("cargo:rerun-if-changed={}", path.display());
        build_data::local_bulk(&path)?
    } else {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(200))
            .user_agent("archidekt-live-set-completion")
            .build()?;
        let list: build_data::BulkList = client
            .get("https://api.scryfall.com/bulk-data")
            .header("Accept", "application/json")
            .send()?
            .error_for_status()?
            .json()?;
        let item = list.data.into_iter().find(|item| item.r#type == "default_cards")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Scryfall response has no default_cards entry; set SCRYFALL_BULK_FILE to a local BulkData JSON file for an offline build"))?;
        let (uri, jsonl) = item.download()?;
        let response = client.get(uri).send()?.error_for_status()?;
        let cards = if jsonl {
            build_data::read_cards(flate2::read::GzDecoder::new(response), true)?
        } else {
            build_data::read_cards(response, false)?
        };
        BulkData {
            updated_at: item.updated_at,
            cards,
        }
    };
    if bulk.cards.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Scryfall bulk database is empty",
        )
        .into());
    }
    fs::write(
        out_path,
        bincode::serde::encode_to_vec(&bulk, bincode::config::standard())?,
    )?;
    Ok(())
}
