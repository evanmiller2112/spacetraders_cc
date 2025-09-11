// MAXIMUM SCALE IRON DOMINATION - ABSOLUTE CHAOS MODE! 🔥🔥🔥
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient};
// Removed unused import
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("🔥🔥🔥🔥🔥 MAXIMUM SCALE IRON DOMINATION 🔥🔥🔥🔥🔥");
    println!("==================================================");
    println!("💥 ABSOLUTE CHAOS MODE - NO LIMITS!");
    println!("🎯 TARGET: CRUSH 92+ IRON ORE UNITS!");
    println!("⚡ STRATEGY: EVERYTHING AT ONCE!");
    
    let mining_location = "X1-N5-BA5F";
    let target_iron_ore = 100;
    
    // Get starting status
    let ships = priority_client.get_ships().await?;
    let mut starting_iron_ore = 0;
    
    for ship in &ships {
        let iron_ore: i32 = ship.cargo.inventory.iter()
            .filter(|item| item.symbol == "IRON_ORE")
            .map(|item| item.units)
            .sum();
        starting_iron_ore += iron_ore;
    }
    
    println!("\\n📊 DOMINATION STATUS:");
    println!("   Starting iron ore: {} units", starting_iron_ore);
    println!("   Target: {} units", target_iron_ore);
    println!("   NEED TO CRUSH: {} units", target_iron_ore - starting_iron_ore);
    
    if starting_iron_ore >= target_iron_ore {
        println!("\\n🎉🎉🎉 ALREADY DOMINATED! 🎉🎉🎉");
        return Ok(());
    }
    
    // Analyze fleet for MAXIMUM CHAOS
    let mut surveyors = Vec::new();
    let mut miners = Vec::new();
    let mut refuelers = Vec::new();
    
    for ship in &ships {
        let has_surveyor = ship.mounts.iter().any(|m| m.symbol.contains("SURVEYOR"));
        let has_mining_laser = ship.mounts.iter().any(|m| m.symbol.contains("MINING_LASER"));
        let at_location = ship.nav.waypoint_symbol == mining_location;
        
        if has_surveyor {
            surveyors.push(ship.symbol.clone());
        }
        if has_mining_laser && at_location {
            miners.push(ship.symbol.clone());
        }
        if ship.nav.waypoint_symbol == "X1-N5-A2" || ship.nav.waypoint_symbol == "X1-N5-B6" {
            refuelers.push(ship.symbol.clone()); // Ships near fuel stations
        }
    }
    
    println!("\\n🚀 MAXIMUM SCALE FLEET:");
    println!("   🔍 Surveyors: {} ships", surveyors.len());
    println!("   ⛏️ Miners at location: {} ships", miners.len());
    println!("   ⛽ Ships near fuel: {} ships", refuelers.len());
    
    // CONTINUOUS DOMINATION LOOP
    let max_domination_cycles = 20;
    let mut total_dominated = 0;
    let mut consecutive_successes = 0;
    
    for cycle in 1..=max_domination_cycles {
        println!("\\n💥💥💥 DOMINATION CYCLE {}/{} 💥💥💥", cycle, max_domination_cycles);
        println!("=======================================");
        
        let cycle_start_time = std::time::Instant::now();
        let mut cycle_iron_ore = 0;
        
        // PARALLEL SURVEY ATTEMPTS - Try all surveyors simultaneously!
        println!("📊 PARALLEL SURVEY BLITZ - ALL SURVEYORS!");
        let mut survey_tasks = Vec::new();
        
        for (i, surveyor) in surveyors.iter().enumerate() {
            let surveyor_clone = surveyor.clone();
            let client_clone = priority_client.clone();
            
            let task = tokio::spawn(async move {
                let result: Result<(String, Option<usize>, String), String> = async {
                    let ship = client_clone.get_ship(&surveyor_clone).await.map_err(|e| e.to_string())?;
                    if ship.nav.waypoint_symbol != mining_location {
                        return Ok((surveyor_clone, None, "Wrong location".to_string()));
                    }
                    
                    if ship.nav.status != "IN_ORBIT" {
                        if let Err(e) = client_clone.orbit_ship(&surveyor_clone).await {
                            return Ok((surveyor_clone, None, format!("Orbit failed: {}", e)));
                        }
                    }
                    
                    match client_clone.create_survey(&surveyor_clone).await {
                        Ok(survey_data) => {
                            let iron_survey_count = survey_data.surveys.iter()
                                .filter(|survey| survey.deposits.iter().any(|d| d.symbol == "IRON_ORE"))
                                .count();
                            
                            if iron_survey_count == 0 {
                                Ok((surveyor_clone, None, "No iron ore".to_string()))
                            } else {
                                Ok((surveyor_clone, Some(iron_survey_count), format!("{} iron surveys", iron_survey_count)))
                            }
                        }
                        Err(e) => {
                            if e.to_string().contains("cooldown") {
                                Ok((surveyor_clone, None, "Cooldown".to_string()))
                            } else {
                                Ok((surveyor_clone, None, format!("Failed: {}", e)))
                            }
                        }
                    }
                }.await;
                result
            });
            
            survey_tasks.push((i, task));
            
            // Brief stagger to avoid overwhelming the API
            sleep(Duration::from_millis(500)).await;
        }
        
        // Collect survey results
        let mut all_iron_survey_count = 0;
        for (i, task) in survey_tasks {
            match task.await {
                Ok(Ok((surveyor, count_opt, status))) => {
                    println!("   🔍 Surveyor {}: {}", surveyor, status);
                    if let Some(count) = count_opt {
                        all_iron_survey_count += count;
                    }
                }
                Ok(Err(e)) => println!("   ❌ Surveyor task error: {}", e),
                Err(e) => println!("   ❌ Surveyor join error: {}", e),
            }
        }
        
        println!("\\n📊 SURVEY BLITZ RESULTS:");
        println!("   Total iron ore surveys found: {}", all_iron_survey_count);
        
        if all_iron_survey_count == 0 {
            println!("   ⚠️ No iron ore surveys - continuing domination...");
            consecutive_successes = 0;
            sleep(Duration::from_secs(5)).await;
            continue;
        }
        
        // PARALLEL EXTRACTION BLITZ - Use all miners simultaneously!
        println!("\\n⛏️⛏️⛏️ PARALLEL EXTRACTION BLITZ! ⛏️⛏️⛏️");
        
        let mut extraction_tasks: Vec<tokio::task::JoinHandle<Result<(String, i32, String), String>>> = Vec::new();
        for (i, miner) in miners.iter().enumerate() {
            if i >= all_iron_survey_count {
                break; // More miners than surveys available
            }
            let miner_clone = miner.clone();
            let client_clone = priority_client.clone();
            
            let task = tokio::spawn(async move {
                let ship = client_clone.get_ship(&miner_clone).await.map_err(|e| e.to_string())?;
                
                if ship.cargo.units >= ship.cargo.capacity {
                    return Ok((miner_clone, 0, "Cargo full".to_string()));
                }
                
                if ship.nav.status != "IN_ORBIT" {
                    if let Err(e) = client_clone.orbit_ship(&miner_clone).await {
                        return Ok((miner_clone, 0, format!("Orbit failed: {}", e)));
                    }
                }
                
                match client_clone.extract_resources(&miner_clone).await {
                    Ok(extraction_data) => {
                        let material = &extraction_data.extraction.extraction_yield.symbol;
                        let amount = extraction_data.extraction.extraction_yield.units;
                        
                        if material == "IRON_ORE" {
                            Ok((miner_clone, amount, format!("IRON_ORE x{} 🎉", amount)))
                        } else {
                            Ok((miner_clone, 0, format!("{} x{}", material, amount)))
                        }
                    }
                    Err(e) => {
                        if e.to_string().contains("cooldown") {
                            Ok((miner_clone, 0, "Cooldown".to_string()))
                        } else {
                            Ok((miner_clone, 0, format!("Failed: {}", e)))
                        }
                    }
                }
            });
            
            extraction_tasks.push(task);
            
            // Brief stagger
            sleep(Duration::from_millis(300)).await;
        }
        
        // Collect extraction results
        for task in extraction_tasks {
            match task.await {
                Ok(Ok((miner, iron_ore_amount, status))) => {
                    println!("   ⛏️ {}: {}", miner, status);
                    cycle_iron_ore += iron_ore_amount;
                }
                Ok(Err(e)) => println!("   ❌ Miner task error: {}", e),
                Err(e) => println!("   ❌ Miner join error: {}", e),
            }
        }
        
        total_dominated += cycle_iron_ore;
        
        println!("\\n📊 CYCLE {} DOMINATION RESULTS:", cycle);
        println!("   Iron ore extracted: {} units", cycle_iron_ore);
        println!("   Total dominated: {} units", total_dominated);
        println!("   Cycle duration: {:.1}s", cycle_start_time.elapsed().as_secs_f64());
        
        if cycle_iron_ore > 0 {
            consecutive_successes += 1;
            println!("   🎉 SUCCESS STREAK: {}", consecutive_successes);
        } else {
            consecutive_successes = 0;
        }
        
        // Check if we've reached domination
        let current_ships = priority_client.get_ships().await?;
        let mut current_total = 0;
        for ship in &current_ships {
            let iron_ore: i32 = ship.cargo.inventory.iter()
                .filter(|item| item.symbol == "IRON_ORE")
                .map(|item| item.units)
                .sum();
            current_total += iron_ore;
        }
        
        println!("   📊 Fleet iron ore: {} units", current_total);
        
        if current_total >= target_iron_ore {
            println!("\\n🎉🎉🎉 DOMINATION ACHIEVED! 🎉🎉🎉");
            break;
        }
        
        // ADAPTIVE COOLDOWN - Scale based on success
        let adaptive_cooldown = if consecutive_successes >= 3 {
            5 // Minimal cooldown for hot streaks
        } else if cycle_iron_ore > 0 {
            10 // Short cooldown for successful cycles
        } else {
            20 // Longer cooldown if no extraction
        };
        
        println!("   ⏱️ Adaptive cooldown: {}s", adaptive_cooldown);
        sleep(Duration::from_secs(adaptive_cooldown)).await;
    }
    
    // FINAL DOMINATION STATUS
    println!("\\n🏁🏁🏁 MAXIMUM SCALE DOMINATION COMPLETE! 🏁🏁🏁");
    println!("====================================================");
    
    let final_ships = priority_client.get_ships().await?;
    let mut final_iron_ore = 0;
    
    println!("📊 FINAL IRON ORE DOMINATION INVENTORY:");
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
    
    println!("\\n🎯 DOMINATION SUMMARY:");
    println!("   Starting iron ore: {} units", starting_iron_ore);
    println!("   Dominated this session: {} units", total_dominated);
    println!("   Final iron ore: {} units", final_iron_ore);
    println!("   Target: {} units", target_iron_ore);
    
    if final_iron_ore >= target_iron_ore {
        println!("\\n🎉🎉🎉🎉🎉 IRON ORE DOMINATION COMPLETE! 🎉🎉🎉🎉🎉");
        println!("🏭🏭🏭 READY FOR REFINERY DOMINATION! 🏭🏭🏭");
        println!("⚡ MAXIMUM SCALE ACHIEVED!");
        println!("💥 NO LIMITS EXCEEDED!");
    } else {
        println!("\\n🚀 DOMINATION IN PROGRESS!");
        println!("   Progress: {}%", (final_iron_ore * 100) / target_iron_ore);
        println!("   Still need: {} units", target_iron_ore - final_iron_ore);
        println!("💡 Run maximum scale domination again for TOTAL CONQUEST!");
    }
    
    println!("\\n🔥 MAXIMUM SCALE STATISTICS:");
    println!("   Parallel surveyors: {}", surveyors.len());
    println!("   Parallel miners: {}", miners.len());
    println!("   Success rate: {:.1}%", if max_domination_cycles > 0 { (consecutive_successes * 100) as f64 / max_domination_cycles as f64 } else { 0.0 });
    
    Ok(())
}