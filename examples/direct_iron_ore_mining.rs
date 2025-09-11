// Direct iron ore mining approach - simple and fast
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("⛏️ DIRECT IRON ORE MINING CAMPAIGN");
    println!("==================================");
    
    // Check current iron ore inventory
    let ships = priority_client.get_ships().await?;
    let mut total_iron_ore = 0;
    
    for ship in &ships {
        let ship_iron_ore: i32 = ship.cargo.inventory.iter()
            .filter(|item| item.symbol == "IRON_ORE")
            .map(|item| item.units)
            .sum();
        total_iron_ore += ship_iron_ore;
    }
    
    println!("📊 Current iron ore: {} units", total_iron_ore);
    println!("🎯 Target: 100 units");
    let needed = std::cmp::max(0, 100 - total_iron_ore);
    println!("📊 Still need: {} units", needed);
    
    if needed == 0 {
        println!("🎉 Already have sufficient iron ore!");
        return Ok(());
    }
    
    // Find ships with cargo space and fuel
    let mut miners = Vec::new();
    for ship in &ships {
        let has_cargo_space = ship.cargo.units < ship.cargo.capacity;
        let has_fuel = ship.fuel.current > 50;
        let has_processor = ship.modules.iter().any(|m| m.symbol.contains("MINERAL_PROCESSOR"));
        let not_refiner = ship.symbol != "CLAUDE_AGENT_2-1";
        
        if has_cargo_space && has_fuel && has_processor && not_refiner {
            miners.push(ship.symbol.clone());
            println!("⛏️ Miner: {} (cargo: {}/{}, fuel: {})", 
                     ship.symbol, ship.cargo.units, ship.cargo.capacity, ship.fuel.current);
        }
    }
    
    if miners.is_empty() {
        println!("❌ No suitable miners found");
        return Ok(());
    }
    
    // Known mining locations with mineral deposits
    let mining_sites = vec![
        "X1-N5-B8",  // MINERAL_DEPOSITS + EXPLOSIVE_GASES + DEEP_CRATERS
        "X1-N5-B9",  // MINERAL_DEPOSITS + EXPLOSIVE_GASES  
        "X1-N5-B10", // MINERAL_DEPOSITS + EXPLOSIVE_GASES
        "X1-N5-B13", // MINERAL_DEPOSITS + EXPLOSIVE_GASES
    ];
    
    println!("\n🎯 Mining sites: {:?}", mining_sites);
    
    // Start mining campaign - simple approach
    let mut campaign_iron_ore = 0;
    let max_attempts = 30; // Limit attempts to avoid infinite loops
    
    for attempt in 1..=max_attempts {
        if campaign_iron_ore >= needed {
            break;
        }
        
        println!("\n🔄 Mining attempt {}/{} (campaign ore: {})", attempt, max_attempts, campaign_iron_ore);
        
        // Try each miner at different sites
        for (i, miner_symbol) in miners.iter().enumerate() {
            let site = &mining_sites[i % mining_sites.len()];
            
            println!("⛏️ {} mining at {}", miner_symbol, site);
            
            // Get current ship status  
            let ship = priority_client.get_ship(miner_symbol).await?;
            
            // Navigate to mining site if needed
            if ship.nav.waypoint_symbol != *site {
                println!("🚀 Navigating to mining site...");
                
                // Orbit if docked
                if ship.nav.status == "DOCKED" {
                    priority_client.orbit_ship(miner_symbol).await?;
                }
                
                // Navigate
                match priority_client.navigate_ship(miner_symbol, site).await {
                    Ok(nav_result) => {
                        // Brief wait for arrival
                        if let Ok(arrival_time) = nav_result.nav.route.arrival.parse::<chrono::DateTime<chrono::Utc>>() {
                            let now = chrono::Utc::now();
                            let wait_seconds = (arrival_time - now).num_seconds().max(0) as u64;
                            if wait_seconds > 0 && wait_seconds < 120 { // Max 2 minutes
                                println!("⏳ Waiting {} seconds...", wait_seconds);
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
            
            // Try basic extraction
            match priority_client.extract_resources(miner_symbol).await {
                Ok(extraction_data) => {
                    let material = &extraction_data.extraction.extraction_yield.symbol;
                    let amount = extraction_data.extraction.extraction_yield.units;
                    
                    println!("✅ Extracted {} x{}", material, amount);
                    
                    if material == "IRON_ORE" {
                        campaign_iron_ore += amount;
                        println!("🎉 IRON_ORE found! Campaign total: {}", campaign_iron_ore);
                    }
                }
                Err(e) => {
                    println!("⚠️ Extraction failed: {}", e);
                    
                    // Try with survey if basic extraction fails
                    println!("🔍 Trying survey method...");
                    match priority_client.create_survey(miner_symbol).await {
                        Ok(survey_data) => {
                            // Look for iron ore survey
                            let iron_survey = survey_data.surveys.iter().find(|survey| {
                                survey.deposits.iter().any(|deposit| deposit.symbol == "IRON_ORE")
                            });
                            
                            if let Some(survey) = iron_survey {
                                println!("📊 Found iron ore survey, extracting...");
                                
                                match priority_client.extract_resources_with_survey(miner_symbol, survey).await {
                                    Ok(survey_extraction) => {
                                        let material = &survey_extraction.extraction.extraction_yield.symbol;
                                        let amount = survey_extraction.extraction.extraction_yield.units;
                                        
                                        println!("✅ Survey extraction: {} x{}", material, amount);
                                        
                                        if material == "IRON_ORE" {
                                            campaign_iron_ore += amount;
                                            println!("🎉 Survey IRON_ORE! Campaign total: {}", campaign_iron_ore);
                                        }
                                    }
                                    Err(e2) => {
                                        println!("❌ Survey extraction failed: {}", e2);
                                    }
                                }
                            } else {
                                println!("⚠️ No iron ore in survey");
                            }
                        }
                        Err(e2) => {
                            println!("❌ Survey failed: {}", e2);
                        }
                    }
                }
            }
            
            // Brief pause between ships
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            
            // Check if we've hit our target
            if campaign_iron_ore >= needed {
                break;
            }
        }
        
        // Pause between attempts
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    }
    
    // Final inventory check
    println!("\n📊 FINAL IRON ORE INVENTORY:");
    let final_ships = priority_client.get_ships().await?;
    let mut final_iron_ore = 0;
    
    for ship in &final_ships {
        let ship_iron_ore: i32 = ship.cargo.inventory.iter()
            .filter(|item| item.symbol == "IRON_ORE")
            .map(|item| item.units)
            .sum();
        
        if ship_iron_ore > 0 {
            println!("   ⛏️ {}: {} IRON_ORE", ship.symbol, ship_iron_ore);
            final_iron_ore += ship_iron_ore;
        }
    }
    
    println!("📊 CAMPAIGN RESULTS:");
    println!("   Total iron ore mined this campaign: {}", campaign_iron_ore);
    println!("   Total iron ore across fleet: {}", final_iron_ore);
    println!("   Target achieved: {}", if final_iron_ore >= 100 { "YES ✅" } else { "NO ❌" });
    
    if final_iron_ore >= 100 {
        println!("🎉 SUCCESS: Ready for refinery operations!");
        println!("💡 Next: Run refinery validation example");
    } else {
        println!("⚠️ Need {} more iron ore units", 100 - final_iron_ore);
        println!("💡 Run this mining campaign again or check mining efficiency");
    }
    
    Ok(())
}