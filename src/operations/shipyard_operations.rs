// Shipyard operations for purchasing and outfitting ships
use crate::client::SpaceTradersClient;
use crate::{o_info};
use crate::models::*;

pub struct ShipyardOperations {
    client: SpaceTradersClient,
}

impl ShipyardOperations {
    pub fn new(client: SpaceTradersClient) -> Self {
        Self { client }
    }
    
    /// Find shipyards in available systems, using exploration if needed
    pub async fn find_shipyards(&self) -> Result<Vec<ShipyardLocation>, Box<dyn std::error::Error>> {
        o_info!("🔍 Searching for shipyards across known systems...");
        
        let mut shipyard_locations = Vec::new();
        
        // First, check our home system thoroughly
        o_info!("🏠 Checking home system for shipyards...");
        match self.client.get_system_waypoints_with_traits("X1-N5", "SHIPYARD").await {
            Ok(waypoints) => {
                for waypoint in &waypoints {
                    let location = ShipyardLocation {
                        system_symbol: "X1-N5".to_string(),
                        waypoint_symbol: waypoint.symbol.clone(),
                        waypoint_type: waypoint.waypoint_type.clone(),
                        traits: waypoint.traits.clone(),
                    };
                    shipyard_locations.push(location);
                    o_info!("🏭 Found shipyard in home system: {}", waypoint.symbol);
                }
            }
            Err(e) => {
                o_info!("⚠️ Failed to check home system: {}", e);
            }
        }
        
        if !shipyard_locations.is_empty() {
            o_info!("✅ Found {} shipyards in home system", shipyard_locations.len());
            return Ok(shipyard_locations);
        }
        
        // If no shipyards in home system, search nearby systems
        o_info!("🔍 No shipyards in home system, searching nearby systems...");
        
        let systems = self.client.get_systems(Some(1), Some(20)).await?;
        o_info!("📊 Searching {} systems for shipyards", systems.len());
        
        for system in &systems {
            // Skip home system since we already checked it
            if system.symbol == "X1-N5" {
                continue;
            }
            
            match self.client.get_system_waypoints_with_traits(&system.symbol, "SHIPYARD").await {
                Ok(waypoints) => {
                    // Only check charted waypoints
                    let charted_waypoints: Vec<_> = waypoints.iter()
                        .filter(|w| !w.traits.iter().any(|t| t.symbol == "UNCHARTED"))
                        .collect();
                    
                    if !charted_waypoints.is_empty() {
                        o_info!("📊 System {} has {} shipyards", system.symbol, charted_waypoints.len());
                    }
                    
                    for waypoint in charted_waypoints {
                        let location = ShipyardLocation {
                            system_symbol: system.symbol.clone(),
                            waypoint_symbol: waypoint.symbol.clone(),
                            waypoint_type: waypoint.waypoint_type.clone(),
                            traits: waypoint.traits.clone(),
                        };
                        shipyard_locations.push(location);
                        o_info!("🏭 Found shipyard: {} in {}", waypoint.symbol, system.symbol);
                    }
                }
                Err(e) => {
                    o_info!("⚠️ Failed to access system {}: {}", system.symbol, e);
                }
            }
        }
        
        if shipyard_locations.is_empty() {
            o_info!("❌ No shipyards found in charted systems");
            o_info!("💡 Suggestion: Use probe ship to explore and scan more systems");
            
            // For now, return an error that indicates we need exploration
            return Err("No shipyards found - exploration required to discover shipyards in uncharted systems".into());
        }
        
        o_info!("🏭 Found {} shipyards total", shipyard_locations.len());
        for location in &shipyard_locations {
            o_info!("  • {} in {}", location.waypoint_symbol, location.system_symbol);
        }
        
        Ok(shipyard_locations)
    }
    
    /// Purchase a mining ship similar to the reference ship
    pub async fn purchase_mining_ship(&self, shipyard_location: &ShipyardLocation, reference_ship: &Ship) -> Result<Ship, Box<dyn std::error::Error>> {
        o_info!("🏗️ Purchasing mining ship at {}", shipyard_location.waypoint_symbol);
        
        // Get shipyard details to see available ships
        let shipyard = self.client.get_shipyard(&shipyard_location.system_symbol, &shipyard_location.waypoint_symbol).await?;
        
        if let Some(ships) = &shipyard.ships {
            // Look for a suitable mining ship
            let suitable_ship = ships.iter().find(|ship| {
                // Look for ships with similar cargo capacity or mining capability
                ship.frame.symbol == reference_ship.frame.symbol ||
                ship.mounts.iter().any(|m| m.symbol.contains("MINING")) ||
                ship.ship_type.contains("MINING") ||
                ship.ship_type.contains("FRIGATE")
            });
            
            if let Some(ship_to_buy) = suitable_ship {
                o_info!("🎯 Found suitable ship: {} - {} credits", ship_to_buy.ship_type, ship_to_buy.purchase_price);
                o_info!("   📝 {}", ship_to_buy.description);
                
                // Check if we have enough credits
                let agent = self.client.get_agent().await?;
                if agent.credits >= ship_to_buy.purchase_price as i64 {
                    o_info!("💰 Purchasing {} for {} credits", ship_to_buy.ship_type, ship_to_buy.purchase_price);
                    
                    let purchase_data = self.client.purchase_ship(&ship_to_buy.ship_type, &shipyard_location.waypoint_symbol).await?;
                    
                    o_info!("✅ Successfully purchased ship: {}", purchase_data.ship.symbol);
                    o_info!("   💸 Transaction cost: {} credits", purchase_data.transaction.price);
                    o_info!("   💰 Remaining credits: {}", purchase_data.agent.credits);
                    
                    Ok(purchase_data.ship)
                } else {
                    let needed = ship_to_buy.purchase_price as i64 - agent.credits;
                    Err(format!("Insufficient credits: need {} more credits", needed).into())
                }
            } else {
                Err("No suitable mining ships available at this shipyard".into())
            }
        } else {
            Err("No ships available for purchase at this shipyard".into())
        }
    }
    
    /// Outfit a newly purchased ship with mining equipment
    pub async fn outfit_mining_ship(&self, ship: &Ship, reference_ship: &Ship) -> Result<(), Box<dyn std::error::Error>> {
        o_info!("🛠️ Outfitting {} with mining equipment", ship.symbol);
        
        // For now, this would require additional mount/module installation APIs
        // which may not be implemented yet
        o_info!("   🎯 Target configuration (based on {}):", reference_ship.symbol);
        
        for mount in &reference_ship.mounts {
            if mount.symbol.contains("MINING") || mount.symbol.contains("SURVEYOR") {
                o_info!("     • Install {} - {}", mount.symbol, mount.name);
            }
        }
        
        for module in &reference_ship.modules {
            if module.symbol.contains("CARGO") || module.symbol.contains("PROCESSOR") {
                o_info!("     • Install {} - {}", module.symbol, module.name);
            }
        }
        
        o_info!("   ⚠️ Outfitting system needs ship modification APIs");
        o_info!("   💡 Ship can be used as-is for basic mining operations");
        
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ShipyardLocation {
    pub system_symbol: String,
    pub waypoint_symbol: String,
    pub waypoint_type: String,
    pub traits: Vec<crate::models::waypoint::Trait>,
}