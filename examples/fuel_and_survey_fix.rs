// Quick fix: Refuel surveyor and resume blitz campaign
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("⛽ EMERGENCY FUEL & SURVEY FIX");
    println!("==============================");
    
    let surveyor = "CLAUDE_AGENT_2-1";
    
    // Check surveyor status
    let surveyor_ship = priority_client.get_ship(surveyor).await?;
    println!("🔍 Surveyor status:");
    println!("   Location: {}", surveyor_ship.nav.waypoint_symbol);
    println!("   Status: {}", surveyor_ship.nav.status);
    println!("   Fuel: {}/{}", surveyor_ship.fuel.current, surveyor_ship.fuel.capacity);
    
    // Try to refuel at current location
    if surveyor_ship.fuel.current < surveyor_ship.fuel.capacity / 2 {
        println!("⛽ Attempting refuel at current location...");
        
        // Dock if not already docked
        if surveyor_ship.nav.status != "DOCKED" {
            match priority_client.dock_ship(surveyor).await {
                Ok(_) => println!("🛸 Docked for refuel"),
                Err(e) => println!("⚠️ Dock failed: {}", e),
            }
        }
        
        // Try refuel
        match priority_client.refuel_ship(surveyor).await {
            Ok(refuel_data) => {
                println!("✅ Surveyor refueled!");
                println!("   New fuel: {}/{}", refuel_data.fuel.current, refuel_data.fuel.capacity);
                
                // Now try to get to mining location
                println!("\n🚀 Moving surveyor to mining location...");
                
                priority_client.orbit_ship(surveyor).await?;
                
                match priority_client.navigate_ship(surveyor, "X1-N5-BA5F").await {
                    Ok(nav_result) => {
                        println!("✅ Surveyor en route to X1-N5-BA5F");
                        
                        if let Ok(arrival_time) = nav_result.nav.route.arrival.parse::<chrono::DateTime<chrono::Utc>>() {
                            let now = chrono::Utc::now();
                            let wait_seconds = (arrival_time - now).num_seconds().max(0) as u64 + 3;
                            println!("⏳ Waiting {} seconds for arrival...", wait_seconds);
                            tokio::time::sleep(tokio::time::Duration::from_secs(wait_seconds)).await;
                        }
                        
                        // Now test survey capability
                        println!("\n📊 Testing survey at mining location...");
                        
                        priority_client.orbit_ship(surveyor).await?;
                        
                        match priority_client.create_survey(surveyor).await {
                            Ok(survey_data) => {
                                let iron_surveys: Vec<_> = survey_data.surveys.iter()
                                    .filter(|survey| survey.deposits.iter().any(|d| d.symbol == "IRON_ORE"))
                                    .collect();
                                
                                println!("🎉 SURVEY SUCCESS!");
                                println!("   Total surveys: {}", survey_data.surveys.len());
                                println!("   Iron ore surveys: {}", iron_surveys.len());
                                
                                if !iron_surveys.is_empty() {
                                    println!("🎯 Ready for blitz mining campaign!");
                                    println!("💡 Run: cargo run --example iron_ore_blitz_campaign");
                                } else {
                                    println!("⚠️ No iron ore in this survey - try again");
                                }
                            }
                            Err(e) => {
                                println!("❌ Survey failed: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("❌ Navigation still failed: {}", e);
                        println!("💡 This is the multi-hop routing issue!");
                        
                        // Alternative: try probe ship for survey
                        println!("\n🚁 Trying alternative surveyor strategy...");
                        
                        let ships = priority_client.get_ships().await?;
                        for ship in &ships {
                            let has_surveyor = ship.mounts.iter().any(|m| m.symbol.contains("SURVEYOR"));
                            let at_mining_location = ship.nav.waypoint_symbol == "X1-N5-BA5F";
                            
                            if has_surveyor && at_mining_location {
                                println!("🎯 Found {} with surveyor at mining location!", ship.symbol);
                                
                                if ship.nav.status != "IN_ORBIT" {
                                    priority_client.orbit_ship(&ship.symbol).await?;
                                }
                                
                                match priority_client.create_survey(&ship.symbol).await {
                                    Ok(survey_data) => {
                                        let iron_surveys: Vec<_> = survey_data.surveys.iter()
                                            .filter(|survey| survey.deposits.iter().any(|d| d.symbol == "IRON_ORE"))
                                            .collect();
                                        
                                        println!("🎉 ALTERNATIVE SURVEY SUCCESS!");
                                        println!("   Iron ore surveys: {}", iron_surveys.len());
                                        
                                        if !iron_surveys.is_empty() {
                                            println!("🎯 Can proceed with mining using {}!", ship.symbol);
                                        }
                                        
                                        return Ok(());
                                    }
                                    Err(e) => {
                                        println!("❌ Alternative survey failed: {}", e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                println!("❌ Refuel failed: {}", e);
                println!("💡 Current location may not have fuel services");
            }
        }
    }
    
    // Final status
    println!("\n📊 FINAL STATUS:");
    let final_surveyor = priority_client.get_ship(surveyor).await?;
    println!("   Surveyor fuel: {}/{}", final_surveyor.fuel.current, final_surveyor.fuel.capacity);
    println!("   Surveyor location: {}", final_surveyor.nav.waypoint_symbol);
    
    if final_surveyor.nav.waypoint_symbol == "X1-N5-BA5F" {
        println!("✅ Surveyor positioned correctly!");
        println!("🎯 Ready for iron ore blitz campaign!");
    } else {
        println!("⚠️ Surveyor positioning issue");
        println!("💡 Need to implement multi-hop routing system");
        println!("💡 Or use alternative surveyor strategy");
    }
    
    Ok(())
}