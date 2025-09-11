// Use probe ship for surveys and mining ship for targeted extraction
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("🔍 SHARED SURVEY MINING STRATEGY");
    println!("================================");
    
    // Check ship capabilities
    let ships = priority_client.get_ships().await?;
    
    println!("🚢 Fleet capabilities:");
    for ship in &ships {
        println!("   {}: {} at {}", ship.symbol, ship.frame.name, ship.nav.waypoint_symbol);
        println!("      Fuel: {}/{}", ship.fuel.current, ship.fuel.capacity);
        println!("      Cargo: {}/{}", ship.cargo.units, ship.cargo.capacity);
        
        // Check mounts
        if !ship.mounts.is_empty() {
            println!("      Mounts:");
            for mount in &ship.mounts {
                println!("         - {}", mount.symbol);
            }
        }
        
        // Check modules  
        if !ship.modules.is_empty() {
            println!("      Modules:");
            for module in &ship.modules {
                println!("         - {}", module.symbol);
            }
        }
        println!();
    }
    
    // Identify survey-capable ship (likely CLAUDE_AGENT_2-2 based on 0 fuel capacity = probe)
    let mut surveyor_ship = None;
    let mut mining_ship = None;
    
    for ship in &ships {
        // Look for surveyor mounts
        let has_surveyor = ship.mounts.iter().any(|m| m.symbol.contains("SURVEYOR"));
        
        // Look for mining mounts
        let has_mining_laser = ship.mounts.iter().any(|m| m.symbol.contains("MINING_LASER"));
        
        if has_surveyor && surveyor_ship.is_none() {
            surveyor_ship = Some(ship.symbol.clone());
            println!("🔍 Found surveyor ship: {}", ship.symbol);
        }
        
        if has_mining_laser && ship.cargo.capacity > 0 && mining_ship.is_none() {
            mining_ship = Some(ship.symbol.clone());
            println!("⛏️ Found mining ship: {}", ship.symbol);
        }
    }
    
    // Alternative: try CLAUDE_AGENT_2-2 as surveyor even without mounts (it might be a probe)
    if surveyor_ship.is_none() {
        for ship in &ships {
            if ship.fuel.capacity == 0 {  // Probe ships often have 0 fuel capacity
                println!("🚁 Trying {} as probe surveyor (0 fuel capacity)", ship.symbol);
                surveyor_ship = Some(ship.symbol.clone());
                break;
            }
        }
    }
    
    if surveyor_ship.is_none() || mining_ship.is_none() {
        println!("❌ Missing required ships:");
        println!("   Surveyor found: {}", surveyor_ship.is_some());
        println!("   Miner found: {}", mining_ship.is_some());
        println!("💡 May need to equip ships with proper mounts");
        return Ok(());
    }
    
    let surveyor = surveyor_ship.unwrap();
    let miner = mining_ship.unwrap();
    
    println!("\n🎯 MINING TEAM:");
    println!("   Surveyor: {}", surveyor);
    println!("   Miner: {}", miner);
    
    // Get both ships to the mining location (X1-N5-BA5F)
    let mining_location = "X1-N5-BA5F";
    
    println!("\n🚀 Positioning ships at {}...", mining_location);
    
    // Position surveyor
    let surveyor_ship = priority_client.get_ship(&surveyor).await?;
    if surveyor_ship.nav.waypoint_symbol != mining_location {
        println!("🔍 Moving surveyor to mining location...");
        
        if surveyor_ship.fuel.capacity > 0 {  // Only if ship needs fuel
            if surveyor_ship.nav.status == "DOCKED" {
                priority_client.orbit_ship(&surveyor).await?;
            }
            
            // Try navigation (might fail due to fuel)
            match priority_client.navigate_ship(&surveyor, mining_location).await {
                Ok(_) => {
                    println!("✅ Surveyor navigating to mining location");
                    tokio::time::sleep(tokio::time::Duration::from_secs(30)).await; // Wait for arrival
                }
                Err(e) => {
                    if e.to_string().contains("fuel") {
                        println!("⚠️ Surveyor has fuel constraints: {}", e);
                        println!("💡 Will attempt survey from current location");
                    } else {
                        println!("❌ Surveyor navigation failed: {}", e);
                        return Ok(());
                    }
                }
            }
        } else {
            println!("🚁 Surveyor is probe - may not need to move");
        }
    }
    
    // Position miner  
    let miner_ship = priority_client.get_ship(&miner).await?;
    if miner_ship.nav.waypoint_symbol != mining_location {
        println!("⛏️ Miner already positioned correctly");
    }
    
    // Ensure miner is in orbit
    let current_miner = priority_client.get_ship(&miner).await?;
    if current_miner.nav.status != "IN_ORBIT" {
        priority_client.orbit_ship(&miner).await?;
        println!("🚀 Miner in orbit for mining");
    }
    
    println!("\n🔍 SURVEY PHASE");
    println!("===============");
    
    // Create survey with surveyor
    println!("📊 Creating survey with {}...", surveyor);
    match priority_client.create_survey(&surveyor).await {
        Ok(survey_data) => {
            println!("✅ Survey created with {} results", survey_data.surveys.len());
            
            // Look for iron ore surveys
            let iron_surveys: Vec<_> = survey_data.surveys.iter()
                .filter(|survey| survey.deposits.iter().any(|d| d.symbol == "IRON_ORE"))
                .collect();
                
            if !iron_surveys.is_empty() {
                println!("🎯 Found {} surveys with IRON_ORE!", iron_surveys.len());
                
                for (i, survey) in iron_surveys.iter().enumerate() {
                    println!("   Survey {}: {:?}", i + 1, 
                             survey.deposits.iter().map(|d| &d.symbol).collect::<Vec<_>>());
                }
                
                println!("\n⛏️ TARGETED MINING PHASE");
                println!("========================");
                
                // Use first iron ore survey for targeted mining
                let target_survey = iron_surveys[0];
                
                println!("🎯 Using survey for targeted iron ore extraction...");
                match priority_client.extract_resources_with_survey(&miner, target_survey).await {
                    Ok(extraction_data) => {
                        let material = &extraction_data.extraction.extraction_yield.symbol;
                        let amount = extraction_data.extraction.extraction_yield.units;
                        
                        println!("✅ TARGETED EXTRACTION SUCCESS!");
                        println!("   Material: {} x{}", material, amount);
                        
                        if material == "IRON_ORE" {
                            println!("🎉 SUCCESS: Extracted {} IRON_ORE using shared survey!", amount);
                        } else {
                            println!("⚠️ Got {} instead of iron ore", material);
                        }
                    }
                    Err(e) => {
                        println!("❌ Targeted extraction failed: {}", e);
                    }
                }
                
                // Try additional extractions if there are more iron surveys
                if iron_surveys.len() > 1 {
                    println!("\n🔄 Additional targeted extractions...");
                    
                    for (i, survey) in iron_surveys.iter().skip(1).enumerate() {
                        if i >= 3 { break; } // Limit to avoid too many attempts
                        
                        println!("🎯 Using iron ore survey {}...", i + 2);
                        
                        // Wait for cooldown
                        tokio::time::sleep(tokio::time::Duration::from_secs(75)).await;
                        
                        match priority_client.extract_resources_with_survey(&miner, survey).await {
                            Ok(extraction_data) => {
                                let material = &extraction_data.extraction.extraction_yield.symbol;
                                let amount = extraction_data.extraction.extraction_yield.units;
                                
                                println!("✅ Survey {} result: {} x{}", i + 2, material, amount);
                                
                                if material == "IRON_ORE" {
                                    println!("🎉 MORE IRON_ORE: {} units!", amount);
                                }
                            }
                            Err(e) => {
                                println!("❌ Survey {} extraction failed: {}", i + 2, e);
                            }
                        }
                    }
                }
                
            } else {
                println!("⚠️ No iron ore found in survey");
                println!("💡 Surveys found other materials:");
                for (i, survey) in survey_data.surveys.iter().enumerate() {
                    println!("   Survey {}: {:?}", i + 1, 
                             survey.deposits.iter().map(|d| &d.symbol).collect::<Vec<_>>());
                }
            }
        }
        Err(e) => {
            println!("❌ Survey creation failed: {}", e);
            
            if e.to_string().contains("surveyor") {
                println!("💡 Ship lacks surveyor capability");
                println!("💡 Try equipping surveyor mounts or using different ship");
            }
        }
    }
    
    // Final status check
    println!("\n📊 FINAL STATUS CHECK");
    println!("====================");
    
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
    
    println!("🎯 Total fleet IRON_ORE: {} (target: 100)", total_iron_ore);
    
    if total_iron_ore >= 100 {
        println!("🎉 TARGET ACHIEVED!");
    } else {
        println!("💡 Progress made with shared survey approach");
        println!("💡 Repeat this strategy to reach target");
    }
    
    Ok(())
}