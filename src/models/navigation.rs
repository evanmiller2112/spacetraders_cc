use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct JumpGate {
    #[serde(rename = "jumpRange")]
    pub jump_range: i32,
    #[serde(rename = "factionSymbol")]
    pub faction_symbol: Option<String>,
    #[serde(rename = "connectedSystems")]
    pub connected_systems: Vec<ConnectedSystem>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ConnectedSystem {
    pub symbol: String,
    #[serde(rename = "sectorSymbol")]
    pub sector_symbol: String,
    #[serde(rename = "type")]
    pub system_type: String,
    #[serde(rename = "factionSymbol")]
    pub faction_symbol: Option<String>,
    pub x: i32,
    pub y: i32,
    pub distance: i32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct JumpData {
    pub cooldown: crate::models::ship::ShipCooldown,
    pub nav: crate::models::ship::ShipNav,
    pub agent: crate::models::Agent,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ScannedShip {
    pub symbol: String,
    pub registration: crate::models::ship::ShipRegistration,
    pub nav: crate::models::ship::ShipNav,
    pub frame: Option<crate::models::ship::ShipFrame>,
    pub reactor: Option<crate::models::ship::ShipModule>,
    pub engine: crate::models::ship::ShipModule,
    pub mounts: Option<Vec<crate::models::ship::ShipMount>>,
}