// Iron ore mining coordinator with survey-based targeting
use crate::models::{Ship, Survey};
use crate::client::priority_client::{PriorityApiClient, ApiPriority};
use crate::models::transaction::SurveyData;
use crate::{o_debug, o_info};

pub struct IronOreMiner {
    target_ore_amount: i32,
    mining_sites: Vec<String>,
}

impl IronOreMiner {
    pub fn new(target_ore_amount: i32) -> Self {
        Self {
            target_ore_amount,
            mining_sites: vec![
                "X1-N5-B7".to_string(),  // Known iron ore site from market data
            ],
        }
    }
    
    /// Execute survey-based iron ore mining campaign
    pub async fn execute_mining_campaign(&mut self, client: &PriorityApiClient) -> Result<bool, String> {
        o_info!("⛏️ STARTING SURVEY-BASED IRON ORE MINING CAMPAIGN");
        o_info!("🎯 Target: {} iron ore units", self.target_ore_amount);
        
        // Step 1: Find and prepare miners
        let miners = self.find_mining_capable_ships(client).await?;
        if miners.is_empty() {
            o_info!("❌ No mining-capable ships found - need ships with mining equipment");
            return Ok(false);
        }
        
        o_info!("⛏️ Found {} mining-capable ships", miners.len());
        
        // Step 2: Execute mining operations
        let mut total_mined = 0;
        let mut mining_attempts = 0;
        const MAX_MINING_ATTEMPTS: i32 = 50; // Safety limit
        
        while total_mined < self.target_ore_amount && mining_attempts < MAX_MINING_ATTEMPTS {
            mining_attempts += 1;
            o_info!("🔄 Mining attempt {}/{} (mined: {}/{})", 
                   mining_attempts, MAX_MINING_ATTEMPTS, total_mined, self.target_ore_amount);
            
            // Try mining with each available ship
            for miner_symbol in &miners {
                if total_mined >= self.target_ore_amount {
                    break;
                }
                
                match self.mine_iron_ore_with_survey(client, miner_symbol).await {
                    Ok(ore_mined) => {
                        total_mined += ore_mined;
                        o_info!("✅ {} mined {} iron ore (total: {})", 
                               miner_symbol, ore_mined, total_mined);
                    }
                    Err(e) => {
                        o_info!("⚠️ Mining failed on {}: {}", miner_symbol, e);
                    }
                }
                
                // Brief pause between operations
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        }
        
        o_info!("📊 MINING CAMPAIGN RESULTS:");
        o_info!("   ⛏️ Total iron ore mined: {} units", total_mined);
        o_info!("   🎯 Target achieved: {}", if total_mined >= self.target_ore_amount { "YES" } else { "NO" });
        o_info!("   🔄 Mining attempts: {}", mining_attempts);
        
        Ok(total_mined >= self.target_ore_amount)
    }
    
    /// Find ships with mining capability
    async fn find_mining_capable_ships(&self, client: &PriorityApiClient) -> Result<Vec<String>, String> {
        o_info!("🔍 Scanning fleet for mining-capable ships...");
        
        let ships = client.get_ships().await.map_err(|e| e.to_string())?;
        let mut miners = Vec::new();
        
        for ship in &ships {
            let has_mining_equipment = self.has_mining_equipment(ship);
            let has_cargo_space = ship.cargo.units < ship.cargo.capacity;
            
            if has_mining_equipment && has_cargo_space {
                miners.push(ship.symbol.clone());
                o_info!("⛏️ Mining ship: {} ({}/{} cargo)", 
                       ship.symbol, ship.cargo.units, ship.cargo.capacity);
                
                // Show mining equipment
                for module in &ship.modules {
                    if module.symbol.contains("MINING") || module.symbol.contains("SURVEYOR") {
                        o_info!("   🔧 Equipment: {}", module.symbol);
                    }
                }
            } else {
                let reason = if !has_mining_equipment {
                    "no mining equipment"
                } else {
                    "no cargo space"
                };
                o_debug!("❌ {} - {}", ship.symbol, reason);
            }
        }
        
        Ok(miners)
    }
    
    /// Check if ship has mining equipment (including basic processing capability)
    fn has_mining_equipment(&self, ship: &Ship) -> bool {
        ship.modules.iter().any(|module| {
            module.symbol.contains("MINING_LASER") ||
            module.symbol.contains("SURVEYOR") ||
            module.symbol.contains("MINING") ||
            module.symbol.contains("MINERAL_PROCESSOR") // Accept basic processing as mining capability
        })
    }
    
    /// Mine iron ore using survey-guided extraction
    async fn mine_iron_ore_with_survey(&self, client: &PriorityApiClient, ship_symbol: &str) -> Result<i32, String> {
        o_info!("🔍 Starting survey-based mining on {}", ship_symbol);
        
        // Step 1: Get current ship location
        let ship = client.get_ship(ship_symbol).await.map_err(|e| e.to_string())?;
        let current_location = &ship.nav.waypoint_symbol;
        
        // Step 2: Navigate to mining site if needed
        let target_site = self.find_best_mining_site(current_location);
        if *current_location != target_site {
            o_info!("🚀 Navigating {} to mining site {}", ship_symbol, target_site);
            
            // Ensure ship is in orbit for navigation
            if ship.nav.status == "DOCKED" {
                client.orbit_ship(ship_symbol).await.map_err(|e| e.to_string())?;
            }
            
            // Navigate to mining site
            let nav_data = client.navigate_ship(ship_symbol, &target_site).await.map_err(|e| e.to_string())?;
            
            // Wait for arrival
            let wait_time = if let Ok(arrival_time) = nav_data.nav.route.arrival.parse::<chrono::DateTime<chrono::Utc>>() {
                let now = chrono::Utc::now();
                let duration = arrival_time - now;
                duration.num_seconds().max(0) as u64 + 3
            } else {
                30
            };
            
            o_info!("⏳ Waiting {} seconds for {} to arrive...", wait_time, ship_symbol);
            tokio::time::sleep(tokio::time::Duration::from_secs(wait_time)).await;
        }
        
        // Step 3: Create survey for targeted mining
        o_info!("📊 Creating survey at {} for iron ore targeting", target_site);
        let survey_data = client.create_survey_with_priority(ship_symbol, ApiPriority::ActiveGoal)
            .await.map_err(|e| e.to_string())?;
        
        // Step 4: Analyze survey for iron ore
        let best_survey = self.find_best_iron_ore_survey(&survey_data);
        
        match best_survey {
            Some(survey) => {
                o_info!("🎯 Found iron ore survey! Extracting with targeted survey...");
                
                // Step 5: Extract using the iron ore survey
                let extraction_data = client.extract_resources_with_survey(ship_symbol, survey)
                    .await.map_err(|e| e.to_string())?;
                
                // Step 6: Calculate iron ore extracted
                let iron_ore_extracted = extraction_data.extraction.extraction_yield.units;
                let extracted_material = &extraction_data.extraction.extraction_yield.symbol;
                
                if extracted_material == "IRON_ORE" {
                    o_info!("✅ Successfully extracted {} IRON_ORE", iron_ore_extracted);
                    Ok(iron_ore_extracted)
                } else {
                    o_info!("⚠️ Extracted {} {} (not iron ore)", iron_ore_extracted, extracted_material);
                    Ok(0) // Didn't get iron ore
                }
            }
            None => {
                o_info!("⚠️ No iron ore found in survey - trying regular extraction");
                
                // Fallback to regular extraction
                let extraction_data = client.extract_resources(ship_symbol)
                    .await.map_err(|e| e.to_string())?;
                
                let iron_ore_extracted = if extraction_data.extraction.extraction_yield.symbol == "IRON_ORE" {
                    extraction_data.extraction.extraction_yield.units
                } else {
                    0
                };
                
                Ok(iron_ore_extracted)
            }
        }
    }
    
    /// Find best mining site based on current location
    fn find_best_mining_site(&self, current_location: &str) -> String {
        // For now, just return the first mining site
        // In the future, this could analyze distance, resource availability, etc.
        if self.mining_sites.contains(&current_location.to_string()) {
            current_location.to_string()
        } else {
            self.mining_sites.first().unwrap_or(&"X1-N5-B7".to_string()).clone()
        }
    }
    
    /// Find the best survey for iron ore extraction
    fn find_best_iron_ore_survey<'a>(&self, survey_data: &'a SurveyData) -> Option<&'a Survey> {
        // Look for surveys that contain iron ore deposits
        survey_data.surveys.iter().find(|survey| {
            survey.deposits.iter().any(|deposit| deposit.symbol == "IRON_ORE")
        })
    }
}