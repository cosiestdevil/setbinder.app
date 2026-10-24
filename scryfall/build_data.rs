use models::{BulkData, ScryfallCard};
use serde::Deserialize;
use std::io::{self, BufRead, BufReader, Read};

#[derive(Deserialize)]
pub struct BulkList {
    pub data: Vec<BulkItem>,
}
#[derive(Deserialize)]
pub struct BulkItem {
    pub r#type: String,
    pub updated_at: String,
    pub download_uri: Option<String>,
    pub jsonl_download_uri: Option<String>,
}
impl BulkItem {
    pub fn download(&self) -> Result<(&str, bool), io::Error> {
        if let Some(uri) = &self.jsonl_download_uri {
            Ok((uri, true))
        } else if let Some(uri) = &self.download_uri {
            Ok((uri, false))
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Scryfall default_cards has no supported download URI",
            ))
        }
    }
}

pub fn read_cards(
    reader: impl Read,
    jsonl: bool,
) -> Result<Vec<ScryfallCard>, Box<dyn std::error::Error>> {
    if jsonl {
        BufReader::new(reader)
            .lines()
            .filter_map(|line| match line {
                Ok(line) if line.trim().is_empty() => None,
                line => Some(line),
            })
            .map(|line| Ok(serde_json::from_str(&line?)?))
            .collect()
    } else {
        Ok(serde_json::from_reader(reader)?)
    }
}

pub fn local_bulk(path: &std::path::Path) -> Result<BulkData, Box<dyn std::error::Error>> {
    Ok(serde_json::from_reader(std::fs::File::open(path)?)?)
}
