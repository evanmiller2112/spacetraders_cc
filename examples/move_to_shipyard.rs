use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    
    println!("🚀 MOVING CRITICAL SHIPS TO NEAREST SHIPYARD FOR REPAIR!");
    
    let target_shipyard = "X1-N5-A2"; // Nearest shipyard to our current location
    
    let ships = client.get_ships().await?;
    
    for ship in &ships {
        let condition = ship.frame.condition.unwrap_or(100.0);
        
        if condition < 70.0 && ship.nav.waypoint_symbol != target_shipyard {
            println!("🚨 {} ({:.0}% condition) at {} - MOVING TO SHIPYARD", 
                     ship.symbol, condition, ship.nav.waypoint_symbol);
            
            // Navigate to shipyard
            match client.navigate_ship(&ship.symbol, target_shipyard).await {
                Ok(_) => {
                    println!("   ✅ NAVIGATION TO {} STARTED!", target_shipyard);
                }
                Err(e) => {
                    println!("   ❌ Navigation failed: {}", e);
                }
            }
        } else if ship.nav.waypoint_symbol == target_shipyard {
            println!("✅ {} already at shipyard {}", ship.symbol, target_shipyard);
        }
    }
    
    Ok(())
}
