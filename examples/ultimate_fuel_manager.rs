// ULTIMATE FUEL MANAGEMENT SYSTEM - Multi-hop routing and surveyor positioning!
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient};
use std::collections::{HashMap, VecDeque};
// Remove unused imports

#[derive(Debug, Clone)]
struct WaypointInfo {
    symbol: String,
    waypoint_type: String,
    has_marketplace: bool,
    has_shipyard: bool,
    fuel_available: bool,
    distance_from_target: f64,
}

#[derive(Debug, Clone)]
struct RouteStep {
    from: String,
    to: String,
    fuel_cost: i32,
    refuel_available: bool,
}

#[derive(Debug)]
struct FuelManager {
    system_waypoints: Vec<WaypointInfo>,
    fuel_stations: Vec<String>,
    route_cache: HashMap<String, Vec<RouteStep>>,
}

impl FuelManager {
    async fn new(client: &PriorityApiClient, system: &str) -> Result<Self, Box<dyn std::error::Error>> {
        println!("🔍 Scanning system {} for fuel infrastructure...", system);
        
        let waypoints = client.get_system_waypoints(system, None).await?;
        let mut system_waypoints = Vec::new();
        let mut fuel_stations = Vec::new();
        
        for waypoint in waypoints {
            let has_marketplace = waypoint.traits.iter().any(|t| t.symbol == "MARKETPLACE");
            let has_shipyard = waypoint.traits.iter().any(|t| t.symbol == "SHIPYARD");
            let fuel_available = has_marketplace || has_shipyard;
            
            if fuel_available {
                fuel_stations.push(waypoint.symbol.clone());
            }
            
            system_waypoints.push(WaypointInfo {
                symbol: waypoint.symbol,
                waypoint_type: waypoint.waypoint_type,
                has_marketplace,
                has_shipyard,
                fuel_available,
                distance_from_target: 0.0, // Will calculate later
            });
        }
        
        println!("✅ Found {} waypoints, {} fuel stations", system_waypoints.len(), fuel_stations.len());
        
        Ok(FuelManager {
            system_waypoints,
            fuel_stations,
            route_cache: HashMap::new(),
        })
    }
    
    fn calculate_fuel_cost(&self, from: &str, to: &str) -> i32 {
        // Simplified fuel calculation - in reality would use actual coordinates
        // For demo purposes, assume intra-system travel costs 30-50 fuel
        if from == to {
            0
        } else {
            40 // Average fuel cost for intra-system travel
        }
    }
    
    fn find_optimal_route(&mut self, ship_symbol: &str, current_location: &str, target: &str, current_fuel: i32, max_fuel: i32) -> Vec<RouteStep> {
        let cache_key = format!("{}_{}_{}_{}", current_location, target, current_fuel, max_fuel);
        
        if let Some(cached_route) = self.route_cache.get(&cache_key) {
            return cached_route.clone();
        }
        
        println!("🧭 Calculating optimal fuel route for {} from {} to {}", ship_symbol, current_location, target);
        println!("   Current fuel: {}/{}", current_fuel, max_fuel);
        
        // Direct route check
        let direct_fuel_cost = self.calculate_fuel_cost(current_location, target);
        if current_fuel >= direct_fuel_cost {
            println!("✅ Direct route possible! Fuel cost: {}", direct_fuel_cost);
            let route = vec![RouteStep {
                from: current_location.to_string(),
                to: target.to_string(),
                fuel_cost: direct_fuel_cost,
                refuel_available: false,
            }];
            self.route_cache.insert(cache_key, route.clone());
            return route;
        }
        
        println!("⚠️ Direct route not possible (need {} fuel, have {})", direct_fuel_cost, current_fuel);
        println!("🔍 Finding multi-hop route via fuel stations...");
        
        // Multi-hop routing using BFS
        let mut queue = VecDeque::new();
        let mut visited = HashMap::new();
        
        // Start with current location
        queue.push_back((current_location.to_string(), current_fuel, Vec::new()));
        visited.insert(current_location.to_string(), current_fuel);
        
        while let Some((current_pos, fuel_remaining, route_so_far)) = queue.pop_front() {
            // Check if we can reach target from current position
            let fuel_to_target = self.calculate_fuel_cost(&current_pos, target);
            if fuel_remaining >= fuel_to_target {
                let mut final_route = route_so_far.clone();
                final_route.push(RouteStep {
                    from: current_pos,
                    to: target.to_string(),
                    fuel_cost: fuel_to_target,
                    refuel_available: false,
                });
                
                println!("✅ Multi-hop route found! Steps: {}", final_route.len());
                for (i, step) in final_route.iter().enumerate() {
                    println!("   {}. {} → {} (fuel: {}, refuel: {})", 
                             i + 1, step.from, step.to, step.fuel_cost, step.refuel_available);
                }
                
                self.route_cache.insert(cache_key, final_route.clone());
                return final_route;
            }
            
            // Try all fuel stations as intermediate stops
            for fuel_station in &self.fuel_stations {
                if fuel_station == &current_pos {
                    continue; // Already at this station
                }
                
                let fuel_to_station = self.calculate_fuel_cost(&current_pos, fuel_station);
                if fuel_remaining >= fuel_to_station {
                    // Check if we've been to this station with better fuel
                    if let Some(&previous_fuel) = visited.get(fuel_station) {
                        if previous_fuel >= max_fuel {
                            continue; // Already visited with full fuel
                        }
                    }
                    
                    visited.insert(fuel_station.clone(), max_fuel);
                    
                    let mut new_route = route_so_far.clone();
                    new_route.push(RouteStep {
                        from: current_pos.clone(),
                        to: fuel_station.clone(),
                        fuel_cost: fuel_to_station,
                        refuel_available: true,
                    });
                    
                    queue.push_back((fuel_station.clone(), max_fuel, new_route));
                }
            }
        }
        
        println!("❌ No route found to target with available fuel stations");
        Vec::new()
    }
    
    async fn execute_route(&self, client: &PriorityApiClient, ship_symbol: &str, route: &[RouteStep]) -> Result<(), Box<dyn std::error::Error>> {
        println!("🚀 Executing fuel-optimized route for {}", ship_symbol);
        
        for (i, step) in route.iter().enumerate() {
            println!("\\n📍 ROUTE STEP {}/{}: {} → {}", i + 1, route.len(), step.from, step.to);
            
            // Ensure ship is in orbit before navigation
            let ship = client.get_ship(ship_symbol).await?;
            if ship.nav.status == "DOCKED" {
                println!("🛸 Undocking from {}", step.from);
                client.orbit_ship(ship_symbol).await?;
            }
            
            // Navigate to destination
            println!("🧭 Navigating to {} (fuel cost: {})", step.to, step.fuel_cost);
            match client.navigate_ship(ship_symbol, &step.to).await {
                Ok(nav_result) => {
                    println!("✅ Navigation successful!");
                    
                    // Wait for arrival if needed
                    if let Ok(arrival_time) = nav_result.nav.route.arrival.parse::<chrono::DateTime<chrono::Utc>>() {
                        let now = chrono::Utc::now();
                        let wait_seconds = (arrival_time - now).num_seconds().max(0) as u64;
                        if wait_seconds > 0 && wait_seconds < 300 { // Wait up to 5 minutes
                            println!("⏳ Waiting {} seconds for arrival...", wait_seconds);
                            tokio::time::sleep(tokio::time::Duration::from_secs(wait_seconds + 3)).await;
                        }
                    }
                    
                    // Refuel if this is a fuel station
                    if step.refuel_available {
                        println!("⛽ Refueling at {}...", step.to);
                        
                        // Dock for refuel
                        client.dock_ship(ship_symbol).await?;
                        
                        match client.refuel_ship(ship_symbol).await {
                            Ok(refuel_data) => {
                                println!("✅ Refueled! New fuel: {}/{}", 
                                         refuel_data.fuel.current, refuel_data.fuel.capacity);
                            }
                            Err(e) => {
                                println!("⚠️ Refuel failed: {}", e);
                            }
                        }
                        
                        // Return to orbit for next navigation
                        if i < route.len() - 1 { // Not the final destination
                            client.orbit_ship(ship_symbol).await?;
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Navigation failed: {}", e);
                    return Err(e);
                }
            }
        }
        
        println!("🎯 Route execution complete!");
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("🔥🔥🔥 ULTIMATE FUEL MANAGEMENT SYSTEM 🔥🔥🔥");
    println!("==============================================");
    println!("💡 Multi-hop routing and surveyor positioning!");
    
    let system = "X1-N5";
    let target_location = "X1-N5-BA5F"; // Our iron ore mining location
    
    // Initialize fuel manager
    let mut fuel_manager = FuelManager::new(&priority_client, system).await?;
    
    println!("\\n⛽ FUEL INFRASTRUCTURE ANALYSIS:");
    println!("   Fuel stations found: {}", fuel_manager.fuel_stations.len());
    for station in &fuel_manager.fuel_stations {
        println!("     🏪 {}", station);
    }
    
    // Get all ships and find ones that need positioning
    let ships = priority_client.get_ships().await?;
    let mut surveyors_to_position = Vec::new();
    let mut miners_to_position = Vec::new();
    
    println!("\\n🔍 FLEET ANALYSIS FOR POSITIONING:");
    for ship in &ships {
        let has_surveyor = ship.mounts.iter().any(|m| m.symbol.contains("SURVEYOR"));
        let has_mining_laser = ship.mounts.iter().any(|m| m.symbol.contains("MINING_LASER"));
        let at_target = ship.nav.waypoint_symbol == target_location;
        
        if has_surveyor && !at_target {
            surveyors_to_position.push((ship.symbol.clone(), ship.nav.waypoint_symbol.clone(), ship.fuel.current, ship.fuel.capacity));
            println!("   📊 Surveyor {} at {} (fuel: {}/{}) - NEEDS POSITIONING", 
                     ship.symbol, ship.nav.waypoint_symbol, ship.fuel.current, ship.fuel.capacity);
        } else if has_surveyor && at_target {
            println!("   📊 Surveyor {} at {} (fuel: {}/{}) - ✅ POSITIONED", 
                     ship.symbol, ship.nav.waypoint_symbol, ship.fuel.current, ship.fuel.capacity);
        }
        
        if has_mining_laser && !at_target && ship.cargo.units < ship.cargo.capacity {
            miners_to_position.push((ship.symbol.clone(), ship.nav.waypoint_symbol.clone(), ship.fuel.current, ship.fuel.capacity));
            println!("   ⛏️ Miner {} at {} (fuel: {}/{}) - could be positioned", 
                     ship.symbol, ship.nav.waypoint_symbol, ship.fuel.current, ship.fuel.capacity);
        }
    }
    
    println!("\\n🚀 POSITIONING STRATEGY:");
    println!("   Surveyors to position: {}", surveyors_to_position.len());
    println!("   Miners that could be positioned: {}", miners_to_position.len());
    
    // Position surveyors first (highest priority for multi-surveyor strategy)
    for (ship_symbol, current_location, current_fuel, max_fuel) in surveyors_to_position {
        println!("\\n🎯 POSITIONING SURVEYOR: {}", ship_symbol);
        println!("========================================");
        
        let route = fuel_manager.find_optimal_route(&ship_symbol, &current_location, target_location, current_fuel, max_fuel);
        
        if route.is_empty() {
            println!("❌ No route found for {} - may need manual intervention", ship_symbol);
            continue;
        }
        
        println!("📋 ROUTE PLAN:");
        let total_steps = route.len();
        let refuel_stops = route.iter().filter(|s| s.refuel_available).count();
        println!("   Total steps: {}", total_steps);
        println!("   Refuel stops: {}", refuel_stops);
        
        // Execute the route
        match fuel_manager.execute_route(&priority_client, &ship_symbol, &route).await {
            Ok(_) => {
                println!("🎉 {} successfully positioned at {}!", ship_symbol, target_location);
            }
            Err(e) => {
                println!("❌ Failed to position {}: {}", ship_symbol, e);
            }
        }
        
        // Brief pause between ship positioning
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
    
    // Position additional miners if we have surveyor capacity
    let positioned_surveyors = ships.iter()
        .filter(|s| s.mounts.iter().any(|m| m.symbol.contains("SURVEYOR")) && s.nav.waypoint_symbol == target_location)
        .count();
    
    let max_miners_to_position = positioned_surveyors * 4; // 4 miners per surveyor
    let miners_to_position_count = miners_to_position.len().min(max_miners_to_position);
    
    if miners_to_position_count > 0 {
        println!("\\n⛏️ POSITIONING {} ADDITIONAL MINERS:", miners_to_position_count);
        
        for (ship_symbol, current_location, current_fuel, max_fuel) in miners_to_position.into_iter().take(miners_to_position_count) {
            println!("\\n🎯 POSITIONING MINER: {}", ship_symbol);
            
            let route = fuel_manager.find_optimal_route(&ship_symbol, &current_location, target_location, current_fuel, max_fuel);
            
            if !route.is_empty() {
                match fuel_manager.execute_route(&priority_client, &ship_symbol, &route).await {
                    Ok(_) => println!("✅ {} positioned at mining location!", ship_symbol),
                    Err(e) => println!("❌ Failed to position {}: {}", ship_symbol, e),
                }
            }
            
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        }
    }
    
    // FINAL STATUS
    println!("\\n🏁 FUEL MANAGEMENT MISSION COMPLETE!");
    println!("=====================================");
    
    let final_ships = priority_client.get_ships().await?;
    let surveyors_at_target = final_ships.iter()
        .filter(|s| s.mounts.iter().any(|m| m.symbol.contains("SURVEYOR")) && s.nav.waypoint_symbol == target_location)
        .count();
    let miners_at_target = final_ships.iter()
        .filter(|s| s.mounts.iter().any(|m| m.symbol.contains("MINING_LASER")) && s.nav.waypoint_symbol == target_location)
        .count();
    
    println!("📊 FINAL FLEET STATUS AT {}:", target_location);
    println!("   Surveyors positioned: {}", surveyors_at_target);
    println!("   Miners positioned: {}", miners_at_target);
    
    if surveyors_at_target > 1 {
        println!("\\n🎉🎉🎉 MULTI-SURVEYOR CAPABILITY ACHIEVED! 🎉🎉🎉");
        println!("⚡ Estimated throughput increase: {}x", surveyors_at_target.min(3));
        println!("🚀 Ready for maximum efficiency mining operations!");
    } else if surveyors_at_target == 1 {
        println!("\\n✅ Single surveyor operations maintained");
        println!("💡 Position more surveyors for multi-surveyor blitz!");
    }
    
    println!("\\n🎯 NEXT STEPS:");
    println!("   1. Run multi-surveyor blitz with {} surveyors", surveyors_at_target);
    println!("   2. Monitor fuel levels during extended operations");
    println!("   3. Scale up to maximum iron ore extraction rate!");
    
    Ok(())
}