// Exploration operations module - PROBE ship management
use crate::client::SpaceTradersClient;
use crate::models::*;
use crate::operations::ShipOperations;
use std::collections::HashSet;

pub struct ExplorationOperations<'a> {
    client: &'a SpaceTradersClient,
    ship_ops: ShipOperations<'a>,
}

impl<'a> ExplorationOperations<'a> {
    pub fn new(client: &'a SpaceTradersClient) -> Self {
        let ship_ops = ShipOperations::new(client);
        Self { client, ship_ops }
    }

    pub fn get_probe_ships<'b>(&self, ships: &'b [Ship]) -> Vec<&'b Ship> {
        ships.iter().filter(|ship| {
            ship.registration.role == "SATELLITE" || 
            ship.frame.symbol.contains("PROBE")
        }).collect()
    }

    pub async fn explore_nearby_systems_for_shipyards(&self, probe_ships: &[&Ship]) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        println!("🔍 PROBE MISSION: Exploring nearby systems for shipyards...");
        
        if probe_ships.is_empty() {
            println!("❌ No probe ships available for exploration");
            return Ok(vec![]);
        }

        let mut shipyard_locations = Vec::new();
        let mut explored_systems = HashSet::new();

        for probe in probe_ships {
            println!("🛰️  {} starting exploration mission...", probe.symbol);
            
            // Get current system
            let current_system = if probe.nav.waypoint_symbol.contains('-') {
                probe.nav.waypoint_symbol.split('-').take(2).collect::<Vec<&str>>().join("-")
            } else {
                println!("⚠️  Could not determine system for {}", probe.symbol);
                continue;
            };

            if explored_systems.contains(&current_system) {
                continue;
            }
            explored_systems.insert(current_system.clone());

            // Search current system for shipyards
            println!("🔍 {} exploring system {}...", probe.symbol, current_system);
            match self.search_system_for_shipyards(&current_system).await {
                Ok(mut yards) => {
                    if !yards.is_empty() {
                        println!("🎉 {} found {} shipyard(s) in {}!", 
                                probe.symbol, yards.len(), current_system);
                        shipyard_locations.append(&mut yards);
                    } else {
                        println!("📍 {} - No shipyards found in {}", probe.symbol, current_system);
                    }
                }
                Err(e) => {
                    println!("⚠️  {} failed to explore {}: {}", probe.symbol, current_system, e);
                }
            }

            // TODO: Add logic to navigate to nearby systems for broader exploration
            // For now, we'll explore the current system only
        }

        if shipyard_locations.is_empty() {
            println!("🔍 EXPLORATION COMPLETE: No shipyards found in explored systems");
        } else {
            println!("🎉 EXPLORATION SUCCESS: Found shipyards at: {:?}", shipyard_locations);
        }

        Ok(shipyard_locations)
    }

    async fn search_system_for_shipyards(&self, system_symbol: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let waypoints = self.client.get_system_waypoints(system_symbol, None).await?;
        let mut shipyard_waypoints = Vec::new();

        for waypoint in waypoints {
            // Check if waypoint has shipyard trait
            let has_shipyard = waypoint.traits.iter().any(|t| 
                t.name.to_lowercase().contains("shipyard") ||
                t.description.to_lowercase().contains("shipyard")
            );

            if has_shipyard {
                println!("🚢 SHIPYARD FOUND: {} ({})", waypoint.symbol, waypoint.waypoint_type);
                
                // Try to get detailed shipyard info
                match self.client.get_shipyard(system_symbol, &waypoint.symbol).await {
                    Ok(shipyard) => {
                        println!("   ✅ Accessible shipyard with {} ship types", shipyard.ship_types.len());
                        for ship_type in &shipyard.ship_types {
                            println!("      - {}", ship_type.ship_type);
                        }
                        shipyard_waypoints.push(waypoint.symbol);
                    }
                    Err(e) => {
                        println!("   ⚠️  Could not access shipyard details: {}", e);
                    }
                }
            }
        }

        Ok(shipyard_waypoints)
    }

    pub async fn move_probe_to_explore(&self, probe: &Ship, target_waypoint: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("🛰️  Moving {} to explore {}...", probe.symbol, target_waypoint);
        
        if probe.nav.waypoint_symbol == target_waypoint {
            println!("✅ {} already at target location", probe.symbol);
            return Ok(());
        }

        // Put in orbit if docked
        if probe.nav.status == "DOCKED" {
            match self.ship_ops.orbit(&probe.symbol).await {
                Ok(_) => println!("✅ {} put into orbit", probe.symbol),
                Err(e) => println!("⚠️  Could not orbit {}: {}", probe.symbol, e),
            }
        }

        // Navigate to target
        match self.ship_ops.navigate(&probe.symbol, target_waypoint).await {
            Ok(_nav_data) => {
                println!("✅ {} navigation started to {}", probe.symbol, target_waypoint);
                // Note: PROBE ships might have special movement with 0 fuel capacity
            }
            Err(e) => {
                println!("❌ {} navigation failed: {}", probe.symbol, e);
                return Err(e);
            }
        }

        Ok(())
    }

    pub async fn get_nearby_systems(&self, current_system: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        // This is a placeholder - we'd need jump gate or system discovery API
        // For now, we'll explore well-known nearby systems
        let nearby_systems = vec![
            format!("{}", current_system), // Current system
            // Add logic to discover nearby systems through jump gates
        ];
        
        Ok(nearby_systems)
    }
}