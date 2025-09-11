// Install mining equipment directly using API calls
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("⛏️ INSTALLING MINING EQUIPMENT");
    println!("==============================");
    
    // Step 1: Find shipyards
    println!("🔍 Finding shipyards...");
    let systems = priority_client.get_systems().await?;
    let mut shipyard_waypoints = Vec::new();
    
    for system in &systems {
        let waypoints = priority_client.get_system_waypoints(&system.symbol).await?;
        for waypoint in &waypoints {
            if waypoint.traits.iter().any(|t| t.symbol == "SHIPYARD") {
                shipyard_waypoints.push(waypoint.symbol.clone());
                println!("🏗️ Found shipyard: {}", waypoint.symbol);
            }
        }
    }
    
    if shipyard_waypoints.is_empty() {
        println!("❌ No shipyards found");
        return Ok(());
    }
    
    // Step 2: Check what modules are available
    let shipyard = &shipyard_waypoints[0];
    println!("\n🔍 Checking modules at {}...", shipyard);
    
    let shipyard_data = priority_client.get_shipyard(shipyard).await?;
    println!("📦 Available modules:");
    
    let mut has_mining_laser = false;
    let mut has_surveyor = false;
    
    for module in &shipyard_data.modules {
        println!("   - {} ({})", module.name, module.symbol);
        if module.symbol == "MODULE_MINING_LASER_I" {
            has_mining_laser = true;
        }
        if module.symbol == "MODULE_SURVEYOR_I" {
            has_surveyor = true;
        }
    }
    
    if !has_mining_laser && !has_surveyor {
        println!("❌ No mining equipment available at this shipyard");
        println!("💡 Try checking other shipyards or different locations");
        return Ok(());
    }
    
    // Step 3: Find ships to equip
    let ships = priority_client.get_ships().await?;
    let mut candidates = Vec::new();
    
    for ship in &ships {
        let has_cargo = ship.cargo.capacity > 0;
        let has_module_space = ship.modules.len() < 3;
        let not_refiner = ship.symbol != "CLAUDE_AGENT_2-1";
        
        if has_cargo && has_module_space && not_refiner {
            candidates.push(ship.symbol.clone());
        }
    }
    
    println!("\n🎯 Ships to equip: {:?}", candidates);
    
    // Step 4: Install mining equipment
    for (i, ship_symbol) in candidates.iter().enumerate() {
        if i >= 2 { break; } // Limit to 2 ships
        
        println!("\n⛏️ Equipping {}...", ship_symbol);
        
        // First, navigate ship to shipyard
        let ship = priority_client.get_ship(ship_symbol).await?;
        if ship.nav.waypoint_symbol != *shipyard {
            println!("🚀 Navigating {} to shipyard {}", ship_symbol, shipyard);
            
            // Ensure ship is in orbit
            if ship.nav.status == "DOCKED" {
                priority_client.orbit_ship(ship_symbol).await?;
            }
            
            // Navigate to shipyard
            let nav_data = priority_client.navigate_ship(ship_symbol, shipyard).await?;
            
            // Wait for arrival
            if let Ok(arrival_time) = nav_data.nav.route.arrival.parse::<chrono::DateTime<chrono::Utc>>() {
                let now = chrono::Utc::now();
                let duration = arrival_time - now;
                let wait_time = duration.num_seconds().max(0) as u64 + 3;
                println!("⏳ Waiting {} seconds for arrival...", wait_time);
                tokio::time::sleep(tokio::time::Duration::from_secs(wait_time)).await;
            }
        }
        
        // Dock at shipyard
        priority_client.dock_ship(ship_symbol).await?;
        println!("🛸 Docked at shipyard");
        
        // Install mining equipment
        let module_to_install = if has_mining_laser {
            "MODULE_MINING_LASER_I"
        } else {
            "MODULE_SURVEYOR_I"  
        };
        
        println!("🔧 Installing {}...", module_to_install);
        
        match priority_client.install_ship_module(ship_symbol, module_to_install).await {
            Ok(_) => {
                println!("✅ Successfully installed {} on {}", module_to_install, ship_symbol);
            }
            Err(e) => {
                println!("❌ Failed to install {} on {}: {}", module_to_install, ship_symbol, e);
                println!("💡 This might be due to insufficient credits or module unavailability");
            }
        }
        
        // Brief pause
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    }
    
    // Step 5: Verify installation
    println!("\n🔍 Verifying installations...");
    let updated_ships = priority_client.get_ships().await?;
    let mut equipped_count = 0;
    
    for ship in &updated_ships {
        let has_mining = ship.modules.iter().any(|m| 
            m.symbol.contains("MINING") || m.symbol.contains("SURVEYOR")
        );
        
        if has_mining {
            equipped_count += 1;
            println!("✅ {} equipped with mining gear", ship.symbol);
        }
    }
    
    println!("\n📊 RESULTS:");
    println!("   Ships equipped: {}/{}", equipped_count, candidates.len().min(2));
    
    if equipped_count > 0 {
        println!("🎉 SUCCESS: Mining fleet ready!");
        println!("💡 Next: Run cargo run --example test_survey_mining");
    } else {
        println!("❌ No ships successfully equipped");
        println!("💡 Check credits and shipyard inventory");
    }
    
    Ok(())
}