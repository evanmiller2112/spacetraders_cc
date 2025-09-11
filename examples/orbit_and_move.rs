use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    
    println!("🚀 ORBIT DOCKED SHIPS AND MOVE TO SHIPYARD!");
    
    let target_shipyard = "X1-N5-A2";
    let docked_ships = ["CLAUDE_AGENT_2-1", "CLAUDE_AGENT_2-2", "CLAUDE_AGENT_2-3"];
    
    for ship_symbol in &docked_ships {
        println!("🚢 Processing {}...", ship_symbol);
        
        // Orbit first
        match client.orbit_ship(ship_symbol).await {
            Ok(_) => {
                println!("   ✅ ORBITED");
                
                // Then navigate
                match client.navigate_ship(ship_symbol, target_shipyard).await {
                    Ok(_) => {
                        println!("   ✅ NAVIGATION TO {} STARTED!", target_shipyard);
                    }
                    Err(e) => {
                        println!("   ❌ Navigation failed: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("   ❌ Orbit failed: {}", e);
            }
        }
    }
    
    Ok(())
}
