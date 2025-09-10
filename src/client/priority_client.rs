// Priority-aware API client for goal-based request ordering
use crate::client::SpaceTradersClient;
use crate::models::*;
use crate::models::ship::{NavigationData, ShipNav};
use crate::models::transaction::{ExtractionData, RefuelData, SellCargoData};
use crate::{o_debug, o_info};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ApiPriority {
    Deferred = 10,
    Background = 20,
    Normal = 40,
    ActiveGoal = 60,
    Urgent = 80,
    Override = 100,
}

pub struct PriorityApiClient {
    client: SpaceTradersClient,
}

impl PriorityApiClient {
    pub fn new(client: SpaceTradersClient) -> Self {
        Self { client }
    }

    pub async fn get_agent(&self) -> Result<Agent, Box<dyn std::error::Error>> {
        self.log_request(ApiPriority::Normal, "get_agent");
        self.client.get_agent().await
    }

    pub async fn get_ships(&self) -> Result<Vec<Ship>, Box<dyn std::error::Error>> {
        self.log_request(ApiPriority::Normal, "get_ships");
        self.client.get_ships().await
    }

    pub async fn get_ship(&self, ship_symbol: &str) -> Result<Ship, Box<dyn std::error::Error>> {
        self.log_request(ApiPriority::ActiveGoal, &format!("get_ship({})", ship_symbol));
        self.client.get_ship(ship_symbol).await
    }

    pub async fn get_contracts(&self) -> Result<Vec<Contract>, Box<dyn std::error::Error>> {
        self.log_request(ApiPriority::Normal, "get_contracts");
        self.client.get_contracts().await
    }

    pub async fn get_system_waypoints(&self, system_symbol: &str, waypoint_type: Option<&str>) -> Result<Vec<Waypoint>, Box<dyn std::error::Error>> {
        self.log_request(ApiPriority::Background, &format!("get_system_waypoints({})", system_symbol));
        self.client.get_system_waypoints(system_symbol, waypoint_type).await
    }

    pub async fn get_waypoint_with_priority(&self, system_symbol: &str, waypoint_symbol: &str, priority: ApiPriority) -> Result<Waypoint, Box<dyn std::error::Error>> {
        self.log_request(priority, &format!("get_waypoint({}) [PRIORITY]", waypoint_symbol));
        let waypoints = self.client.get_system_waypoints(system_symbol, None).await?;
        waypoints.into_iter()
            .find(|w| w.symbol == waypoint_symbol)
            .ok_or_else(|| format!("Waypoint {} not found", waypoint_symbol).into())
    }

    pub async fn get_market_with_priority(&self, system_symbol: &str, waypoint_symbol: &str, priority: ApiPriority) -> Result<Market, Box<dyn std::error::Error>> {
        self.log_request(priority, &format!("get_market({}) [PRIORITY]", waypoint_symbol));
        self.client.get_market(system_symbol, waypoint_symbol).await
    }

    pub async fn navigate_ship_with_priority(&self, ship_symbol: &str, destination: &str, priority: ApiPriority) -> Result<NavigationData, Box<dyn std::error::Error>> {
        self.log_request(priority, &format!("navigate_ship({} -> {}) [PRIORITY]", ship_symbol, destination));
        self.client.navigate_ship(ship_symbol, destination).await
    }

    pub async fn dock_ship_with_priority(&self, ship_symbol: &str, priority: ApiPriority) -> Result<ShipNav, Box<dyn std::error::Error>> {
        self.log_request(priority, &format!("dock_ship({}) [PRIORITY]", ship_symbol));
        self.client.dock_ship(ship_symbol).await
    }

    pub async fn orbit_ship_with_priority(&self, ship_symbol: &str, priority: ApiPriority) -> Result<ShipNav, Box<dyn std::error::Error>> {
        self.log_request(priority, &format!("orbit_ship({}) [PRIORITY]", ship_symbol));
        self.client.orbit_ship(ship_symbol).await
    }

    pub async fn extract_resources_with_priority(&self, ship_symbol: &str, _survey: Option<&Survey>, priority: ApiPriority) -> Result<ExtractionData, Box<dyn std::error::Error>> {
        self.log_request(priority, &format!("extract_resources({}) [PRIORITY]", ship_symbol));
        self.client.extract_resources(ship_symbol).await
    }

    pub async fn refuel_ship_with_priority(&self, ship_symbol: &str, _units: Option<i32>, priority: ApiPriority) -> Result<RefuelData, Box<dyn std::error::Error>> {
        self.log_request(priority, &format!("refuel_ship({}) [PRIORITY]", ship_symbol));
        self.client.refuel_ship(ship_symbol).await
    }

    pub async fn sell_cargo_with_priority(&self, ship_symbol: &str, trade_symbol: &str, units: i32, priority: ApiPriority) -> Result<SellCargoData, Box<dyn std::error::Error>> {
        self.log_request(priority, &format!("sell_cargo({}, {}, {}) [PRIORITY]", ship_symbol, trade_symbol, units));
        self.client.sell_cargo(ship_symbol, trade_symbol, units).await
    }

    fn log_request(&self, priority: ApiPriority, description: &str) {
        if priority >= ApiPriority::Override {
            o_info!("⚡ OVERRIDE API: {}", description);
        } else if priority >= ApiPriority::Urgent {
            o_info!("🚨 URGENT API: {}", description);
        } else if priority >= ApiPriority::ActiveGoal {
            o_debug!("🎯 GOAL API [{}]: {}", priority as u8, description);
        } else {
            o_debug!("📋 API [{}]: {}", priority as u8, description);
        }
    }

    pub async fn get_queue_status(&self) -> (usize, ApiPriority) {
        (0, ApiPriority::Normal) // Placeholder for future queue implementation
    }
}

// Provide access to underlying client for operations that don't need prioritization
impl std::ops::Deref for PriorityApiClient {
    type Target = SpaceTradersClient;
    
    fn deref(&self) -> &Self::Target {
        &self.client
    }
}