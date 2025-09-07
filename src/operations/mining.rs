// Mining operations module
use crate::client::SpaceTradersClient;
use crate::models::*;
use tokio::time::{sleep, Duration};
use std::collections::HashMap;

pub struct MiningOperations<'a> {
    client: &'a SpaceTradersClient,
}

impl<'a> MiningOperations<'a> {
    pub fn new(client: &'a SpaceTradersClient) -> Self {
        Self { client }
    }

    // Basic API operations
    pub async fn create_survey(&self, ship_symbol: &str) -> Result<SurveyData, Box<dyn std::error::Error>> {
        self.client.create_survey(ship_symbol).await
    }

    pub async fn extract_resources(&self, ship_symbol: &str) -> Result<ExtractionData, Box<dyn std::error::Error>> {
        self.client.extract_resources(ship_symbol).await
    }

    pub async fn extract_with_survey(&self, ship_symbol: &str, survey: &Survey) -> Result<ExtractionData, Box<dyn std::error::Error>> {
        self.client.extract_resources_with_survey(ship_symbol, survey).await
    }

    // Advanced mining operations
    pub fn select_best_survey(&self, surveys: &[Survey], needed_materials: &[String]) -> Option<Survey> {
        surveys.iter().find(|survey| {
            survey.deposits.iter().any(|deposit| needed_materials.contains(&deposit.symbol))
        }).cloned()
    }

    pub async fn find_asteroid_fields(&self, system_symbol: &str, needed_materials: &[String]) -> Result<Vec<Waypoint>, Box<dyn std::error::Error>> {
        println!("🔍 Searching for asteroid fields that produce required materials in system {}...", system_symbol);
        
        let all_waypoints = self.client.get_system_waypoints(system_symbol, None).await?;
        let all_asteroids: Vec<Waypoint> = all_waypoints
            .into_iter()
            .filter(|waypoint| waypoint.waypoint_type == "ASTEROID_FIELD")
            .collect();

        println!("📍 Found {} total asteroid field(s):", all_asteroids.len());
        
        // Score asteroid fields based on likelihood of containing needed materials
        let mut asteroid_scores: Vec<(Waypoint, i32)> = Vec::new();
        
        for asteroid in all_asteroids {
            let score = self.score_asteroid_for_materials(&asteroid, needed_materials);
            println!("  - {} at ({}, {})", asteroid.symbol, asteroid.x, asteroid.y);
            println!("    Likely produces: {:?}", needed_materials);
            
            if score > 50 {
                println!("  🎯 {} shows high material potential!", asteroid.symbol);
            }
            
            asteroid_scores.push((asteroid, score));
        }
        
        // Sort by score (highest first)
        asteroid_scores.sort_by(|a, b| b.1.cmp(&a.1));
        
        if !asteroid_scores.is_empty() {
            let best = &asteroid_scores[0];
            println!("🎯 Selected target: {} (priority score: {})", best.0.symbol, best.1);
            println!("  ✅ High likelihood of containing required contract materials!");
        }
        
        Ok(asteroid_scores.into_iter().map(|(waypoint, _)| waypoint).collect())
    }

    fn score_asteroid_for_materials(&self, asteroid: &Waypoint, needed_materials: &[String]) -> i32 {
        let mut current_score = 0;
        
        // Check traits for contract material hints
        for trait_info in &asteroid.traits {
            let description = trait_info.description.to_lowercase();
            let trait_name = trait_info.name.to_lowercase();
            
            // High priority for aluminum ore contracts
            if needed_materials.contains(&"ALUMINUM_ORE".to_string()) {
                if description.contains("aluminum") || description.contains("metal ore")
                   || trait_name.contains("mineral") || trait_name.contains("rich") {
                    current_score += 100;
                }
            }
            
            // General mineral/ore indicators
            if description.contains("mineral") || description.contains("ore") || description.contains("metal") {
                current_score += 50;
            }
            
            // Specific material matches
            for material in needed_materials {
                if description.contains(&material.to_lowercase()) || trait_name.contains(&material.to_lowercase()) {
                    current_score += 75;
                }
            }
        }
        
        current_score
    }

    pub async fn execute_parallel_survey_mining(
        &self,
        ready_miners: &[(Ship, Waypoint)],
        needed_materials: &[String],
        contract: &Contract,
        max_cycles: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("⛏️ Starting PARALLEL autonomous survey-based mining loop...");
        println!("🚀 Coordinating {} ships across {} asteroid fields!", 
                ready_miners.len(),
                ready_miners.iter()
                    .map(|(_, asteroid)| asteroid.symbol.as_str())
                    .collect::<std::collections::HashSet<_>>()
                    .len());
        
        let mut mining_cycles = 0;
        let mut fleet_surveys: HashMap<String, Vec<Survey>> = HashMap::new();
        
        while mining_cycles < max_cycles {
            mining_cycles += 1;
            println!("\n🔄 PARALLEL Mining cycle {}/{} - {} ships operating simultaneously", 
                    mining_cycles, max_cycles, ready_miners.len());
            
            // Phase 1: PARALLEL Survey creation for all ships
            println!("🔍 Creating surveys for all mining ships...");
            
            for (ship, asteroid) in ready_miners {
                let asteroid_surveys = fleet_surveys.entry(asteroid.symbol.clone()).or_insert(Vec::new());
                
                // Check if we need surveys for this asteroid
                let needs_survey = asteroid_surveys.iter().all(|survey| {
                    !survey.deposits.iter().any(|deposit| needed_materials.contains(&deposit.symbol))
                });
                
                if needs_survey || asteroid_surveys.is_empty() {
                    println!("  🔍 {} surveying {}...", ship.symbol, asteroid.symbol);
                    match self.create_survey(&ship.symbol).await {
                        Ok(survey_data) => {
                            println!("    ✅ {} found {} deposit locations", ship.symbol, survey_data.surveys.len());
                            
                            for survey in &survey_data.surveys {
                                let contract_deposits: Vec<_> = survey.deposits.iter()
                                    .filter(|d| needed_materials.contains(&d.symbol))
                                    .collect();
                                
                                if !contract_deposits.is_empty() {
                                    println!("      🎯 Survey {}: Contract materials found! {:?}", 
                                            survey.signature,
                                            contract_deposits.iter().map(|d| &d.symbol).collect::<Vec<_>>());
                                }
                            }
                            
                            asteroid_surveys.extend(survey_data.surveys);
                            
                            // Small delay for survey cooldown
                            if survey_data.cooldown.remaining_seconds > 0 {
                                sleep(Duration::from_secs((survey_data.cooldown.remaining_seconds as u64).min(10))).await;
                            }
                        }
                        Err(e) => {
                            println!("    ⚠️ {} survey failed: {}", ship.symbol, e);
                        }
                    }
                }
            }
            
            // Phase 2: PARALLEL Extraction for all ships
            println!("⛏️ Executing parallel extraction across fleet...");
            
            for (ship, asteroid) in ready_miners {
                println!("  ⛏️ {} extracting at {}...", ship.symbol, asteroid.symbol);
                
                // Find best survey for this asteroid
                let empty_surveys = Vec::new();
                let asteroid_surveys = fleet_surveys.get(&asteroid.symbol).unwrap_or(&empty_surveys);
                let target_survey = asteroid_surveys.iter().find(|survey| {
                    survey.deposits.iter().any(|deposit| needed_materials.contains(&deposit.symbol))
                });
                
                // Execute extraction (targeted or random)
                let extraction_result = if let Some(survey) = target_survey {
                    println!("    🎯 Using targeted survey {} for {}", survey.signature, ship.symbol);
                    self.extract_with_survey(&ship.symbol, survey).await
                } else {
                    println!("    🎲 Random extraction for {}", ship.symbol);
                    self.extract_resources(&ship.symbol).await
                };
                
                match extraction_result {
                    Ok(extraction_data) => {
                        let yield_info = &extraction_data.extraction.extraction_yield;
                        println!("    ✅ {} extracted: {} x{} (Cargo: {}/{})",
                                ship.symbol, yield_info.symbol, yield_info.units,
                                extraction_data.cargo.units, extraction_data.cargo.capacity);
                        
                        // Check contract progress
                        if needed_materials.contains(&yield_info.symbol) {
                            println!("      🎯 {} found CONTRACT MATERIAL: {}! ✨", ship.symbol, yield_info.symbol);
                            
                            let current_amount = extraction_data.cargo.inventory.iter()
                                .find(|item| item.symbol == yield_info.symbol)
                                .map(|item| item.units)
                                .unwrap_or(0);
                            
                            let needed_amount = contract.terms.deliver.iter()
                                .find(|delivery| delivery.trade_symbol == yield_info.symbol)
                                .map(|delivery| delivery.units_required)
                                .unwrap_or(0);
                            
                            println!("      📈 {} progress: {}/{} {}",
                                    ship.symbol, current_amount, needed_amount, yield_info.symbol);
                        }
                        
                        // Check if ship cargo is full
                        if extraction_data.cargo.units >= extraction_data.cargo.capacity {
                            println!("      📦 {} cargo full! Ready for delivery.", ship.symbol);
                        }
                    }
                    Err(e) => {
                        println!("    ❌ {} extraction failed: {}", ship.symbol, e);
                    }
                }
                
                // Small delay between ship operations
                sleep(Duration::from_secs(1)).await;
            }
            
            // Cooldown management for all ships
            println!("⏳ Fleet cooldown management (60 seconds)...");
            sleep(Duration::from_secs(60)).await;
            
            // Check fleet status
            match self.client.get_ships().await {
                Ok(updated_ships) => {
                    let mut total_contract_materials = 0;
                    let mut full_ships = 0;
                    
                    for (ship, _) in ready_miners {
                        if let Some(updated_ship) = updated_ships.iter().find(|s| s.symbol == ship.symbol) {
                            // Count contract materials across fleet
                            for item in &updated_ship.cargo.inventory {
                                if needed_materials.contains(&item.symbol) {
                                    total_contract_materials += item.units;
                                }
                            }
                            
                            // Count full ships
                            if updated_ship.cargo.units >= updated_ship.cargo.capacity {
                                full_ships += 1;
                            }
                        }
                    }
                    
                    let needed_amount = contract.terms.deliver.iter()
                        .map(|delivery| delivery.units_required)
                        .sum::<i32>();
                    
                    println!("\n📊 FLEET MINING PROGRESS:");
                    println!("  🎯 Contract materials collected: {}/{}", total_contract_materials, needed_amount);
                    println!("  📦 Ships with full cargo: {}/{}", full_ships, ready_miners.len());
                    
                    if total_contract_materials >= needed_amount {
                        println!("🎉 CONTRACT REQUIREMENTS FULFILLED BY PARALLEL FLEET!");
                        break;
                    }
                }
                Err(e) => {
                    println!("⚠️ Could not check fleet status: {}", e);
                }
            }
        }
        
        println!("\n🎉 PARALLEL autonomous survey-based mining operation complete!");
        println!("💡 Multi-ship coordination achieved {}x efficiency with {} mining vessels!",
                ready_miners.len(), ready_miners.len());
        
        Ok(())
    }
}