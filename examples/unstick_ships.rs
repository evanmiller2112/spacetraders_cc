// Unstick ships by refueling them at their current location
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    
    println!("🔧 Unsticking ships by refueling...");
    
    let stuck_ship_symbols = ["CLAUDE_AGENT_2-4", "CLAUDE_AGENT_2-5"];
    
    for ship_symbol in &stuck_ship_symbols {
        println!("\n🚢 Processing {}...", ship_symbol);
        
        // Get current ship status
        let ships = client.get_ships().await?;
        let ship = ships.iter().find(|s| s.symbol == *ship_symbol);
        
        if let Some(ship) = ship {
            println!("  📍 Current location: {}", ship.nav.waypoint_symbol);
            println!("  ⛽ Current fuel: {}/{}", ship.fuel.current, ship.fuel.capacity);
            println!("  🚀 Status: {}", ship.nav.status);
            
            // Check if ship needs fuel
            let fuel_needed = ship.fuel.capacity - ship.fuel.current;
            
            if fuel_needed > 0 {
                println!("  💡 Needs {} fuel units", fuel_needed);
                
                // Dock if not already docked
                if ship.nav.status != "DOCKED" {
                    println!("  🛸 Docking at {}...", ship.nav.waypoint_symbol);
                    match client.dock_ship(&ship.symbol).await {
                        Ok(_) => println!("    ✅ Successfully docked"),
                        Err(e) => {
                            println!("    ❌ Failed to dock: {}", e);
                            continue;
                        }
                    }
                } else {
                    println!("  ✅ Already docked");
                }
                
                // Refuel
                println!("  ⛽ Refueling {} units...", fuel_needed);
                match client.refuel_ship(&ship.symbol).await {
                    Ok(refuel_data) => {
                        println!("    ✅ Successfully refueled!");
                        println!("    💰 Cost: {} credits", refuel_data.transaction.total_price);
                        println!("    ⛽ New fuel level: {}/{}", refuel_data.fuel.current, refuel_data.fuel.capacity);
                        println!("    💰 Agent credits: {}", refuel_data.agent.credits);
                    }
                    Err(e) => {
                        println!("    ❌ Failed to refuel: {}", e);
                        
                        // Try partial refuel if full refuel failed
                        let partial_fuel = std::cmp::min(fuel_needed, 50);
                        println!("  🔄 Trying partial refuel of {} units...", partial_fuel);
                        match client.refuel_ship(&ship.symbol).await {
                            Ok(refuel_data) => {
                                println!("    ✅ Partial refuel successful!");
                                println!("    ⛽ New fuel level: {}/{}", refuel_data.fuel.current, refuel_data.fuel.capacity);
                            }
                            Err(e) => {
                                println!("    ❌ Partial refuel also failed: {}", e);
                            }
                        }
                    }
                }
                
                // Put back in orbit
                println!("  🚀 Putting ship back in orbit...");
                match client.orbit_ship(&ship.symbol).await {
                    Ok(_) => println!("    ✅ Ship back in orbit"),
                    Err(e) => println!("    ⚠️ Could not orbit: {}", e),
                }
                
            } else {
                println!("  ✅ Ship already has full fuel");
            }
        } else {
            println!("  ❌ Ship {} not found", ship_symbol);
        }
    }
    
    println!("\n🎉 Ship unsticking operation complete!");
    Ok(())
}