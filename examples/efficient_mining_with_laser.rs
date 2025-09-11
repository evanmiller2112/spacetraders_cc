// Efficient mining using existing mining laser, respecting cooldowns
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("⛏️ EFFICIENT MINING WITH LASER AT X1-N5-BA5F");
    println!("=============================================");
    
    let miner_symbol = "CLAUDE_AGENT_2-6";
    
    // Check miner status
    let miner = priority_client.get_ship(miner_symbol).await?;
    println!("🔍 Miner status:");
    println!("   Location: {}", miner.nav.waypoint_symbol);
    println!("   Has MINING_LASER: {}", 
             miner.mounts.iter().any(|m| m.symbol.contains("MINING_LASER")));
    println!("   Cargo: {}/{}", miner.cargo.units, miner.cargo.capacity);
    
    // Ensure in orbit for mining
    if miner.nav.status != "IN_ORBIT" {
        println!("🚀 Putting miner in orbit...");
        priority_client.orbit_ship(miner_symbol).await?;
    }
    
    println!("\n⛏️ PATIENT MINING STRATEGY");
    println!("💡 Mining laser equipped - should be efficient!");
    println!("💡 Working within 70-second cooldowns");
    println!("💡 Target: Find iron ore among random extractions");
    
    let mut iron_ore_found = 0;
    let mut total_extractions = 0;
    let mut materials_found = std::collections::HashMap::new();
    
    // Run mining for a reasonable number of cycles
    let max_cycles = 30; // About 35+ minutes with cooldowns
    
    for cycle in 1..=max_cycles {
        println!("\n🔄 Mining cycle {}/{}", cycle, max_cycles);
        
        // Check cargo space
        let current_miner = priority_client.get_ship(miner_symbol).await?;
        if current_miner.cargo.units >= current_miner.cargo.capacity {
            println!("📦 Cargo full! Stopping mining.");
            break;
        }
        
        // Attempt extraction
        println!("⛏️ Extracting with mining laser...");
        match priority_client.extract_resources(miner_symbol).await {
            Ok(extraction_data) => {
                let material = &extraction_data.extraction.extraction_yield.symbol;
                let amount = extraction_data.extraction.extraction_yield.units;
                total_extractions += 1;
                
                // Track all materials found
                *materials_found.entry(material.clone()).or_insert(0) += amount;
                
                println!("✅ Extracted: {} x{}", material, amount);
                
                if material == "IRON_ORE" {
                    iron_ore_found += amount;
                    println!("🎉 IRON_ORE! Total iron ore this session: {}", iron_ore_found);
                }
                
                // Show cooldown info from the response
                let cooldown_secs = extraction_data.cooldown.total_seconds;
                println!("⏱️ Cooldown: {} seconds", cooldown_secs);
                if cooldown_secs > 0.0 {
                    println!("⏳ Waiting for cooldown...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(cooldown_secs as u64 + 2)).await;
                } else {
                    // Default cooldown wait
                    println!("⏳ Default cooldown wait...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(72)).await;
                }
            }
            Err(e) => {
                println!("❌ Extraction failed: {}", e);
                
                if e.to_string().contains("cooldown") {
                    // Parse cooldown time if possible
                    if let Some(start) = e.to_string().find("for ") {
                        if let Some(end) = e.to_string()[start + 4..].find(" second") {
                            if let Ok(cooldown) = e.to_string()[start + 4..start + 4 + end].parse::<u64>() {
                                println!("⏳ Waiting {} seconds for cooldown...", cooldown);
                                tokio::time::sleep(tokio::time::Duration::from_secs(cooldown + 2)).await;
                            } else {
                                tokio::time::sleep(tokio::time::Duration::from_secs(72)).await;
                            }
                        }
                    } else {
                        tokio::time::sleep(tokio::time::Duration::from_secs(72)).await;
                    }
                } else {
                    println!("💡 Non-cooldown error - continuing after brief pause");
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        }
        
        // Progress report every 5 cycles
        if cycle % 5 == 0 {
            println!("\n📊 Progress after {} cycles:", cycle);
            println!("   Iron ore found: {} units", iron_ore_found);
            println!("   Total successful extractions: {}", total_extractions);
            println!("   Materials discovered:");
            for (material, amount) in &materials_found {
                println!("      {} x{}", material, amount);
            }
            
            if iron_ore_found >= 10 {
                println!("🎯 Good iron ore progress! Continuing...");
            }
        }
    }
    
    // Final summary
    println!("\n📊 FINAL MINING SESSION RESULTS");
    println!("=================================");
    
    let final_miner = priority_client.get_ship(miner_symbol).await?;
    println!("🔍 Final miner status:");
    println!("   Cargo: {}/{}", final_miner.cargo.units, final_miner.cargo.capacity);
    
    // Check all cargo for iron ore
    let total_iron_ore_in_cargo: i32 = final_miner.cargo.inventory.iter()
        .filter(|item| item.symbol == "IRON_ORE")
        .map(|item| item.units)
        .sum();
    
    println!("\n⛏️ Mining session results:");
    println!("   Iron ore mined this session: {}", iron_ore_found);
    println!("   Total iron ore in cargo: {}", total_iron_ore_in_cargo);
    println!("   Total successful extractions: {}", total_extractions);
    
    if total_extractions > 0 {
        let iron_ore_rate = (iron_ore_found as f64 / total_extractions as f64) * 100.0;
        println!("   Iron ore hit rate: {:.1}%", iron_ore_rate);
    }
    
    println!("\n🎯 All materials found:");
    for (material, amount) in materials_found.iter() {
        println!("   {} x{}", material, amount);
    }
    
    // Check total fleet iron ore
    println!("\n📊 FLEET IRON ORE STATUS:");
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
    
    println!("\n🎯 OVERALL PROGRESS:");
    println!("   Total fleet IRON_ORE: {}", fleet_iron_ore);
    println!("   Target needed: 100 units");
    println!("   Still needed: {} units", std::cmp::max(0, 100 - fleet_iron_ore));
    
    if fleet_iron_ore >= 100 {
        println!("🎉 SUCCESS: Target achieved! Ready for refinery operations!");
        println!("💡 Next: Transfer iron ore to refiner and start processing");
    } else if iron_ore_found > 5 {
        println!("✅ GOOD SESSION: Found {} iron ore this round", iron_ore_found);
        println!("💡 Continue mining sessions to reach target");
    } else if total_extractions > 10 {
        println!("⚠️ Low iron ore yield this session");
        println!("💡 Iron ore might be rare at this location");
        println!("💡 Consider trying different asteroid locations");
    } else {
        println!("⚠️ Short session or technical issues");
        println!("💡 Try running mining session again");
    }
    
    Ok(())
}