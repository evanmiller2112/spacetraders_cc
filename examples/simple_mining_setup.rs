// Simple approach: just try to install mining equipment directly
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("⛏️ SIMPLE MINING EQUIPMENT SETUP");
    println!("================================");
    
    // Step 1: Get ships
    let ships = priority_client.get_ships().await?;
    
    // Step 2: Find suitable ships (not the refiner, has cargo, has module space)
    let mut candidates = Vec::new();
    for ship in &ships {
        let has_cargo = ship.cargo.capacity > 0;
        let has_space = ship.modules.len() < 3; // Leave room for new module
        let not_refiner = ship.symbol != "CLAUDE_AGENT_2-1"; // Keep refiner separate
        
        if has_cargo && has_space && not_refiner {
            candidates.push(ship.symbol.clone());
            println!("🎯 Mining candidate: {} (cargo: {}, modules: {})", 
                     ship.symbol, ship.cargo.capacity, ship.modules.len());
        }
    }
    
    if candidates.is_empty() {
        println!("❌ No suitable ships found");
        return Ok(());
    }
    
    // Step 3: Try to install mining equipment on first 2 ships
    let target_ships = &candidates[..candidates.len().min(2)];
    
    for ship_symbol in target_ships {
        println!("\n⛏️ Setting up mining equipment on {}...", ship_symbol);
        
        // Get ship current location
        let ship = priority_client.get_ship(ship_symbol).await?;
        println!("📍 Current location: {} ({})", ship.nav.waypoint_symbol, ship.nav.status);
        
        // Try to find a nearby shipyard
        let system_symbol = ship.nav.system_symbol;
        let waypoints = priority_client.get_system_waypoints(&system_symbol, None).await?;
        
        let mut shipyard_waypoint = None;
        for waypoint in &waypoints {
            if waypoint.traits.iter().any(|t| t.symbol == "SHIPYARD") {
                shipyard_waypoint = Some(waypoint.symbol.clone());
                break;
            }
        }
        
        match shipyard_waypoint {
            Some(shipyard) => {
                println!("🏗️ Found shipyard: {}", shipyard);
                
                // Navigate to shipyard if needed
                if ship.nav.waypoint_symbol != shipyard {
                    println!("🚀 Navigating to shipyard...");
                    
                    // Orbit if docked
                    if ship.nav.status == "DOCKED" {
                        priority_client.orbit_ship(ship_symbol).await?;
                    }
                    
                    // Navigate
                    let nav_result = priority_client.navigate_ship(ship_symbol, &shipyard).await?;
                    
                    // Wait for arrival
                    if let Ok(arrival_time) = nav_result.nav.route.arrival.parse::<chrono::DateTime<chrono::Utc>>() {
                        let now = chrono::Utc::now();
                        let wait_seconds = (arrival_time - now).num_seconds().max(0) as u64 + 2;
                        if wait_seconds > 0 {
                            println!("⏳ Waiting {} seconds...", wait_seconds);
                            tokio::time::sleep(tokio::time::Duration::from_secs(wait_seconds)).await;
                        }
                    }
                }
                
                // Dock at shipyard
                priority_client.dock_ship(ship_symbol).await?;
                println!("🛸 Docked at shipyard");
                
                // Try to install mining laser first, then surveyor as fallback
                let modules_to_try = vec!["MODULE_MINING_LASER_I", "MODULE_SURVEYOR_I"];
                let mut installed = false;
                
                for module_type in &modules_to_try {
                    println!("🔧 Attempting to install {}...", module_type);
                    
                    match priority_client.install_ship_module(ship_symbol, module_type).await {
                        Ok(_) => {
                            println!("✅ Successfully installed {} on {}", module_type, ship_symbol);
                            installed = true;
                            break;
                        }
                        Err(e) => {
                            println!("⚠️ Failed to install {}: {}", module_type, e);
                        }
                    }
                }
                
                if !installed {
                    println!("❌ Could not install any mining equipment on {}", ship_symbol);
                    println!("💡 Possible issues: insufficient credits, module not available, or ship full");
                }
            }
            None => {
                println!("❌ No shipyard found in system {}", system_symbol);
            }
        }
        
        // Brief pause between ships
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }
    
    // Step 4: Verify what we accomplished
    println!("\n🔍 VERIFICATION: Checking mining equipment...");
    let updated_ships = priority_client.get_ships().await?;
    let mut mining_ready = 0;
    
    for ship in &updated_ships {
        let has_mining_equipment = ship.modules.iter().any(|module| {
            module.symbol.contains("MINING_LASER") || 
            module.symbol.contains("SURVEYOR") ||
            module.symbol.contains("MINING")
        });
        
        if has_mining_equipment && ship.cargo.capacity > 0 {
            mining_ready += 1;
            println!("✅ {} ready for mining:", ship.symbol);
            for module in &ship.modules {
                if module.symbol.contains("MINING") || module.symbol.contains("SURVEYOR") {
                    println!("   🔧 {}", module.symbol);
                }
            }
        }
    }
    
    println!("\n📊 FINAL STATUS:");
    println!("   Ships with mining equipment: {}", mining_ready);
    
    if mining_ready > 0 {
        println!("🎉 SUCCESS: {} ships equipped for mining!", mining_ready);
        println!("💡 Ready to run: cargo run --example test_survey_mining");
    } else {
        println!("❌ No ships successfully equipped with mining gear");
        println!("💡 You may need to:");
        println!("   - Check credit balance");
        println!("   - Visit different shipyards");
        println!("   - Remove existing modules to make space");
    }
    
    Ok(())
}