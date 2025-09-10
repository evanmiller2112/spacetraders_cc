// Admiral module - High-level autonomous game loop orchestration
use crate::client::SpaceTradersClient;
use crate::config::ConfigManager;
use std::fs;

// Use global verbosity macros and output broker
use crate::{o_summary, o_info, o_debug, o_error};

pub struct Admiral {
    pub client: SpaceTradersClient,
    config_manager: ConfigManager,
    debug_mode: bool,
    full_debug: bool,
}

impl Admiral {
    pub fn new(token: String) -> Result<Self, Box<dyn std::error::Error>> {
        let client = SpaceTradersClient::new(token);
        let config_manager = ConfigManager::new("config.toml")?;
        Ok(Self { 
            client,
            config_manager,
            debug_mode: false,
            full_debug: false,
        })
    }
    
    pub fn set_debug_mode(&mut self, debug: bool) {
        self.debug_mode = debug;
        self.client.set_debug_mode(debug);
    }
    
    pub fn set_api_logging(&mut self, logging: bool) {
        self.client.set_api_logging(logging);
    }

    pub fn set_full_debug(&mut self, full_debug: bool) {
        self.full_debug = full_debug;
        crate::debug::set_full_debug(full_debug);
        if full_debug {
            o_debug!("🐛 FULL DEBUG MODE ENABLED - All function calls will be logged");
        }
    }


    pub async fn run_autonomous_cycle(&self) -> Result<(), Box<dyn std::error::Error>> {
        crate::debug_fn_enter!("Admiral::run_autonomous_cycle");
        
        // Level 1: Show cycle start and detailed startup info  
        o_info!("🔄 ═══════ CYCLE START ═══════");
        o_debug!("🎖️  Admiral starting complete autonomous operations cycle...");
        o_debug!("🎯 PRIME DIRECTIVE: 100% autonomous gameplay - no user interaction");
        o_debug!("🚀 Using advanced fleet coordination with per-ship action queues...");
        
        // Initialize skip_mining flag (will be set by marketplace trading logic)
        let mut skip_mining = false;
        
        // Use operations modules
        use crate::operations::*;
        
        // Step 1: Agent status and fleet analysis
        o_debug!( "\n═══ STEP 1: Agent Status & Fleet Analysis ═══");
        let agent = self.client.get_agent().await?;
        
        // Level 0: Always show key status
        o_summary!( "💰 Credits: {} | 🚢 Ships: {}", agent.credits, agent.ship_count);
        
        // Level 1: Show detailed agent info
        o_debug!( "📊 Agent Info:");
        o_debug!( "  Symbol: {}", agent.symbol);
        o_debug!( "  Credits: {}", agent.credits);
        o_debug!( "  Ships: {}", agent.ship_count);
        
        let fleet_ops = FleetOperations::new(&self.client);
        let ships = fleet_ops.get_all_ships().await?;
        let analysis = fleet_ops.analyze_fleet(&ships);
        
        // Level 0: Always show fleet status
        o_summary!( "🚢 Fleet: {} ships ({} mining) | 📦 Cargo: {}/{}", 
                  analysis.total_ships, analysis.mining_ships,
                  analysis.total_cargo_used, analysis.total_cargo_capacity);
        
        // Level 1: Show detailed fleet breakdown
        o_debug!( "🚢 Fleet Analysis:");
        o_debug!( "  Total ships: {}", analysis.total_ships);
        o_debug!( "  Mining ships: {}", analysis.mining_ships);
        o_debug!( "  Hauler ships: {}", analysis.hauler_ships);
        o_debug!( "  Cargo: {}/{} units", analysis.total_cargo_used, analysis.total_cargo_capacity);
        
        // Step 2: Contract analysis and selection
        o_debug!( "\n═══ STEP 2: Contract Management ═══");
        let contract_ops = ContractOperations::new(&self.client);
        let active_contract = match contract_ops.analyze_and_accept_best_contract().await? {
            Some(contract) => {
                // Level 0: Always show active contract
                o_summary!( "📋 Contract: {}", contract.id);
                
                // Level 1: Show detailed info 
                o_debug!( "✅ Active contract: {}", contract.id);
                contract
            }
            None => {
                // Level 0: Show no contract status
                o_summary!( "📋 No active contracts");
                
                // Level 1: Show detailed info
                o_debug!( "📋 No active contracts available");
                o_debug!( "   This could mean:");
                o_debug!( "   • All contracts are fulfilled (great job!)");
                o_debug!( "   • No new contracts offered yet");
                o_debug!( "   • Need to wait for contract refresh");
                o_debug!( "🔄 Continuing with fleet operations and exploration...");
                
                // Create a dummy contract for fleet operations to continue
                // This allows mining, exploration, and fleet management to continue
                use crate::models::*;
                Contract {
                    id: "NO_ACTIVE_CONTRACT".to_string(),
                    faction_symbol: "SYSTEM".to_string(),
                    contract_type: "NONE".to_string(),
                    terms: ContractTerms {
                        deadline: "2099-01-01T00:00:00.000Z".to_string(),
                        payment: Payment {
                            on_accepted: 0,
                            on_fulfilled: 0,
                        },
                        deliver: vec![], // Empty delivery requirements
                    },
                    accepted: false,
                    fulfilled: false,
                    deadline_to_accept: "2099-01-01T00:00:00.000Z".to_string(),
                    expiration: "2099-01-01T00:00:00.000Z".to_string(),
                }
            }
        };
        
        // Step 2.5: Contract Fulfillment Strategy (BEFORE fleet coordination)
        o_debug!( "\n═══ STEP 2.5: Contract Fulfillment Strategy ═══");
        
        // Get contract materials early to determine strategy
        let needed_materials = {
            let contract_ops = ContractOperations::new(&self.client);
            contract_ops.get_required_materials(&active_contract)
        };
        
        let manufactured_goods = ["ELECTRONICS", "MACHINERY", "MEDICINE", "DRUGS", "CLOTHING", "FOOD", "JEWELRY", "TOOLS", "WEAPONS", "EQUIPMENT"];
        let requires_marketplace_trading = needed_materials.iter()
            .any(|material| manufactured_goods.contains(&material.as_str()));
        
        if requires_marketplace_trading {
            // Level 0: Always show strategy decision
            o_summary!( "🏪 Strategy: Marketplace trading for {:?}", needed_materials);
            
            // Level 1: Show detailed info
            o_debug!( "🏭 Contract requires MANUFACTURED goods: {:?}", needed_materials);
            o_debug!( "🏪 Routing to marketplace trading system...");
            
            let contract_ops = ContractOperations::new(&self.client);
            match contract_ops.handle_marketplace_trading(&active_contract).await {
                Ok(trading_initiated) => {
                    if trading_initiated {
                        o_summary!("✅ Marketplace trading operations completed!");
                        o_info!("🔄 Skipping mining operations - using marketplace trading instead");
                        skip_mining = true;
                    } else {
                        o_info!("⚠️ No marketplace trading opportunities found");
                        o_info!("💡 May need mining operations for budget - will attempt mining");
                    }
                }
                Err(e) => {
                    o_error!("❌ Marketplace trading failed: {}", e);
                    o_info!("🔄 Falling back to mining operations for budget");
                }
            }
        } else {
            // Level 0: Always show strategy decision
            o_summary!( "⛏️ Strategy: Mining for {:?}", needed_materials);
            
            // Level 1: Show detailed info
            o_debug!( "⛏️ Contract requires MINEABLE goods: {:?}", needed_materials);
            o_debug!( "🔄 Will use standard mining operations");
        }

        // Step 3: Advanced Fleet Coordination (now conditional)
        o_info!("\n═══ STEP 3: Advanced Fleet Coordination ═══");
        
        // First, check if contract is already complete before starting fleet operations
        // Skip this check for dummy contracts
        let contract_already_complete = if active_contract.id == "NO_ACTIVE_CONTRACT" {
            o_debug!("🔍 No active contract - skipping completion check");
            false
        } else {
            o_debug!("🔍 Pre-flight check: Is contract already complete?");
            let contracts_for_check = self.client.get_contracts().await?;
            let current_contract = contracts_for_check.iter().find(|c| c.id == active_contract.id);
            
            if let Some(contract) = current_contract {
                let total_units_fulfilled: i32 = contract.terms.deliver.iter()
                    .map(|d| d.units_fulfilled)
                    .sum();
                let total_units_required: i32 = contract.terms.deliver.iter()
                    .map(|d| d.units_required)
                    .sum();
                
                let completion_percentage = (total_units_fulfilled * 100) / total_units_required.max(1);
                o_debug!("  📊 Contract status: {}/{} units fulfilled ({}%)", 
                        total_units_fulfilled, total_units_required, completion_percentage);
                
                if total_units_fulfilled >= total_units_required {
                    o_summary!("  🎉 Contract is already 100% complete! Skipping fleet coordination.");
                    true
                } else {
                    o_debug!("  📈 Contract needs more work - proceeding with fleet coordination");
                    false
                }
            } else {
                o_info!("  ⚠️ Could not verify contract status - proceeding with fleet coordination");
                false
            }
        };
        
        // Use config manager for hot-reloading configuration
        let config = self.config_manager.config();
        
        if !contract_already_complete && !skip_mining {
            let mut fleet_coordinator = FleetCoordinator::new(self.client.clone(), config.clone());
            fleet_coordinator.initialize_fleet().await?;
            
            o_info!("🎯 Starting autonomous fleet MINING operations with per-ship action queues");
            
            // Run autonomous operations for limited cycles (instead of infinite loop)
            let coordination_result = tokio::time::timeout(
                tokio::time::Duration::from_secs(config.timing.fleet_coordination_timeout_seconds as u64),
                fleet_coordinator.run_autonomous_operations(&active_contract)
            ).await;
            
            match coordination_result {
                Ok(_) => o_summary!("✅ Fleet mining coordination cycle completed successfully"),
                Err(_) => o_info!("⏰ Fleet mining coordination cycle timed out - continuing to next step"),
            }
        } else if skip_mining {
            o_info!("⚡ Skipping fleet mining coordination - using marketplace trading strategy");
        } else {
            o_info!("⚡ Skipping fleet coordination - contract ready for fulfillment");
        }
        
        // Get needed materials from the already-executed Step 2.5 for remaining operations
        let contract_ops = ContractOperations::new(&self.client);
        let needed_materials = {
            let contract_ops = ContractOperations::new(&self.client);
            contract_ops.get_required_materials(&active_contract)
        };
        
        // Step 4: Cargo trading operations
        o_info!("\n═══ STEP 4: Cargo Trading ═══");
        let trading_ops = TradingOperations::new(&self.client);
        let updated_ships = fleet_ops.get_all_ships().await?;
        let (revenue, items_sold) = trading_ops.execute_autonomous_cargo_selling(&updated_ships, &needed_materials).await?;
        
        o_summary!("💰 Trading results: {} credits from {} items", revenue, items_sold);
        
        // Step 5: Contract delivery and fulfillment
        o_info!("\n═══ STEP 5: Contract Delivery ═══");
        let contract_fulfilled = contract_ops.execute_autonomous_contract_delivery(&active_contract, &needed_materials).await?;
        
        if contract_fulfilled {
            o_summary!("🎉 CONTRACT FULFILLED SUCCESSFULLY!");
        } else {
            o_info!("📦 Contract in progress - more materials needed");
        }
        
        // Step 6: PROBE Exploration for Shipyards
        o_info!("\n═══ STEP 6: PROBE Shipyard Exploration ═══");
        let exploration_ops = ExplorationOperations::new(&self.client);
        let updated_ships_for_probes = fleet_ops.get_all_ships().await?;
        let probe_ships = exploration_ops.get_probe_ships(&updated_ships_for_probes);
        
        if !probe_ships.is_empty() {
            o_info!("🛰️  {} PROBE ship(s) available for exploration", probe_ships.len());
            match exploration_ops.explore_nearby_systems_for_shipyards(&probe_ships).await {
                Ok(shipyards) => {
                    if !shipyards.is_empty() {
                        o_summary!("🎉 PROBE MISSION SUCCESS: Found {} shipyard(s)!", shipyards.len());
                        for shipyard in &shipyards {
                            o_info!("   🚢 Shipyard available at: {}", shipyard);
                        }
                    } else {
                        o_info!("📍 PROBE MISSION: No new shipyards discovered this cycle");
                    }
                }
                Err(e) => {
                    o_error!("⚠️  PROBE exploration failed: {}", e);
                }
            }
        } else {
            o_info!("📡 No PROBE ships available for exploration");
        }

        // Step 7: Fleet expansion analysis
        o_info!("\n═══ STEP 7: Fleet Expansion Analysis ═══");
        let updated_agent = self.client.get_agent().await?;
        o_info!("💰 Current credits: {}", updated_agent.credits);
        
        // Basic expansion logic - could be enhanced
        if updated_agent.credits > config.fleet.min_credits_for_ship_purchase && analysis.mining_ships < config.fleet.max_mining_ships as usize {
            o_info!("💡 Fleet expansion recommended:");
            o_info!("  Sufficient credits for new mining ship");
            o_info!("  Current mining capacity: {} ships", analysis.mining_ships);
            // Ship purchasing logic would go here
        }
        
        // Level 1: Show detailed cycle completion
        o_info!("\n🎖️  Admiral autonomous cycle completed successfully!");
        o_info!("📈 Cycle summary:");
        o_info!("  ✅ Contract management");
        o_info!("  ✅ Fleet mining operations");
        o_info!("  ✅ Cargo trading");
        o_info!("  ✅ Contract delivery");
        o_info!("  ✅ PROBE exploration");
        o_info!("  ✅ Fleet analysis");

        // Level 0: Show comprehensive cycle summary
        o_summary!("");
        o_summary!("🔄 ═══ CYCLE SUMMARY ═══");
        
        // Contract summary
        let fresh_contracts = contract_ops.get_contracts().await.unwrap_or_default();
        for contract in fresh_contracts {
            if !contract.fulfilled {
                let total_required: i32 = contract.terms.deliver.iter().map(|d| d.units_required).sum();
                let total_fulfilled: i32 = contract.terms.deliver.iter().map(|d| d.units_fulfilled).sum();
                let progress = if total_required > 0 { (total_fulfilled * 100) / total_required } else { 0 };
                let materials: Vec<String> = contract.terms.deliver.iter().map(|d| d.trade_symbol.clone()).collect();
                
                let time_left = contract.terms.deadline.parse::<chrono::DateTime<chrono::Utc>>()
                    .map(|deadline| {
                        let now = chrono::Utc::now();
                        let duration = deadline.signed_duration_since(now);
                        format!("{}d {}h", duration.num_days(), duration.num_hours() % 24)
                    })
                    .unwrap_or("unknown".to_string());
                
                o_summary!("📋 {} | {} {}/{} ({}%) | ⏰ {} left", 
                          contract.id, materials.join(","), total_fulfilled, total_required, progress, time_left);
            }
        }
        
        // Fleet summary
        let final_ships = fleet_ops.get_all_ships().await.unwrap_or_default();
        for ship in final_ships {
            let ship_type = if ship.registration.role.contains("EXCAVATOR") { "EXCAVATOR" }
                           else if ship.registration.role.contains("COMMAND") { "COMMAND" }  
                           else if ship.registration.role.contains("SATELLITE") { "SATELLITE" }
                           else { "UNKNOWN" };
            
            let status = if ship.nav.status == "DOCKED" { "🏗️ Docked" }
                        else if ship.nav.status == "IN_ORBIT" { "🌍 Orbit" }
                        else { "🚀 Transit" };
            
            let cargo_used = ship.cargo.units;
            let cargo_capacity = ship.cargo.capacity;
            let fuel_current = ship.fuel.current;
            let fuel_capacity = ship.fuel.capacity;
            
            o_summary!("🚢 {} | {} | {} | 📦 {}/{} | ⛽ {}/{}", 
                      ship.symbol, ship_type, status, cargo_used, cargo_capacity, fuel_current, fuel_capacity);
        }
        
        // Credits summary
        let final_agent = self.client.get_agent().await.unwrap_or_else(|_| agent.clone());
        o_summary!("💰 Credits: {} | 🎯 Strategy: {} for {:?}", 
                   final_agent.credits, 
                   if requires_marketplace_trading { "Marketplace trading" } else { "Mining" },
                   needed_materials);
        o_summary!(""); // Empty line for readability
        
        let result = Ok(());
        crate::debug_fn_exit!("Admiral::run_autonomous_cycle", &result);
        result
    }
    
    pub async fn debug_waypoints(&self, system_symbol: &str) -> Result<(), Box<dyn std::error::Error>> {
        o_debug!("🔍 DEBUG: Analyzing waypoints in system {}...", system_symbol);
        
        let waypoints = self.client.get_system_waypoints(system_symbol, None).await?;
        
        o_debug!("📍 Found {} total waypoints:", waypoints.len());
        
        // Group by type
        let mut type_counts = std::collections::HashMap::new();
        
        for (i, waypoint) in waypoints.iter().enumerate() {
            if i < 10 { // Show first 10 waypoints in detail
                o_debug!("\n{}. {} (Type: {})", i + 1, waypoint.symbol, waypoint.waypoint_type);
                o_debug!("   Coordinates: ({}, {})", waypoint.x, waypoint.y);
                o_debug!("   Traits: {:?}", waypoint.traits.iter().map(|t| &t.name).collect::<Vec<_>>());
            }
            
            *type_counts.entry(&waypoint.waypoint_type).or_insert(0) += 1;
        }
        
        o_debug!("\n📊 Waypoint Types Summary:");
        for (waypoint_type, count) in type_counts {
            o_debug!("   {}: {} waypoints", waypoint_type, count);
        }
        
        // Specifically look for asteroid-related waypoints
        let asteroid_candidates: Vec<_> = waypoints.iter()
            .filter(|w| w.waypoint_type.contains("ASTEROID") || 
                       w.traits.iter().any(|t| t.name.to_lowercase().contains("mineral") || 
                                             t.name.to_lowercase().contains("mining") ||
                                             t.name.to_lowercase().contains("ore")))
            .collect();
            
        o_debug!("\n🗿 Mining/Asteroid Candidates: {} found", asteroid_candidates.len());
        for candidate in asteroid_candidates {
            o_debug!("   {} (Type: {}) - Traits: {:?}", 
                    candidate.symbol, 
                    candidate.waypoint_type,
                    candidate.traits.iter().map(|t| &t.name).collect::<Vec<_>>());
        }
        
        Ok(())
    }

    pub async fn debug_ship_capabilities(&self) -> Result<(), Box<dyn std::error::Error>> {
        o_debug!("🔍 DEBUG: Analyzing all ships for mining capability...");
        
        let ships = self.client.get_ships().await?;
        
        o_debug!("🚢 Found {} total ships:\n", ships.len());
        
        use crate::operations::ShipOperations;
        let ship_ops = ShipOperations::new(&self.client);
        
        for (i, ship) in ships.iter().enumerate() {
            o_debug!("{}. Ship: {} ({})", i + 1, ship.symbol, ship.registration.name);
            o_debug!("   📋 Frame: {} - {}", ship.frame.symbol, ship.frame.name);
            o_debug!("   📦 Cargo Capacity: {} units", ship.cargo.capacity);
            o_debug!("   🔧 Module Slots: {}", ship.frame.module_slots);
            o_debug!("   ⚙️  Mounting Points: {}", ship.frame.mounting_points);
            o_debug!("   ⛽ Fuel Capacity: {}", ship.frame.fuel_capacity);
            
            o_debug!("   🎯 Current Role: {}", ship.registration.role);
            o_debug!("   📍 Location: {}", ship.nav.waypoint_symbol);
            
            // Current modules
            o_debug!("   📦 Current Modules ({}):", ship.modules.len());
            for module in &ship.modules {
                o_debug!("      - {} ({})", module.symbol, module.name);
            }
            
            // Current mounts
            o_debug!("   ⚙️  Current Mounts ({}):", ship.mounts.len());
            for mount in &ship.mounts {
                o_debug!("      - {} ({})", mount.symbol, mount.name);
                if let Some(strength) = mount.strength {
                    o_debug!("        Strength: {}", strength);
                }
                if let Some(deposits) = &mount.deposits {
                    o_debug!("        Can extract: {:?}", deposits);
                }
            }
            
            // Mining capability analysis
            let has_mining = ship_ops.has_mining_capability(ship);
            let is_hauler = ship_ops.is_hauler(ship);
            
            o_debug!("   ⛏️  Mining Capability: {}", if has_mining { "✅ YES" } else { "❌ NO" });
            o_debug!("   🚛 Hauler Capability: {}", if is_hauler { "✅ YES" } else { "❌ NO" });
            
            // Available capacity analysis
            let available_mounts = ship.frame.mounting_points - ship.mounts.len() as i32;
            let available_modules = ship.frame.module_slots - ship.modules.len() as i32;
            
            o_debug!("   💡 Available Mount Slots: {}", available_mounts);
            o_debug!("   💡 Available Module Slots: {}", available_modules);
            
            if !has_mining && available_mounts > 0 {
                o_debug!("   🔧 POTENTIAL: Could be equipped with mining mounts!");
            }
            
            o_debug!("");
        }
        
        // Summary
        let mining_ships = ships.iter().filter(|s| ship_ops.has_mining_capability(s)).count();
        let hauler_ships = ships.iter().filter(|s| ship_ops.is_hauler(s)).count();
        let modifiable_ships = ships.iter().filter(|s| {
            let available_mounts = s.frame.mounting_points - s.mounts.len() as i32;
            !ship_ops.has_mining_capability(s) && available_mounts > 0
        }).count();
        
        o_debug!("📊 Fleet Summary:");
        o_debug!("   ⛏️  Ships with mining capability: {}", mining_ships);
        o_debug!("   🚛 Ships with hauler capability: {}", hauler_ships);
        o_debug!("   🔧 Ships that could be modified for mining: {}", modifiable_ships);
        
        Ok(())
    }

    pub async fn debug_waypoint_facilities(&self, waypoint_symbol: &str) -> Result<(), Box<dyn std::error::Error>> {
        o_debug!("🔍 DEBUG: Analyzing waypoint {} for facilities...", waypoint_symbol);
        
        // Get waypoint details
        let system_symbol = waypoint_symbol.split('-').take(2).collect::<Vec<&str>>().join("-");
        o_debug!("📍 Getting details for waypoint {} in system {}", waypoint_symbol, system_symbol);
        
        match self.client.get_system_waypoints(&system_symbol, None).await {
            Ok(waypoints) => {
                if let Some(waypoint) = waypoints.iter().find(|w| w.symbol == waypoint_symbol) {
                    o_debug!("\n🏢 Waypoint: {} (Type: {})", waypoint.symbol, waypoint.waypoint_type);
                    o_debug!("📍 Coordinates: ({}, {})", waypoint.x, waypoint.y);
                    
                    o_debug!("\n🎯 Traits:");
                    for trait_info in &waypoint.traits {
                        o_debug!("  - {} ({})", trait_info.name, trait_info.description);
                    }
                    
                    // Check for shipyard
                    let has_shipyard = waypoint.traits.iter().any(|t| 
                        t.name.to_lowercase().contains("shipyard") || 
                        t.description.to_lowercase().contains("shipyard")
                    );
                    
                    // Check for marketplace
                    let has_marketplace = waypoint.traits.iter().any(|t| 
                        t.name.to_lowercase().contains("marketplace") || 
                        t.description.to_lowercase().contains("market")
                    );
                    
                    o_debug!("\n🏪 FACILITIES ANALYSIS:");
                    o_debug!("  🚢 Shipyard: {}", if has_shipyard { "✅ YES" } else { "❌ NO" });
                    o_debug!("  🏪 Marketplace: {}", if has_marketplace { "✅ YES" } else { "❌ NO" });
                    
                    // If there's a shipyard, try to get shipyard data
                    if has_shipyard {
                        o_debug!("\n🚢 SHIPYARD DETECTED! Getting shipyard details...");
                        match self.client.get_shipyard(&system_symbol, waypoint_symbol).await {
                            Ok(shipyard) => {
                                o_debug!("✅ Shipyard accessible!");
                                o_debug!("🏗️  Available Ship Types: {}", shipyard.ship_types.len());
                                for ship_type in &shipyard.ship_types {
                                    o_debug!("    - {}", ship_type.ship_type);
                                }
                                
                                if let Some(ships) = &shipyard.ships {
                                    o_debug!("🛒 Ships for Sale: {}", ships.len());
                                    for ship in ships {
                                        o_debug!("    - {} ({}) - {} credits", 
                                                ship.name, ship.ship_type, ship.purchase_price);
                                        o_debug!("      Frame: {} - {}", ship.frame.symbol, ship.frame.name);
                                        o_debug!("      Cargo: {} units, Mounts: {}, Modules: {}", 
                                                ship.frame.fuel_capacity, // This might be cargo capacity in the display
                                                ship.frame.mounting_points,
                                                ship.frame.module_slots);
                                    }
                                } else {
                                    o_debug!("⚠️  No ships currently for sale");
                                }
                                
                                o_debug!("💰 Modification Fee: {} credits", shipyard.modifications_fee);
                            }
                            Err(e) => {
                                o_error!("❌ Could not access shipyard details: {}", e);
                            }
                        }
                    }
                    
                    // Check nearby waypoints for additional facilities
                    o_debug!("\n🗺️  NEARBY WAYPOINTS:");
                    let nearby_waypoints: Vec<_> = waypoints.iter()
                        .filter(|w| {
                            let distance = ((w.x - waypoint.x).pow(2) + (w.y - waypoint.y).pow(2)) as f64;
                            distance.sqrt() <= 100.0 && w.symbol != waypoint.symbol
                        })
                        .take(5)
                        .collect();
                    
                    for nearby in nearby_waypoints {
                        let nearby_shipyard = nearby.traits.iter().any(|t| 
                            t.name.to_lowercase().contains("shipyard"));
                        let nearby_marketplace = nearby.traits.iter().any(|t| 
                            t.name.to_lowercase().contains("marketplace") || 
                            t.description.to_lowercase().contains("market"));
                        
                        let distance = (((nearby.x - waypoint.x).pow(2) + (nearby.y - waypoint.y).pow(2)) as f64).sqrt();
                        o_debug!("  📍 {} (Type: {}) - Distance: {:.1}", 
                                nearby.symbol, 
                                nearby.waypoint_type,
                                distance);
                        
                        if nearby_shipyard || nearby_marketplace {
                            o_debug!("    🏪 Facilities: {}{}",
                                    if nearby_shipyard { "🚢 Shipyard " } else { "" },
                                    if nearby_marketplace { "🏪 Market" } else { "" });
                        }
                    }
                    
                } else {
                    o_error!("❌ Waypoint {} not found in system {}", waypoint_symbol, system_symbol);
                }
            }
            Err(e) => {
                o_error!("❌ Failed to get waypoint details: {}", e);
            }
        }
        
        Ok(())
    }

    pub async fn debug_contracts(&self) -> Result<(), Box<dyn std::error::Error>> {
        o_debug!("🔍 DEBUG: Analyzing current contract status...");
        
        let contracts = self.client.get_contracts().await?;
        
        o_debug!("📋 Found {} total contracts:", contracts.len());
        
        for (i, contract) in contracts.iter().enumerate() {
            o_debug!("\n{}. Contract ID: {}", i + 1, contract.id);
            o_debug!("   Type: {}", contract.contract_type);
            o_debug!("   Faction: {}", contract.faction_symbol);
            o_debug!("   ✅ ACCEPTED: {}", contract.accepted);
            o_debug!("   ✅ FULFILLED: {}", contract.fulfilled);
            o_debug!("   Payment: {} + {} = {}", 
                    contract.terms.payment.on_accepted, 
                    contract.terms.payment.on_fulfilled,
                    contract.terms.payment.on_accepted + contract.terms.payment.on_fulfilled);
            o_debug!("   Deadline: {}", contract.deadline_to_accept);
            
            o_debug!("   Delivery requirements:");
            for delivery in &contract.terms.deliver {
                o_debug!("     - {} x{} to {} (fulfilled: {}/{})", 
                        delivery.trade_symbol, 
                        delivery.units_required, 
                        delivery.destination_symbol,
                        delivery.units_fulfilled,
                        delivery.units_required);
            }
        }
        
        // Count status
        let accepted_count = contracts.iter().filter(|c| c.accepted).count();
        let fulfilled_count = contracts.iter().filter(|c| c.fulfilled).count();
        let unaccepted_count = contracts.iter().filter(|c| !c.accepted).count();
        
        o_debug!("\n📊 Contract Status Summary:");
        o_debug!("   📝 Unaccepted contracts: {}", unaccepted_count);
        o_debug!("   ✅ Accepted contracts: {}", accepted_count);
        o_debug!("   🎉 Fulfilled contracts: {}", fulfilled_count);
        
        Ok(())
    }

    pub async fn run_continuous_operations(&self) -> Result<(), Box<dyn std::error::Error>> {
        crate::debug_fn_enter!("Admiral::run_continuous_operations");
        
        o_summary!("🎖️  Admiral starting CONTINUOUS autonomous operations...");
        o_info!("⚠️  This will run indefinitely - Press Ctrl+C to stop");
        o_info!("🌟 SpaceTraders Autonomous Agent v0.1.1 - Fully Autonomous Gameplay");
        
        let mut cycle_count = 0;
        
        // Setup Ctrl+C handler
        let ctrl_c = async {
            tokio::signal::ctrl_c().await.expect("Failed to install Ctrl+C handler");
        };
        
        let operations = async {
            loop {
                cycle_count += 1;
                o_summary!("\n🔄 ═══════ AUTONOMOUS CYCLE #{} ═══════", cycle_count);
                
                match self.run_autonomous_cycle().await {
                    Ok(()) => {
                        o_summary!("✅ Cycle #{} completed successfully", cycle_count);
                        o_info!("💰 Agent continuing autonomous operations...");
                    }
                    Err(e) => {
                        o_error!("❌ Cycle #{} failed: {}", cycle_count, e);
                        let config = self.config_manager.config();
                        o_error!("⏳ Waiting {} seconds before retry...", config.timing.error_retry_delay_seconds);
                        
                        // Check for Ctrl+C during error recovery delay
                        tokio::select! {
                            _ = tokio::time::sleep(tokio::time::Duration::from_secs(config.timing.error_retry_delay_seconds as u64)) => {},
                            _ = tokio::signal::ctrl_c() => {
                                o_info!("\n⚠️  Ctrl+C received during error recovery. Shutting down...");
                                return Ok::<(), Box<dyn std::error::Error>>(());
                            }
                        }
                    }
                }
                
                // Brief pause between cycles with Ctrl+C handling
                let config = self.config_manager.config();
                o_info!("⏳ Cycle complete. Waiting {} seconds before next cycle...", config.timing.main_cycle_delay_seconds);
                
                tokio::select! {
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(config.timing.main_cycle_delay_seconds as u64)) => {},
                    _ = tokio::signal::ctrl_c() => {
                        o_info!("\n⚠️  Ctrl+C received. Shutting down gracefully...");
                        return Ok::<(), Box<dyn std::error::Error>>(());
                    }
                }
            }
        };
        
        // Run operations with Ctrl+C handling
        tokio::select! {
            result = operations => {
                crate::debug_fn_exit!("Admiral::run_continuous_operations", &result);
                result
            },
            _ = ctrl_c => {
                o_summary!("\n🛑 CTRL+C RECEIVED - Graceful shutdown initiated");
                o_summary!("🎖️  Admiral reporting: Operations terminated by user command");
                o_summary!("📊 Total cycles completed: {}", cycle_count);
                let result = Ok(());
                crate::debug_fn_exit!("Admiral::run_continuous_operations", &result);
                result
            }
        }
    }
}

pub fn load_agent_token() -> Result<String, Box<dyn std::error::Error>> {
    let token = fs::read_to_string(crate::AGENT_TOKEN_FILE)
        .map_err(|e| format!("Failed to read {}: {}", crate::AGENT_TOKEN_FILE, e))?
        .trim()
        .to_string();
    Ok(token)
}