// Persistent iron ore hunting - keep surveying until we find iron ore deposits!
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("🎯 PERSISTENT IRON ORE HUNT");
    println!("===========================");
    println!("💡 Strategy: Keep surveying until iron ore found, then extract!");
    
    let surveyor = "CLAUDE_AGENT_2-1";
    let mining_location = "X1-N5-BA5F";
    
    // Verify surveyor is positioned
    let surveyor_ship = priority_client.get_ship(surveyor).await?;
    println!("🔍 Surveyor status:");
    println!("   Location: {} ✅", surveyor_ship.nav.waypoint_symbol);
    println!("   Fuel: {}/{} ✅", surveyor_ship.fuel.current, surveyor_ship.fuel.capacity);
    
    if surveyor_ship.nav.waypoint_symbol != mining_location {
        println!("❌ Surveyor not at mining location");
        return Ok(());
    }
    
    // Find available miners
    let ships = priority_client.get_ships().await?;
    let mut available_miners = Vec::new();
    
    for ship in &ships {
        let has_mining_laser = ship.mounts.iter().any(|m| m.symbol.contains("MINING_LASER"));
        let has_cargo_space = ship.cargo.units < ship.cargo.capacity;
        let at_mining_location = ship.nav.waypoint_symbol == mining_location;
        
        if has_mining_laser && has_cargo_space && at_mining_location {
            available_miners.push(ship.symbol.clone());
        }
    }
    
    println!("⛏️ Available miners at location: {} ships", available_miners.len());
    for miner in &available_miners {
        println!("   - {}", miner);
    }
    
    if available_miners.is_empty() {
        println!("⚠️ No miners available at mining location");
        println!("💡 Move miners to {} or clear cargo space", mining_location);
        return Ok(());
    }
    
    // PERSISTENT HUNT: Keep surveying until iron ore found
    let max_survey_attempts = 20; // Reasonable limit
    let mut total_iron_ore_extracted = 0;
    
    for attempt in 1..=max_survey_attempts {
        println!("\n🔍 SURVEY ATTEMPT {}/{}", attempt, max_survey_attempts);
        println!("==============================");
        
        // Ensure surveyor is in orbit
        let current_surveyor = priority_client.get_ship(surveyor).await?;
        if current_surveyor.nav.status != "IN_ORBIT" {
            priority_client.orbit_ship(surveyor).await?;
        }
        
        // Create survey
        println!("📊 Creating survey...");
        match priority_client.create_survey(surveyor).await {
            Ok(survey_data) => {
                // Analyze for iron ore
                let iron_surveys: Vec<_> = survey_data.surveys.iter()
                    .filter(|survey| survey.deposits.iter().any(|d| d.symbol == "IRON_ORE"))
                    .collect();
                
                println!("✅ Survey complete!");
                println!("   Total surveys: {}", survey_data.surveys.len());
                println!("   🎯 Iron ore surveys: {}", iron_surveys.len());
                
                // Show all materials found
                let mut all_materials = std::collections::HashMap::new();
                for survey in &survey_data.surveys {
                    for deposit in &survey.deposits {
                        *all_materials.entry(&deposit.symbol).or_insert(0) += 1;
                    }
                }
                
                println!("   📦 Materials in this survey:");
                for (material, count) in &all_materials {
                    let icon = if *material == "IRON_ORE" { "🎯" } else { "📦" };
                    println!("      {} {}: {} instances", icon, material, count);
                }
                
                if iron_surveys.is_empty() {
                    println!("   ⚠️ No iron ore this round - continuing hunt...");
                    
                    // Brief pause before next survey
                    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                    continue;
                }
                
                // IRON ORE FOUND! Start extraction
                println!("\n🎉🎉🎉 IRON ORE SURVEYS FOUND! 🎉🎉🎉");
                println!("⛏️⛏️⛏️ COMMENCING EXTRACTION! ⛏️⛏️⛏️");
                
                // Use all available miners with iron ore surveys
                for (miner_index, miner_symbol) in available_miners.iter().enumerate() {
                    if miner_index >= iron_surveys.len() {
                        break; // More miners than iron surveys
                    }
                    
                    let survey = iron_surveys[miner_index];
                    
                    println!("\n🎯 {} extracting with iron ore survey {}", miner_symbol, miner_index + 1);
                    println!("   Survey deposits: {:?}", 
                             survey.deposits.iter().map(|d| &d.symbol).collect::<Vec<_>>());
                    
                    // Ensure miner is ready
                    let miner_ship = priority_client.get_ship(miner_symbol).await?;
                    
                    if miner_ship.cargo.units >= miner_ship.cargo.capacity {
                        println!("   📦 {} cargo full, skipping", miner_symbol);
                        continue;
                    }
                    
                    if miner_ship.nav.status != "IN_ORBIT" {
                        priority_client.orbit_ship(miner_symbol).await?;
                    }
                    
                    // TARGETED EXTRACTION!
                    match priority_client.extract_resources_with_survey(miner_symbol, survey).await {
                        Ok(extraction_data) => {
                            let material = &extraction_data.extraction.extraction_yield.symbol;
                            let amount = extraction_data.extraction.extraction_yield.units;
                            
                            println!("   ✅ {}: {} x{}", miner_symbol, material, amount);
                            
                            if material == "IRON_ORE" {
                                total_iron_ore_extracted += amount;
                                println!("   🎉 IRON_ORE SUCCESS! Session total: {} units", total_iron_ore_extracted);
                            }
                        }
                        Err(e) => {
                            println!("   ❌ {} extraction failed: {}", miner_symbol, e);
                        }
                    }
                    
                    // Brief pause between miners
                    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                }
                
                println!("\n📊 EXTRACTION ROUND COMPLETE!");
                println!("   Iron ore extracted this round: {} units", total_iron_ore_extracted);
                
                // Continue hunting for more if we haven't hit a good amount
                if total_iron_ore_extracted < 20 {
                    println!("💡 Continuing hunt for more iron ore...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;
                } else {
                    println!("🎯 Good haul! Mission successful!");
                    break;
                }
                
            }
            Err(e) => {
                println!("❌ Survey failed: {}", e);
                
                if e.to_string().contains("cooldown") {
                    println!("⏱️ Survey cooldown - waiting...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                } else {
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        }
    }
    
    // Final status
    println!("\n🏁 PERSISTENT HUNT COMPLETE!");
    println!("=============================");
    
    let final_ships = priority_client.get_ships().await?;
    let mut total_fleet_iron_ore = 0;
    
    println!("📊 FINAL IRON ORE INVENTORY:");
    for ship in &final_ships {
        let iron_ore: i32 = ship.cargo.inventory.iter()
            .filter(|item| item.symbol == "IRON_ORE")
            .map(|item| item.units)
            .sum();
        
        if iron_ore > 0 {
            println!("   ⛏️ {}: {} IRON_ORE", ship.symbol, iron_ore);
            total_fleet_iron_ore += iron_ore;
        }
    }
    
    println!("\n🎯 HUNT RESULTS:");
    println!("   Iron ore extracted this session: {} units", total_iron_ore_extracted);
    println!("   Total fleet iron ore: {} units", total_fleet_iron_ore);
    println!("   Target: 100 units");
    println!("   Progress: {}%", (total_fleet_iron_ore * 100) / 100);
    
    if total_fleet_iron_ore >= 100 {
        println!("\n🎉🎉🎉 MISSION ACCOMPLISHED! 🎉🎉🎉");
        println!("🏭 READY FOR REFINERY OPERATIONS!");
    } else if total_iron_ore_extracted > 0 {
        println!("\n✅ SUCCESSFUL HUNT!");
        println!("💡 Found and extracted {} iron ore units", total_iron_ore_extracted);
        println!("💡 Run this hunt again to reach 100 units");
    } else {
        println!("\n⚠️ No iron ore found in {} attempts", max_survey_attempts);
        println!("💡 Iron ore deposits might be rare at this location");
        println!("💡 Try different mining locations or run hunt again");
    }
    
    Ok(())
}