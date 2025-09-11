// Install mining equipment at the X1-N5-A2 shipyard
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("⛏️ INSTALLING MINING EQUIPMENT AT X1-N5-A2");
    println!("==========================================");
    
    // Check what's available at this shipyard
    println!("🏗️ Checking X1-N5-A2 shipyard inventory...");
    
    // Note: For shipyard API, we need system and waypoint symbol separately
    let system_symbol = "X1-N5";
    let shipyard_symbol = "X1-N5-A2";
    
    match priority_client.get_shipyard(system_symbol, shipyard_symbol).await {
        Ok(shipyard_data) => {
            println!("📦 Shipyard info:");
            println!("   Symbol: {}", shipyard_data.symbol);
            
            // Check available ships (though we want modules)
            if let Some(ships) = &shipyard_data.ships {
                println!("   Ships for sale: {}", ships.len());
            }
            
            // Show modification fee
            println!("   Modification fee: {} credits", shipyard_data.modifications_fee);
        }
        Err(e) => {
            println!("⚠️ Could not get shipyard details: {}", e);
            println!("💡 Proceeding with installation attempts anyway...");
        }
    }
    
    // Get our ships and find candidates for mining equipment
    let ships = priority_client.get_ships().await?;
    let mut mining_candidates = Vec::new();
    
    for ship in &ships {
        let has_cargo = ship.cargo.capacity > 0;
        let has_module_space = ship.modules.len() < 3; // Leave room for mining module
        let not_refiner = ship.symbol != "CLAUDE_AGENT_2-1"; // Keep refiner dedicated
        
        if has_cargo && has_module_space && not_refiner {
            mining_candidates.push(ship.symbol.clone());
            println!("🎯 Mining candidate: {} (modules: {}/3, cargo: {})", 
                     ship.symbol, ship.modules.len(), ship.cargo.capacity);
        }
    }
    
    if mining_candidates.is_empty() {
        println!("❌ No suitable ships found for mining equipment");
        return Ok(());
    }
    
    // Take first 2 ships and equip them with mining gear
    let ships_to_equip = &mining_candidates[..mining_candidates.len().min(2)];
    println!("\n⛏️ Equipping {} ships with mining equipment...", ships_to_equip.len());
    
    for ship_symbol in ships_to_equip {
        println!("\n🔧 Processing {}...", ship_symbol);
        
        let ship = priority_client.get_ship(ship_symbol).await?;
        
        // Navigate to shipyard if needed
        if ship.nav.waypoint_symbol != shipyard_symbol {
            println!("🚀 Navigating {} to shipyard X1-N5-A2", ship_symbol);
            
            // Orbit if docked elsewhere
            if ship.nav.status == "DOCKED" {
                priority_client.orbit_ship(ship_symbol).await?;
            }
            
            // Navigate to shipyard
            let nav_result = priority_client.navigate_ship(ship_symbol, shipyard_symbol).await?;
            
            // Wait for arrival
            if let Ok(arrival_time) = nav_result.nav.route.arrival.parse::<chrono::DateTime<chrono::Utc>>() {
                let now = chrono::Utc::now();
                let wait_seconds = (arrival_time - now).num_seconds().max(0) as u64 + 2;
                if wait_seconds > 0 {
                    println!("⏳ Waiting {} seconds for arrival...", wait_seconds);
                    tokio::time::sleep(tokio::time::Duration::from_secs(wait_seconds)).await;
                }
            }
        } else {
            println!("✅ {} already at shipyard", ship_symbol);
        }
        
        // Dock at shipyard
        priority_client.dock_ship(ship_symbol).await?;
        println!("🛸 Docked at shipyard");
        
        // Try to install mining equipment - start with mining laser, fallback to surveyor
        let modules_to_try = vec![
            ("MODULE_MINING_LASER_I", "Mining Laser"),
            ("MODULE_SURVEYOR_I", "Surveyor"),
            ("MODULE_MINING_LASER_II", "Advanced Mining Laser"),
            ("MODULE_SURVEYOR_II", "Advanced Surveyor"),
        ];
        
        let mut equipped = false;
        
        for (module_symbol, module_name) in &modules_to_try {
            println!("🔧 Attempting to install {}...", module_name);
            
            match priority_client.install_ship_module(ship_symbol, module_symbol).await {
                Ok(_) => {
                    println!("✅ Successfully installed {} on {}", module_name, ship_symbol);
                    equipped = true;
                    break;
                }
                Err(e) => {
                    println!("⚠️ Failed to install {}: {}", module_name, e);
                    if e.to_string().contains("credits") {
                        println!("   💰 Likely insufficient credits");
                    } else if e.to_string().contains("not available") {
                        println!("   📦 Module not available at this shipyard");
                    } else if e.to_string().contains("full") || e.to_string().contains("capacity") {
                        println!("   🚢 Ship module capacity full");
                    }
                }
            }
            
            // Small delay between attempts
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
        
        if !equipped {
            println!("❌ Could not install any mining equipment on {}", ship_symbol);
        }
        
        // Brief pause before next ship
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }
    
    // Final verification
    println!("\n🔍 FINAL VERIFICATION:");
    let updated_ships = priority_client.get_ships().await?;
    let mut equipped_count = 0;
    
    for ship in &updated_ships {
        let mining_modules: Vec<_> = ship.modules.iter()
            .filter(|m| m.symbol.contains("MINING") || m.symbol.contains("SURVEYOR"))
            .collect();
            
        if !mining_modules.is_empty() {
            equipped_count += 1;
            println!("✅ {} equipped with:", ship.symbol);
            for module in mining_modules {
                println!("   🔧 {}", module.symbol);
            }
        }
    }
    
    println!("\n📊 RESULTS:");
    println!("   Ships with mining equipment: {}", equipped_count);
    
    if equipped_count > 0 {
        println!("🎉 SUCCESS! {} ships ready for mining operations!", equipped_count);
        println!("💡 Next step: Run cargo run --example test_survey_mining");
        println!("💡 Goal: Mine 100+ iron ore units for refinery operations");
    } else {
        println!("❌ No ships successfully equipped");
        println!("💡 Possible issues:");
        println!("   - Insufficient credits for modules");
        println!("   - Modules not available at this shipyard"); 
        println!("   - Ships at module capacity");
        println!("💡 Try checking agent credits and ship module space");
    }
    
    Ok(())
}