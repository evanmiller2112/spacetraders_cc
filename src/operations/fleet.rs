// Fleet management operations module
use crate::client::SpaceTradersClient;
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
        println!("🚀 Deploying fleet to mining positions...");
        
        if asteroid_fields.is_empty() {
            return Err("No asteroid fields available for deployment".into());
        }
        
        // Assign ships to asteroid fields (round-robin distribution)
        let mut target_assignments = Vec::new();
        println!("🎯 Deploying {} ships to {} asteroid fields (multiple ships per field)", 
                mining_ships.len(), asteroid_fields.len());
        
        for (i, ship) in mining_ships.iter().enumerate() {
            let target_asteroid = &asteroid_fields[i % asteroid_fields.len()];
            target_assignments.push((ship, target_asteroid));
            println!("  📍 {} → {}", ship.symbol, target_asteroid.symbol);
        }
        
        // Navigate all ships to their assigned positions
        for (ship, target_asteroid) in &target_assignments {
            if ship.nav.waypoint_symbol != target_asteroid.symbol {
                println!("🧭 Navigating {} to {}...", ship.symbol, target_asteroid.symbol);
                
                // Put ship in orbit if docked
                if ship.nav.status == "DOCKED" {
                    match self.ship_ops.orbit(&ship.symbol).await {
                        Ok(_) => println!("  ✅ {} put into orbit", ship.symbol),
                        Err(e) => {
                            eprintln!("  ❌ Could not orbit {}: {}", ship.symbol, e);
                            continue;
                        }
                    }
                }
                
                // Navigate to asteroid field
                match self.ship_ops.navigate(&ship.symbol, &target_asteroid.symbol).await {
                    Ok(nav_data) => {
                        println!("  ✅ {} navigation started (fuel: {}/{})", 
                                ship.symbol, nav_data.fuel.current, nav_data.fuel.capacity);
                    }
                    Err(e) => {
                        eprintln!("  ❌ {} navigation failed: {}", ship.symbol, e);
                    }
                }
            } else {
                println!("  ✅ {} already at {}", ship.symbol, target_asteroid.symbol);
            }
        }
        
        // Wait for all ships to arrive
        println!("⏳ Waiting for fleet deployment (30 seconds)...");
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        
        // Assess readiness for mining operations
        self.assess_mining_readiness(target_assignments).await
    }

    pub async fn assess_mining_readiness(
        &self,
        target_assignments: Vec<(&Ship, &Waypoint)>,
    ) -> Result<Vec<(Ship, Waypoint)>, Box<dyn std::error::Error>> {
        println!("🛸 Ensuring all ships are in orbit for mining...");
        
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
                                println!("  ✅ {} in orbit at {}", current_ship.symbol, target_asteroid.symbol);
                                ready_miners.push(((*current_ship).clone(), (*target_asteroid).clone()));
                            }
                            Err(e) => {
                                eprintln!("  ❌ Could not orbit {}: {}", current_ship.symbol, e);
                            }
                        }
                    } else {
                        println!("  ✅ {} already in orbit at {}", current_ship.symbol, target_asteroid.symbol);
                        ready_miners.push(((*current_ship).clone(), (*target_asteroid).clone()));
                    }
                } else {
                    eprintln!("  ⚠️  {} not at target (at {} instead of {})", 
                             current_ship.symbol, current_ship.nav.waypoint_symbol, target_asteroid.symbol);
                }
            }
        }
        
        if ready_miners.is_empty() {
            return Err("No ships ready for mining!".into());
        }
        
        println!("🎉 Fleet deployment complete: {}/{} ships ready for mining!", 
                ready_miners.len(), target_assignments.len());
        
        Ok(ready_miners)
    }

    pub async fn coordinate_fleet_operations(
        &self,
        mining_ships: &[Ship],
        asteroid_fields: &[Waypoint],
    ) -> Result<Vec<(Ship, Waypoint)>, Box<dyn std::error::Error>> {
        println!("🚀 Coordinating fleet operations for autonomous mining...");
        
        // Deploy fleet to mining positions
        let ready_miners = self.deploy_mining_fleet(mining_ships, asteroid_fields).await?;
        
        println!("🚀 Coordinating {} ships across {} asteroid fields!", 
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