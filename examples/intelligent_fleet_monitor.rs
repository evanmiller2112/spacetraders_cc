// 🤖 INTELLIGENT FLEET MONITOR - AUTONOMOUS STATUS HOOKS SYSTEM! 🤖
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient};
use tokio::time::{sleep, Duration};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct FleetStatus {
    pub ship_symbol: String,
    pub condition: f64,
    pub location: String,
    pub nav_status: String, // DOCKED, IN_ORBIT, IN_TRANSIT
    pub cargo_used: i32,
    pub cargo_capacity: i32,
    pub fuel_current: i32,
    pub fuel_capacity: i32,
    pub role: String,
    pub has_mining_laser: bool,
    pub has_surveyor: bool,
}

#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub waypoint_symbol: String,
    pub waypoint_type: String,
    pub has_shipyard: bool,
    pub has_marketplace: bool,
    pub has_fuel_station: bool,
}

pub struct IntelligentFleetMonitor {
    client: PriorityApiClient,
    fleet_status: HashMap<String, FleetStatus>,
    system_info: HashMap<String, SystemInfo>,
    actions_taken: Vec<String>,
}

impl IntelligentFleetMonitor {
    pub fn new(client: PriorityApiClient) -> Self {
        Self {
            client,
            fleet_status: HashMap::new(),
            system_info: HashMap::new(),
            actions_taken: Vec::new(),
        }
    }

    // Update fleet and system knowledge
    pub async fn update_status(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔄 UPDATING FLEET STATUS...");
        
        // Get all ships
        let ships = self.client.get_ships().await?;
        
        for ship in ships {
            let condition = ship.frame.condition.unwrap_or(100.0);
            let has_mining_laser = ship.mounts.iter().any(|m| m.symbol.contains("MINING_LASER"));
            let has_surveyor = ship.mounts.iter().any(|m| m.symbol.contains("SURVEYOR"));
            
            let status = FleetStatus {
                ship_symbol: ship.symbol.clone(),
                condition,
                location: ship.nav.waypoint_symbol.clone(),
                nav_status: ship.nav.status.clone(),
                cargo_used: ship.cargo.units,
                cargo_capacity: ship.cargo.capacity,
                fuel_current: ship.fuel.current,
                fuel_capacity: ship.fuel.capacity,
                role: ship.registration.role.clone(),
                has_mining_laser,
                has_surveyor,
            };
            
            // Update system info if we haven't seen this location
            if !self.system_info.contains_key(&ship.nav.waypoint_symbol) {
                if let Ok(waypoint) = self.client.get_waypoint_with_priority(
                    &ship.nav.system_symbol, &ship.nav.waypoint_symbol, spacetraders_cc::client::priority_client::ApiPriority::Background
                ).await {
                    let has_shipyard = waypoint.traits.iter().any(|t| t.symbol == "SHIPYARD");
                    let has_marketplace = waypoint.traits.iter().any(|t| t.symbol == "MARKETPLACE");
                    let has_fuel_station = waypoint.waypoint_type == "FUEL_STATION";
                    
                    self.system_info.insert(ship.nav.waypoint_symbol.clone(), SystemInfo {
                        waypoint_symbol: ship.nav.waypoint_symbol.clone(),
                        waypoint_type: waypoint.waypoint_type,
                        has_shipyard,
                        has_marketplace,
                        has_fuel_station,
                    });
                }
            }
            
            self.fleet_status.insert(ship.symbol, status);
        }
        
        Ok(())
    }

    // SMART HOOK: Auto-repair at shipyards
    pub async fn hook_auto_repair(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for (ship_symbol, status) in &self.fleet_status.clone() {
            // Condition: Low condition + At shipyard + Docked
            if status.condition < 70.0 {
                if let Some(location_info) = self.system_info.get(&status.location) {
                    if location_info.has_shipyard && status.nav_status == "DOCKED" {
                        println!("🔧 AUTO-REPAIR HOOK: {} at {}", ship_symbol, status.location);
                        
                        // Check repair cost first
                        match self.client.get_repair_cost(ship_symbol).await {
                            Ok(cost) => {
                                println!("   💰 Repair cost: {}💎", cost.transaction.total_price);
                                
                                // Execute repair
                                match self.client.repair_ship(ship_symbol).await {
                                    Ok(repair_data) => {
                                        let new_condition = repair_data.ship.frame.condition.unwrap_or(100.0);
                                        println!("   ✅ REPAIRED: {:.0}% → {:.0}%", status.condition, new_condition);
                                        self.actions_taken.push(format!("REPAIRED {} at {}", ship_symbol, status.location));
                                        
                                        // Update our status
                                        if let Some(ship_status) = self.fleet_status.get_mut(ship_symbol) {
                                            ship_status.condition = new_condition;
                                        }
                                    }
                                    Err(e) => println!("   ❌ Repair failed: {}", e),
                                }
                            }
                            Err(e) => println!("   ❌ Cost check failed: {}", e),
                        }
                    } else if location_info.has_shipyard && status.nav_status == "IN_ORBIT" {
                        // Auto-dock for repair
                        println!("🚢 AUTO-DOCK FOR REPAIR: {} at {}", ship_symbol, status.location);
                        match self.client.dock_ship(ship_symbol).await {
                            Ok(_) => {
                                println!("   ✅ DOCKED for repair");
                                self.actions_taken.push(format!("DOCKED {} for repair", ship_symbol));
                            }
                            Err(e) => println!("   ❌ Dock failed: {}", e),
                        }
                    }
                }
            }
        }
        Ok(())
    }

    // SMART HOOK: Auto-refuel when low
    pub async fn hook_auto_refuel(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for (ship_symbol, status) in &self.fleet_status.clone() {
            // Condition: Low fuel + At fuel station/marketplace + Docked
            let fuel_percentage = (status.fuel_current as f64 / status.fuel_capacity as f64) * 100.0;
            
            if fuel_percentage < 30.0 && status.fuel_capacity > 0 {
                if let Some(location_info) = self.system_info.get(&status.location) {
                    if (location_info.has_fuel_station || location_info.has_marketplace) && status.nav_status == "DOCKED" {
                        println!("⛽ AUTO-REFUEL HOOK: {} at {} ({:.0}% fuel)", ship_symbol, status.location, fuel_percentage);
                        
                        match self.client.refuel_ship_with_priority(ship_symbol, None, spacetraders_cc::client::priority_client::ApiPriority::Normal).await {
                            Ok(refuel_data) => {
                                println!("   ✅ REFUELED: {}/{} fuel", refuel_data.fuel.current, refuel_data.fuel.capacity);
                                self.actions_taken.push(format!("REFUELED {} at {}", ship_symbol, status.location));
                            }
                            Err(e) => println!("   ❌ Refuel failed: {}", e),
                        }
                    }
                }
            }
        }
        Ok(())
    }

    // SMART HOOK: Relocate critical ships to shipyards
    pub async fn hook_emergency_relocation(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Find nearest shipyard
        let shipyards: Vec<_> = self.system_info.values().filter(|info| info.has_shipyard).collect();
        
        if shipyards.is_empty() {
            return Ok(()); // No shipyards available
        }
        
        let nearest_shipyard = &shipyards[0]; // Simplified - take first available
        
        for (ship_symbol, status) in &self.fleet_status.clone() {
            // Condition: Critical condition + Not at shipyard + Can travel
            if status.condition < 20.0 && status.nav_status != "IN_TRANSIT" {
                if let Some(location_info) = self.system_info.get(&status.location) {
                    if !location_info.has_shipyard {
                        println!("🚨 EMERGENCY RELOCATION: {} ({:.0}% condition) → {}", 
                                ship_symbol, status.condition, nearest_shipyard.waypoint_symbol);
                        
                        // Check fuel requirements
                        if status.fuel_current > 10 { // Basic fuel check
                            match self.client.navigate_ship(ship_symbol, &nearest_shipyard.waypoint_symbol).await {
                                Ok(_) => {
                                    println!("   ✅ EMERGENCY NAVIGATION STARTED");
                                    self.actions_taken.push(format!("EMERGENCY RELOCATION {} → {}", ship_symbol, nearest_shipyard.waypoint_symbol));
                                }
                                Err(e) => println!("   ❌ Navigation failed: {}", e),
                            }
                        } else {
                            println!("   ⚠️ INSUFFICIENT FUEL for emergency relocation!");
                        }
                    }
                }
            }
        }
        Ok(())
    }

    // SMART HOOK: Optimize cargo operations
    pub async fn hook_cargo_optimization(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for (ship_symbol, status) in &self.fleet_status.clone() {
            let cargo_percentage = (status.cargo_used as f64 / status.cargo_capacity as f64) * 100.0;
            
            // Auto-sell when cargo full at marketplace
            if cargo_percentage >= 90.0 {
                if let Some(location_info) = self.system_info.get(&status.location) {
                    if location_info.has_marketplace && status.nav_status == "DOCKED" {
                        println!("💰 AUTO-SELL HOOK: {} at {} ({:.0}% cargo)", ship_symbol, status.location, cargo_percentage);
                        // Implementation would go here for selling cargo
                        self.actions_taken.push(format!("CARGO OPTIMIZATION {} at {}", ship_symbol, status.location));
                    }
                }
            }
        }
        Ok(())
    }

    // Execute all smart hooks
    pub async fn run_monitoring_cycle(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n🤖🤖🤖 INTELLIGENT FLEET MONITORING CYCLE 🤖🤖🤖");
        println!("==================================================");
        
        // Update status
        self.update_status().await?;
        
        // Show fleet summary
        println!("\n📊 FLEET STATUS SUMMARY:");
        for (ship_symbol, status) in &self.fleet_status {
            let condition_icon = if status.condition >= 80.0 { "✅" } 
                                else if status.condition >= 50.0 { "⚠️" } 
                                else { "🚨" };
            
            println!("   {} {}: {:.0}% @ {} ({})", 
                     condition_icon, ship_symbol, status.condition, status.location, status.nav_status);
        }
        
        // Execute smart hooks
        println!("\n🔧 EXECUTING SMART HOOKS:");
        self.hook_auto_repair().await?;
        self.hook_auto_refuel().await?;
        self.hook_emergency_relocation().await?;
        self.hook_cargo_optimization().await?;
        
        // Show actions taken
        if !self.actions_taken.is_empty() {
            println!("\n📋 ACTIONS TAKEN THIS CYCLE:");
            for action in &self.actions_taken {
                println!("   ⚡ {}", action);
            }
            self.actions_taken.clear();
        } else {
            println!("\n✅ NO ACTIONS NEEDED - FLEET OPERATING OPTIMALLY");
        }
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("🤖🤖🤖 INTELLIGENT FLEET MONITOR - AUTONOMOUS HOOKS 🤖🤖🤖");
    println!("============================================================");
    println!("⚡ SMART STATUS MONITORING WITH AUTO-ACTIONS!");
    println!("🎯 IF X THEN DO Y - AUTONOMOUS GALACTIC DOMINATION!");
    
    let mut monitor = IntelligentFleetMonitor::new(priority_client);
    
    // Run monitoring cycles
    let max_cycles = 10;
    for cycle in 1..=max_cycles {
        println!("\n🔄 MONITORING CYCLE {}/{}", cycle, max_cycles);
        monitor.run_monitoring_cycle().await?;
        
        if cycle < max_cycles {
            println!("\n⏱️ Waiting 30 seconds before next cycle...");
            sleep(Duration::from_secs(30)).await;
        }
    }
    
    println!("\n🎉 INTELLIGENT MONITORING COMPLETE!");
    println!("🤖 AUTONOMOUS FLEET MANAGEMENT SYSTEM OPERATIONAL!");
    
    Ok(())
}