use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Market {
    pub symbol: String,
    pub exports: Vec<TradeGood>,
    pub imports: Vec<TradeGood>,
    pub exchange: Vec<TradeGood>,
    pub transactions: Option<Vec<MarketTransaction>>,
    #[serde(rename = "tradeGoods")]
    pub trade_goods: Option<Vec<MarketTradeGood>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TradeGood {
    pub symbol: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MarketTradeGood {
    pub symbol: String,
    #[serde(rename = "tradeVolume")]
    pub trade_volume: i32,
    pub supply: String,
    pub activity: Option<String>,
    #[serde(rename = "purchasePrice")]
    pub purchase_price: i32,
    #[serde(rename = "sellPrice")]
    pub sell_price: i32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MarketTransaction {
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

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PurchaseCargoData {
    pub agent: crate::models::Agent,
    pub cargo: crate::models::ship::ShipCargo,
    pub transaction: MarketTransaction,
}