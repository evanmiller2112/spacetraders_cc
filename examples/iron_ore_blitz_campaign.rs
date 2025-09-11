// HIGH-EFFICIENCY IRON ORE BLITZ CAMPAIGN
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("🚀🚀🚀 IRON ORE BLITZ CAMPAIGN 🚀🚀🚀");
    println!("=====================================");
    println!("🎯 MISSION: GET 100+ IRON ORE UNITS FAST!");
    println!("💡 STRATEGY: Coordinated survey + targeted mining");
    
    // Our proven dream team
    let surveyor = "CLAUDE_AGENT_2-1";  // Frigate with MOUNT_SURVEYOR_II (100% success rate!)
    let miners = vec!["CLAUDE_AGENT_2-3", "CLAUDE_AGENT_2-4", "CLAUDE_AGENT_2-5", "CLAUDE_AGENT_2-6"];
    let mining_location = "X1-N5-BA5F"; // ENGINEERED_ASTEROID with confirmed iron ore
    
    println!("\n🎯 BLITZ TEAM ASSEMBLED:");
    println!("   📊 Master Surveyor: {} (MOUNT_SURVEYOR_II)", surveyor);
    println!("   ⛏️ Mining Squadron: {} ships with MOUNT_MINING_LASER_I", miners.len());
    println!("   📍 Target Zone: {}", mining_location);
    
    // Check starting iron ore
    let ships = priority_client.get_ships().await?;
    let mut starting_iron_ore = 0;
    for ship in &ships {
        let iron_ore: i32 = ship.cargo.inventory.iter()
            .filter(|item| item.symbol == "IRON_ORE")
            .map(|item| item.units)
            .sum();
        starting_iron_ore += iron_ore;
    }
    
    println!("\n📊 MISSION STATUS:");
    println!("   Starting iron ore: {} units", starting_iron_ore);
    println!("   Target: 100 units");
    println!("   Needed: {} units", std::cmp::max(0, 100 - starting_iron_ore));
    
    if starting_iron_ore >= 100 {
        println!("🎉 MISSION ALREADY COMPLETE!");
        return Ok(());
    }
    
    // BLITZ CAMPAIGN: Multiple rapid cycles
    let max_cycles = 8; // Enough to get 100+ units
    let mut total_mined_this_campaign = 0;
    
    for cycle in 1..=max_cycles {
        println!("\n🔄🔄🔄 BLITZ CYCLE {}/{} 🔄🔄🔄", cycle, max_cycles);
        println!("===============================");
        
        // Phase 1: Position surveyor (if needed)
        let surveyor_ship = priority_client.get_ship(surveyor).await?;
        if surveyor_ship.nav.waypoint_symbol != mining_location {
            println!("🚀 Moving surveyor to target zone...");
            
            if surveyor_ship.nav.status == "DOCKED" {
                priority_client.orbit_ship(surveyor).await?;
            }
            
            match priority_client.navigate_ship(surveyor, mining_location).await {
                Ok(nav_result) => {
                    if let Ok(arrival_time) = nav_result.nav.route.arrival.parse::<chrono::DateTime<chrono::Utc>>() {
                        let now = chrono::Utc::now();
                        let wait_seconds = std::cmp::min((arrival_time - now).num_seconds().max(0) as u64, 180);
                        if wait_seconds > 0 {
                            println!("⏳ Surveyor en route, {} seconds...", wait_seconds);
                            tokio::time::sleep(tokio::time::Duration::from_secs(wait_seconds + 3)).await;
                        }
                    }
                }
                Err(e) => {
                    println!("⚠️ Surveyor navigation issue: {}", e);
                    // Continue anyway - might be fuel related
                }
            }
        }
        
        // Phase 2: Create iron ore survey
        println!("📊 Creating targeted survey...");
        
        // Ensure surveyor is in orbit
        let current_surveyor = priority_client.get_ship(surveyor).await?;
        if current_surveyor.nav.status != "IN_ORBIT" {
            priority_client.orbit_ship(surveyor).await?;
        }
        
        match priority_client.create_survey(surveyor).await {
            Ok(survey_data) => {
                // Find ALL iron ore surveys
                let iron_surveys: Vec<_> = survey_data.surveys.iter()
                    .filter(|survey| survey.deposits.iter().any(|d| d.symbol == "IRON_ORE"))
                    .collect();
                
                println!("✅ Survey complete: {} iron ore surveys found!", iron_surveys.len());
                
                if iron_surveys.is_empty() {
                    println!("⚠️ No iron ore in this survey cycle - rare but happens");
                    continue;
                }
                
                // Phase 3: BLITZ MINING with all available miners
                println!("⛏️⛏️⛏️ COMMENCING BLITZ MINING! ⛏️⛏️⛏️");
                
                let mut cycle_iron_ore = 0;
                
                // Use multiple miners with multiple surveys
                for (miner_index, miner) in miners.iter().enumerate() {
                    if miner_index >= iron_surveys.len() {
                        break; // More miners than surveys
                    }
                    
                    let survey = iron_surveys[miner_index];
                    
                    println!("\n🎯 Miner {} targeting survey {} with deposits: {:?}", 
                             miner, miner_index + 1, 
                             survey.deposits.iter().map(|d| &d.symbol).collect::<Vec<_>>());
                    
                    // Check miner readiness
                    let miner_ship = priority_client.get_ship(miner).await?;
                    
                    // Navigate miner if needed
                    if miner_ship.nav.waypoint_symbol != mining_location {
                        if miner_ship.fuel.current > 50 { // Only if has enough fuel
                            println!("🚀 Moving {} to target zone...", miner);
                            
                            if miner_ship.nav.status == "DOCKED" {
                                priority_client.orbit_ship(miner).await?;
                            }
                            
                            match priority_client.navigate_ship(miner, mining_location).await {
                                Ok(_) => {
                                    tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                                }
                                Err(e) => {
                                    println!("⚠️ {} navigation failed: {}", miner, e);
                                    continue;
                                }
                            }
                        } else {
                            println!("⚠️ {} low fuel, skipping", miner);
                            continue;
                        }
                    }
                    
                    // Ensure miner is in orbit
                    let ready_miner = priority_client.get_ship(miner).await?;
                    if ready_miner.nav.status != "IN_ORBIT" {
                        priority_client.orbit_ship(miner).await?;
                    }
                    
                    // Check cargo space
                    if ready_miner.cargo.units >= ready_miner.cargo.capacity {
                        println!("📦 {} cargo full, skipping", miner);
                        continue;
                    }
                    
                    // TARGETED EXTRACTION!
                    println!("⛏️ {} extracting with iron ore survey...", miner);
                    
                    match priority_client.extract_resources_with_survey(miner, survey).await {
                        Ok(extraction_data) => {
                            let material = &extraction_data.extraction.extraction_yield.symbol;
                            let amount = extraction_data.extraction.extraction_yield.units;
                            
                            println!("✅ {}: {} x{}", miner, material, amount);
                            
                            if material == "IRON_ORE" {
                                cycle_iron_ore += amount;
                                total_mined_this_campaign += amount;
                                println!("🎉 IRON_ORE HIT! Cycle: +{}, Campaign: +{}", amount, total_mined_this_campaign);
                            }
                        }
                        Err(e) => {
                            println!("❌ {} extraction failed: {}", miner, e);
                        }
                    }
                    
                    // Brief pause between miners
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                }
                
                println!("\n📊 CYCLE {} RESULTS:", cycle);
                println!("   Iron ore mined: {} units", cycle_iron_ore);
                println!("   Campaign total: {} units", total_mined_this_campaign);
                
            }
            Err(e) => {
                println!("❌ Survey failed: {}", e);
                continue;
            }
        }
        
        // Check if we've hit our target
        let current_ships = priority_client.get_ships().await?;
        let mut current_total = 0;
        for ship in &current_ships {
            let iron_ore: i32 = ship.cargo.inventory.iter()
                .filter(|item| item.symbol == "IRON_ORE")
                .map(|item| item.units)
                .sum();
            current_total += iron_ore;
        }
        
        println!("📊 Fleet iron ore: {} units", current_total);
        
        if current_total >= 100 {
            println!("🎉🎉🎉 TARGET ACHIEVED! 🎉🎉🎉");
            break;
        }
        
        // Cooldown between cycles
        if cycle < max_cycles {
            println!("⏱️ Cooldown between cycles...");
            tokio::time::sleep(tokio::time::Duration::from_secs(20)).await;
        }
    }
    
    // FINAL MISSION STATUS
    println!("\n🏁🏁🏁 BLITZ CAMPAIGN COMPLETE! 🏁🏁🏁");
    println!("========================================");
    
    let final_ships = priority_client.get_ships().await?;
    let mut final_iron_ore = 0;
    
    println!("📊 FINAL IRON ORE INVENTORY:");
    for ship in &final_ships {
        let iron_ore: i32 = ship.cargo.inventory.iter()
            .filter(|item| item.symbol == "IRON_ORE")
            .map(|item| item.units)
            .sum();
        
        if iron_ore > 0 {
            println!("   ⛏️ {}: {} IRON_ORE", ship.symbol, iron_ore);
            final_iron_ore += iron_ore;
        }
    }
    
    println!("\n🎯 MISSION SUMMARY:");
    println!("   Starting iron ore: {} units", starting_iron_ore);
    println!("   Mined this campaign: {} units", total_mined_this_campaign);
    println!("   Final iron ore: {} units", final_iron_ore);
    println!("   Target: 100 units");
    
    if final_iron_ore >= 100 {
        println!("\n🎉🎉🎉 MISSION ACCOMPLISHED! 🎉🎉🎉");
        println!("🏭 READY FOR REFINERY OPERATIONS!");
        println!("💡 Next step: Transfer iron ore to refiner and start processing!");
    } else {
        println!("\n✅ EXCELLENT PROGRESS!");
        println!("   Progress: {}%", (final_iron_ore * 100) / 100);
        println!("   Still needed: {} units", 100 - final_iron_ore);
        println!("💡 Run this blitz campaign again to reach 100 units!");
    }
    
    Ok(())
}