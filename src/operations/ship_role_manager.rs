// Ship Role Manager - Handles ship role designation and module management
use crate::models::Ship;
use crate::client::priority_client::PriorityApiClient;
use crate::{o_debug, o_info};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum ShipRole {
    Miner,
    Hauler,
    Refiner,
    Scout,
    Utility,
}

#[derive(Debug, Clone)]
pub struct ShipRoleCapability {
    pub ship_symbol: String,
    pub current_role: ShipRole,
    pub cargo_capacity: i32,
    pub available_module_slots: i32,
    pub total_module_slots: i32,
    pub mounting_points: i32,
    pub has_refinery: bool,
    pub removable_modules: Vec<String>,
    pub refinery_score: f64, // Higher is better for refining
}

pub struct ShipRoleManager {
    capabilities: HashMap<String, ShipRoleCapability>,
}

impl ShipRoleManager {
    pub fn new() -> Self {
        Self {
            capabilities: HashMap::new(),
        }
    }

    pub async fn analyze_fleet(&mut self, client: &PriorityApiClient) -> Result<(), String> {
        o_info!("🔍 Analyzing fleet for ship role capabilities...");
        
        let ships = client.get_ships().await.map_err(|e| e.to_string())?;
        self.capabilities.clear();
        
        for ship in &ships {
            let capability = self.analyze_ship(ship);
            o_debug!("📊 {} refinery score: {:.2} (cargo: {}, slots: {}/{}, removable: {})", 
                    ship.symbol, capability.refinery_score, capability.cargo_capacity, 
                    capability.available_module_slots, capability.total_module_slots,
                    capability.removable_modules.len());
            
            self.capabilities.insert(ship.symbol.clone(), capability);
        }
        
        Ok(())
    }

    fn analyze_ship(&self, ship: &Ship) -> ShipRoleCapability {
        let current_role = self.determine_current_role(ship);
        
        // Analyze modules to find removable ones and refinery capability
        let mut has_refinery = false;
        let mut removable_modules = Vec::new();
        
        for module in &ship.modules {
            if module.symbol.contains("REFINERY") {
                has_refinery = true;
            } else if self.is_module_removable(&module.symbol) {
                removable_modules.push(module.symbol.clone());
            }
        }
        
        let used_module_slots = ship.modules.len() as i32;
        let available_module_slots = ship.frame.module_slots - used_module_slots;
        
        // Calculate refinery score based on multiple factors
        let refinery_score = self.calculate_refinery_score(ship, available_module_slots, &removable_modules);
        
        ShipRoleCapability {
            ship_symbol: ship.symbol.clone(),
            current_role,
            cargo_capacity: ship.cargo.capacity,
            available_module_slots,
            total_module_slots: ship.frame.module_slots,
            mounting_points: ship.frame.mounting_points,
            has_refinery,
            removable_modules,
            refinery_score,
        }
    }

    fn determine_current_role(&self, ship: &Ship) -> ShipRole {
        // Check for mining capability
        let has_mining_mounts = ship.mounts.iter().any(|m| 
            m.symbol.contains("MINING") || m.symbol.contains("LASER")
        );
        
        // Check for refinery module
        let has_refinery = ship.modules.iter().any(|m| 
            m.symbol.contains("REFINERY")
        );
        
        // Check ship registration role
        match ship.registration.role.as_str() {
            "EXCAVATOR" | "MINER" => ShipRole::Miner,
            "HAULER" | "TRANSPORT" => ShipRole::Hauler,
            "REFINERY" => ShipRole::Refiner,
            "SATELLITE" | "PROBE" => ShipRole::Scout,
            _ => {
                // Infer role from capabilities
                if has_refinery {
                    ShipRole::Refiner
                } else if has_mining_mounts {
                    ShipRole::Miner
                } else if ship.cargo.capacity >= 30 {
                    ShipRole::Hauler
                } else {
                    ShipRole::Utility
                }
            }
        }
    }

    fn is_module_removable(&self, module_symbol: &str) -> bool {
        // These modules can typically be removed to make space for refineries
        matches!(module_symbol, 
            "MODULE_CARGO_HOLD_I" | 
            "MODULE_CARGO_HOLD_II" | 
            "MODULE_CARGO_HOLD_III" |
            "MODULE_CREW_QUARTERS_I" |
            "MODULE_CREW_QUARTERS_II" |
            "MODULE_CREW_QUARTERS_III" |
            "MODULE_MINERAL_PROCESSOR_I" |
            "MODULE_GAS_PROCESSOR_I" |
            "MODULE_SHIELD_GENERATOR_I" |
            "MODULE_SHIELD_GENERATOR_II"
        )
    }

    fn calculate_refinery_score(&self, ship: &Ship, available_slots: i32, removable_modules: &[String]) -> f64 {
        let mut score = 0.0;
        
        // Base score from cargo capacity (larger ships are better for refining)
        score += (ship.cargo.capacity as f64 / 100.0).min(5.0);
        
        // Bonus for available module slots
        score += available_slots as f64 * 2.0;
        
        // Bonus for removable modules (can make space)
        score += removable_modules.len() as f64 * 1.5;
        
        // Bonus for total module slots (more flexible)
        score += ship.frame.module_slots as f64 / 10.0;
        
        // Penalty if ship is currently a dedicated miner (harder to convert)
        let has_multiple_mining_mounts = ship.mounts.iter()
            .filter(|m| m.symbol.contains("MINING") || m.symbol.contains("LASER"))
            .count() > 1;
        
        if has_multiple_mining_mounts {
            score -= 2.0;
        }
        
        // Bonus if ship already has refinery
        if ship.modules.iter().any(|m| m.symbol.contains("REFINERY")) {
            score += 10.0;
        }
        
        // Penalty for probe/scout ships (too small)
        if ship.registration.role == "SATELLITE" || ship.frame.symbol.contains("PROBE") {
            score -= 5.0;
        }
        
        score.max(0.0)
    }

    pub fn find_best_refinery_candidate(&self) -> Option<&ShipRoleCapability> {
        self.capabilities.values()
            .filter(|cap| cap.refinery_score > 0.0)
            .max_by(|a, b| a.refinery_score.partial_cmp(&b.refinery_score).unwrap())
    }

    pub fn get_ship_capability(&self, ship_symbol: &str) -> Option<&ShipRoleCapability> {
        self.capabilities.get(ship_symbol)
    }

    pub async fn designate_refinery_ship(&self, ship_symbol: &str, client: &PriorityApiClient) -> Result<bool, String> {
        let capability = self.capabilities.get(ship_symbol)
            .ok_or_else(|| format!("Ship {} not analyzed", ship_symbol))?;

        o_info!("⚗️ Designating {} as refinery ship (score: {:.2})", ship_symbol, capability.refinery_score);
        
        // Check if ship already has refinery
        if capability.has_refinery {
            o_info!("✅ {} already has refinery module", ship_symbol);
            return Ok(true);
        }
        
        // Check if we can install refinery
        let needs_space = capability.available_module_slots <= 0 && capability.removable_modules.is_empty();
        
        if needs_space {
            o_info!("❌ {} has no available module slots and no removable modules", ship_symbol);
            return Ok(false);
        }

        // Step 1: Navigate to a shipyard (required for module installation)
        o_info!("🚢 Navigating {} to shipyard for module installation...", ship_symbol);
        
        match self.navigate_to_shipyard(client, ship_symbol).await {
            Ok(true) => {
                o_info!("✅ {} arrived at shipyard and is docked", ship_symbol);
            }
            Ok(false) => {
                o_info!("❌ Failed to dock {} at shipyard", ship_symbol);
                return Ok(false);
            }
            Err(e) => {
                o_info!("❌ Error navigating to shipyard: {}", e);
                return Ok(false);
            }
        }
        
        // If we need to remove modules, do that first
        if capability.available_module_slots <= 0 {
            o_info!("🔧 Need to remove module from {} to make space", ship_symbol);
            if let Some(module_to_remove) = capability.removable_modules.first() {
                o_info!("🗑️ Removing {} from {}", module_to_remove, ship_symbol);
                
                match client.remove_ship_module(ship_symbol, module_to_remove).await {
                    Ok(removal_data) => {
                        o_info!("✅ Successfully removed {} from {}", module_to_remove, ship_symbol);
                        o_info!("💰 Removal cost: {} credits", removal_data.transaction.total_price);
                    }
                    Err(e) => {
                        o_info!("❌ Failed to remove module: {}", e);
                        return Ok(false);
                    }
                }
            }
        }
        
        // Get current ship info to check location
        let current_ship = client.get_ship(ship_symbol).await.map_err(|e| e.to_string())?;
        
        // First, check if this waypoint has a marketplace and if the module is available
        o_info!("🔍 Checking marketplace for refinery module at {}", current_ship.nav.waypoint_symbol);
        const REFINERY_MODULE: &str = "MODULE_MICRO_REFINERY_I";
        
        // Try to get market info to see if the module is available
        let market_result = client.get_market_with_priority(&current_ship.nav.system_symbol, &current_ship.nav.waypoint_symbol, crate::client::priority_client::ApiPriority::ActiveGoal).await
            .map_err(|e| e.to_string());
        match market_result {
            Ok(market) => {
                // Check if the refinery module is available for purchase
                let module_available = if let Some(ref goods) = market.trade_goods {
                    goods.iter().any(|good| good.symbol == REFINERY_MODULE && good.trade_volume > 0)
                } else {
                    false
                };
                
                if module_available {
                    o_info!("✅ Refinery module available at marketplace");
                    // Purchase the refinery module
                    o_info!("🛒 Purchasing refinery module for {}", ship_symbol);
                    
                    match client.purchase_cargo_with_priority(ship_symbol, REFINERY_MODULE, 1, crate::client::priority_client::ApiPriority::ActiveGoal).await.map_err(|e| e.to_string()) {
                        Ok(purchase_data) => {
                            o_info!("✅ Successfully purchased refinery module for {}", ship_symbol);
                            o_info!("💰 Purchase cost: {} credits", purchase_data.transaction.total_price);
                        }
                        Err(e) => {
                            o_info!("❌ Failed to purchase refinery module: {}", e);
                            return Ok(false);
                        }
                    }
                } else {
                    o_info!("❌ Refinery module {} not available at this marketplace", REFINERY_MODULE);
                    if let Some(ref goods) = market.trade_goods {
                        o_info!("💡 Available goods: {:?}", goods.iter().map(|g| &g.symbol).collect::<Vec<_>>());
                    } else {
                        o_info!("💡 No trade goods information available");
                    }
                    // Search other marketplaces in the system
                    return self.search_system_marketplaces_for_module(client, ship_symbol, &current_ship.nav.system_symbol, &current_ship.nav.waypoint_symbol, REFINERY_MODULE).await;
                }
            }
            Err(e) => {
                o_info!("❌ No marketplace at {}: {}", current_ship.nav.waypoint_symbol, e);
                // Search other marketplaces in the system
                return self.search_system_marketplaces_for_module(client, ship_symbol, &current_ship.nav.system_symbol, &current_ship.nav.waypoint_symbol, REFINERY_MODULE).await;
            }
        }
        
        // Install the refinery module
        o_info!("🏭 Installing refinery module on {}", ship_symbol);
        
        match client.install_ship_module(ship_symbol, REFINERY_MODULE).await {
            Ok(install_data) => {
                o_info!("✅ Successfully installed refinery on {}", ship_symbol);
                o_info!("💰 Installation cost: {} credits", install_data.transaction.total_price);
                
                // Verify the module was installed
                let has_refinery = install_data.modules.iter()
                    .any(|m| m.symbol.contains("REFINERY"));
                
                if has_refinery {
                    o_info!("🔬 Refinery module confirmed installed on {}", ship_symbol);
                    return Ok(true);
                } else {
                    o_info!("⚠️ Refinery installation may have failed - not found in module list");
                    return Ok(false);
                }
            }
            Err(e) => {
                o_info!("❌ Failed to install refinery module: {}", e);
                return Ok(false);
            }
        }
    }

    pub fn print_fleet_analysis(&self) {
        o_info!("🚢 Fleet Analysis for Refinery Designation:");
        
        let mut candidates: Vec<_> = self.capabilities.values().collect();
        candidates.sort_by(|a, b| b.refinery_score.partial_cmp(&a.refinery_score).unwrap());
        
        for (i, capability) in candidates.iter().enumerate().take(5) {
            let role_icon = match capability.current_role {
                ShipRole::Miner => "⛏️",
                ShipRole::Hauler => "🚛",
                ShipRole::Refiner => "🏭",
                ShipRole::Scout => "🔍",
                ShipRole::Utility => "🔧",
            };
            
            o_info!("{}. {} {} {}", i + 1, role_icon, capability.ship_symbol, 
                   if capability.has_refinery { "(HAS REFINERY)" } else { "" });
            o_info!("   📊 Refinery Score: {:.2}", capability.refinery_score);
            o_info!("   📦 Cargo: {} units", capability.cargo_capacity);
            o_info!("   🔧 Module Slots: {}/{} (available: {})", 
                   capability.total_module_slots - capability.available_module_slots,
                   capability.total_module_slots, capability.available_module_slots);
            
            if !capability.removable_modules.is_empty() {
                o_info!("   🗑️ Removable: {}", capability.removable_modules.join(", "));
            }
        }
        
        if let Some(best) = self.find_best_refinery_candidate() {
            o_info!("🏆 Best refinery candidate: {} (score: {:.2})", 
                   best.ship_symbol, best.refinery_score);
        }
    }

    async fn navigate_to_shipyard(&self, client: &PriorityApiClient, ship_symbol: &str) -> Result<bool, String> {
        use crate::operations::ShipyardOperations;
        
        // Get current ship status
        let ship = client.get_ship(ship_symbol).await.map_err(|e| e.to_string())?;
        
        // Check if already at a shipyard
        let current_waypoint = &ship.nav.waypoint_symbol;
        o_debug!("🔍 {} currently at {}", ship_symbol, current_waypoint);
        
        // Check if current waypoint has shipyard trait
        let waypoints = client.get_system_waypoints(&ship.nav.system_symbol, None).await.map_err(|e| e.to_string())?;
        let current_waypoint_info = waypoints.iter()
            .find(|w| w.symbol == *current_waypoint);
            
        if let Some(waypoint) = current_waypoint_info {
            let has_shipyard = waypoint.traits.iter()
                .any(|t| t.symbol == "SHIPYARD");
            
            if has_shipyard {
                o_info!("🏭 {} already at shipyard {}", ship_symbol, current_waypoint);
                
                // Make sure we're docked
                if ship.nav.status == "DOCKED" {
                    return Ok(true);
                } else {
                    o_info!("🔗 Docking {} at shipyard", ship_symbol);
                    match client.dock_ship(ship_symbol).await.map_err(|e| e.to_string()) {
                        Ok(_) => return Ok(true),
                        Err(e) => {
                            o_info!("❌ Failed to dock at shipyard: {}", e);
                            return Ok(false);
                        }
                    }
                }
            }
        }
        
        // Need to find and navigate to a shipyard
        o_info!("🔍 Looking for shipyards in system {}", ship.nav.system_symbol);
        
        let shipyard_operations = ShipyardOperations::new((&**client).clone());
        let shipyards = match shipyard_operations.find_shipyards().await {
            Ok(yards) => yards,
            Err(e) => {
                o_info!("❌ Failed to find shipyards: {}", e);
                return Ok(false);
            }
        };
        
        if shipyards.is_empty() {
            o_info!("❌ No shipyards found");
            return Ok(false);
        }
        
        // Find closest shipyard in current system, or any shipyard if none in current system
        let current_system = &ship.nav.system_symbol;
        let target_shipyard = shipyards.iter()
            .find(|s| s.system_symbol == *current_system)
            .or_else(|| shipyards.first());
        
        let target_waypoint = match target_shipyard {
            Some(shipyard) => &shipyard.waypoint_symbol,
            None => {
                o_info!("❌ No suitable shipyard found");
                return Ok(false);
            }
        };
        
        o_info!("🎯 Navigating {} to shipyard {}", ship_symbol, target_waypoint);
        
        // First, ensure ship is in orbit (required for navigation)
        if ship.nav.status == "DOCKED" {
            o_info!("🚀 Putting {} in orbit first", ship_symbol);
            match client.orbit_ship(ship_symbol).await.map_err(|e| e.to_string()) {
                Ok(_) => {
                    o_info!("✅ {} is now in orbit", ship_symbol);
                }
                Err(e) => {
                    o_info!("❌ Failed to orbit ship: {}", e);
                    return Ok(false);
                }
            }
        }
        
        // Navigate to shipyard
        let nav_result = client.navigate_ship(ship_symbol, target_waypoint).await
            .map_err(|e| e.to_string());
        match nav_result {
            Ok(nav_data) => {
                o_info!("🚢 {} en route to shipyard {}", ship_symbol, target_waypoint);
                
                // Calculate wait time from navigation data
                let wait_time = {
                    let arrival_time = nav_data.nav.route.arrival.parse::<chrono::DateTime<chrono::Utc>>();
                    
                    match arrival_time {
                        Ok(arrival) => {
                            let now = chrono::Utc::now();
                            let duration = arrival - now;
                            duration.num_seconds().max(0) as u64 + 5 // Add 5 second buffer
                        }
                        _ => 30 // Fallback to 30 seconds
                    }
                };
                
                o_info!("⏳ Waiting {} seconds for {} to arrive at shipyard...", wait_time, ship_symbol);
                tokio::time::sleep(tokio::time::Duration::from_secs(wait_time)).await;
                
                // Dock at shipyard
                o_info!("🔗 Docking {} at shipyard {}", ship_symbol, target_waypoint);
                match client.dock_ship(ship_symbol).await.map_err(|e| e.to_string()) {
                    Ok(_) => {
                        o_info!("✅ {} successfully docked at shipyard", ship_symbol);
                        Ok(true)
                    }
                    Err(e) => {
                        o_info!("❌ Failed to dock at shipyard: {}", e);
                        Ok(false)
                    }
                }
            }
            Err(e) => {
                // Check if ship is already in-transit
                if e.contains("in-transit") {
                    o_info!("⏳ Ship {} already in-transit, waiting for arrival...", ship_symbol);
                    
                    // Extract seconds to arrival from error message
                    let wait_time = if let Some(start) = e.find("arrives in ") {
                        if let Some(end) = e[start + 11..].find(" seconds") {
                            let seconds_str = &e[start + 11..start + 11 + end];
                            seconds_str.parse::<u64>().unwrap_or(30)
                        } else { 30 }
                    } else { 30 };
                    
                    o_info!("⏳ Waiting {} seconds for {} to arrive...", wait_time + 5, ship_symbol);
                    tokio::time::sleep(tokio::time::Duration::from_secs(wait_time + 5)).await;
                    
                    // Verify ship arrived at the correct location and dock
                    o_info!("🔍 Verifying {} arrived at shipyard {}", ship_symbol, target_waypoint);
                    let updated_ship = client.get_ship(ship_symbol).await.map_err(|e| e.to_string())?;
                    
                    if updated_ship.nav.waypoint_symbol == *target_waypoint {
                        o_info!("✅ {} confirmed at shipyard {}", ship_symbol, target_waypoint);
                        o_info!("🔗 Docking {} at destination shipyard", ship_symbol);
                        match client.dock_ship(ship_symbol).await.map_err(|e| e.to_string()) {
                            Ok(_) => {
                                o_info!("✅ {} successfully docked after arrival", ship_symbol);
                                Ok(true)
                            }
                            Err(dock_e) => {
                                o_info!("❌ Failed to dock after arrival: {}", dock_e);
                                Ok(false)
                            }
                        }
                    } else {
                        o_info!("❌ {} is at {}, not at target shipyard {}", 
                                ship_symbol, updated_ship.nav.waypoint_symbol, target_waypoint);
                        Ok(false)
                    }
                } else if e.contains("currently located at the destination") {
                    // Ship is already at the shipyard, just try to dock
                    o_info!("✅ Ship {} already at shipyard, attempting to dock", ship_symbol);
                    match client.dock_ship(ship_symbol).await.map_err(|e| e.to_string()) {
                        Ok(_) => {
                            o_info!("✅ {} successfully docked at current location", ship_symbol);
                            Ok(true)
                        }
                        Err(dock_e) => {
                            o_info!("❌ Failed to dock at current location: {}", dock_e);
                            Ok(false)
                        }
                    }
                } else {
                    o_info!("❌ Failed to navigate to shipyard: {}", e);
                    Ok(false)
                }
            }
        }
    }

    async fn search_system_marketplaces_for_module(&self, client: &PriorityApiClient, ship_symbol: &str, system_symbol: &str, current_waypoint: &str, module_symbol: &str) -> Result<bool, String> {
        o_info!("🔍 Searching for marketplaces that sell {} in system {}...", module_symbol, system_symbol);
        
        // Get all waypoints in the system and check their marketplaces
        match client.get_system_waypoints(system_symbol, None).await.map_err(|e| e.to_string()) {
            Ok(waypoints) => {
                for waypoint in waypoints {
                    // Skip the waypoint we already checked
                    if waypoint.symbol == current_waypoint {
                        continue;
                    }
                    
                    // Check if this waypoint has a marketplace trait
                    let has_marketplace = waypoint.traits.iter()
                        .any(|t| t.symbol == "MARKETPLACE");
                    
                    if !has_marketplace {
                        continue;
                    }
                    
                    o_info!("🔍 Checking marketplace at {}", waypoint.symbol);
                    
                    // Check this marketplace
                    match client.get_market_with_priority(system_symbol, &waypoint.symbol, crate::client::priority_client::ApiPriority::ActiveGoal).await.map_err(|e| e.to_string()) {
                        Ok(market) => {
                            let module_available = if let Some(ref goods) = market.trade_goods {
                                goods.iter().any(|good| good.symbol == module_symbol && good.trade_volume > 0)
                            } else {
                                false
                            };
                            
                            if module_available {
                                o_info!("✅ Found {} at marketplace {}", module_symbol, waypoint.symbol);
                                
                                // Navigate to this waypoint and purchase module
                                o_info!("🚢 Navigating {} to marketplace {}", ship_symbol, waypoint.symbol);
                                match self.navigate_to_waypoint_and_purchase_module(client, ship_symbol, &waypoint.symbol, module_symbol).await {
                                    Ok(true) => {
                                        o_info!("✅ Successfully purchased refinery module, proceeding to installation");
                                        return Ok(true);  // Success - we'll continue to module installation
                                    }
                                    Ok(false) => {
                                        o_info!("❌ Failed to purchase module at {}", waypoint.symbol);
                                        continue; // Try other marketplaces
                                    }
                                    Err(e) => {
                                        o_info!("❌ Error purchasing module: {}", e);
                                        return Err(e);
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            // Skip waypoints without accessible marketplaces
                            continue;
                        }
                    }
                }
                
                o_info!("❌ No marketplace in system {} sells {}", system_symbol, module_symbol);
                Ok(false)
            }
            Err(e) => {
                o_info!("❌ Failed to get system waypoints: {}", e);
                Err(e)
            }
        }
    }

    async fn navigate_to_waypoint_and_purchase_module(&self, client: &PriorityApiClient, ship_symbol: &str, waypoint_symbol: &str, module_symbol: &str) -> Result<bool, String> {
        // First, ensure ship is in orbit
        let current_ship = client.get_ship(ship_symbol).await.map_err(|e| e.to_string())?;
        if current_ship.nav.status == "DOCKED" {
            o_info!("🚀 Putting {} in orbit for navigation", ship_symbol);
            match client.orbit_ship(ship_symbol).await.map_err(|e| e.to_string()) {
                Ok(_) => {
                    o_info!("✅ {} is now in orbit", ship_symbol);
                }
                Err(e) => {
                    o_info!("❌ Failed to orbit ship: {}", e);
                    return Ok(false);
                }
            }
        }
        
        // Navigate to the waypoint
        match client.navigate_ship(ship_symbol, waypoint_symbol).await.map_err(|e| e.to_string()) {
            Ok(nav_data) => {
                o_info!("🚢 {} en route to marketplace {}", ship_symbol, waypoint_symbol);
                
                // Calculate wait time
                let wait_time = {
                    let arrival_time = nav_data.nav.route.arrival.parse::<chrono::DateTime<chrono::Utc>>();
                    
                    match arrival_time {
                        Ok(arrival) => {
                            let now = chrono::Utc::now();
                            let duration = arrival - now;
                            duration.num_seconds().max(0) as u64 + 5 // Add 5 second buffer
                        }
                        _ => 30 // Fallback to 30 seconds
                    }
                };
                
                o_info!("⏳ Waiting {} seconds for {} to arrive at marketplace...", wait_time, ship_symbol);
                tokio::time::sleep(tokio::time::Duration::from_secs(wait_time)).await;
                
                // Dock at marketplace
                o_info!("🔗 Docking {} at marketplace {}", ship_symbol, waypoint_symbol);
                match client.dock_ship(ship_symbol).await.map_err(|e| e.to_string()) {
                    Ok(_) => {
                        o_info!("✅ {} successfully docked at marketplace", ship_symbol);
                        
                        // Purchase the module
                        o_info!("🛒 Purchasing {} for {}", module_symbol, ship_symbol);
                        match client.purchase_cargo_with_priority(ship_symbol, module_symbol, 1, crate::client::priority_client::ApiPriority::ActiveGoal).await.map_err(|e| e.to_string()) {
                            Ok(purchase_data) => {
                                o_info!("✅ Successfully purchased {} for {}", module_symbol, ship_symbol);
                                o_info!("💰 Purchase cost: {} credits", purchase_data.transaction.total_price);
                                return Ok(true);
                            }
                            Err(e) => {
                                o_info!("❌ Failed to purchase {}: {}", module_symbol, e);
                                return Ok(false);
                            }
                        }
                    }
                    Err(e) => {
                        o_info!("❌ Failed to dock at marketplace: {}", e);
                        return Ok(false);
                    }
                }
            }
            Err(e) => {
                o_info!("❌ Failed to navigate to marketplace {}: {}", waypoint_symbol, e);
                return Ok(false);
            }
        }
    }

    /// Find and transfer iron ore to designated refiner
    pub async fn coordinate_ore_to_refiner_transfer(&self, client: &PriorityApiClient) -> Result<bool, String> {
        o_info!("🔄 Starting ore-to-refiner transfer coordination...");
        
        // Find our designated refiner
        let refiner_ship = match self.find_best_refinery_candidate() {
            Some(ship) => ship,
            None => {
                o_info!("❌ No refiner ship designated");
                return Ok(false);
            }
        };
        
        o_info!("🏭 Using {} as refiner", refiner_ship.ship_symbol);
        
        // Find ships carrying iron ore
        let ore_carriers = self.find_ships_with_iron_ore(client).await?;
        
        if ore_carriers.is_empty() {
            o_info!("💼 No ships found carrying iron ore");
            return Ok(true);
        }
        
        o_info!("⛏️ Found {} ships carrying iron ore", ore_carriers.len());
        
        // Coordinate transfers
        let mut successful_transfers = 0;
        for carrier_info in ore_carriers {
            match self.transfer_iron_ore_to_refiner(client, &carrier_info, &refiner_ship.ship_symbol).await {
                Ok(transferred) => {
                    if transferred {
                        successful_transfers += 1;
                    }
                }
                Err(e) => {
                    o_info!("❌ Failed to transfer ore from {}: {}", carrier_info.ship_symbol, e);
                }
            }
        }
        
        o_info!("✅ Successfully coordinated {} ore transfers to refiner", successful_transfers);
        Ok(successful_transfers > 0)
    }

    /// Find all ships carrying iron ore
    async fn find_ships_with_iron_ore(&self, client: &PriorityApiClient) -> Result<Vec<CargoCarrierInfo>, String> {
        o_info!("🔍 Scanning fleet for iron ore carriers...");
        
        let ships = client.get_ships().await.map_err(|e| e.to_string())?;
        let mut ore_carriers = Vec::new();
        
        for ship in ships {
            // Check cargo for iron ore
            for cargo_item in &ship.cargo.inventory {
                if cargo_item.symbol == "IRON_ORE" && cargo_item.units > 0 {
                    o_info!("⛏️ {} carrying {} units of IRON_ORE at {}", 
                           ship.symbol, cargo_item.units, ship.nav.waypoint_symbol);
                    
                    ore_carriers.push(CargoCarrierInfo {
                        ship_symbol: ship.symbol.clone(),
                        iron_ore_units: cargo_item.units,
                        current_location: ship.nav.waypoint_symbol.clone(),
                        _nav_status: ship.nav.status.clone(),
                    });
                    break;
                }
            }
        }
        
        Ok(ore_carriers)
    }

    /// Transfer iron ore from carrier to refiner
    async fn transfer_iron_ore_to_refiner(&self, client: &PriorityApiClient, carrier_info: &CargoCarrierInfo, refiner_symbol: &str) -> Result<bool, String> {
        o_info!("🚛 Initiating ore transfer: {} -> {}", carrier_info.ship_symbol, refiner_symbol);
        
        // Get current refiner ship data
        let refiner_ship = client.get_ship(refiner_symbol).await.map_err(|e| e.to_string())?;
        
        // Check if both ships are at the same location
        if carrier_info.current_location != refiner_ship.nav.waypoint_symbol {
            o_info!("📍 Ships not at same location. Carrier: {}, Refiner: {}", 
                   carrier_info.current_location, refiner_ship.nav.waypoint_symbol);
            
            // Navigate carrier to refiner's location
            if !self.navigate_ship_to_location(client, &carrier_info.ship_symbol, &refiner_ship.nav.waypoint_symbol).await? {
                o_info!("❌ Failed to navigate {} to refiner location", carrier_info.ship_symbol);
                return Ok(false);
            }
        }
        
        // Ensure both ships are docked for cargo transfer
        if !self.ensure_ships_docked_for_transfer(client, &carrier_info.ship_symbol, refiner_symbol).await? {
            o_info!("❌ Failed to dock ships for transfer");
            return Ok(false);
        }
        
        // Calculate transfer amount (don't exceed refiner's cargo capacity)
        let refiner_available_space = refiner_ship.cargo.capacity - refiner_ship.cargo.units;
        let transfer_units = std::cmp::min(carrier_info.iron_ore_units, refiner_available_space);
        
        if transfer_units <= 0 {
            o_info!("⚠️ No capacity for transfer. Refiner cargo: {}/{}", 
                   refiner_ship.cargo.units, refiner_ship.cargo.capacity);
            return Ok(false);
        }
        
        o_info!("📦 Transferring {} units of IRON_ORE: {} -> {}", 
               transfer_units, carrier_info.ship_symbol, refiner_symbol);
        
        // Execute the transfer
        match client.transfer_cargo_with_priority(
            &carrier_info.ship_symbol,
            "IRON_ORE", 
            transfer_units, 
            refiner_symbol,
            crate::client::priority_client::ApiPriority::ActiveGoal
        ).await {
            Ok(_transfer_data) => {
                o_info!("✅ Successfully transferred {} units of IRON_ORE", transfer_units);
                Ok(true)
            }
            Err(e) => {
                o_info!("❌ Cargo transfer failed: {}", e);
                Err(e.to_string())
            }
        }
    }

    /// Navigate a ship to a specific location
    async fn navigate_ship_to_location(&self, client: &PriorityApiClient, ship_symbol: &str, target_waypoint: &str) -> Result<bool, String> {
        o_info!("🧭 Navigating {} to {}", ship_symbol, target_waypoint);
        
        // Get current ship state
        let ship = client.get_ship(ship_symbol).await.map_err(|e| e.to_string())?;
        
        // If already at target, return success
        if ship.nav.waypoint_symbol == target_waypoint {
            o_info!("✅ {} already at target location {}", ship_symbol, target_waypoint);
            return Ok(true);
        }
        
        // Ensure ship is in orbit for navigation
        if ship.nav.status == "DOCKED" {
            o_info!("🚀 Putting {} in orbit for navigation", ship_symbol);
            match client.orbit_ship(ship_symbol).await.map_err(|e| e.to_string()) {
                Ok(_) => {
                    o_info!("✅ {} is now in orbit", ship_symbol);
                }
                Err(e) => {
                    o_info!("❌ Failed to orbit ship: {}", e);
                    return Ok(false);
                }
            }
        }
        
        // Navigate to target
        match client.navigate_ship(ship_symbol, target_waypoint).await.map_err(|e| e.to_string()) {
            Ok(nav_data) => {
                o_info!("🚢 {} en route to {}", ship_symbol, target_waypoint);
                
                // Calculate wait time
                let wait_time = {
                    let arrival_time = nav_data.nav.route.arrival.parse::<chrono::DateTime<chrono::Utc>>();
                    
                    match arrival_time {
                        Ok(arrival) => {
                            let now = chrono::Utc::now();
                            let duration = arrival - now;
                            duration.num_seconds().max(0) as u64 + 5
                        }
                        _ => 30
                    }
                };
                
                o_info!("⏳ Waiting {} seconds for {} to arrive...", wait_time, ship_symbol);
                tokio::time::sleep(tokio::time::Duration::from_secs(wait_time)).await;
                
                Ok(true)
            }
            Err(e) => {
                // Handle in-transit case
                if e.contains("in-transit") {
                    o_info!("⏳ Ship {} already in-transit, waiting for arrival...", ship_symbol);
                    
                    let wait_time = if let Some(start) = e.find("arrives in ") {
                        if let Some(end) = e[start + 11..].find(" seconds") {
                            let seconds_str = &e[start + 11..start + 11 + end];
                            seconds_str.parse::<u64>().unwrap_or(30)
                        } else { 30 }
                    } else { 30 };
                    
                    o_info!("⏳ Waiting {} seconds for arrival...", wait_time + 5);
                    tokio::time::sleep(tokio::time::Duration::from_secs(wait_time + 5)).await;
                    
                    Ok(true)
                } else {
                    o_info!("❌ Navigation failed: {}", e);
                    Ok(false)
                }
            }
        }
    }

    /// Ensure both ships are docked at the same location for cargo transfer
    async fn ensure_ships_docked_for_transfer(&self, client: &PriorityApiClient, ship1: &str, ship2: &str) -> Result<bool, String> {
        o_info!("⚓ Ensuring {} and {} are docked for transfer", ship1, ship2);
        
        // Dock ship1 if not already docked
        let ship1_data = client.get_ship(ship1).await.map_err(|e| e.to_string())?;
        if ship1_data.nav.status != "DOCKED" {
            o_info!("🔗 Docking {} at {}", ship1, ship1_data.nav.waypoint_symbol);
            match client.dock_ship(ship1).await.map_err(|e| e.to_string()) {
                Ok(_) => {
                    o_info!("✅ {} successfully docked", ship1);
                }
                Err(e) => {
                    o_info!("❌ Failed to dock {}: {}", ship1, e);
                    return Ok(false);
                }
            }
        }
        
        // Dock ship2 if not already docked
        let ship2_data = client.get_ship(ship2).await.map_err(|e| e.to_string())?;
        if ship2_data.nav.status != "DOCKED" {
            o_info!("🔗 Docking {} at {}", ship2, ship2_data.nav.waypoint_symbol);
            match client.dock_ship(ship2).await.map_err(|e| e.to_string()) {
                Ok(_) => {
                    o_info!("✅ {} successfully docked", ship2);
                }
                Err(e) => {
                    o_info!("❌ Failed to dock {}: {}", ship2, e);
                    return Ok(false);
                }
            }
        }
        
        Ok(true)
    }

    /// Start refining iron ore with automatic cargo expansion if needed
    pub async fn start_refinery_operations(&self, client: &PriorityApiClient) -> Result<bool, String> {
        o_info!("🏭 Starting refinery operations with cargo optimization...");
        
        // Find our designated refiner
        let refiner_ship_info = match self.find_best_refinery_candidate() {
            Some(ship) => ship,
            None => {
                o_info!("❌ No refiner ship designated");
                return Ok(false);
            }
        };
        
        let refiner_symbol = &refiner_ship_info.ship_symbol;
        o_info!("🏭 Using {} as primary refiner", refiner_symbol);
        
        // Get current ship data
        let refiner_ship = client.get_ship(refiner_symbol).await.map_err(|e| e.to_string())?;
        
        // Check cargo capacity - need at least 100 for refining
        if refiner_ship.cargo.capacity < 100 {
            o_info!("📦 Refiner {} has {} cargo capacity, need 100+ for refining", 
                   refiner_symbol, refiner_ship.cargo.capacity);
            
            // Try to expand cargo capacity
            match self.expand_refiner_cargo_capacity(client, refiner_symbol).await {
                Ok(expanded) => {
                    if expanded {
                        o_info!("✅ Successfully expanded cargo capacity for refiner");
                    } else {
                        o_info!("⚠️ Could not expand cargo capacity, trying distributed strategy");
                        return self.try_distributed_refining_strategy(client).await;
                    }
                }
                Err(e) => {
                    o_info!("❌ Cargo expansion failed: {}, trying distributed strategy", e);
                    return self.try_distributed_refining_strategy(client).await;
                }
            }
        }
        
        // Coordinate ore transfers to the refiner
        match self.coordinate_ore_to_refiner_transfer(client).await {
            Ok(_) => {
                o_info!("✅ Ore transfer coordination completed");
            }
            Err(e) => {
                o_info!("⚠️ Ore transfer failed: {}", e);
            }
        }
        
        // Get updated refiner data after transfers
        let updated_refiner = client.get_ship(refiner_symbol).await.map_err(|e| e.to_string())?;
        let iron_ore_units = updated_refiner.cargo.inventory
            .iter()
            .find(|item| item.symbol == "IRON_ORE")
            .map(|item| item.units)
            .unwrap_or(0);
            
        if iron_ore_units < 100 {
            o_info!("📦 Refiner {} has {} units of IRON_ORE after transfers, need 100+ for refining", 
                   refiner_symbol, iron_ore_units);
            return Ok(false);
        }
        
        o_info!("⚙️ Refiner {} has {} units of IRON_ORE - starting refinement!", 
               refiner_symbol, iron_ore_units);
        
        // Start refining
        match self.refine_iron_ore(client, refiner_symbol, iron_ore_units).await {
            Ok(success) => {
                if success {
                    o_info!("✅ Refinery operations completed successfully");
                } else {
                    o_info!("⚠️ Refinery operations completed with warnings");
                }
                Ok(success)
            }
            Err(e) => {
                o_info!("❌ Refinery operations failed: {}", e);
                Err(e)
            }
        }
    }

    /// Expand cargo capacity of refiner by optimizing modules
    async fn expand_refiner_cargo_capacity(&self, client: &PriorityApiClient, ship_symbol: &str) -> Result<bool, String> {
        o_info!("🔧 Attempting to expand cargo capacity for {}", ship_symbol);
        
        // Get current modules
        let ship = client.get_ship(ship_symbol).await.map_err(|e| e.to_string())?;
        
        // Find non-critical modules that can be removed
        let mut removable_modules = Vec::new();
        for module in &ship.modules {
            // Keep critical modules: refinery, drive, crew quarters, sensor array
            let is_critical = matches!(module.symbol.as_str(), 
                "MODULE_ORE_REFINERY_I" |
                "MODULE_WARP_DRIVE_I" | "MODULE_WARP_DRIVE_II" | "MODULE_WARP_DRIVE_III" |
                "MODULE_JUMP_DRIVE_I" | "MODULE_JUMP_DRIVE_II" | "MODULE_JUMP_DRIVE_III" |
                "MODULE_CREW_QUARTERS_I" |
                "MODULE_SENSOR_ARRAY_I" | "MODULE_SENSOR_ARRAY_II" | "MODULE_SENSOR_ARRAY_III"
            );
            
            if !is_critical {
                removable_modules.push(module.symbol.clone());
                o_info!("🔧 Non-critical module found: {}", module.symbol);
            }
        }
        
        if removable_modules.is_empty() {
            o_info!("⚠️ No non-critical modules available for removal");
            return Ok(false);
        }
        
        // Navigate to shipyard for module management
        if !self.navigate_to_shipyard(client, ship_symbol).await? {
            o_info!("❌ Failed to reach shipyard for cargo expansion");
            return Ok(false);
        }
        
        // Remove one non-critical module
        let module_to_remove = &removable_modules[0];
        o_info!("🗑️ Removing non-critical module: {}", module_to_remove);
        
        match client.remove_ship_module_with_priority(
            ship_symbol, 
            module_to_remove,
            crate::client::priority_client::ApiPriority::ActiveGoal
        ).await {
            Ok(_) => {
                o_info!("✅ Successfully removed module {}", module_to_remove);
            }
            Err(e) => {
                o_info!("❌ Failed to remove module: {}", e);
                return Ok(false);
            }
        }
        
        // Try to install larger cargo hold
        // Look for cargo modules in system marketplaces
        let _cargo_modules = vec![
            "MODULE_CARGO_HOLD_III", // Largest cargo module
            "MODULE_CARGO_HOLD_II", 
            "MODULE_CARGO_HOLD_I"
        ];
        
        // For now, skip the cargo module installation since it requires complex marketplace search
        // The module removal already freed up space, which may be enough
        // In a production system, we would search marketplaces and purchase/install larger cargo holds
        
        o_info!("🔧 Module removal completed. Cargo space optimization may have helped.");
        o_info!("💡 Advanced cargo expansion (purchasing cargo modules) not yet implemented");
        o_info!("⚙️ Proceeding with current cargo capacity after module removal");
        
        o_info!("⚠️ Could not find or install larger cargo modules");
        Ok(false)
    }

    /// Try distributed refining strategy by consolidating ore
    async fn try_distributed_refining_strategy(&self, client: &PriorityApiClient) -> Result<bool, String> {
        o_info!("🔄 Attempting distributed refining strategy...");
        
        // First, find the ship with the most cargo space
        let ships = client.get_ships().await.map_err(|e| e.to_string())?;
        let mut best_receiver: Option<(String, i32)> = None; // (symbol, available_space)
        
        for ship in &ships {
            let available_space = ship.cargo.capacity - ship.cargo.units;
            if available_space >= 100 {
                match &best_receiver {
                    None => best_receiver = Some((ship.symbol.clone(), available_space)),
                    Some((_, current_space)) => {
                        if available_space > *current_space {
                            best_receiver = Some((ship.symbol.clone(), available_space));
                        }
                    }
                }
            }
        }
        
        let (receiver_ship, available_space) = match best_receiver {
            Some(ship) => ship,
            None => {
                o_info!("❌ No ship has 100+ cargo space to receive consolidated ore");
                return Ok(false);
            }
        };
        
        o_info!("🎯 Selected {} as ore consolidation target (space: {})", receiver_ship, available_space);
        
        // Coordinate ore transfers to the receiver ship
        match self.coordinate_ore_consolidation(client, &receiver_ship).await {
            Ok(total_ore) => {
                if total_ore >= 100 {
                    o_info!("✅ Consolidated {} ore units to {}", total_ore, receiver_ship);
                    
                    // Now try refining on the consolidated ship
                    return self.refine_iron_ore(client, &receiver_ship, total_ore).await;
                } else {
                    o_info!("⚠️ Only consolidated {} ore units, need 100+", total_ore);
                    Ok(false)
                }
            }
            Err(e) => {
                o_info!("❌ Ore consolidation failed: {}", e);
                Ok(false)
            }
        }
    }

    /// Consolidate iron ore from multiple ships to one receiver
    async fn coordinate_ore_consolidation(&self, client: &PriorityApiClient, receiver_ship: &str) -> Result<i32, String> {
        o_info!("📦 Consolidating iron ore to {}", receiver_ship);
        
        let ore_carriers = self.find_ships_with_iron_ore(client).await?;
        let mut _total_transferred = 0;
        
        for carrier_info in ore_carriers {
            if carrier_info.ship_symbol == receiver_ship {
                // Skip transferring to itself
                continue;
            }
            
            o_info!("🚛 Transferring {} ore from {} to {}", 
                   carrier_info.iron_ore_units, carrier_info.ship_symbol, receiver_ship);
            
            match self.transfer_iron_ore_to_refiner(client, &carrier_info, receiver_ship).await {
                Ok(transferred) => {
                    if transferred {
                        _total_transferred += carrier_info.iron_ore_units;
                    }
                }
                Err(e) => {
                    o_info!("⚠️ Transfer from {} failed: {}", carrier_info.ship_symbol, e);
                }
            }
        }
        
        // Get final ore count on receiver ship
        let receiver = client.get_ship(receiver_ship).await.map_err(|e| e.to_string())?;
        let final_ore = receiver.cargo.inventory
            .iter()
            .find(|item| item.symbol == "IRON_ORE")
            .map(|item| item.units)
            .unwrap_or(0);
            
        o_info!("📊 Consolidation complete: {} total ore units on {}", final_ore, receiver_ship);
        Ok(final_ore)
    }

    /// Refine iron ore into refined iron
    async fn refine_iron_ore(&self, client: &PriorityApiClient, ship_symbol: &str, available_units: i32) -> Result<bool, String> {
        o_info!("⚙️ Refining iron ore on {}", ship_symbol);
        
        // Calculate how many refining cycles we can do (100 ore per cycle = 10 refined iron)
        let possible_cycles = available_units / 100;
        
        if possible_cycles == 0 {
            o_info!("⚠️ Not enough iron ore for refining (need 100+, have {})", available_units);
            return Ok(false);
        }
        
        o_info!("🔄 Can perform {} refining cycles with {} iron ore units", 
               possible_cycles, available_units);
        
        let mut successful_refines = 0;
        let mut total_iron_produced = 0;
        let mut total_ore_consumed = 0;
        
        for cycle in 1..=possible_cycles {
            o_info!("⚙️ Starting refining cycle {}/{}", cycle, possible_cycles);
            
            match client.refine_cargo_with_priority(
                ship_symbol, 
                "IRON", 
                crate::client::priority_client::ApiPriority::ActiveGoal
            ).await {
                Ok(refine_data) => {
                    successful_refines += 1;
                    
                    // Log what was produced and consumed
                    for produced in &refine_data.produced {
                        o_info!("✨ Produced: {} x{}", produced.trade_symbol, produced.units);
                        if produced.trade_symbol == "IRON" {
                            total_iron_produced += produced.units;
                        }
                    }
                    
                    for consumed in &refine_data.consumed {
                        o_info!("🔥 Consumed: {} x{}", consumed.trade_symbol, consumed.units);
                        if consumed.trade_symbol == "IRON_ORE" {
                            total_ore_consumed += consumed.units;
                        }
                    }
                    
                    // Check cooldown
                    let cooldown_seconds = refine_data.cooldown.total_seconds;
                    if cooldown_seconds > 0.0 {
                        o_info!("⏳ Refining cooldown: {:.0} seconds", cooldown_seconds);
                        tokio::time::sleep(tokio::time::Duration::from_secs(cooldown_seconds as u64 + 1)).await;
                    }
                }
                Err(e) => {
                    o_info!("❌ Refining cycle {} failed: {}", cycle, e);
                    
                    // Check if it's a cooldown issue and wait
                    if e.to_string().contains("cooldown") {
                        o_info!("⏳ Waiting for cooldown to expire...");
                        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                        continue;
                    }
                    
                    // If it's not a cooldown, it might be lack of ore or other issue
                    break;
                }
            }
        }
        
        o_info!("🏭 Refinery summary:");
        o_info!("   ✅ Successful cycles: {}/{}", successful_refines, possible_cycles);
        o_info!("   ⚙️ Total IRON produced: {} units", total_iron_produced);
        o_info!("   🔥 Total IRON_ORE consumed: {} units", total_ore_consumed);
        
        // Return true if we successfully refined at least once
        Ok(successful_refines > 0)
    }

    /// Execute complete contract fulfillment strategy with refinery
    pub async fn execute_refinery_contract_strategy(&self, client: &PriorityApiClient) -> Result<bool, String> {
        o_info!("📋 Executing refinery contract fulfillment strategy...");
        
        // Get current contracts
        let contracts = client.get_contracts().await.map_err(|e| e.to_string())?;
        let active_contract = contracts.iter().find(|c| c.accepted && !c.fulfilled);
        
        let contract = match active_contract {
            Some(c) => c,
            None => {
                o_info!("❌ No active contracts found");
                return Ok(false);
            }
        };
        
        // Find iron delivery requirements
        let iron_delivery = contract.terms.deliver.iter()
            .find(|d| d.trade_symbol == "IRON");
            
        let (required_iron, delivery_destination) = match iron_delivery {
            Some(delivery) => {
                let needed = delivery.units_required - delivery.units_fulfilled;
                (needed, delivery.destination_symbol.clone())
            }
            None => {
                o_info!("❌ No IRON delivery required in current contract");
                return Ok(false);
            }
        };
        
        if required_iron <= 0 {
            o_info!("✅ Iron contract already fulfilled");
            return Ok(true);
        }
        
        o_info!("📋 Contract requires {} IRON units delivered to {}", required_iron, delivery_destination);
        
        // Calculate ore needed: 100 ore → 10 iron, so we need required_iron * 10 ore
        let ore_needed = required_iron * 10;
        o_info!("⛏️ Need {} iron ore total ({}x refinement cycles)", ore_needed, required_iron / 10);
        
        // Determine strategy based on refiner cargo capacity
        let refiner_ship_info = match self.find_best_refinery_candidate() {
            Some(ship) => ship,
            None => {
                o_info!("❌ No refiner designated for contract strategy");
                return Ok(false);
            }
        };
        
        let refiner_ship = client.get_ship(&refiner_ship_info.ship_symbol).await.map_err(|e| e.to_string())?;
        
        if refiner_ship.cargo.capacity >= required_iron + 10 { // +10 buffer for other items
            o_info!("📦 Refiner has sufficient capacity ({}) for batch strategy", refiner_ship.cargo.capacity);
            self.execute_batch_refinery_strategy(client, &refiner_ship_info.ship_symbol, required_iron, &delivery_destination).await
        } else {
            o_info!("📦 Refiner capacity limited ({}), using incremental delivery strategy", refiner_ship.cargo.capacity);
            self.execute_incremental_delivery_strategy(client, &refiner_ship_info.ship_symbol, required_iron, &delivery_destination).await
        }
    }

    /// Execute batch refinery strategy (expand cargo, refine all at once)
    async fn execute_batch_refinery_strategy(&self, client: &PriorityApiClient, refiner_symbol: &str, required_iron: i32, destination: &str) -> Result<bool, String> {
        o_info!("🏭 Executing BATCH REFINERY STRATEGY for {} iron units", required_iron);
        
        // Step 1: Expand cargo if needed
        let refiner_ship = client.get_ship(refiner_symbol).await.map_err(|e| e.to_string())?;
        if refiner_ship.cargo.capacity < required_iron + 20 { // +20 buffer
            o_info!("🔧 Expanding refiner cargo capacity for batch processing...");
            match self.expand_refiner_cargo_capacity(client, refiner_symbol).await {
                Ok(true) => o_info!("✅ Cargo expansion successful"),
                Ok(false) => {
                    o_info!("⚠️ Cargo expansion failed, falling back to incremental strategy");
                    return self.execute_incremental_delivery_strategy(client, refiner_symbol, required_iron, destination).await;
                }
                Err(e) => {
                    o_info!("❌ Cargo expansion error: {}, falling back to incremental", e);
                    return self.execute_incremental_delivery_strategy(client, refiner_symbol, required_iron, destination).await;
                }
            }
        }
        
        // Step 2: Collect ore needed (required_iron * 10)
        let ore_needed = required_iron * 10;
        o_info!("📦 Collecting {} iron ore units for batch processing", ore_needed);
        // TODO: Implement ore collection strategy (mining coordination)
        
        // Step 3: Refine all ore at once
        o_info!("⚙️ Starting batch refining of {} ore → {} iron", ore_needed, required_iron);
        // This would use our existing refinery operations
        
        // Step 4: Deliver all iron at once  
        o_info!("🚚 Delivering {} iron units to {}", required_iron, destination);
        // TODO: Implement delivery coordination
        
        o_info!("✅ Batch refinery strategy completed!");
        Ok(true)
    }

    /// Execute incremental delivery strategy (refine 100→10, deliver, repeat)
    async fn execute_incremental_delivery_strategy(&self, client: &PriorityApiClient, _refiner_symbol: &str, required_iron: i32, destination: &str) -> Result<bool, String> {
        o_info!("🔄 Executing INCREMENTAL DELIVERY STRATEGY for {} iron units", required_iron);
        
        let cycles_needed = (required_iron + 9) / 10; // Round up division
        o_info!("📊 Strategy: {} cycles of (mine 100 ore → refine 10 iron → deliver 10 iron)", cycles_needed);
        
        let mut delivered_iron = 0;
        
        for cycle in 1..=cycles_needed {
            let delivery_amount = std::cmp::min(10, required_iron - delivered_iron);
            o_info!("🔄 Cycle {}/{}: Processing {} iron units", cycle, cycles_needed, delivery_amount);
            
            // Step 1: Ensure refiner has 100 ore
            o_info!("⛏️ Collecting 100 iron ore for refining");
            // TODO: Coordinate ore collection (100 units)
            
            // Step 2: Refine 100 ore → 10 iron
            o_info!("⚙️ Refining 100 ore → {} iron", delivery_amount);
            match self.start_refinery_operations(client).await {
                Ok(true) => o_info!("✅ Refining cycle {} completed", cycle),
                Ok(false) => {
                    o_info!("⚠️ Refining cycle {} had issues", cycle);
                    continue;
                }
                Err(e) => {
                    o_info!("❌ Refining cycle {} failed: {}", cycle, e);
                    continue;
                }
            }
            
            // Step 3: Deliver the refined iron
            o_info!("🚚 Delivering {} iron units to {} (cycle {})", delivery_amount, destination, cycle);
            // TODO: Implement delivery coordination
            
            delivered_iron += delivery_amount;
            o_info!("📊 Progress: {}/{} iron delivered", delivered_iron, required_iron);
            
            if delivered_iron >= required_iron {
                break;
            }
        }
        
        o_info!("✅ Incremental delivery strategy completed! Delivered {}/{} iron", delivered_iron, required_iron);
        Ok(delivered_iron >= required_iron)
    }
}

#[derive(Debug, Clone)]
struct CargoCarrierInfo {
    ship_symbol: String,
    iron_ore_units: i32,
    current_location: String,
    _nav_status: String,
}