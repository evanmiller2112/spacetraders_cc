// Fleet management operations module
use crate::client::SpaceTradersClient;
use crate::{o_info};
use crate::models::*;
use crate::operations::ShipOperations;
use std::collections::HashSet;

pub struct FleetOperations<'a> {
    client: &'a SpaceTradersClient,
    ship_ops: ShipOperations<'a>,
}

impl<'a> FleetOperations<'a> {
    pub fn new(client: &'a SpaceTradersClient) -> Self {
        let ship_ops = ShipOperations::new(client);
        Self { client, ship_ops }
    }

    pub async fn get_all_ships(&self) -> Result<Vec<Ship>, Box<dyn std::error::Error>> {
        self.client.get_ships().await
    }

    pub fn get_mining_ships<'b>(&self, ships: &'b [Ship]) -> Vec<&'b Ship> {
        ships.iter().filter(|ship| self.ship_ops.has_mining_capability(ship)).collect()
    }

    pub fn get_hauler_ships<'b>(&self, ships: &'b [Ship]) -> Vec<&'b Ship> {
        ships.iter().filter(|ship| self.ship_ops.is_hauler(ship)).collect()
    }

    pub fn analyze_fleet(&self, ships: &[Ship]) -> FleetAnalysis {
        let miners = self.get_mining_ships(ships);
        let haulers = self.get_hauler_ships(ships);
        
        FleetAnalysis {
            total_ships: ships.len(),
            mining_ships: miners.len(),
            hauler_ships: haulers.len(),
            total_cargo_capacity: ships.iter().map(|s| s.cargo.capacity).sum(),
            total_cargo_used: ships.iter().map(|s| s.cargo.units).sum(),
        }
    }

    pub async fn deploy_mining_fleet(
        &self,
        mining_ships: &[Ship],
        asteroid_fields: &[Waypoint],
    ) -> Result<Vec<(Ship, Waypoint)>, Box<dyn std::error::Error>> {
        o_info!("🚀 Deploying fleet to mining positions...");
        
        if asteroid_fields.is_empty() {
            return Err("No asteroid fields available for deployment".into());
        }
        
        // Assign ships to asteroid fields (round-robin distribution)
        let mut target_assignments = Vec::new();
        o_info!("🎯 Deploying {} ships to {} asteroid fields (multiple ships per field)", 
                mining_ships.len(), asteroid_fields.len());
        
        for (i, ship) in mining_ships.iter().enumerate() {
            let target_asteroid = &asteroid_fields[i % asteroid_fields.len()];
            target_assignments.push((ship, target_asteroid));
            o_info!("  📍 {} → {}", ship.symbol, target_asteroid.symbol);
        }
        
        // Navigate all ships to their assigned positions with fuel management
        for (ship, target_asteroid) in &target_assignments {
            if ship.nav.waypoint_symbol != target_asteroid.symbol {
                o_info!("🧭 Navigating {} to {}...", ship.symbol, target_asteroid.symbol);
                
                // Check fuel before navigation
                o_info!("  ⛽ Current fuel: {}/{}", ship.fuel.current, ship.fuel.capacity);
                
                // Always refuel if fuel is below 90% to ensure successful navigation
                let fuel_safety_threshold = (ship.fuel.capacity as f64 * 0.9) as i32;
                if ship.fuel.current < fuel_safety_threshold {
                    o_info!("  ⚠️ Low fuel detected ({} < {} safety threshold). Attempting to refuel...", 
                            ship.fuel.current, fuel_safety_threshold);
                    
                    // Dock if not already docked for refueling
                    if ship.nav.status != "DOCKED" {
                        match self.ship_ops.dock(&ship.symbol).await {
                            Ok(_) => o_info!("    🛸 {} docked for refueling", ship.symbol),
                            Err(e) => {
                                o_info!("    ❌ Could not dock {} for refueling: {}", ship.symbol, e);
                                continue;
                            }
                        }
                    }
                    
                    // Refuel ship at current location
                    match self.ship_ops.refuel(&ship.symbol).await {
                        Ok(refuel_data) => {
                            o_info!("    ⛽ {} refueled! Fuel: {}/{} (Cost: {} credits)", 
                                    ship.symbol,
                                    refuel_data.fuel.current, 
                                    refuel_data.fuel.capacity,
                                    refuel_data.transaction.total_price);
                        }
                        Err(e) => {
                            o_info!("    ⚠️ Could not refuel {}: {}", ship.symbol, e);
                            o_info!("    💡 Current location may not offer refueling services");
                            
                            // Try to find a fuel station dynamically
                            let system_symbol = &ship.nav.system_symbol;
                            let waypoints = match self.client.get_system_waypoints(system_symbol, None).await {
                                Ok(waypoints) => waypoints,
                                Err(e) => {
                                    o_info!("    ⚠️ Could not get waypoints to find fuel station: {}", e);
                                    continue;
                                }
                            };
                            
                            // Find nearest marketplace for refueling
                            let fuel_station = waypoints.iter()
                                .find(|w| w.traits.iter().any(|t| t.symbol == "MARKETPLACE"))
                                .map(|w| w.symbol.clone());
                                
                            match fuel_station {
                                Some(station) => {
                                    o_info!("    🚀 Attempting to navigate to fuel station {}...", station);
                                    
                                    // Try to go to the fuel station first
                                    match self.ship_ops.orbit(&ship.symbol).await {
                                        Ok(_) => {},
                                        Err(e) => o_info!("    ⚠️ Could not orbit: {}", e),
                                    }
                                    
                                    match self.ship_ops.navigate(&ship.symbol, &station).await {
                                Ok(_) => {
                                    o_info!("    ✅ Navigating to fuel station");
                                    // Wait a bit for arrival
                                    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                                    
                                    // Dock and try to refuel at fuel station
                                    if let Ok(_) = self.ship_ops.dock(&ship.symbol).await {
                                        if let Ok(refuel_data) = self.ship_ops.refuel(&ship.symbol).await {
                                            o_info!("    ⛽ {} refueled at fuel station! Fuel: {}/{}", 
                                                    ship.symbol, refuel_data.fuel.current, refuel_data.fuel.capacity);
                                        }
                                    }
                                }
                                Err(e) => {
                                    o_info!("    ❌ Could not navigate to fuel station: {}", e);
                                    o_info!("    💡 Continuing with current fuel - may cause navigation issues");
                                }
                            }
                                }
                                None => {
                                    o_info!("    ⚠️ No fuel stations found in {}", system_symbol);
                                    o_info!("    💡 Continuing with current fuel - may cause navigation issues");
                                }
                            }
                        }
                    }
                }
                
                // Put ship in orbit if docked
                if ship.nav.status == "DOCKED" {
                    match self.ship_ops.orbit(&ship.symbol).await {
                        Ok(_) => o_info!("  ✅ {} put into orbit", ship.symbol),
                        Err(e) => {
                            o_info!("  ❌ Could not orbit {}: {}", ship.symbol, e);
                            continue;
                        }
                    }
                }
                
                // Navigate to asteroid field
                match self.ship_ops.navigate(&ship.symbol, &target_asteroid.symbol).await {
                    Ok(nav_data) => {
                        o_info!("  ✅ {} navigation started (fuel: {}/{})", 
                                ship.symbol, nav_data.fuel.current, nav_data.fuel.capacity);
                    }
                    Err(e) => {
                        o_info!("  ❌ {} navigation failed: {}", ship.symbol, e);
                        o_info!("  💡 This may be due to insufficient fuel or other navigation constraints");
                    }
                }
            } else {
                o_info!("  ✅ {} already at {}", ship.symbol, target_asteroid.symbol);
            }
        }
        
        // Check if any ships are actually navigating
        let mut ships_navigating = false;
        let mut max_navigation_time = 0;
        
        for (ship, target_asteroid) in &target_assignments {
            if ship.nav.waypoint_symbol != target_asteroid.symbol {
                ships_navigating = true;
                // Estimate navigation time based on distance or use a reasonable default
                // For now, we'll use a conservative 20 seconds as ships were just given navigation commands
                max_navigation_time = max_navigation_time.max(20);
            }
        }
        
        if ships_navigating {
            o_info!("⏳ Waiting for {} ships to complete navigation ({} seconds)...", 
                    target_assignments.iter().filter(|(ship, target)| ship.nav.waypoint_symbol != target.symbol).count(),
                    max_navigation_time);
            tokio::time::sleep(tokio::time::Duration::from_secs(max_navigation_time)).await;
        } else {
            o_info!("✅ All ships already at target locations - no deployment wait needed");
        }
        
        // Assess readiness for mining operations
        self.assess_mining_readiness(target_assignments).await
    }

    pub async fn assess_mining_readiness(
        &self,
        target_assignments: Vec<(&Ship, &Waypoint)>,
    ) -> Result<Vec<(Ship, Waypoint)>, Box<dyn std::error::Error>> {
        o_info!("🛸 Ensuring all ships are in orbit for mining...");
        
        // Get current status of all ships
        let deployed_ships = self.client.get_ships().await?;
        let mut ready_miners = Vec::new();
        
        for (original_ship, target_asteroid) in &target_assignments {
            if let Some(current_ship) = deployed_ships.iter().find(|s| s.symbol == original_ship.symbol) {
                if current_ship.nav.waypoint_symbol == target_asteroid.symbol {
                    // Ship is at correct location
                    if current_ship.nav.status != "IN_ORBIT" {
                        match self.ship_ops.orbit(&current_ship.symbol).await {
                            Ok(_) => {
                                o_info!("  ✅ {} in orbit at {}", current_ship.symbol, target_asteroid.symbol);
                                ready_miners.push(((*current_ship).clone(), (*target_asteroid).clone()));
                            }
                            Err(e) => {
                                o_info!("  ❌ Could not orbit {}: {}", current_ship.symbol, e);
                            }
                        }
                    } else {
                        o_info!("  ✅ {} already in orbit at {}", current_ship.symbol, target_asteroid.symbol);
                        ready_miners.push(((*current_ship).clone(), (*target_asteroid).clone()));
                    }
                } else {
                    o_info!("  ⚠️  {} not at target (at {} instead of {})", 
                             current_ship.symbol, current_ship.nav.waypoint_symbol, target_asteroid.symbol);
                }
            }
        }
        
        if ready_miners.is_empty() {
            return Err("No ships ready for mining!".into());
        }
        
        o_info!("🎉 Fleet deployment complete: {}/{} ships ready for mining!", 
                ready_miners.len(), target_assignments.len());
        
        Ok(ready_miners)
    }

    pub async fn coordinate_fleet_operations(
        &self,
        mining_ships: &[Ship],
        asteroid_fields: &[Waypoint],
    ) -> Result<Vec<(Ship, Waypoint)>, Box<dyn std::error::Error>> {
        o_info!("🚀 Coordinating fleet operations for autonomous mining...");
        
        // Deploy fleet to mining positions
        let ready_miners = self.deploy_mining_fleet(mining_ships, asteroid_fields).await?;
        
        o_info!("🚀 Coordinating {} ships across {} asteroid fields!", 
                ready_miners.len(),
                ready_miners.iter()
                    .map(|(_, asteroid)| asteroid.symbol.as_str())
                    .collect::<HashSet<_>>()
                    .len());
        
        Ok(ready_miners)
    }

    pub fn calculate_fleet_efficiency(&self, ships: &[Ship]) -> FleetEfficiencyMetrics {
        let total_mining_power = ships.iter()
            .filter(|ship| self.ship_ops.has_mining_capability(ship))
            .count();
        
        let total_cargo_space = ships.iter().map(|s| s.cargo.capacity).sum::<i32>();
        let used_cargo_space = ships.iter().map(|s| s.cargo.units).sum::<i32>();
        
        FleetEfficiencyMetrics {
            mining_power: total_mining_power,
            cargo_utilization: if total_cargo_space > 0 {
                (used_cargo_space as f64 / total_cargo_space as f64) * 100.0
            } else { 0.0 },
            active_ships: ships.iter().filter(|s| s.nav.status == "IN_ORBIT").count(),
            idle_ships: ships.iter().filter(|s| s.nav.status == "DOCKED").count(),
        }
    }
}

pub struct FleetAnalysis {
    pub total_ships: usize,
    pub mining_ships: usize,
    pub hauler_ships: usize,
    pub total_cargo_capacity: i32,
    pub total_cargo_used: i32,
}

pub struct FleetEfficiencyMetrics {
    pub mining_power: usize,
    pub cargo_utilization: f64,
    pub active_ships: usize,
    pub idle_ships: usize,
}