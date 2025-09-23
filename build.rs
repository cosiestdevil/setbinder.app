use std::{env, fs, path::PathBuf, time::Duration};

use lightningcss::{
    printer::PrinterOptions,
    stylesheet::ParserOptions,
};

use crate::archidekt_live_set_completion_lib::*;
const CSS: &str = include_str!("./static/style.css");
#[path = "src/lib.rs"]
mod archidekt_live_set_completion_lib;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let out_path = out_dir.join("scryfall-cards.json");
    let client = reqwest::blocking::Client::new();
    let res: serde_json::Value = client
        .get("https://api.scryfall.com/bulk-data")
        .header("User-Agent", "archidekt-live-set-completion")
        .header("Accept", "application/json")
        .send()
        .unwrap()
        .json()
        .unwrap();
    let items = res["data"].as_array().unwrap();
    for item in items {
        if item["type"] == "default_cards" {
            let updated_at = item["updated_at"].as_str().unwrap();
            let download_uri = item["download_uri"].as_str().unwrap();
            let res: Vec<ScryfallCard> = client
                .get(download_uri)
                .header("User-Agent", "archidekt-live-set-completion")
                .header("Accept", "application/json")
                .timeout(Duration::from_secs(200))
                .send()
                .unwrap()
                .json()
                .unwrap();
            fs::write(
                out_path,
                serde_json::to_string_pretty(&BulkData {
                    updated_at: updated_at.to_owned(),
                    cards: res.clone(),
                })
                .unwrap(),
            )
            .unwrap();
            break;
        }
    }

    let styles = lightningcss::stylesheet::StyleSheet::parse(CSS, ParserOptions::default())?;
    let css = styles.to_css(PrinterOptions{minify:true,..PrinterOptions::default()})?;
    fs::write(out_dir.join("styles.min.css"), css.code)?;
    Ok(())
}
