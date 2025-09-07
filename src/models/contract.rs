use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone)]
pub struct Contract {
    pub id: String,
    #[serde(rename = "factionSymbol")]
    pub faction_symbol: String,
    #[serde(rename = "type")]
    pub contract_type: String,
    pub terms: ContractTerms,
    pub accepted: bool,
    pub fulfilled: bool,
    pub expiration: String,
    #[serde(rename = "deadlineToAccept")]
    pub deadline_to_accept: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ContractTerms {
    pub deadline: String,
    pub payment: Payment,
    pub deliver: Vec<DeliveryItem>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Payment {
    #[serde(rename = "onAccepted")]
    pub on_accepted: i64,
    #[serde(rename = "onFulfilled")]
    pub on_fulfilled: i64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DeliveryItem {
    #[serde(rename = "tradeSymbol")]
    pub trade_symbol: String,
    #[serde(rename = "destinationSymbol")]
    pub destination_symbol: String,
    #[serde(rename = "unitsRequired")]
    pub units_required: i32,
    #[serde(rename = "unitsFulfilled")]
    pub units_fulfilled: i32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ContractAcceptData {
    pub contract: Contract,
    pub agent: crate::models::Agent,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DeliverCargoData {
    pub contract: Contract,
    pub cargo: crate::models::ShipCargo,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FulfillContractData {
    pub agent: crate::models::Agent,
    pub contract: Contract,
}