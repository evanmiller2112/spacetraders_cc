// Try mining at ships' current locations without travel
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("⛏️ MINING AT CURRENT LOCATIONS (NO TRAVEL)");
    println!("==========================================");
    
    let ships = priority_client.get_ships().await?;
    
    // Check current locations and try mining there
    for ship in &ships {
        let has_cargo_space = ship.cargo.units < ship.cargo.capacity;
        let has_processor = ship.modules.iter().any(|m| m.symbol.contains("MINERAL_PROCESSOR"));
        let not_refiner = ship.symbol != "CLAUDE_AGENT_2-1";
        
        if has_cargo_space && has_processor && not_refiner {
            println!("\n⛏️ Trying to mine with {} at {}", ship.symbol, ship.nav.waypoint_symbol);
            println!("   Cargo: {}/{}, Fuel: {}/{}", 
                     ship.cargo.units, ship.cargo.capacity, ship.fuel.current, ship.fuel.capacity);
            
            // Make sure ship is in orbit for mining
            if ship.nav.status == "DOCKED" {
                println!("🚀 Putting ship in orbit for mining...");
                match priority_client.orbit_ship(&ship.symbol).await {
                    Ok(_) => println!("✅ Ship now in orbit"),
                    Err(e) => {
                        println!("❌ Orbit failed: {}", e);
                        continue;
                    }
                }
            }
            
            // Try basic extraction at current location
            println!("⛏️ Attempting mining at current location...");
            match priority_client.extract_resources(&ship.symbol).await {
                Ok(extraction_data) => {
                    let material = &extraction_data.extraction.extraction_yield.symbol;
                    let amount = extraction_data.extraction.extraction_yield.units;
                    
                    println!("✅ Mined {} x{} at {}", material, amount, ship.nav.waypoint_symbol);
                    
                    if material == "IRON_ORE" {
                        println!("🎉 IRON_ORE found at {}!", ship.nav.waypoint_symbol);
                    } else {
                        println!("📦 Found {} (not iron ore)", material);
                    }
                }
                Err(e) => {
                    println!("⚠️ Mining failed: {}", e);
                    
                    // Check if the error suggests this location can't be mined
                    if e.to_string().contains("extractable") || e.to_string().contains("asteroid") {
                        println!("💡 Location {} not mineable", ship.nav.waypoint_symbol);
                    } else {
                        // Try survey method
                        println!("🔍 Trying survey method...");
                        match priority_client.create_survey(&ship.symbol).await {
                            Ok(survey_data) => {
                                println!("📊 Survey created with {} results", survey_data.surveys.len());
                                
                                if let Some(survey) = survey_data.surveys.first() {
                                    println!("📊 Survey deposits: {:?}", 
                                             survey.deposits.iter().map(|d| &d.symbol).collect::<Vec<_>>());
                                    
                                    // Try extraction with survey
                                    match priority_client.extract_resources_with_survey(&ship.symbol, survey).await {
                                        Ok(survey_extraction) => {
                                            let material = &survey_extraction.extraction.extraction_yield.symbol;
                                            let amount = survey_extraction.extraction.extraction_yield.units;
                                            
                                            println!("✅ Survey mining: {} x{}", material, amount);
                                            
                                            if material == "IRON_ORE" {
                                                println!("🎉 IRON_ORE via survey!");
                                            }
                                        }
                                        Err(e2) => {
                                            println!("❌ Survey extraction failed: {}", e2);
                                        }
                                    }
                                }
                            }
                            Err(e2) => {
                                println!("❌ Survey failed: {}", e2);
                            }
                        }
                    }
                }
            }
            
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        }
    }
    
    // Check current iron ore status
    println!("\n📊 CURRENT IRON ORE INVENTORY:");
    let updated_ships = priority_client.get_ships().await?;
    let mut total_iron_ore = 0;
    
    for ship in &updated_ships {
        let ship_iron_ore: i32 = ship.cargo.inventory.iter()
            .filter(|item| item.symbol == "IRON_ORE")
            .map(|item| item.units)
            .sum();
        
        if ship_iron_ore > 0 {
            println!("   ⛏️ {}: {} IRON_ORE", ship.symbol, ship_iron_ore);
            total_iron_ore += ship_iron_ore;
        }
    }
    
    println!("\n📊 SUMMARY:");
    println!("   Total iron ore: {} units", total_iron_ore);
    println!("   Target: 100 units");
    println!("   Still needed: {} units", std::cmp::max(0, 100 - total_iron_ore));
    
    if total_iron_ore >= 100 {
        println!("🎉 TARGET ACHIEVED! Ready for refinery operations");
    } else {
        println!("💡 ALTERNATIVES:");
        println!("   1. Find closer mining sites");
        println!("   2. Get more fuel for long trips");
        println!("   3. Try trading for iron ore");
        println!("   4. Check if MINERAL_PROCESSOR can process existing materials");
    }
    
    Ok(())
}