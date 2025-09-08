use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Waypoint {
    pub symbol: String,
    #[serde(rename = "type")]
    pub waypoint_type: String,
    #[serde(rename = "systemSymbol")]
    pub system_symbol: String,
    pub x: i32,
    pub y: i32,
    pub orbitals: Vec<Orbital>,
    pub traits: Vec<Trait>,
    pub chart: Option<Chart>,
    pub faction: Option<WaypointFaction>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Orbital {
    pub symbol: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Trait {
    pub symbol: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Chart {
    #[serde(rename = "waypointSymbol")]
    pub waypoint_symbol: Option<String>,
    #[serde(rename = "submittedBy")]
    pub submitted_by: Option<String>,
    #[serde(rename = "submittedOn")]
    pub submitted_on: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct WaypointFaction {
    pub symbol: String,
}