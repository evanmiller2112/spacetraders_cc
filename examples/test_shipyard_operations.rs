// Test shipyard operations system
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token};
use spacetraders_cc::operations::ShipyardOperations;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    
    println!("🏗️ Testing Shipyard Operations System");
    
    let shipyard_ops = ShipyardOperations::new(client.clone());
    
    // Find shipyards
    match shipyard_ops.find_shipyards().await {
        Ok(shipyards) => {
            if shipyards.is_empty() {
                println!("❌ No shipyards found - need to explore more systems");
                println!("💡 The autonomous bot should continue exploring until shipyards are discovered");
            } else {
                println!("✅ Found {} shipyards!", shipyards.len());
                
                // Get our current mining ship for reference
                let ships = client.get_ships().await?;
                if let Some(mining_ship) = ships.iter().find(|s| s.mounts.iter().any(|m| m.symbol.contains("MINING"))) {
                    println!("\n🎯 Reference mining ship: {}", mining_ship.symbol);
                    
                    // Show what would happen if we tried to purchase
                    let first_shipyard = &shipyards[0];
                    println!("🏭 Would attempt purchase at: {}", first_shipyard.waypoint_symbol);
                    
                    // For safety, don't actually purchase in this test
                    println!("⚠️ Purchase simulation only - not actually buying");
                    println!("💡 Ready to purchase when integrated with fleet coordinator");
                } else {
                    println!("❌ No mining ship found in current fleet");
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to find shipyards: {}", e);
        }
    }
    
    Ok(())
}