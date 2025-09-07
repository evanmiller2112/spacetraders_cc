use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone)]
pub struct Ship {
    pub symbol: String,
    pub registration: ShipRegistration,
    pub nav: ShipNav,
    pub crew: ShipCrew,
    pub frame: ShipFrame,
    pub reactor: ShipModule,
    pub engine: ShipModule,
    pub cooldown: ShipCooldown,
    pub modules: Vec<ShipModule>,
    pub mounts: Vec<ShipMount>,
    pub cargo: ShipCargo,
    pub fuel: ShipFuel,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ShipRegistration {
    pub name: String,
    #[serde(rename = "factionSymbol")]
    pub faction_symbol: String,
    pub role: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ShipNav {
    #[serde(rename = "systemSymbol")]
    pub system_symbol: String,
    #[serde(rename = "waypointSymbol")]
    pub waypoint_symbol: String,
    pub route: ShipRoute,
    pub status: String,
    #[serde(rename = "flightMode")]
    pub flight_mode: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ShipRoute {
    pub destination: ShipRouteWaypoint,
    pub origin: ShipRouteWaypoint,
    #[serde(rename = "departureTime")]
    pub departure_time: String,
    pub arrival: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ShipRouteWaypoint {
    pub symbol: String,
    #[serde(rename = "type")]
    pub waypoint_type: String,
    #[serde(rename = "systemSymbol")]
    pub system_symbol: String,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ShipCrew {
    pub current: i32,
    pub required: i32,
    pub capacity: i32,
    pub rotation: String,
    pub morale: i32,
    pub wages: i32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ShipFrame {
    pub symbol: String,
    pub name: String,
    pub description: String,
    pub condition: Option<i32>,
    pub integrity: Option<i32>,
    #[serde(rename = "moduleSlots")]
    pub module_slots: i32,
    #[serde(rename = "mountingPoints")]
    pub mounting_points: i32,
    #[serde(rename = "fuelCapacity")]
    pub fuel_capacity: i32,
    pub requirements: ShipRequirements,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ShipModule {
    pub symbol: String,
    pub capacity: Option<i32>,
    pub range: Option<i32>,
    pub name: String,
    pub description: String,
    pub requirements: ShipRequirements,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ShipMount {
    pub symbol: String,
    pub name: String,
    pub description: Option<String>,
    pub strength: Option<i32>,
    pub deposits: Option<Vec<String>>,
    pub requirements: ShipRequirements,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ShipRequirements {
    pub power: Option<i32>,
    pub crew: Option<i32>,
    pub slots: Option<i32>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ShipCooldown {
    #[serde(rename = "shipSymbol")]
    pub ship_symbol: String,
    #[serde(rename = "totalSeconds")]
    pub total_seconds: i32,
    #[serde(rename = "remainingSeconds")]
    pub remaining_seconds: i32,
    pub expiration: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ShipCargo {
    pub capacity: i32,
    pub units: i32,
    pub inventory: Vec<CargoItem>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CargoItem {
    pub symbol: String,
    pub name: String,
    pub description: String,
    pub units: i32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ShipFuel {
    pub current: i32,
    pub capacity: i32,
    pub consumed: Option<ShipFuelConsumed>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ShipFuelConsumed {
    pub amount: i32,
    pub timestamp: String,
}

// Navigation-related structures
#[derive(Debug, Deserialize, Clone)]
pub struct NavigationData {
    pub fuel: ShipFuel,
    pub nav: ShipNav,
}

// Shipyard-related structures
#[derive(Debug, Deserialize, Clone)]
pub struct Shipyard {
    pub symbol: String,
    #[serde(rename = "shipTypes")]
    pub ship_types: Vec<ShipyardShipType>,
    pub transactions: Option<Vec<ShipyardTransaction>>,
    pub ships: Option<Vec<ShipyardShip>>,
    #[serde(rename = "modificationsFee")]
    pub modifications_fee: i32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ShipyardShipType {
    #[serde(rename = "type")]
    pub ship_type: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ShipyardTransaction {
    #[serde(rename = "waypointSymbol")]
    pub waypoint_symbol: String,
    #[serde(rename = "shipSymbol")]
    pub ship_symbol: String,
    pub price: i32,
    #[serde(rename = "agentSymbol")]
    pub agent_symbol: String,
    pub timestamp: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ShipyardShip {
    #[serde(rename = "type")]
    pub ship_type: String,
    pub name: String,
    pub description: String,
    #[serde(rename = "purchasePrice")]
    pub purchase_price: i32,
    pub frame: ShipFrame,
    pub reactor: ShipModule,
    pub engine: ShipModule,
    pub modules: Vec<ShipModule>,
    pub mounts: Vec<ShipMount>,
    pub crew: ShipCrewRequirements,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ShipCrewRequirements {
    pub required: i32,
    pub capacity: i32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ShipPurchaseData {
    pub agent: crate::models::Agent,
    pub ship: Ship,
    pub transaction: ShipPurchaseTransaction,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ShipPurchaseTransaction {
    #[serde(rename = "waypointSymbol")]
    pub waypoint_symbol: String,
    #[serde(rename = "agentSymbol")]
    pub agent_symbol: String,
    #[serde(rename = "shipSymbol")]
    pub ship_symbol: String,
    #[serde(rename = "shipType")]
    pub ship_type: String,
    pub price: i32,
    pub timestamp: String,
}

// Agent structure (ship-related context)
#[derive(Debug, Deserialize, Clone)]
pub struct Agent {
    #[serde(rename = "accountId")]
    pub account_id: String,
    pub symbol: String,
    pub headquarters: String,
    pub credits: i64,
    #[serde(rename = "startingFaction")]
    pub starting_faction: String,
    #[serde(rename = "shipCount")]
    pub ship_count: i32,
}