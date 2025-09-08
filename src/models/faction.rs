use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Faction {
    pub symbol: String,
    pub name: String,
    pub description: String,
    pub headquarters: String,
    pub traits: Vec<FactionTrait>,
    #[serde(rename = "isRecruiting")]
    pub is_recruiting: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FactionTrait {
    pub symbol: String,
    pub name: String,
    pub description: String,
}