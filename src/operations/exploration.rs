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

            // Multi-system exploration: discover and explore connected systems
            println!("🚀 {} expanding exploration to connected systems...", probe.symbol);
            
            // Discover systems connected via jump gates
            let connected_systems = match self.discover_connected_systems(&current_system).await {
                Ok(systems) => systems,
                Err(e) => {
                    println!("  ⚠️  Could not discover connected systems: {}", e);
                    continue;
                }
            };
            
            // Explore each connected system that we haven't explored yet
            for target_system in connected_systems {
                if explored_systems.contains(&target_system) {
                    println!("  ⏭️  Skipping {} (already explored)", target_system);
                    continue;
                }
                
                println!("  🌌 {} exploring new system: {}...", probe.symbol, target_system);
                explored_systems.insert(target_system.clone());
                
                // Check if we need to navigate to the new system (find jump gate in current system)
                match self.navigate_probe_to_system(probe, &current_system, &target_system).await {
                    Ok(()) => {
                        println!("    ✅ {} successfully navigated to {}", probe.symbol, target_system);
                        
                        // Search the new system for shipyards
                        match self.search_system_for_shipyards(&target_system).await {
                            Ok(mut yards) => {
                                if !yards.is_empty() {
                                    println!("    🎉 {} found {} shipyard(s) in {}!", 
                                            probe.symbol, yards.len(), target_system);
                                    shipyard_locations.append(&mut yards);
                                } else {
                                    println!("    📍 {} - No shipyards found in {}", probe.symbol, target_system);
                                }
                            }
                            Err(e) => {
                                println!("    ⚠️  {} failed to explore {}: {}", probe.symbol, target_system, e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("    ❌ {} could not navigate to {}: {}", probe.symbol, target_system, e);
                        // Continue with other systems
                    }
                }
                
                // For efficiency, limit exploration per probe to avoid getting too spread out
                if shipyard_locations.len() >= 5 {
                    println!("  🎯 {} reached shipyard discovery limit, returning...", probe.symbol);
                    break;
                }
            }
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

    /// Navigate a probe to a different system via jump gate
    async fn navigate_probe_to_system(&self, probe: &Ship, from_system: &str, to_system: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("  🚀 Navigating {} from {} to {}...", probe.symbol, from_system, to_system);
        
        // Find jump gate in current system
        let jump_gates = self.find_jump_gates(from_system).await?;
        if jump_gates.is_empty() {
            return Err(format!("No jump gates found in {}", from_system).into());
        }
        
        // Use first available jump gate (could be enhanced to find the best one)
        let jump_gate_waypoint = &jump_gates[0];
        println!("    🚪 Using jump gate: {}", jump_gate_waypoint);
        
        // Navigate to jump gate if not already there
        if probe.nav.waypoint_symbol != *jump_gate_waypoint {
            println!("    📍 Moving to jump gate {}...", jump_gate_waypoint);
            self.move_probe_to_explore(probe, jump_gate_waypoint).await?;
            
            // Wait for arrival (jump gates require physical presence)
            println!("    ⏳ Waiting for arrival at jump gate...");
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        }
        
        // Perform the actual jump using SpaceTraders API
        println!("    ⚡ Executing jump to {}...", to_system);
        
        match self.client.jump_ship(&probe.symbol, to_system).await {
            Ok(jump_data) => {
                println!("    ✅ Jump successful!");
                println!("      🛰️  {} now in system {}", probe.symbol, jump_data.nav.system_symbol);
                println!("      ⏳ Cooldown: {}s", jump_data.cooldown.remaining_seconds);
                
                // Wait for cooldown to finish before continuing exploration
                if jump_data.cooldown.remaining_seconds > 0.0 {
                    println!("    ⏰ Waiting for jump cooldown to complete...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(jump_data.cooldown.remaining_seconds as u64)).await;
                }
            }
            Err(e) => {
                println!("    ❌ Jump failed: {}", e);
                return Err(e);
            }
        }
        
        Ok(())
    }

    /// Discover connected systems through jump gates
    pub async fn discover_connected_systems(&self, system_symbol: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        println!("🔍 Discovering systems connected to {}...", system_symbol);
        
        // First, find jump gates in the current system
        let jump_gates = self.find_jump_gates(system_symbol).await?;
        
        if jump_gates.is_empty() {
            println!("  ❌ No jump gates found in {}", system_symbol);
            return Ok(vec![]);
        }
        
        let mut connected_systems = HashSet::new();
        
        for jump_gate_waypoint in jump_gates {
            println!("  🚪 Querying jump gate at {}...", jump_gate_waypoint);
            
            match self.client.get_jump_gate(system_symbol, &jump_gate_waypoint).await {
                Ok(jump_gate) => {
                    println!("    📡 Jump range: {} | Connected systems: {}", 
                            jump_gate.jump_range, jump_gate.connected_systems.len());
                    
                    for connected in jump_gate.connected_systems {
                        if connected.symbol != system_symbol {
                            connected_systems.insert(connected.symbol.clone());
                            println!("      🌌 {} - {} (distance: {})", 
                                    connected.symbol, connected.system_type, connected.distance);
                        }
                    }
                }
                Err(e) => {
                    println!("    ⚠️  Could not access jump gate {}: {}", jump_gate_waypoint, e);
                }
            }
        }
        
        let result: Vec<String> = connected_systems.into_iter().collect();
        println!("  ✅ Discovered {} connected systems", result.len());
        
        Ok(result)
    }
    
    /// Find jump gates in a system
    async fn find_jump_gates(&self, system_symbol: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let waypoints = self.client.get_system_waypoints(system_symbol, None).await?;
        let mut jump_gate_waypoints = Vec::new();
        
        for waypoint in waypoints {
            // Check for jump gate trait
            let has_jump_gate = waypoint.traits.iter().any(|t| 
                t.symbol == "JUMP_GATE" || 
                t.name.to_lowercase().contains("jump") ||
                waypoint.waypoint_type == "JUMP_GATE"
            );
            
            if has_jump_gate {
                println!("    🚪 Jump gate found: {} ({})", waypoint.symbol, waypoint.waypoint_type);
                jump_gate_waypoints.push(waypoint.symbol);
            }
        }
        
        Ok(jump_gate_waypoints)
    }

    pub async fn get_nearby_systems(&self, current_system: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        // Use jump gate discovery for real nearby systems
        self.discover_connected_systems(current_system).await
    }
}