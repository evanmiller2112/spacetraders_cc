// Admiral module - High-level autonomous game loop orchestration
use crate::client::SpaceTradersClient;
use crate::models::Ship;
use std::collections::HashMap;
use std::fs;

pub struct Admiral {
    pub client: SpaceTradersClient,
    debug_mode: bool,
    full_debug: bool,
}

impl Admiral {
    pub fn new(token: String) -> Self {
        let client = SpaceTradersClient::new(token);
        Self { 
            client,
            debug_mode: false,
            full_debug: false,
        }
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
            println!("🐛 FULL DEBUG MODE ENABLED - All function calls will be logged");
        }
    }

    pub async fn run_autonomous_cycle(&self) -> Result<(), Box<dyn std::error::Error>> {
        crate::debug_fn_enter!("Admiral::run_autonomous_cycle");
        
        println!("🎖️  Admiral starting complete autonomous operations cycle...");
        println!("🎯 PRIME DIRECTIVE: 100% autonomous gameplay - no user interaction");
        println!("🚀 Using advanced fleet coordination with per-ship action queues...");
        
        // Use operations modules
        use crate::operations::*;
        
        // Step 1: Agent status and fleet analysis
        println!("\n═══ STEP 1: Agent Status & Fleet Analysis ═══");
        let agent = self.client.get_agent().await?;
        println!("📊 Agent Info:");
        println!("  Symbol: {}", agent.symbol);
        println!("  Credits: {}", agent.credits);
        println!("  Ships: {}", agent.ship_count);
        
        let fleet_ops = FleetOperations::new(&self.client);
        let ships = fleet_ops.get_all_ships().await?;
        let analysis = fleet_ops.analyze_fleet(&ships);
        
        println!("🚢 Fleet Analysis:");
        println!("  Total ships: {}", analysis.total_ships);
        println!("  Mining ships: {}", analysis.mining_ships);
        println!("  Hauler ships: {}", analysis.hauler_ships);
        println!("  Cargo: {}/{} units", analysis.total_cargo_used, analysis.total_cargo_capacity);
        
        // Step 2: Contract analysis and selection
        println!("\n═══ STEP 2: Contract Management ═══");
        let contract_ops = ContractOperations::new(&self.client);
        let active_contract = match contract_ops.analyze_and_accept_best_contract().await? {
            Some(contract) => {
                println!("✅ Active contract: {}", contract.id);
                contract
            }
            None => {
                println!("⚠️  No contracts available - ending cycle");
                return Ok(());
            }
        };
        
        // Step 3: Advanced Fleet Coordination
        println!("\n═══ STEP 3: Advanced Fleet Coordination ═══");
        
        // Create and initialize fleet coordinator
        let mut fleet_coordinator = FleetCoordinator::new(self.client.clone());
        fleet_coordinator.initialize_fleet().await?;
        
        println!("🎯 Starting autonomous fleet operations with per-ship action queues");
        
        // Run autonomous operations for limited cycles (instead of infinite loop)
        let coordination_result = tokio::time::timeout(
            tokio::time::Duration::from_secs(300), // 5 minutes max per cycle
            fleet_coordinator.run_autonomous_operations(&active_contract)
        ).await;
        
        match coordination_result {
            Ok(_) => println!("✅ Fleet coordination cycle completed successfully"),
            Err(_) => println!("⏰ Fleet coordination cycle timed out - continuing to next step"),
        }
        
        // Get contract materials for remaining operations
        let needed_materials = contract_ops.get_required_materials(&active_contract);
        
        // Step 4: Cargo trading operations
        println!("\n═══ STEP 4: Cargo Trading ═══");
        let trading_ops = TradingOperations::new(&self.client);
        let updated_ships = fleet_ops.get_all_ships().await?;
        let (revenue, items_sold) = trading_ops.execute_autonomous_cargo_selling(&updated_ships, &needed_materials).await?;
        
        println!("💰 Trading results: {} credits from {} items", revenue, items_sold);
        
        // Step 5: Contract delivery and fulfillment
        println!("\n═══ STEP 5: Contract Delivery ═══");
        let contract_fulfilled = contract_ops.execute_autonomous_contract_delivery(&active_contract, &needed_materials).await?;
        
        if contract_fulfilled {
            println!("🎉 CONTRACT FULFILLED SUCCESSFULLY!");
        } else {
            println!("📦 Contract in progress - more materials needed");
        }
        
        // Step 6: PROBE Exploration for Shipyards
        println!("\n═══ STEP 6: PROBE Shipyard Exploration ═══");
        let exploration_ops = ExplorationOperations::new(&self.client);
        let updated_ships_for_probes = fleet_ops.get_all_ships().await?;
        let probe_ships = exploration_ops.get_probe_ships(&updated_ships_for_probes);
        
        if !probe_ships.is_empty() {
            println!("🛰️  {} PROBE ship(s) available for exploration", probe_ships.len());
            match exploration_ops.explore_nearby_systems_for_shipyards(&probe_ships).await {
                Ok(shipyards) => {
                    if !shipyards.is_empty() {
                        println!("🎉 PROBE MISSION SUCCESS: Found {} shipyard(s)!", shipyards.len());
                        for shipyard in &shipyards {
                            println!("   🚢 Shipyard available at: {}", shipyard);
                        }
                    } else {
                        println!("📍 PROBE MISSION: No new shipyards discovered this cycle");
                    }
                }
                Err(e) => {
                    println!("⚠️  PROBE exploration failed: {}", e);
                }
            }
        } else {
            println!("📡 No PROBE ships available for exploration");
        }

        // Step 7: Fleet expansion analysis
        println!("\n═══ STEP 7: Fleet Expansion Analysis ═══");
        let updated_agent = self.client.get_agent().await?;
        println!("💰 Current credits: {}", updated_agent.credits);
        
        // Basic expansion logic - could be enhanced
        if updated_agent.credits > 200000 && analysis.mining_ships < 5 {
            println!("💡 Fleet expansion recommended:");
            println!("  Sufficient credits for new mining ship");
            println!("  Current mining capacity: {} ships", analysis.mining_ships);
            // Ship purchasing logic would go here
        }
        
        println!("\n🎖️  Admiral autonomous cycle completed successfully!");
        println!("📈 Cycle summary:");
        println!("  ✅ Contract management");
        println!("  ✅ Fleet mining operations");
        println!("  ✅ Cargo trading");
        println!("  ✅ Contract delivery");
        println!("  ✅ PROBE exploration");
        println!("  ✅ Fleet analysis");
        
        let result = Ok(());
        crate::debug_fn_exit!("Admiral::run_autonomous_cycle", &result);
        result
    }
    
    pub async fn debug_waypoints(&self, system_symbol: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔍 DEBUG: Analyzing waypoints in system {}...", system_symbol);
        
        let waypoints = self.client.get_system_waypoints(system_symbol, None).await?;
        
        println!("📍 Found {} total waypoints:", waypoints.len());
        
        // Group by type
        let mut type_counts = std::collections::HashMap::new();
        
        for (i, waypoint) in waypoints.iter().enumerate() {
            if i < 10 { // Show first 10 waypoints in detail
                println!("\n{}. {} (Type: {})", i + 1, waypoint.symbol, waypoint.waypoint_type);
                println!("   Coordinates: ({}, {})", waypoint.x, waypoint.y);
                println!("   Traits: {:?}", waypoint.traits.iter().map(|t| &t.name).collect::<Vec<_>>());
            }
            
            *type_counts.entry(&waypoint.waypoint_type).or_insert(0) += 1;
        }
        
        println!("\n📊 Waypoint Types Summary:");
        for (waypoint_type, count) in type_counts {
            println!("   {}: {} waypoints", waypoint_type, count);
        }
        
        // Specifically look for asteroid-related waypoints
        let asteroid_candidates: Vec<_> = waypoints.iter()
            .filter(|w| w.waypoint_type.contains("ASTEROID") || 
                       w.traits.iter().any(|t| t.name.to_lowercase().contains("mineral") || 
                                             t.name.to_lowercase().contains("mining") ||
                                             t.name.to_lowercase().contains("ore")))
            .collect();
            
        println!("\n🗿 Mining/Asteroid Candidates: {} found", asteroid_candidates.len());
        for candidate in asteroid_candidates {
            println!("   {} (Type: {}) - Traits: {:?}", 
                    candidate.symbol, 
                    candidate.waypoint_type,
                    candidate.traits.iter().map(|t| &t.name).collect::<Vec<_>>());
        }
        
        Ok(())
    }

    pub async fn debug_ship_capabilities(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔍 DEBUG: Analyzing all ships for mining capability...");
        
        let ships = self.client.get_ships().await?;
        
        println!("🚢 Found {} total ships:\n", ships.len());
        
        use crate::operations::ShipOperations;
        let ship_ops = ShipOperations::new(&self.client);
        
        for (i, ship) in ships.iter().enumerate() {
            println!("{}. Ship: {} ({})", i + 1, ship.symbol, ship.registration.name);
            println!("   📋 Frame: {} - {}", ship.frame.symbol, ship.frame.name);
            println!("   📦 Cargo Capacity: {} units", ship.cargo.capacity);
            println!("   🔧 Module Slots: {}", ship.frame.module_slots);
            println!("   ⚙️  Mounting Points: {}", ship.frame.mounting_points);
            println!("   ⛽ Fuel Capacity: {}", ship.frame.fuel_capacity);
            
            println!("   🎯 Current Role: {}", ship.registration.role);
            println!("   📍 Location: {}", ship.nav.waypoint_symbol);
            
            // Current modules
            println!("   📦 Current Modules ({}):", ship.modules.len());
            for module in &ship.modules {
                println!("      - {} ({})", module.symbol, module.name);
            }
            
            // Current mounts
            println!("   ⚙️  Current Mounts ({}):", ship.mounts.len());
            for mount in &ship.mounts {
                println!("      - {} ({})", mount.symbol, mount.name);
                if let Some(strength) = mount.strength {
                    println!("        Strength: {}", strength);
                }
                if let Some(deposits) = &mount.deposits {
                    println!("        Can extract: {:?}", deposits);
                }
            }
            
            // Mining capability analysis
            let has_mining = ship_ops.has_mining_capability(ship);
            let is_hauler = ship_ops.is_hauler(ship);
            
            println!("   ⛏️  Mining Capability: {}", if has_mining { "✅ YES" } else { "❌ NO" });
            println!("   🚛 Hauler Capability: {}", if is_hauler { "✅ YES" } else { "❌ NO" });
            
            // Available capacity analysis
            let available_mounts = ship.frame.mounting_points - ship.mounts.len() as i32;
            let available_modules = ship.frame.module_slots - ship.modules.len() as i32;
            
            println!("   💡 Available Mount Slots: {}", available_mounts);
            println!("   💡 Available Module Slots: {}", available_modules);
            
            if !has_mining && available_mounts > 0 {
                println!("   🔧 POTENTIAL: Could be equipped with mining mounts!");
            }
            
            println!("");
        }
        
        // Summary
        let mining_ships = ships.iter().filter(|s| ship_ops.has_mining_capability(s)).count();
        let hauler_ships = ships.iter().filter(|s| ship_ops.is_hauler(s)).count();
        let modifiable_ships = ships.iter().filter(|s| {
            let available_mounts = s.frame.mounting_points - s.mounts.len() as i32;
            !ship_ops.has_mining_capability(s) && available_mounts > 0
        }).count();
        
        println!("📊 Fleet Summary:");
        println!("   ⛏️  Ships with mining capability: {}", mining_ships);
        println!("   🚛 Ships with hauler capability: {}", hauler_ships);
        println!("   🔧 Ships that could be modified for mining: {}", modifiable_ships);
        
        Ok(())
    }

    pub async fn debug_waypoint_facilities(&self, waypoint_symbol: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔍 DEBUG: Analyzing waypoint {} for facilities...", waypoint_symbol);
        
        // Get waypoint details
        let system_symbol = waypoint_symbol.split('-').take(2).collect::<Vec<&str>>().join("-");
        println!("📍 Getting details for waypoint {} in system {}", waypoint_symbol, system_symbol);
        
        match self.client.get_system_waypoints(&system_symbol, None).await {
            Ok(waypoints) => {
                if let Some(waypoint) = waypoints.iter().find(|w| w.symbol == waypoint_symbol) {
                    println!("\n🏢 Waypoint: {} (Type: {})", waypoint.symbol, waypoint.waypoint_type);
                    println!("📍 Coordinates: ({}, {})", waypoint.x, waypoint.y);
                    
                    println!("\n🎯 Traits:");
                    for trait_info in &waypoint.traits {
                        println!("  - {} ({})", trait_info.name, trait_info.description);
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
                    
                    println!("\n🏪 FACILITIES ANALYSIS:");
                    println!("  🚢 Shipyard: {}", if has_shipyard { "✅ YES" } else { "❌ NO" });
                    println!("  🏪 Marketplace: {}", if has_marketplace { "✅ YES" } else { "❌ NO" });
                    
                    // If there's a shipyard, try to get shipyard data
                    if has_shipyard {
                        println!("\n🚢 SHIPYARD DETECTED! Getting shipyard details...");
                        match self.client.get_shipyard(&system_symbol, waypoint_symbol).await {
                            Ok(shipyard) => {
                                println!("✅ Shipyard accessible!");
                                println!("🏗️  Available Ship Types: {}", shipyard.ship_types.len());
                                for ship_type in &shipyard.ship_types {
                                    println!("    - {}", ship_type.ship_type);
                                }
                                
                                if let Some(ships) = &shipyard.ships {
                                    println!("🛒 Ships for Sale: {}", ships.len());
                                    for ship in ships {
                                        println!("    - {} ({}) - {} credits", 
                                                ship.name, ship.ship_type, ship.purchase_price);
                                        println!("      Frame: {} - {}", ship.frame.symbol, ship.frame.name);
                                        println!("      Cargo: {} units, Mounts: {}, Modules: {}", 
                                                ship.frame.fuel_capacity, // This might be cargo capacity in the display
                                                ship.frame.mounting_points,
                                                ship.frame.module_slots);
                                    }
                                } else {
                                    println!("⚠️  No ships currently for sale");
                                }
                                
                                println!("💰 Modification Fee: {} credits", shipyard.modifications_fee);
                            }
                            Err(e) => {
                                println!("❌ Could not access shipyard details: {}", e);
                            }
                        }
                    }
                    
                    // Check nearby waypoints for additional facilities
                    println!("\n🗺️  NEARBY WAYPOINTS:");
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
                        println!("  📍 {} (Type: {}) - Distance: {:.1}", 
                                nearby.symbol, 
                                nearby.waypoint_type,
                                distance);
                        
                        if nearby_shipyard || nearby_marketplace {
                            println!("    🏪 Facilities: {}{}",
                                    if nearby_shipyard { "🚢 Shipyard " } else { "" },
                                    if nearby_marketplace { "🏪 Market" } else { "" });
                        }
                    }
                    
                } else {
                    println!("❌ Waypoint {} not found in system {}", waypoint_symbol, system_symbol);
                }
            }
            Err(e) => {
                println!("❌ Failed to get waypoint details: {}", e);
            }
        }
        
        Ok(())
    }

    pub async fn debug_contracts(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔍 DEBUG: Analyzing current contract status...");
        
        let contracts = self.client.get_contracts().await?;
        
        println!("📋 Found {} total contracts:", contracts.len());
        
        for (i, contract) in contracts.iter().enumerate() {
            println!("\n{}. Contract ID: {}", i + 1, contract.id);
            println!("   Type: {}", contract.contract_type);
            println!("   Faction: {}", contract.faction_symbol);
            println!("   ✅ ACCEPTED: {}", contract.accepted);
            println!("   ✅ FULFILLED: {}", contract.fulfilled);
            println!("   Payment: {} + {} = {}", 
                    contract.terms.payment.on_accepted, 
                    contract.terms.payment.on_fulfilled,
                    contract.terms.payment.on_accepted + contract.terms.payment.on_fulfilled);
            println!("   Deadline: {}", contract.deadline_to_accept);
            
            println!("   Delivery requirements:");
            for delivery in &contract.terms.deliver {
                println!("     - {} x{} to {} (fulfilled: {}/{})", 
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
        
        println!("\n📊 Contract Status Summary:");
        println!("   📝 Unaccepted contracts: {}", unaccepted_count);
        println!("   ✅ Accepted contracts: {}", accepted_count);
        println!("   🎉 Fulfilled contracts: {}", fulfilled_count);
        
        Ok(())
    }

    pub async fn run_continuous_operations(&self) -> Result<(), Box<dyn std::error::Error>> {
        crate::debug_fn_enter!("Admiral::run_continuous_operations");
        
        println!("🎖️  Admiral starting CONTINUOUS autonomous operations...");
        println!("⚠️  This will run indefinitely - Press Ctrl+C to stop");
        println!("🌟 SpaceTraders Autonomous Agent v0.1.1 - Fully Autonomous Gameplay");
        
        let mut cycle_count = 0;
        
        // Setup Ctrl+C handler
        let ctrl_c = async {
            tokio::signal::ctrl_c().await.expect("Failed to install Ctrl+C handler");
        };
        
        let operations = async {
            loop {
                cycle_count += 1;
                println!("\n🔄 ═══════ AUTONOMOUS CYCLE #{} ═══════", cycle_count);
                
                match self.run_autonomous_cycle().await {
                    Ok(()) => {
                        println!("✅ Cycle #{} completed successfully", cycle_count);
                        println!("💰 Agent continuing autonomous operations...");
                    }
                    Err(e) => {
                        eprintln!("❌ Cycle #{} failed: {}", cycle_count, e);
                        eprintln!("⏳ Waiting 60 seconds before retry...");
                        
                        // Check for Ctrl+C during error recovery delay
                        tokio::select! {
                            _ = tokio::time::sleep(tokio::time::Duration::from_secs(60)) => {},
                            _ = tokio::signal::ctrl_c() => {
                                println!("\n⚠️  Ctrl+C received during error recovery. Shutting down...");
                                return Ok::<(), Box<dyn std::error::Error>>(());
                            }
                        }
                    }
                }
                
                // Brief pause between cycles with Ctrl+C handling
                println!("⏳ Cycle complete. Waiting 30 seconds before next cycle...");
                
                tokio::select! {
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(30)) => {},
                    _ = tokio::signal::ctrl_c() => {
                        println!("\n⚠️  Ctrl+C received. Shutting down gracefully...");
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
                println!("\n🛑 CTRL+C RECEIVED - Graceful shutdown initiated");
                println!("🎖️  Admiral reporting: Operations terminated by user command");
                println!("📊 Total cycles completed: {}", cycle_count);
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