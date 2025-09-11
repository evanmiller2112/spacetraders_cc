// Check our credits and try basic mining without specialized equipment
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("💰 CHECKING CREDITS AND TRYING BASIC MINING");
    println!("==========================================");
    
    // Check agent status and credits
    let agent = priority_client.get_agent().await?;
    println!("🤖 Agent: {}", agent.symbol);
    println!("💰 Credits: {}", agent.credits);
    
    if agent.credits < 1000 {
        println!("⚠️ Low credits ({}) - may need to earn more before buying modules", agent.credits);
    }
    
    // Since we can't seem to get mining equipment easily, let's try basic extraction
    // Our ships have MINERAL_PROCESSOR modules - maybe they can do basic mining?
    println!("\n🔍 Testing basic extraction capabilities...");
    
    let ships = priority_client.get_ships().await?;
    
    // Find a ship with cargo space at a mineable location
    let mut test_miner = None;
    for ship in &ships {
        let has_cargo_space = ship.cargo.units < ship.cargo.capacity;
        let has_mineral_processor = ship.modules.iter().any(|m| m.symbol.contains("MINERAL_PROCESSOR"));
        
        if has_cargo_space && has_mineral_processor {
            test_miner = Some(ship.symbol.clone());
            println!("🎯 Test miner candidate: {} (cargo: {}/{}, has mineral processor: {})", 
                     ship.symbol, ship.cargo.units, ship.cargo.capacity, has_mineral_processor);
            break;
        }
    }
    
    if let Some(miner_symbol) = test_miner {
        println!("\n⛏️ Testing basic extraction on {}...", miner_symbol);
        
        // Navigate to a known mining location (asteroid)
        let ship = priority_client.get_ship(&miner_symbol).await?;
        let target_asteroid = "X1-N5-B8"; // This had MINERAL_DEPOSITS
        
        if ship.nav.waypoint_symbol != target_asteroid {
            println!("🚀 Navigating to asteroid {}", target_asteroid);
            
            // Orbit if docked
            if ship.nav.status == "DOCKED" {
                priority_client.orbit_ship(&miner_symbol).await?;
            }
            
            // Navigate
            let nav_result = priority_client.navigate_ship(&miner_symbol, target_asteroid).await?;
            
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
            println!("✅ Already at asteroid location");
        }
        
        // Try basic extraction (without survey first)
        println!("⛏️ Attempting basic resource extraction...");
        
        match priority_client.extract_resources(&miner_symbol).await {
            Ok(extraction_data) => {
                let yield_amount = extraction_data.extraction.extraction_yield.units;
                let yield_material = &extraction_data.extraction.extraction_yield.symbol;
                
                println!("✅ Extraction successful!");
                println!("   Material: {} x{}", yield_material, yield_amount);
                
                if yield_material == "IRON_ORE" {
                    println!("🎉 Found IRON_ORE! Basic mining works!");
                } else {
                    println!("💎 Found other material - mining system works");
                }
            }
            Err(e) => {
                println!("❌ Extraction failed: {}", e);
                
                if e.to_string().contains("mining") || e.to_string().contains("equipment") {
                    println!("💡 Confirmed: Need proper mining equipment");
                } else if e.to_string().contains("survey") {
                    println!("💡 May need survey first");
                } else {
                    println!("💡 Other issue - check location or ship capability");
                }
            }
        }
        
        // If basic extraction failed, try with survey
        println!("\n🔍 Trying survey-based extraction...");
        match priority_client.create_survey(&miner_symbol).await {
            Ok(survey_data) => {
                println!("✅ Survey created with {} results", survey_data.surveys.len());
                
                if let Some(first_survey) = survey_data.surveys.first() {
                    println!("🔍 Survey deposits:");
                    for deposit in &first_survey.deposits {
                        println!("   - {}", deposit.symbol);
                    }
                    
                    // Try extraction with survey
                    match priority_client.extract_resources_with_survey(&miner_symbol, first_survey).await {
                        Ok(extraction_data) => {
                            let yield_amount = extraction_data.extraction.extraction_yield.units;
                            let yield_material = &extraction_data.extraction.extraction_yield.symbol;
                            
                            println!("✅ Survey extraction successful!");
                            println!("   Material: {} x{}", yield_material, yield_amount);
                            
                            if yield_material == "IRON_ORE" {
                                println!("🎉 IRON_ORE extracted! Survey-based mining works!");
                            }
                        }
                        Err(e) => {
                            println!("❌ Survey extraction failed: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                println!("❌ Survey creation failed: {}", e);
            }
        }
        
    } else {
        println!("❌ No suitable ships found for testing extraction");
    }
    
    println!("\n📊 SUMMARY:");
    println!("   Agent credits: {}", agent.credits);
    println!("   Basic extraction capability: testing above");
    println!("   Next steps depend on test results");
    
    Ok(())
}