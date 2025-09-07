use serde::{Deserialize, Serialize};

// Mining and Survey structures
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Survey {
    pub signature: String,
    pub symbol: String,
    pub deposits: Vec<SurveyDeposit>,
    pub expiration: String,
    pub size: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SurveyDeposit {
    pub symbol: String,
}

#[derive(Debug, Deserialize)]
pub struct SurveyData {
    pub cooldown: crate::models::ShipCooldown,
    pub surveys: Vec<Survey>,
}

#[derive(Debug, Deserialize)]
pub struct ExtractionData {
    pub cooldown: crate::models::ShipCooldown,
    pub extraction: ExtractionResult,
    pub cargo: crate::models::ShipCargo,
}

#[derive(Debug, Deserialize)]
pub struct ExtractionResult {
    #[serde(rename = "shipSymbol")]
    pub ship_symbol: String,
    #[serde(rename = "yield")]
    pub extraction_yield: ExtractionYield,
}

#[derive(Debug, Deserialize)]
pub struct ExtractionYield {
    pub symbol: String,
    pub units: i32,
}

// Trading structures
#[derive(Debug, Deserialize)]
pub struct SellCargoData {
    pub agent: crate::models::Agent,
    pub cargo: crate::models::ShipCargo,
    pub transaction: SellTransaction,
}

#[derive(Debug, Deserialize)]
pub struct SellTransaction {
    #[serde(rename = "waypointSymbol")]
    pub waypoint_symbol: String,
    #[serde(rename = "shipSymbol")]
    pub ship_symbol: String,
    #[serde(rename = "tradeSymbol")]
    pub trade_symbol: String,
    #[serde(rename = "type")]
    pub transaction_type: String,
    pub units: i32,
    #[serde(rename = "pricePerUnit")]
    pub price_per_unit: i32,
    #[serde(rename = "totalPrice")]
    pub total_price: i32,
    pub timestamp: String,
}

// Refueling structures
#[derive(Debug, Deserialize)]
pub struct RefuelData {
    pub agent: crate::models::Agent,
    pub fuel: crate::models::ShipFuel,
    pub transaction: RefuelTransaction,
}

#[derive(Debug, Deserialize)]
pub struct RefuelTransaction {
    #[serde(rename = "waypointSymbol")]
    pub waypoint_symbol: String,
    #[serde(rename = "shipSymbol")]
    pub ship_symbol: String,
    #[serde(rename = "totalPrice")]
    pub total_price: i32,
    #[serde(rename = "fuelPrice")]
    pub fuel_price: i32,
    pub units: i32,
    pub timestamp: String,
}