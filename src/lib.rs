// SpaceTraders Autonomous Agent Library
// Modular architecture for 100% autonomous gameplay

pub mod models;
pub mod client;
pub mod operations;
pub mod admiral;
pub mod storage;
pub mod debug;

// Re-export commonly used types
pub use models::{
    ship::{Ship, ShipNav, ShipCargo, CargoItem},
    contract::{Contract, DeliveryItem},
    waypoint::Waypoint,
    responses::*,
};

pub use client::SpaceTradersClient;
pub use admiral::Admiral;

// Constants
pub const API_BASE_URL: &str = "https://api.spacetraders.io/v2";
pub const AGENT_TOKEN_FILE: &str = "AGENT_TOKEN";