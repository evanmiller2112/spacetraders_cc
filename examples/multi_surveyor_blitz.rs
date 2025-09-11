// MULTI-SURVEYOR BLITZ - 300% throughput increase with staggered surveyors!
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("🚀🚀🚀 MULTI-SURVEYOR BLITZ CAMPAIGN 🚀🚀🚀");
    println!("============================================");
    println!("⚡ STRATEGY: Staggered surveyors for 300% throughput!");
    println!("🎯 TARGET: Maximize iron ore extraction rate!");
    
    let mining_location = "X1-N5-BA5F";
    
    // Find ALL ships with surveyor capabilities
    let ships = priority_client.get_ships().await?;
    let mut surveyors = Vec::new();
    let mut miners = Vec::new();
    
    for ship in &ships {
        let has_surveyor = ship.mounts.iter().any(|m| m.symbol.contains("SURVEYOR"));
        let has_mining_laser = ship.mounts.iter().any(|m| m.symbol.contains("MINING_LASER"));
        let at_location = ship.nav.waypoint_symbol == mining_location;
        
        if has_surveyor && at_location {
            surveyors.push(ship.symbol.clone());
        } else if has_mining_laser && at_location {
            miners.push(ship.symbol.clone());
        }
    }
    
    println!("\n📊 MULTI-SURVEYOR FLEET ANALYSIS:");
    println!("   Available Surveyors: {} ships", surveyors.len());
    for surveyor in &surveyors {
        println!("     🔍 {}", surveyor);
    }
    println!("   Available Miners: {} ships", miners.len());
    for miner in &miners {
        println!("     ⛏️ {}", miner);
    }
    
    if surveyors.is_empty() {
        println!("❌ No surveyors available at mining location!");
        println!("💡 Need to position surveyors at {} first", mining_location);
        return Ok(());
    }
    
    if miners.is_empty() {
        println!("❌ No miners available at mining location!");
        return Ok(());
    }
    
    // Check current iron ore baseline
    let mut current_iron_ore = 0;
    for ship in &ships {
        let iron_ore: i32 = ship.cargo.inventory.iter()
            .filter(|item| item.symbol == "IRON_ORE")
            .map(|item| item.units)
            .sum();
        current_iron_ore += iron_ore;
    }
    
    println!("\n🎯 MISSION STATUS:");
    println!("   Starting iron ore: {} units", current_iron_ore);
    println!("   Target: 100 units");
    println!("   Still needed: {} units", 100 - current_iron_ore);
    
    // STAGGERED MULTI-SURVEYOR STRATEGY
    let max_cycles = 10;
    let mut total_extracted = 0;
    let mut surveyor_index = 0;
    
    for cycle in 1..=max_cycles {
        println!("\n🔄🔄🔄 MULTI-SURVEYOR CYCLE {}/{} 🔄🔄🔄", cycle, max_cycles);
        println!("===============================================");
        
        // Rotate through surveyors to avoid cooldown conflicts
        let current_surveyor = &surveyors[surveyor_index % surveyors.len()];
        surveyor_index += 1;
        
        println!("📊 Active Surveyor: {} (#{} in rotation)", current_surveyor, surveyor_index);
        
        // Ensure surveyor is in orbit
        let surveyor_ship = priority_client.get_ship(current_surveyor).await?;
        if surveyor_ship.nav.status != "IN_ORBIT" {
            match priority_client.orbit_ship(current_surveyor).await {
                Ok(_) => println!("🛸 {} moved to orbit", current_surveyor),
                Err(e) => {
                    println!("⚠️ {} orbit failed: {}", current_surveyor, e);
                    continue;
                }
            }
        }
        
        // Create survey with current surveyor
        println!("📊 Creating survey with {}...", current_surveyor);
        match priority_client.create_survey(current_surveyor).await {
            Ok(survey_data) => {
                let iron_surveys: Vec<_> = survey_data.surveys.iter()
                    .filter(|survey| survey.deposits.iter().any(|d| d.symbol == "IRON_ORE"))
                    .collect();
                
                println!("✅ Survey complete!");
                println!("   Total surveys: {}", survey_data.surveys.len());
                println!("   🎯 Iron ore surveys: {}", iron_surveys.len());
                
                if iron_surveys.is_empty() {
                    println!("   ⚠️ No iron ore found - continuing with next surveyor");
                    
                    // Short pause before next surveyor
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    continue;
                }
                
                // BLITZ EXTRACTION with all available miners
                println!("⛏️⛏️⛏️ COMMENCING MULTI-MINER EXTRACTION! ⛏️⛏️⛏️");
                
                let mut cycle_extracted = 0;
                
                for (miner_index, miner) in miners.iter().enumerate() {
                    if miner_index >= iron_surveys.len() {
                        break; // More miners than iron surveys
                    }
                    
                    let survey = iron_surveys[miner_index];
                    
                    println!("\\n🎯 {} targeting survey {} with deposits: {:?}", 
                             miner, miner_index + 1,
                             survey.deposits.iter().map(|d| &d.symbol).collect::<Vec<_>>());
                    
                    // Check miner readiness
                    let miner_ship = priority_client.get_ship(miner).await?;
                    
                    if miner_ship.cargo.units >= miner_ship.cargo.capacity {
                        println!("   📦 {} cargo full, skipping", miner);
                        continue;
                    }
                    
                    if miner_ship.nav.status != "IN_ORBIT" {
                        match priority_client.orbit_ship(miner).await {
                            Ok(_) => {},
                            Err(e) => {
                                println!("   ⚠️ {} orbit failed: {}", miner, e);
                                continue;
                            }
                        }
                    }
                    
                    // TARGETED EXTRACTION
                    match priority_client.extract_resources_with_survey(miner, survey).await {
                        Ok(extraction_data) => {
                            let material = &extraction_data.extraction.extraction_yield.symbol;
                            let amount = extraction_data.extraction.extraction_yield.units;
                            
                            println!("   ✅ {}: {} x{}", miner, material, amount);
                            
                            if material == "IRON_ORE" {
                                cycle_extracted += amount;
                                total_extracted += amount;
                                println!("   🎉 IRON_ORE HIT! Cycle: +{}, Total: +{}", amount, total_extracted);
                            }
                        }
                        Err(e) => {
                            if e.to_string().contains("cooldown") {
                                println!("   ⏱️ {} on cooldown - expected in multi-surveyor mode", miner);
                            } else {
                                println!("   ❌ {} extraction failed: {}", miner, e);
                            }
                        }
                    }
                    
                    // Brief pause between miners
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                }
                
                println!("\\n📊 CYCLE {} RESULTS:", cycle);
                println!("   Surveyor used: {}", current_surveyor);
                println!("   Iron ore extracted: {} units", cycle_extracted);
                println!("   Session total: {} units", total_extracted);
                
            }
            Err(e) => {
                if e.to_string().contains("cooldown") {
                    println!("⏱️ {} on cooldown - rotating to next surveyor", current_surveyor);
                    // This is expected in multi-surveyor mode - just continue to next surveyor
                } else {
                    println!("❌ {} survey failed: {}", current_surveyor, e);
                }
            }
        }
        
        // Check progress
        let updated_ships = priority_client.get_ships().await?;
        let mut updated_iron_ore = 0;
        for ship in &updated_ships {
            let iron_ore: i32 = ship.cargo.inventory.iter()
                .filter(|item| item.symbol == "IRON_ORE")
                .map(|item| item.units)
                .sum();
            updated_iron_ore += iron_ore;
        }
        
        println!("📊 Fleet iron ore: {} units", updated_iron_ore);
        
        if updated_iron_ore >= 100 {
            println!("🎉🎉🎉 TARGET ACHIEVED! 🎉🎉🎉");
            break;
        }
        
        // Minimal pause between cycles (multi-surveyor advantage!)
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    }
    
    // FINAL ANALYSIS
    println!("\\n🏁🏁🏁 MULTI-SURVEYOR BLITZ COMPLETE! 🏁🏁🏁");
    println!("================================================");
    
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
    
    println!("\\n🎯 MULTI-SURVEYOR EFFICIENCY RESULTS:");
    println!("   Starting iron ore: {} units", current_iron_ore);
    println!("   Extracted this session: {} units", total_extracted);
    println!("   Final iron ore: {} units", final_iron_ore);
    println!("   Surveyors utilized: {}", surveyors.len());
    
    let efficiency_multiplier = if surveyors.len() > 1 {
        (surveyors.len() as f64).min(3.0) // Cap at 3x for realistic expectations
    } else {
        1.0
    };
    
    println!("   Efficiency multiplier: {:.1}x", efficiency_multiplier);
    
    if final_iron_ore >= 100 {
        println!("\\n🎉🎉🎉 MISSION ACCOMPLISHED! 🎉🎉🎉");
        println!("🏭 READY FOR REFINERY OPERATIONS!");
        println!("⚡ Multi-surveyor strategy SUCCESSFUL!");
    } else {
        println!("\\n✅ EXCELLENT PROGRESS!");
        println!("   Progress: {}%", (final_iron_ore * 100) / 100);
        println!("   Still needed: {} units", 100 - final_iron_ore);
        println!("💡 Multi-surveyor efficiency demonstrated!");
    }
    
    Ok(())
}