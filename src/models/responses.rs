use serde::{Deserialize, Serialize};

// API Response wrappers
#[derive(Debug, Deserialize)]
pub struct AgentResponse {
    pub data: crate::models::Agent,
}

#[derive(Debug, Deserialize)]
pub struct WaypointResponse {
    pub data: crate::models::Waypoint,
}

#[derive(Debug, Deserialize)]
pub struct WaypointsResponse {
    pub data: Vec<crate::models::Waypoint>,
}

#[derive(Debug, Deserialize)]
pub struct ContractsResponse {
    pub data: Vec<crate::models::Contract>,
}

#[derive(Debug, Deserialize)]
pub struct ContractAcceptResponse {
    pub data: crate::models::ContractAcceptData,
}

#[derive(Debug, Deserialize)]
pub struct ShipsResponse {
    pub data: Vec<crate::models::Ship>,
}

#[derive(Debug, Deserialize)]
pub struct ShipResponse {
    pub data: crate::models::Ship,
}

#[derive(Debug, Deserialize)]
pub struct NavigationResponse {
    pub data: crate::models::NavigationData,
}

#[derive(Debug, Deserialize)]
pub struct OrbitResponse {
    pub data: OrbitData,
}

#[derive(Debug, Deserialize)]
pub struct OrbitData {
    pub nav: crate::models::ShipNav,
}

#[derive(Debug, Deserialize)]
pub struct DockResponse {
    pub data: DockData,
}

#[derive(Debug, Deserialize)]
pub struct DockData {
    pub nav: crate::models::ShipNav,
}

#[derive(Debug, Deserialize)]
pub struct ExtractionResponse {
    pub data: crate::models::ExtractionData,
}

#[derive(Debug, Deserialize)]
pub struct SurveyResponse {
    pub data: crate::models::SurveyData,
}

#[derive(Debug, Deserialize)]
pub struct SellCargoResponse {
    pub data: crate::models::SellCargoData,
}

#[derive(Debug, Deserialize)]
pub struct DeliverCargoResponse {
    pub data: crate::models::DeliverCargoData,
}

#[derive(Debug, Deserialize)]
pub struct FulfillContractResponse {
    pub data: crate::models::FulfillContractData,
}

#[derive(Debug, Deserialize)]
pub struct RefuelResponse {
    pub data: crate::models::RefuelData,
}

#[derive(Debug, Deserialize)]
pub struct ShipyardResponse {
    pub data: crate::models::Shipyard,
}

#[derive(Debug, Deserialize)]
pub struct ShipPurchaseResponse {
    pub data: crate::models::ShipPurchaseData,
}

#[derive(Debug, Deserialize)]
pub struct WaypointScanResponse {
    pub data: WaypointScanData,
}

#[derive(Debug, Deserialize)]
pub struct WaypointScanData {
    pub cooldown: crate::models::ShipCooldown,
    pub waypoints: Vec<ScannedWaypoint>,
}

#[derive(Debug, Deserialize)]
pub struct ScannedWaypoint {
    pub symbol: String,
    #[serde(rename = "type")]
    pub waypoint_type: String,
    #[serde(rename = "systemSymbol")]
    pub system_symbol: String,
    pub x: i32,
    pub y: i32,
    pub orbitals: Vec<crate::models::waypoint::Orbital>,
    pub traits: Vec<crate::models::waypoint::Trait>,
    pub chart: Option<crate::models::waypoint::Chart>,
    pub faction: Option<crate::models::waypoint::WaypointFaction>,
}

// New response types for additional API endpoints
#[derive(Debug, Deserialize)]
pub struct SystemsResponse {
    pub data: Vec<crate::models::system::System>,
}

#[derive(Debug, Deserialize)]
pub struct SystemResponse {
    pub data: crate::models::system::System,
}

#[derive(Debug, Deserialize)]
pub struct MarketResponse {
    pub data: crate::models::market::Market,
}

#[derive(Debug, Deserialize)]
pub struct PurchaseCargoResponse {
    pub data: crate::models::market::PurchaseCargoData,
}

#[derive(Debug, Deserialize)]
pub struct SystemScanResponse {
    pub data: SystemScanData,
}

#[derive(Debug, Deserialize)]
pub struct SystemScanData {
    pub cooldown: crate::models::ship::ShipCooldown,
    pub systems: Vec<crate::models::system::ScannedSystem>,
}

#[derive(Debug, Deserialize)]
pub struct ShipScanResponse {
    pub data: ShipScanData,
}

#[derive(Debug, Deserialize)]
pub struct ShipScanData {
    pub cooldown: crate::models::ship::ShipCooldown,
    pub ships: Vec<crate::models::navigation::ScannedShip>,
}

#[derive(Debug, Deserialize)]
pub struct FactionsResponse {
    pub data: Vec<crate::models::faction::Faction>,
}

#[derive(Debug, Deserialize)]
pub struct FactionResponse {
    pub data: crate::models::faction::Faction,
}

#[derive(Debug, Deserialize)]
pub struct JumpGateResponse {
    pub data: crate::models::navigation::JumpGate,
}

#[derive(Debug, Deserialize)]
pub struct JumpResponse {
    pub data: crate::models::navigation::JumpData,
}