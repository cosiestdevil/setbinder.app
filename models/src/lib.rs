use std::collections::HashMap;

use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Set {
    pub code: String,
    pub name: Option<String>,
    pub cards: Vec<Card>,
    pub collected_cards: Vec<Card>,
    pub set_percentage: String,
    pub set_count: usize,
    pub collected_count: usize,
    pub set_completion: f32,
    pub progress: PrintingProgress,
}
/// Counts exact printings: owning a variant does not imply owning its base printing.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PrintingProgress {
    pub base_count: usize,
    pub base_collected: usize,
    pub variant_count: usize,
    pub variant_collected: usize,
    pub unclassified_count: usize,
    pub unclassified_collected: usize,
}
impl PrintingProgress {
    pub fn base_completion(&self) -> f32 {
        self.ratio(self.base_collected)
    }
    pub fn variant_bonus(&self) -> f32 {
        self.ratio(self.variant_collected)
    }
    fn ratio(&self, count: usize) -> f32 {
        if self.base_count == 0 {
            0.0
        } else {
            count as f32 / self.base_count as f32
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PrintingKind {
    Base,
    Variant,
    #[default]
    Unclassified,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SetsResponse {
    pub data: Vec<ScryfallSet>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScryfallSet {
    pub code: String,
    pub name: Option<String>,
    pub search_uri: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Card {
    pub name: String,
    pub set_code: String,
    pub collector_number: String,
    pub scryfall_id: Option<String>,
    pub collected: Option<bool>,
    pub image: Option<String>,
    pub printing_kind: PrintingKind,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScryfallCardList {
    has_more: bool,
    next_page: Option<String>,
    data: Vec<ScryfallCard>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScryfallCard {
    pub name: String,
    pub collector_number: String,
    pub set: String,
    pub id: String,
    pub oracle_id: Option<String>,
    pub image_uris: Option<HashMap<String, String>>,
    pub card_faces: Option<Vec<ScryfallCardFace>>,
}
impl ScryfallCard {
    /// Reversible printings store identity on their faces. Only use that
    /// fallback when every face has the same nonempty Oracle ID.
    pub fn oracle_identity(&self) -> Option<&str> {
        if let Some(id) = self.oracle_id.as_deref().filter(|id| !id.trim().is_empty()) {
            return Some(id);
        }
        let faces = self.card_faces.as_ref()?;
        let id = faces
            .first()?
            .oracle_id
            .as_deref()
            .filter(|id| !id.trim().is_empty())?;
        faces
            .iter()
            .all(|face| face.oracle_id.as_deref() == Some(id))
            .then_some(id)
    }
}
impl From<ScryfallCard> for Card {
    fn from(value: ScryfallCard) -> Self {
        let image = value
            .image_uris
            .as_ref()
            .and_then(|uris| uris.get("png").cloned())
            .or(value.card_faces.as_ref().and_then(|faces| {
                if !faces.is_empty() {
                    faces[0]
                        .image_uris
                        .as_ref()
                        .and_then(|uris| uris.get("png").cloned())
                } else {
                    None
                }
            }));

        Card {
            name: value.name,
            set_code: value.set,
            collector_number: value.collector_number,
            scryfall_id: Some(value.id),
            collected: Some(false),
            printing_kind: PrintingKind::Unclassified,
            image, //: image.map(|i| format!("/scryfall-thumb.webp?url={}", urlencoding::encode(&i.clone())))
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScryfallCardFace {
    pub oracle_id: Option<String>,
    pub image_uris: Option<HashMap<String, String>>,
}
#[derive(Serialize, Deserialize)]
pub struct BulkData {
    pub updated_at: String,
    pub cards: Vec<ScryfallCard>,
}
