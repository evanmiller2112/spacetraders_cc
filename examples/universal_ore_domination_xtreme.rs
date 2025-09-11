// UNIVERSAL ORE DOMINATION XTREME - TOTAL MINING PWNERSHIP! 🔥⛏️💥
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OreDominationStats {
    ore_type: String,
    units_extracted: u32,
    market_value_estimate: u32,
    extraction_priority: u32,
    demand_level: String,
}

#[derive(Debug)]
struct UniversalMiningDominator {
    target_ores: Vec<String>,
    ore_stats: HashMap<String, OreDominationStats>,
    mining_efficiency: HashMap<String, f64>,
}

impl UniversalMiningDominator {
    fn new() -> Self {
        let target_ores = vec![
            "IRON_ORE".to_string(),
            "COPPER_ORE".to_string(),
            "ALUMINUM_ORE".to_string(),
            "SILVER_ORE".to_string(),
            "GOLD_ORE".to_string(),
            "PLATINUM_ORE".to_string(),
            "URANITE_ORE".to_string(),
            "MERITIUM_ORE".to_string(),
            "HYDROCARBON".to_string(),
            "QUARTZ_SAND".to_string(),
            "SILICON_CRYSTALS".to_string(),
            "PRECIOUS_STONES".to_string(),
            "DIAMONDS".to_string(),
            "ICE_WATER".to_string(),
        ];
        
        let mut ore_stats = HashMap::new();
        let mut mining_efficiency = HashMap::new();
        
        // Initialize all ores with default stats
        for ore in &target_ores {
            ore_stats.insert(ore.clone(), OreDominationStats {
                ore_type: ore.clone(),
                units_extracted: 0,
                market_value_estimate: 1,
                extraction_priority: 50,
                demand_level: "UNKNOWN".to_string(),
            });
            mining_efficiency.insert(ore.clone(), 1.0);
        }
        
        // Set initial priorities (higher = more valuable)
        if let Some(iron) = ore_stats.get_mut("IRON_ORE") {
            iron.extraction_priority = 100; // Highest priority for our current goal
            iron.market_value_estimate = 10;
        }
        if let Some(gold) = ore_stats.get_mut("GOLD_ORE") {
            gold.extraction_priority = 95;
            gold.market_value_estimate = 100;
        }
        if let Some(platinum) = ore_stats.get_mut("PLATINUM_ORE") {
            platinum.extraction_priority = 90;
            platinum.market_value_estimate = 200;
        }
        if let Some(copper) = ore_stats.get_mut("COPPER_ORE") {
            copper.extraction_priority = 80;
            copper.market_value_estimate = 5;
        }
        
        Self {
            target_ores,
            ore_stats,
            mining_efficiency,
        }
    }
    
    fn update_ore_stats(&mut self, ore_type: &str, units_extracted: u32) {
        if let Some(stats) = self.ore_stats.get_mut(ore_type) {
            stats.units_extracted += units_extracted;
            
            // Increase efficiency for successfully extracted ores
            if let Some(efficiency) = self.mining_efficiency.get_mut(ore_type) {
                *efficiency = (*efficiency * 1.1).min(5.0); // Cap at 5x efficiency
            }
        }
    }
    
    fn get_priority_ores(&self) -> Vec<String> {
        let mut ore_priority: Vec<_> = self.ore_stats.iter().collect();
        ore_priority.sort_by(|a, b| b.1.extraction_priority.cmp(&a.1.extraction_priority));
        ore_priority.into_iter().map(|(ore, _)| ore.clone()).collect()
    }
    
    fn print_domination_status(&self) {
        println!("\\n📊📊📊 UNIVERSAL ORE DOMINATION STATUS 📊📊📊");
        println!("================================================");
        
        let mut total_value = 0;
        let mut total_units = 0;
        
        for (ore, stats) in &self.ore_stats {
            if stats.units_extracted > 0 {
                let value = stats.units_extracted * stats.market_value_estimate;
                total_value += value;
                total_units += stats.units_extracted;
                
                let efficiency = self.mining_efficiency.get(ore).unwrap_or(&1.0);
                println!("   ⛏️ {}: {} units (value: {}, efficiency: {:.1}x)", 
                         ore, stats.units_extracted, value, efficiency);
            }
        }
        
        println!("\\n🎯 TOTAL DOMINATION:");
        println!("   Total ore units: {}", total_units);
        println!("   Estimated value: {}", total_value);
        println!("   Ore types dominated: {}", self.ore_stats.values().filter(|s| s.units_extracted > 0).count());
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("🔥🔥🔥🔥🔥 UNIVERSAL ORE DOMINATION XTREME 🔥🔥🔥🔥🔥");
    println!("======================================================");
    println!("💥 TOTAL MINING PWNERSHIP - ALL ORES!");
    println!("⛏️ DYNAMIC ORE EXTRACTION SYSTEM!");
    println!("🎯 MAXIMUM RESOURCE ACQUISITION!");
    
    let mining_location = "X1-N5-BA5F";
    let mut dominator = UniversalMiningDominator::new();
    
    println!("\\n🎯 TARGET ORES FOR DOMINATION:");
    for (i, ore) in dominator.get_priority_ores().iter().enumerate() {
        let stats = &dominator.ore_stats[ore];
        println!("   {}. {} (priority: {}, est. value: {})", 
                 i + 1, ore, stats.extraction_priority, stats.market_value_estimate);
    }
    
    // Analyze current fleet ore inventory
    let ships = priority_client.get_ships().await?;
    println!("\\n📊 CURRENT FLEET ORE INVENTORY:");
    
    let mut fleet_ores = HashMap::new();
    for ship in &ships {
        for item in &ship.cargo.inventory {
            if dominator.target_ores.contains(&item.symbol) {
                *fleet_ores.entry(item.symbol.clone()).or_insert(0) += item.units;
            }
        }
    }
    
    for (ore, units) in &fleet_ores {
        println!("   ⛏️ {}: {} units", ore, units);
        dominator.update_ore_stats(ore, *units as u32);
    }
    
    // Find mining fleet
    let mut surveyors = Vec::new();
    let mut miners = Vec::new();
    
    for ship in &ships {
        let has_surveyor = ship.mounts.iter().any(|m| m.symbol.contains("SURVEYOR"));
        let has_mining_laser = ship.mounts.iter().any(|m| m.symbol.contains("MINING_LASER"));
        let at_location = ship.nav.waypoint_symbol == mining_location;
        
        if has_surveyor && at_location {
            surveyors.push(ship.symbol.clone());
        }
        if has_mining_laser && at_location && ship.cargo.units < ship.cargo.capacity {
            miners.push(ship.symbol.clone());
        }
    }
    
    println!("\\n🚀 UNIVERSAL MINING FLEET:");
    println!("   🔍 Surveyors: {}", surveyors.len());
    println!("   ⛏️ Miners: {}", miners.len());
    
    if surveyors.is_empty() || miners.is_empty() {
        println!("\\n❌ INSUFFICIENT FLEET FOR DOMINATION!");
        println!("💡 Need surveyors and miners at {}", mining_location);
        return Ok(());
    }
    
    // UNIVERSAL ORE DOMINATION CAMPAIGN
    let max_domination_cycles = 15;
    let mut cycle_results = Vec::new();
    
    for cycle in 1..=max_domination_cycles {
        println!("\\n💥💥💥 UNIVERSAL DOMINATION CYCLE {}/{} 💥💥💥", cycle, max_domination_cycles);
        println!("=============================================");
        
        let cycle_start = std::time::Instant::now();
        let mut cycle_ore_extracted = HashMap::new();
        
        // DYNAMIC SURVEY PHASE - Look for ANY valuable ore
        println!("📊 UNIVERSAL SURVEY SCAN...");
        
        let surveyor = &surveyors[0]; // Use primary surveyor
        let surveyor_ship = priority_client.get_ship(surveyor).await?;
        
        if surveyor_ship.nav.status != "IN_ORBIT" {
            priority_client.orbit_ship(surveyor).await?;
        }
        
        match priority_client.create_survey(surveyor).await {
            Ok(survey_data) => {
                println!("✅ Survey complete: {} surveys generated", survey_data.surveys.len());
                
                // Analyze ALL ores in surveys
                let mut ore_survey_map = HashMap::new();
                for survey in &survey_data.surveys {
                    for deposit in &survey.deposits {
                        if dominator.target_ores.contains(&deposit.symbol) {
                            ore_survey_map.entry(deposit.symbol.clone())
                                .or_insert_with(Vec::new)
                                .push(survey);
                        }
                    }
                }
                
                println!("\\n🎯 ORE SURVEYS FOUND:");
                for (ore, surveys) in &ore_survey_map {
                    let priority = dominator.ore_stats[ore].extraction_priority;
                    println!("   ⛏️ {}: {} surveys (priority: {})", ore, surveys.len(), priority);
                }
                
                if ore_survey_map.is_empty() {
                    println!("   ⚠️ No target ores found - continuing domination...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                    continue;
                }
                
                // STRATEGIC EXTRACTION - Target highest priority ores first
                println!("\\n⚡⚡⚡ STRATEGIC EXTRACTION PHASE! ⚡⚡⚡");
                
                let priority_ores = dominator.get_priority_ores();
                let mut extraction_count = 0;
                
                for priority_ore in priority_ores {
                    if let Some(surveys) = ore_survey_map.get(&priority_ore) {
                        println!("\\n🎯 TARGETING {}: {} surveys available", priority_ore, surveys.len());
                        
                        // Use multiple miners for this ore type
                        for (miner_idx, miner) in miners.iter().enumerate() {
                            if miner_idx >= surveys.len() || extraction_count >= miners.len() {
                                break;
                            }
                            
                            let survey = surveys[miner_idx];
                            let miner_ship = priority_client.get_ship(miner).await?;
                            
                            if miner_ship.cargo.units >= miner_ship.cargo.capacity {
                                println!("   📦 {} cargo full, skipping", miner);
                                continue;
                            }
                            
                            if miner_ship.nav.status != "IN_ORBIT" {
                                priority_client.orbit_ship(miner).await?;
                            }
                            
                            print!("   ⛏️ {} extracting {} survey... ", miner, priority_ore);
                            match priority_client.extract_resources_with_survey(miner, survey).await {
                                Ok(extraction_data) => {
                                    let material = &extraction_data.extraction.extraction_yield.symbol;
                                    let amount = extraction_data.extraction.extraction_yield.units;
                                    
                                    println!("✅ {} x{}", material, amount);
                                    
                                    if dominator.target_ores.contains(material) {
                                        *cycle_ore_extracted.entry(material.clone()).or_insert(0) += amount;
                                        dominator.update_ore_stats(material, amount as u32);
                                        
                                        if material == &priority_ore {
                                            println!("      🎉 TARGET ORE HIT!");
                                        }
                                    }
                                }
                                Err(e) => {
                                    if e.to_string().contains("cooldown") {
                                        println!("⏱️ Cooldown");
                                    } else {
                                        println!("❌ Failed: {}", e);
                                    }
                                }
                            }
                            
                            extraction_count += 1;
                            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                        }
                        
                        // Break early if we hit extraction limits
                        if extraction_count >= miners.len() {
                            break;
                        }
                    }
                }
                
                cycle_results.push(cycle_ore_extracted.clone());
                
            }
            Err(e) => {
                if e.to_string().contains("cooldown") {
                    println!("⏱️ Survey cooldown - patience in domination...");
                } else {
                    println!("❌ Survey failed: {}", e);
                }
            }
        }
        
        // CYCLE RESULTS
        println!("\\n📊 CYCLE {} DOMINATION RESULTS:", cycle);
        let mut cycle_total_value = 0;
        let mut cycle_total_units = 0;
        
        for (ore, units) in &cycle_ore_extracted {
            let value = (*units as u32) * dominator.ore_stats[ore].market_value_estimate;
            cycle_total_value += value;
            cycle_total_units += units;
            println!("   ⛏️ {}: {} units (value: {})", ore, units, value);
        }
        
        println!("   💎 Cycle total: {} units, {} value", cycle_total_units, cycle_total_value);
        println!("   ⏱️ Duration: {:.1}s", cycle_start.elapsed().as_secs_f64());
        
        // Check if any miner cargo is getting full
        let cargo_status = priority_client.get_ships().await?;
        let mut full_miners = 0;
        for ship in &cargo_status {
            if miners.contains(&ship.symbol) && ship.cargo.units >= ship.cargo.capacity {
                full_miners += 1;
            }
        }
        
        if full_miners > 0 {
            println!("   📦 {} miners with full cargo detected", full_miners);
        }
        
        // Adaptive cooldown based on success
        let cooldown = if cycle_total_units > 0 {
            15 // Short cooldown for successful cycles
        } else {
            30 // Longer cooldown if no extraction
        };
        
        println!("   ⏱️ Adaptive cooldown: {}s", cooldown);
        tokio::time::sleep(tokio::time::Duration::from_secs(cooldown)).await;
    }
    
    // FINAL UNIVERSAL DOMINATION STATUS
    println!("\\n🏁🏁🏁 UNIVERSAL ORE DOMINATION COMPLETE! 🏁🏁🏁");
    println!("====================================================");
    
    dominator.print_domination_status();
    
    // Final fleet scan
    let final_ships = priority_client.get_ships().await?;
    let mut final_fleet_ores = HashMap::new();
    
    for ship in &final_ships {
        for item in &ship.cargo.inventory {
            if dominator.target_ores.contains(&item.symbol) {
                *final_fleet_ores.entry(item.symbol.clone()).or_insert(0) += item.units;
            }
        }
    }
    
    println!("\\n🎯 FINAL FLEET ORE INVENTORY:");
    let mut total_final_units = 0;
    let mut total_final_value = 0;
    
    for (ore, units) in &final_fleet_ores {
        if *units > 0 {
            let value = (*units as u32) * dominator.ore_stats[ore].market_value_estimate;
            total_final_units += units;
            total_final_value += value;
            println!("   ⛏️ {}: {} units (value: {})", ore, units, value);
        }
    }
    
    println!("\\n🎉🎉🎉 UNIVERSAL DOMINATION SUMMARY 🎉🎉🎉");
    println!("   Total ore units: {}", total_final_units);
    println!("   Total estimated value: {}", total_final_value);
    println!("   Ore types in inventory: {}", final_fleet_ores.iter().filter(|(_, v)| **v > 0).count());
    println!("   Domination cycles completed: {}", cycle_results.len());
    
    if total_final_units > 50 {
        println!("\\n🔥🔥🔥 TOTAL MINING PWNERSHIP ACHIEVED! 🔥🔥🔥");
        println!("💎 UNIVERSAL ORE DOMINATION SUCCESSFUL!");
    } else {
        println!("\\n⚡ DOMINATION IN PROGRESS!");
        println!("💡 Continue universal mining operations for total pwnership!");
    }
    
    Ok(())
}