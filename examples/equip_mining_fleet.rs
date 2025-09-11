// Equip ships with mining equipment for iron ore extraction
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient, operations::ShipRoleManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("⛏️ EQUIPPING MINING FLEET FOR IRON ORE EXTRACTION");
    println!("================================================");
    
    // Step 1: Analyze current fleet
    let mut role_manager = ShipRoleManager::new();
    role_manager.analyze_fleet(&priority_client).await?;
    
    // Step 2: Find ships suitable for mining equipment
    let ships = priority_client.get_ships().await?;
    let mut mining_candidates = Vec::new();
    
    for ship in &ships {
        let has_cargo_space = ship.cargo.capacity > 0;
        let has_module_space = ship.modules.len() < 3; // Most ships can hold 3-4 modules
        let not_designated_refiner = ship.symbol != "CLAUDE_AGENT_2-1"; // Keep refiner separate
        
        if has_cargo_space && has_module_space && not_designated_refiner {
            mining_candidates.push(ship.symbol.clone());
            println!("🎯 Mining candidate: {} (modules: {}/3, cargo: {})", 
                     ship.symbol, ship.modules.len(), ship.cargo.capacity);
        }
    }
    
    if mining_candidates.is_empty() {
        println!("❌ No suitable ships found for mining equipment!");
        return Ok(());
    }
    
    println!("\n🔧 Step 1: Installing mining equipment...");
    
    // Step 3: Equip mining candidates with MINING_LASER modules
    for (i, ship_symbol) in mining_candidates.iter().enumerate() {
        if i >= 3 { break; } // Limit to first 3 ships to avoid over-equipping
        
        println!("\n⛏️ Equipping {} with mining equipment...", ship_symbol);
        
        // Navigate to shipyard and install MINING_LASER
        match role_manager.install_module_on_ship(ship_symbol, "MODULE_MINING_LASER_I", &priority_client).await {
            Ok(success) => {
                if success {
                    println!("✅ Successfully installed MINING_LASER on {}", ship_symbol);
                } else {
                    println!("⚠️ Mining laser installation had issues on {}", ship_symbol);
                }
            }
            Err(e) => {
                println!("❌ Failed to install mining laser on {}: {}", ship_symbol, e);
                
                // Try SURVEYOR module as alternative
                println!("🔄 Trying SURVEYOR module instead...");
                match role_manager.install_module_on_ship(ship_symbol, "MODULE_SURVEYOR_I", &priority_client).await {
                    Ok(success) => {
                        if success {
                            println!("✅ Successfully installed SURVEYOR on {}", ship_symbol);
                        } else {
                            println!("⚠️ Surveyor installation had issues on {}", ship_symbol);
                        }
                    }
                    Err(e2) => {
                        println!("❌ Failed to install surveyor on {}: {}", ship_symbol, e2);
                    }
                }
            }
        }
        
        // Brief pause between installations
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    }
    
    // Step 4: Verify mining equipment installation
    println!("\n🔍 Step 2: Verifying mining equipment...");
    let updated_ships = priority_client.get_ships().await?;
    let mut equipped_miners = 0;
    
    for ship in &updated_ships {
        let has_mining_equipment = ship.modules.iter().any(|module| {
            module.symbol.contains("MINING_LASER") || 
            module.symbol.contains("SURVEYOR") ||
            module.symbol.contains("MINING")
        });
        
        if has_mining_equipment {
            equipped_miners += 1;
            println!("✅ {} equipped for mining:", ship.symbol);
            for module in &ship.modules {
                if module.symbol.contains("MINING") || module.symbol.contains("SURVEYOR") {
                    println!("   🔧 {}", module.symbol);
                }
            }
        }
    }
    
    println!("\n📊 MINING FLEET SUMMARY:");
    println!("   Total Ships: {}", updated_ships.len());
    println!("   Mining Equipped: {}", equipped_miners);
    
    if equipped_miners > 0 {
        println!("\n🎉 SUCCESS: Mining fleet equipped!");
        println!("💡 Next step: Run cargo run --example test_survey_mining");
        println!("💡 Target: Mine 100+ iron ore units for refinery operations");
    } else {
        println!("\n❌ No ships successfully equipped with mining gear");
        println!("💡 May need to visit shipyards manually or check credit availability");
    }
    
    Ok(())
}