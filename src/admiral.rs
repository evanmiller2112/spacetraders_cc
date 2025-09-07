// Admiral module - High-level autonomous game loop orchestration
use crate::client::SpaceTradersClient;
use crate::models::Ship;
use std::collections::HashMap;
use std::fs;

pub struct Admiral {
    pub client: SpaceTradersClient,
}

impl Admiral {
    pub fn new(token: String) -> Self {
        let client = SpaceTradersClient::new(token);
        Self { client }
    }

    pub async fn run_autonomous_cycle(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🎖️  Admiral starting complete autonomous operations cycle...");
        println!("🎯 PRIME DIRECTIVE: 100% autonomous gameplay - no user interaction");
        
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
        
        // Step 3: Mining fleet deployment and operations
        println!("\n═══ STEP 3: Mining Operations ═══");
        let mining_ops = MiningOperations::new(&self.client);
        let mining_ships = fleet_ops.get_mining_ships(&ships);
        
        if mining_ships.is_empty() {
            println!("⚠️  No mining ships available");
            return Ok(());
        }
        
        // Get contract materials and find suitable asteroid fields
        let needed_materials = contract_ops.get_required_materials(&active_contract);
        println!("🎯 Contract requires: {:?}", needed_materials);
        
        // Extract system from ship location
        let system_symbol = if let Some(first_ship) = ships.first() {
            let waypoint_parts: Vec<&str> = first_ship.nav.waypoint_symbol.split('-').collect();
            format!("{}-{}", waypoint_parts[0], waypoint_parts[1])
        } else {
            return Err("No ships available".into());
        };
        
        let asteroid_fields = mining_ops.find_asteroid_fields(&system_symbol, &needed_materials).await?;
        
        if asteroid_fields.is_empty() {
            println!("❌ No suitable asteroid fields found");
            return Ok(());
        }
        
        // Deploy fleet to mining positions
        let mining_ships_owned: Vec<Ship> = mining_ships.into_iter().cloned().collect();
        let ready_miners = fleet_ops.coordinate_fleet_operations(&mining_ships_owned, &asteroid_fields).await?;
        
        // Execute parallel mining operations
        mining_ops.execute_parallel_survey_mining(&ready_miners, &needed_materials, &active_contract, 10).await?;
        
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
        
        // Step 6: Fleet expansion analysis
        println!("\n═══ STEP 6: Fleet Expansion Analysis ═══");
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
        println!("  ✅ Fleet analysis");
        
        Ok(())
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
            result = operations => result,
            _ = ctrl_c => {
                println!("\n🛑 CTRL+C RECEIVED - Graceful shutdown initiated");
                println!("🎖️  Admiral reporting: Operations terminated by user command");
                println!("📊 Total cycles completed: {}", cycle_count);
                Ok(())
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