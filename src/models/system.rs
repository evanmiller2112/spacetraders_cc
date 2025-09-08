use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct System {
    pub symbol: String,
    #[serde(rename = "sectorSymbol")]
    pub sector_symbol: String,
    #[serde(rename = "type")]
    pub system_type: String,
    pub x: i32,
    pub y: i32,
    pub waypoints: Vec<SystemWaypoint>,
    pub factions: Vec<SystemFaction>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SystemWaypoint {
    pub symbol: String,
    #[serde(rename = "type")]
    pub waypoint_type: String,
    pub x: i32,
    pub y: i32,
    pub orbitals: Vec<SystemOrbital>,
    pub traits: Option<Vec<SystemTrait>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SystemOrbital {
    pub symbol: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SystemTrait {
    pub symbol: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SystemFaction {
    pub symbol: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ScannedSystem {
    pub symbol: String,
    #[serde(rename = "sectorSymbol")]
    pub sector_symbol: String,
    #[serde(rename = "type")]
    pub system_type: String,
    pub x: i32,
    pub y: i32,
    pub distance: i32,
}