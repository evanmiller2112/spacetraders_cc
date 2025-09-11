// Try to refuel ships at their current locations
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("⛽ LOCAL REFUEL SOLUTION");
    println!("=======================");
    
    let agent = priority_client.get_agent().await?;
    println!("💰 Credits available: {}", agent.credits);
    
    let ships = priority_client.get_ships().await?;
    
    // Focus on ships at X1-N5-A2 since that's a shipyard with marketplace
    println!("\n🏗️ Attempting refuel at X1-N5-A2 (shipyard location):");
    
    for ship in &ships {
        if ship.nav.waypoint_symbol == "X1-N5-A2" {
            println!("\n⛽ Processing {} at X1-N5-A2", ship.symbol);
            println!("   Current fuel: {}/{}", ship.fuel.current, ship.fuel.capacity);
            
            // Ensure ship is docked for refuel operations
            if ship.nav.status != "DOCKED" {
                println!("🛸 Docking {} for refuel", ship.symbol);
                match priority_client.dock_ship(&ship.symbol).await {
                    Ok(_) => println!("✅ Successfully docked"),
                    Err(e) => {
                        println!("❌ Dock failed: {}", e);
                        continue;
                    }
                }
            } else {
                println!("✅ Already docked");
            }
            
            // Try direct refuel at shipyard
            println!("⛽ Attempting direct refuel...");
            match priority_client.refuel_ship(&ship.symbol).await {
                Ok(refuel_data) => {
                    println!("🎉 SUCCESS: {} refueled!", ship.symbol);
                    println!("   New fuel level: {}/{}", refuel_data.fuel.current, refuel_data.fuel.capacity);
                    let cost = refuel_data.transaction.units.unwrap_or(0) * refuel_data.transaction.fuel_price.unwrap_or(0);
                    println!("   Cost: {} credits", cost);
                }
                Err(e) => {
                    println!("⚠️ Direct refuel failed: {}", e);
                    
                    // If direct refuel fails, try buying FUEL as cargo first
                    println!("💡 Trying FUEL purchase approach...");
                    
                    match priority_client.purchase_cargo(&ship.symbol, "FUEL", 10).await {
                        Ok(_) => {
                            println!("✅ Purchased FUEL cargo");
                            
                            // Now try refuel again (FUEL cargo might auto-convert)
                            match priority_client.refuel_ship(&ship.symbol).await {
                                Ok(refuel_data) => {
                                    println!("🎉 SUCCESS after FUEL purchase!");
                                    println!("   New fuel level: {}/{}", refuel_data.fuel.current, refuel_data.fuel.capacity);
                                }
                                Err(e2) => {
                                    println!("⚠️ Still failed after FUEL purchase: {}", e2);
                                }
                            }
                        }
                        Err(e2) => {
                            println!("❌ FUEL purchase also failed: {}", e2);
                            println!("💡 Shipyard might not sell FUEL or we need different approach");
                        }
                    }
                }
            }
            
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    }
    
    // Check ships at X1-N5-A1 (planet with marketplace)
    println!("\n🌍 Attempting refuel at X1-N5-A1 (planet with marketplace):");
    
    for ship in &ships {
        if ship.nav.waypoint_symbol == "X1-N5-A1" {
            println!("\n⛽ Processing {} at X1-N5-A1", ship.symbol);
            
            // Dock and try refuel
            if ship.nav.status != "DOCKED" {
                match priority_client.dock_ship(&ship.symbol).await {
                    Ok(_) => println!("✅ Docked at planet"),
                    Err(e) => {
                        println!("❌ Dock failed: {}", e);
                        continue;
                    }
                }
            }
            
            match priority_client.refuel_ship(&ship.symbol).await {
                Ok(refuel_data) => {
                    println!("🎉 SUCCESS: {} refueled at planet!", ship.symbol);
                    println!("   New fuel level: {}/{}", refuel_data.fuel.current, refuel_data.fuel.capacity);
                }
                Err(e) => {
                    println!("⚠️ Planet refuel failed: {}", e);
                }
            }
        }
    }
    
    // Final fuel status
    println!("\n📊 UPDATED FUEL STATUS:");
    let updated_ships = priority_client.get_ships().await?;
    let mut operational_ships = 0;
    
    for ship in &updated_ships {
        let fuel_percentage = if ship.fuel.capacity > 0 {
            (ship.fuel.current as f64 / ship.fuel.capacity as f64) * 100.0
        } else {
            0.0
        };
        
        let status_icon = if ship.fuel.current > 100 {
            operational_ships += 1;
            "✅"
        } else if ship.fuel.current > 20 {
            "⚠️"
        } else {
            "🚨"
        };
        
        println!("   {} {}: {}/{} fuel ({}%)", 
                 status_icon, ship.symbol, ship.fuel.current, ship.fuel.capacity, fuel_percentage as i32);
    }
    
    println!("\n📊 RESULTS:");
    println!("   Ships with operational fuel: {}/{}", operational_ships, updated_ships.len());
    
    if operational_ships > 0 {
        println!("🎉 SUCCESS: {} ships can now operate!", operational_ships);
        println!("💡 Ready to proceed with mining operations");
        println!("💡 Next: cargo run --example test_survey_mining");
    } else {
        println!("❌ Still no operational ships");
        println!("💡 May need manual intervention or different fuel strategy");
        println!("💡 Consider checking game rules for fuel availability");
    }
    
    Ok(())
}