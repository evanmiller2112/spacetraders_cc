// Intensive mining campaign at X1-N5-BA5F with CLAUDE_AGENT_2-6
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("⛏️ INTENSIVE MINING AT X1-N5-BA5F");
    println!("==================================");
    
    let miner_symbol = "CLAUDE_AGENT_2-6";
    let mining_location = "X1-N5-BA5F"; // ENGINEERED_ASTEROID with mining capability
    
    // Check miner status
    let miner = priority_client.get_ship(miner_symbol).await?;
    println!("🔍 Miner status:");
    println!("   Location: {} (should be {})", miner.nav.waypoint_symbol, mining_location);
    println!("   Fuel: {}/{}", miner.fuel.current, miner.fuel.capacity);
    println!("   Cargo: {}/{}", miner.cargo.units, miner.cargo.capacity);
    
    // Check current iron ore
    let current_iron_ore: i32 = miner.cargo.inventory.iter()
        .filter(|item| item.symbol == "IRON_ORE")
        .map(|item| item.units)
        .sum();
    println!("   Current IRON_ORE: {}", current_iron_ore);
    
    if miner.nav.waypoint_symbol != mining_location {
        println!("❌ Miner not at expected location");
        return Ok(());
    }
    
    // Ensure miner is in orbit for mining
    if miner.nav.status != "IN_ORBIT" {
        println!("🚀 Putting miner in orbit...");
        priority_client.orbit_ship(miner_symbol).await?;
    }
    
    println!("\n⛏️ Starting intensive iron ore mining campaign...");
    println!("🎯 Goal: Find iron ore deposits and extract systematically");
    
    let mut total_iron_ore_mined = 0;
    let mut total_extractions = 0;
    let max_attempts = 50; // Increased attempts for thorough mining
    
    for attempt in 1..=max_attempts {
        // Check cargo space
        let updated_miner = priority_client.get_ship(miner_symbol).await?;
        if updated_miner.cargo.units >= updated_miner.cargo.capacity {
            println!("📦 Cargo full - stopping mining");
            break;
        }
        
        println!("\n🔄 Mining attempt {}/{}", attempt, max_attempts);
        
        // Strategy 1: Try survey-based extraction for iron ore targeting
        println!("🔍 Creating survey for targeted extraction...");
        match priority_client.create_survey(miner_symbol).await {
            Ok(survey_data) => {
                println!("📊 Survey created with {} results", survey_data.surveys.len());
                
                // Look specifically for iron ore in surveys
                let iron_surveys: Vec<_> = survey_data.surveys.iter()
                    .filter(|survey| survey.deposits.iter().any(|d| d.symbol == "IRON_ORE"))
                    .collect();
                
                if !iron_surveys.is_empty() {
                    println!("🎯 Found {} surveys with IRON_ORE!", iron_surveys.len());
                    
                    // Use the first iron ore survey
                    let iron_survey = iron_surveys[0];
                    println!("📊 Using survey with deposits: {:?}", 
                             iron_survey.deposits.iter().map(|d| &d.symbol).collect::<Vec<_>>());
                    
                    match priority_client.extract_resources_with_survey(miner_symbol, iron_survey).await {
                        Ok(extraction_data) => {
                            let material = &extraction_data.extraction.extraction_yield.symbol;
                            let amount = extraction_data.extraction.extraction_yield.units;
                            total_extractions += 1;
                            
                            println!("✅ Survey extraction: {} x{}", material, amount);
                            
                            if material == "IRON_ORE" {
                                total_iron_ore_mined += amount;
                                println!("🎉 IRON_ORE! Campaign total: {}", total_iron_ore_mined);
                            }
                        }
                        Err(e) => {
                            println!("❌ Survey extraction failed: {}", e);
                        }
                    }
                } else {
                    println!("⚠️ No IRON_ORE found in surveys, trying basic extraction...");
                    
                    // Fallback to basic extraction
                    match priority_client.extract_resources(miner_symbol).await {
                        Ok(extraction_data) => {
                            let material = &extraction_data.extraction.extraction_yield.symbol;
                            let amount = extraction_data.extraction.extraction_yield.units;
                            total_extractions += 1;
                            
                            println!("✅ Basic extraction: {} x{}", material, amount);
                            
                            if material == "IRON_ORE" {
                                total_iron_ore_mined += amount;
                                println!("🎉 IRON_ORE! Campaign total: {}", total_iron_ore_mined);
                            }
                        }
                        Err(e) => {
                            println!("❌ Basic extraction failed: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                println!("❌ Survey failed: {}", e);
                
                // Fallback to basic extraction without survey
                println!("🔄 Trying basic extraction without survey...");
                match priority_client.extract_resources(miner_symbol).await {
                    Ok(extraction_data) => {
                        let material = &extraction_data.extraction.extraction_yield.symbol;
                        let amount = extraction_data.extraction.extraction_yield.units;
                        total_extractions += 1;
                        
                        println!("✅ No-survey extraction: {} x{}", material, amount);
                        
                        if material == "IRON_ORE" {
                            total_iron_ore_mined += amount;
                            println!("🎉 IRON_ORE! Campaign total: {}", total_iron_ore_mined);
                        }
                    }
                    Err(e) => {
                        println!("❌ No-survey extraction failed: {}", e);
                    }
                }
            }
        }
        
        // Brief pause to avoid overwhelming the API
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        
        // Progress update every 10 attempts
        if attempt % 10 == 0 {
            println!("\n📊 Progress after {} attempts:", attempt);
            println!("   Iron ore mined this session: {}", total_iron_ore_mined);
            println!("   Total successful extractions: {}", total_extractions);
            
            let current_miner = priority_client.get_ship(miner_symbol).await?;
            let current_total_iron: i32 = current_miner.cargo.inventory.iter()
                .filter(|item| item.symbol == "IRON_ORE")
                .map(|item| item.units)
                .sum();
            println!("   Total IRON_ORE in cargo: {}", current_total_iron);
            
            if current_total_iron >= 30 {
                println!("🎯 Good progress! Continuing until cargo full or target reached...");
            }
        }
    }
    
    // Final status check
    println!("\n📊 FINAL MINING CAMPAIGN RESULTS:");
    println!("=======================================");
    
    let final_miner = priority_client.get_ship(miner_symbol).await?;
    let final_iron_ore: i32 = final_miner.cargo.inventory.iter()
        .filter(|item| item.symbol == "IRON_ORE")
        .map(|item| item.units)
        .sum();
    
    println!("   Iron ore mined this session: {}", total_iron_ore_mined);
    println!("   Total successful extractions: {}", total_extractions);
    println!("   IRON_ORE in miner cargo: {}", final_iron_ore);
    
    // Check entire fleet iron ore
    println!("\n📊 FLEET IRON ORE INVENTORY:");
    let all_ships = priority_client.get_ships().await?;
    let mut fleet_iron_ore = 0;
    
    for ship in &all_ships {
        let ship_iron_ore: i32 = ship.cargo.inventory.iter()
            .filter(|item| item.symbol == "IRON_ORE")
            .map(|item| item.units)
            .sum();
        
        if ship_iron_ore > 0 {
            println!("   ⛏️ {}: {} IRON_ORE", ship.symbol, ship_iron_ore);
            fleet_iron_ore += ship_iron_ore;
        }
    }
    
    println!("\n🎯 CAMPAIGN SUMMARY:");
    println!("   Total fleet IRON_ORE: {}", fleet_iron_ore);
    println!("   Target needed: 100 units");
    println!("   Still needed: {} units", std::cmp::max(0, 100 - fleet_iron_ore));
    
    if fleet_iron_ore >= 100 {
        println!("🎉 SUCCESS: Target achieved! Ready for refinery operations!");
        println!("💡 Next: Transfer iron ore to refiner and start processing");
    } else if fleet_iron_ore > 50 {
        println!("✅ GOOD PROGRESS: Over halfway to target");
        println!("💡 Continue mining campaign to reach 100 units");
    } else {
        println!("⚠️ More mining needed");
        println!("💡 Consider:");
        println!("   - Running this campaign again");
        println!("   - Moving other ships to mineable locations");
        println!("   - Trading for iron ore");
    }
    
    Ok(())
}