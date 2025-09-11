// Coordinated survey and mining with proper timing
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("🎯 COORDINATED SURVEY & MINING OPERATION");
    println!("=======================================");
    
    // Use our discovered optimal team:
    let surveyor = "CLAUDE_AGENT_2-1";  // Frigate with MOUNT_SURVEYOR_II
    let miner = "CLAUDE_AGENT_2-6";     // Drone at BA5F with MOUNT_MINING_LASER_I  
    let mining_location = "X1-N5-BA5F"; // ENGINEERED_ASTEROID location
    
    println!("🎯 MINING TEAM:");
    println!("   📊 Surveyor: {} (MOUNT_SURVEYOR_II)", surveyor);
    println!("   ⛏️ Miner: {} (MOUNT_MINING_LASER_I)", miner);
    println!("   📍 Location: {}", mining_location);
    
    // Phase 1: Position surveyor at mining location
    println!("\n📍 PHASE 1: POSITIONING");
    println!("=====================");
    
    let surveyor_ship = priority_client.get_ship(surveyor).await?;
    
    if surveyor_ship.nav.waypoint_symbol != mining_location {
        println!("🚀 Moving surveyor to mining location...");
        
        // Check if already in transit
        if surveyor_ship.nav.status == "IN_TRANSIT" {
            // Parse arrival time and wait
            if let Ok(arrival_time) = surveyor_ship.nav.route.arrival.parse::<chrono::DateTime<chrono::Utc>>() {
                let now = chrono::Utc::now();
                let wait_seconds = (arrival_time - now).num_seconds().max(0) as u64 + 3;
                
                println!("⏳ Surveyor in transit, waiting {} seconds for arrival...", wait_seconds);
                tokio::time::sleep(tokio::time::Duration::from_secs(wait_seconds)).await;
            } else {
                println!("⏳ Waiting for surveyor arrival...");
                tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;
            }
        } else {
            // Navigate surveyor to location
            if surveyor_ship.nav.status == "DOCKED" {
                priority_client.orbit_ship(surveyor).await?;
            }
            
            match priority_client.navigate_ship(surveyor, mining_location).await {
                Ok(nav_result) => {
                    if let Ok(arrival_time) = nav_result.nav.route.arrival.parse::<chrono::DateTime<chrono::Utc>>() {
                        let now = chrono::Utc::now();
                        let wait_seconds = (arrival_time - now).num_seconds().max(0) as u64 + 3;
                        println!("⏳ Waiting {} seconds for surveyor arrival...", wait_seconds);
                        tokio::time::sleep(tokio::time::Duration::from_secs(wait_seconds)).await;
                    }
                }
                Err(e) => {
                    println!("❌ Surveyor navigation failed: {}", e);
                    return Ok(());
                }
            }
        }
    }
    
    // Verify surveyor arrival
    let arrived_surveyor = priority_client.get_ship(surveyor).await?;
    if arrived_surveyor.nav.waypoint_symbol != mining_location {
        println!("❌ Surveyor failed to reach mining location");
        return Ok(());
    }
    
    println!("✅ Surveyor positioned at {}", mining_location);
    
    // Phase 2: Create targeted survey
    println!("\n📊 PHASE 2: SURVEY CREATION");
    println!("==========================");
    
    // Ensure surveyor is in orbit for survey
    if arrived_surveyor.nav.status != "IN_ORBIT" {
        priority_client.orbit_ship(surveyor).await?;
        println!("🚀 Surveyor in orbit");
    }
    
    println!("🔍 Creating comprehensive survey...");
    match priority_client.create_survey(surveyor).await {
        Ok(survey_data) => {
            println!("✅ Survey successful! {} results found", survey_data.surveys.len());
            
            // Analyze all surveys for iron ore
            let mut iron_surveys = Vec::new();
            let mut other_materials = std::collections::HashMap::new();
            
            for (i, survey) in survey_data.surveys.iter().enumerate() {
                let has_iron_ore = survey.deposits.iter().any(|d| d.symbol == "IRON_ORE");
                
                if has_iron_ore {
                    iron_surveys.push((i, survey));
                }
                
                // Track all materials
                for deposit in &survey.deposits {
                    *other_materials.entry(&deposit.symbol).or_insert(0) += 1;
                }
            }
            
            println!("\n📊 SURVEY ANALYSIS:");
            println!("   🎯 Iron ore surveys: {}", iron_surveys.len());
            println!("   📦 All materials found:");
            for (material, count) in &other_materials {
                let icon = if *material == "IRON_ORE" { "🎯" } else { "📦" };
                println!("      {} {}: {} surveys", icon, material, count);
            }
            
            if iron_surveys.is_empty() {
                println!("⚠️ No iron ore found in surveys at this location");
                println!("💡 This asteroid may not contain iron ore deposits");
                return Ok(());
            }
            
            // Phase 3: Targeted iron ore mining
            println!("\n⛏️ PHASE 3: TARGETED MINING");
            println!("==========================");
            
            // Ensure miner is ready
            let miner_ship = priority_client.get_ship(miner).await?;
            
            // Clear cargo space if full
            if miner_ship.cargo.units >= miner_ship.cargo.capacity {
                println!("📦 Miner cargo full - clearing space for iron ore...");
                
                // First, try to dock and sell/jettison some cargo
                if miner_ship.nav.status != "DOCKED" {
                    // Navigate to a marketplace to sell cargo
                    println!("🚀 Moving to marketplace to clear cargo...");
                    
                    // Try nearby marketplace at BA5F or A1
                    let marketplaces = ["X1-N5-BA5F", "X1-N5-A1"];
                    
                    for marketplace in &marketplaces {
                        match priority_client.navigate_ship(miner, marketplace).await {
                            Ok(_) => {
                                println!("⏳ Navigating to marketplace {}...", marketplace);
                                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                                
                                priority_client.dock_ship(miner).await?;
                                
                                // Sell non-iron ore materials
                                let updated_miner = priority_client.get_ship(miner).await?;
                                for item in &updated_miner.cargo.inventory {
                                    if item.symbol != "IRON_ORE" && item.units > 0 {
                                        match priority_client.sell_cargo(miner, &item.symbol, item.units).await {
                                            Ok(_) => {
                                                println!("💰 Sold {} x{}", item.symbol, item.units);
                                            }
                                            Err(_) => {
                                                println!("⚠️ Could not sell {}", item.symbol);
                                            }
                                        }
                                    }
                                }
                                
                                // Return to mining location
                                priority_client.orbit_ship(miner).await?;
                                priority_client.navigate_ship(miner, mining_location).await?;
                                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                                priority_client.orbit_ship(miner).await?;
                                
                                break;
                            }
                            Err(_) => {
                                continue;
                            }
                        }
                    }
                }
            }
            
            println!("🎯 Starting targeted iron ore extraction...");
            
            let mut total_iron_ore_extracted = 0;
            let max_attempts = std::cmp::min(iron_surveys.len(), 5); // Limit attempts
            
            for (attempt, (survey_index, survey)) in iron_surveys.iter().enumerate() {
                if attempt >= max_attempts {
                    break;
                }
                
                println!("\n🎯 Using iron ore survey {} (attempt {}/{})", 
                         survey_index + 1, attempt + 1, max_attempts);
                
                println!("📊 Survey deposits: {:?}", 
                         survey.deposits.iter().map(|d| &d.symbol).collect::<Vec<_>>());
                
                match priority_client.extract_resources_with_survey(miner, survey).await {
                    Ok(extraction_data) => {
                        let material = &extraction_data.extraction.extraction_yield.symbol;
                        let amount = extraction_data.extraction.extraction_yield.units;
                        
                        println!("✅ Targeted extraction: {} x{}", material, amount);
                        
                        if material == "IRON_ORE" {
                            total_iron_ore_extracted += amount;
                            println!("🎉 IRON_ORE SUCCESS! Session total: {}", total_iron_ore_extracted);
                        }
                        
                        // Wait for cooldown before next extraction
                        let cooldown = extraction_data.cooldown.total_seconds as u64;
                        if attempt < max_attempts - 1 {  // Don't wait after last attempt
                            println!("⏱️ Cooldown: {} seconds", cooldown);
                            tokio::time::sleep(tokio::time::Duration::from_secs(cooldown + 2)).await;
                        }
                    }
                    Err(e) => {
                        println!("❌ Survey extraction failed: {}", e);
                        
                        if e.to_string().contains("cooldown") {
                            tokio::time::sleep(tokio::time::Duration::from_secs(75)).await;
                        }
                    }
                }
            }
            
            println!("\n🎉 TARGETED MINING COMPLETE!");
            println!("   Iron ore extracted this session: {}", total_iron_ore_extracted);
            
        }
        Err(e) => {
            println!("❌ Survey creation failed: {}", e);
            return Ok(());
        }
    }
    
    // Final status
    println!("\n📊 FINAL FLEET IRON ORE STATUS");
    println!("==============================");
    
    let final_ships = priority_client.get_ships().await?;
    let mut total_iron_ore = 0;
    
    for ship in &final_ships {
        let ship_iron_ore: i32 = ship.cargo.inventory.iter()
            .filter(|item| item.symbol == "IRON_ORE")
            .map(|item| item.units)
            .sum();
        
        if ship_iron_ore > 0 {
            println!("   ⛏️ {}: {} IRON_ORE", ship.symbol, ship_iron_ore);
            total_iron_ore += ship_iron_ore;
        }
    }
    
    println!("\n🎯 MISSION SUMMARY:");
    println!("   Total fleet IRON_ORE: {} units", total_iron_ore);
    println!("   Target: 100 units");  
    println!("   Progress: {}%", (total_iron_ore * 100) / 100);
    println!("   Still needed: {} units", std::cmp::max(0, 100 - total_iron_ore));
    
    if total_iron_ore >= 100 {
        println!("🎉 MISSION ACCOMPLISHED! Ready for refinery operations!");
    } else if total_iron_ore >= 50 {
        println!("✅ Excellent progress! Halfway to target!");
        println!("💡 Repeat this coordinated strategy to reach 100 units");
    } else {
        println!("💡 Good foundation established");
        println!("💡 This coordinated survey + mining approach is working");
        println!("💡 Continue running campaigns to build up iron ore reserves");
    }
    
    Ok(())
}