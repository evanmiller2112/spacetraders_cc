use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    
    println!("🔧 FINDING SHIPYARDS IN X1-N5 WITH TRAITS FILTER...");
    
    match client.get_system_waypoints_with_traits("X1-N5", "SHIPYARD").await {
        Ok(shipyards) => {
            println!("✅ FOUND {} SHIPYARDS!", shipyards.len());
            for shipyard in &shipyards {
                println!("🔧 SHIPYARD: {} ({})", shipyard.symbol, shipyard.waypoint_type);
            }
        }
        Err(e) => {
            println!("❌ ERROR: {}", e);
        }
    }
    
    Ok(())
}
