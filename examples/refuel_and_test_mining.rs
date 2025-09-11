// Refuel ships and test basic mining capabilities
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("⛽ REFUELING SHIPS AND TESTING MINING");
    println!("====================================");
    
    let agent = priority_client.get_agent().await?;
    println!("💰 Agent credits: {}", agent.credits);
    
    // Get ships and find fuel station
    let ships = priority_client.get_ships().await?;
    println!("\n🚢 Fleet status:");
    
    for ship in &ships {
        println!("   {}: fuel {}/{} at {}", 
                 ship.symbol, ship.fuel.current, ship.fuel.capacity, ship.nav.waypoint_symbol);
    }
    
    // Fuel station is X1-N5-B6 according to our earlier scan
    let fuel_station = "X1-N5-B6";
    println!("\n⛽ Refueling ships at {}...", fuel_station);
    
    // Refuel ships that need it
    for ship in &ships {
        if ship.fuel.current < ship.fuel.capacity / 2 { // Refuel if less than 50%
            println!("\n⛽ Refueling {}...", ship.symbol);
            
            // Navigate to fuel station if needed
            if ship.nav.waypoint_symbol != fuel_station {
                // Check if we have enough fuel to get to fuel station
                if ship.fuel.current < 100 { // Estimate needed for short jump
                    println!("⚠️ {} may not have enough fuel to reach fuel station", ship.symbol);
                    continue;
                }
                
                println!("🚀 Navigating {} to fuel station", ship.symbol);
                
                // Orbit if docked
                if ship.nav.status == "DOCKED" {
                    priority_client.orbit_ship(&ship.symbol).await?;
                }
                
                // Navigate
                match priority_client.navigate_ship(&ship.symbol, fuel_station).await {
                    Ok(nav_result) => {
                        // Wait for arrival
                        if let Ok(arrival_time) = nav_result.nav.route.arrival.parse::<chrono::DateTime<chrono::Utc>>() {
                            let now = chrono::Utc::now();
                            let wait_seconds = (arrival_time - now).num_seconds().max(0) as u64 + 2;
                            if wait_seconds > 0 {
                                println!("⏳ Waiting {} seconds for arrival...", wait_seconds);
                                tokio::time::sleep(tokio::time::Duration::from_secs(wait_seconds)).await;
                            }
                        }
                    }
                    Err(e) => {
                        println!("❌ Navigation failed: {}", e);
                        continue;
                    }
                }
            }
            
            // Dock and refuel
            priority_client.dock_ship(&ship.symbol).await?;
            
            match priority_client.refuel_ship(&ship.symbol).await {
                Ok(_) => {
                    println!("✅ {} refueled successfully", ship.symbol);
                }
                Err(e) => {
                    println!("❌ Refueling failed for {}: {}", ship.symbol, e);
                }
            }
            
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        } else {
            println!("✅ {} has adequate fuel", ship.symbol);
        }
    }
    
    // Now test basic mining
    println!("\n⛏️ TESTING BASIC MINING CAPABILITIES");
    
    // Find a ship with cargo space and mineral processor
    let updated_ships = priority_client.get_ships().await?;
    let mut test_miner = None;
    
    for ship in &updated_ships {
        let has_cargo_space = ship.cargo.units < ship.cargo.capacity;
        let has_adequate_fuel = ship.fuel.current > 200;
        let has_processor = ship.modules.iter().any(|m| m.symbol.contains("MINERAL_PROCESSOR"));
        
        if has_cargo_space && has_adequate_fuel && has_processor {
            test_miner = Some(ship.symbol.clone());
            println!("🎯 Selected {} for mining test", ship.symbol);
            break;
        }
    }
    
    if let Some(miner_symbol) = test_miner {
        // Navigate to asteroid with mineral deposits
        let mining_target = "X1-N5-B8"; // Has MINERAL_DEPOSITS according to scan
        
        let ship = priority_client.get_ship(&miner_symbol).await?;
        if ship.nav.waypoint_symbol != mining_target {
            println!("🚀 Navigating to mining location {}", mining_target);
            
            if ship.nav.status == "DOCKED" {
                priority_client.orbit_ship(&miner_symbol).await?;
            }
            
            priority_client.navigate_ship(&miner_symbol, mining_target).await?;
            
            // Wait a bit for arrival (rough estimate)
            println!("⏳ Waiting for arrival...");
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        }
        
        // Try basic extraction WITHOUT survey first
        println!("⛏️ Attempting basic resource extraction (no survey)...");
        match priority_client.extract_resources(&miner_symbol).await {
            Ok(extraction_data) => {
                let material = &extraction_data.extraction.extraction_yield.symbol;
                let amount = extraction_data.extraction.extraction_yield.units;
                
                println!("✅ Basic extraction works! Got {} x{}", material, amount);
                
                if material == "IRON_ORE" {
                    println!("🎉 SUCCESS: Found IRON_ORE with basic extraction!");
                }
            }
            Err(e) => {
                println!("⚠️ Basic extraction failed: {}", e);
                
                // Try with survey
                println!("🔍 Trying with survey...");
                match priority_client.create_survey(&miner_symbol).await {
                    Ok(survey_data) => {
                        if let Some(survey) = survey_data.surveys.first() {
                            println!("📊 Survey created, deposits: {:?}", 
                                     survey.deposits.iter().map(|d| &d.symbol).collect::<Vec<_>>());
                            
                            match priority_client.extract_resources_with_survey(&miner_symbol, survey).await {
                                Ok(extraction_data) => {
                                    let material = &extraction_data.extraction.extraction_yield.symbol;
                                    let amount = extraction_data.extraction.extraction_yield.units;
                                    
                                    println!("✅ Survey extraction works! Got {} x{}", material, amount);
                                    
                                    if material == "IRON_ORE" {
                                        println!("🎉 SUCCESS: IRON_ORE found with survey method!");
                                    }
                                }
                                Err(e) => {
                                    println!("❌ Survey extraction also failed: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("❌ Survey creation failed: {}", e);
                    }
                }
            }
        }
    } else {
        println!("❌ No suitable ship found for mining test");
    }
    
    println!("\n📊 RESULT: Basic mining capability tested");
    println!("💡 If mining works, we can proceed without specialized equipment");
    println!("💡 If not, we'll need to solve the module installation issue");
    
    Ok(())
}