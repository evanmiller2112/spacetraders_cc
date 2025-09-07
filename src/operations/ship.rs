// Individual ship operations module
use crate::client::SpaceTradersClient;
use crate::models::*;

pub struct ShipOperations<'a> {
    client: &'a SpaceTradersClient,
}

impl<'a> ShipOperations<'a> {
    pub fn new(client: &'a SpaceTradersClient) -> Self {
        Self { client }
    }

    pub async fn orbit(&self, ship_symbol: &str) -> Result<ShipNav, Box<dyn std::error::Error>> {
        self.client.orbit_ship(ship_symbol).await
    }

    pub async fn dock(&self, ship_symbol: &str) -> Result<ShipNav, Box<dyn std::error::Error>> {
        self.client.dock_ship(ship_symbol).await
    }

    pub async fn navigate(&self, ship_symbol: &str, waypoint_symbol: &str) -> Result<NavigationData, Box<dyn std::error::Error>> {
        self.client.navigate_ship(ship_symbol, waypoint_symbol).await
    }

    pub async fn refuel(&self, ship_symbol: &str) -> Result<RefuelData, Box<dyn std::error::Error>> {
        self.client.refuel_ship(ship_symbol).await
    }

    pub fn has_mining_capability(&self, ship: &Ship) -> bool {
        ship.mounts.iter().any(|mount| {
            mount.symbol.contains("MINING") || mount.symbol.contains("EXTRACTOR")
        })
    }

    pub fn is_hauler(&self, ship: &Ship) -> bool {
        ship.cargo.capacity >= 20 && !self.has_mining_capability(ship)
    }
}