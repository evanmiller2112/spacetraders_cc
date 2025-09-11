// Find shipyards using the traits filter
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("🔍 FINDING SHIPYARDS IN X1-N5 (CORRECT METHOD)");
    println!("===============================================");
    
    // Use the traits filter to find shipyards
    match priority_client.get_system_waypoints_with_traits("X1-N5", "SHIPYARD").await {
        Ok(shipyards) => {
            if shipyards.is_empty() {
                println!("❌ No shipyards found with SHIPYARD trait in X1-N5");
            } else {
                println!("✅ Found {} shipyard(s) in X1-N5:", shipyards.len());
                
                for shipyard in &shipyards {
                    println!("\n🏗️ SHIPYARD: {}", shipyard.symbol);
                    println!("   Type: {}", shipyard.waypoint_type);
                    println!("   Coordinates: ({}, {})", shipyard.x, shipyard.y);
                    
                    println!("   🏷️ Traits:");
                    for trait_item in &shipyard.traits {
                        println!("      - {} ({})", trait_item.name, trait_item.symbol);
                    }
                    
                    if let Some(faction) = &shipyard.faction {
                        println!("   🏛️ Faction: {}", faction.symbol);
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Error getting shipyards: {}", e);
        }
    }
    
    // Also check if there are any orbital stations that might be shipyards
    println!("\n🔍 Also checking for orbital stations...");
    match priority_client.get_system_waypoints("X1-N5", Some("ORBITAL_STATION")).await {
        Ok(stations) => {
            if !stations.is_empty() {
                println!("🏭 Found {} orbital station(s):", stations.len());
                for station in &stations {
                    println!("   {} - checking for shipyard services...", station.symbol);
                    
                    let has_shipyard_trait = station.traits.iter().any(|t| t.symbol == "SHIPYARD");
                    if has_shipyard_trait {
                        println!("   ✅ This station has shipyard services!");
                    } else {
                        println!("   ❌ No shipyard services here");
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Error getting orbital stations: {}", e);
        }
    }
    
    // Show current ship locations for reference
    println!("\n🚢 Current ship locations:");
    let ships = priority_client.get_ships().await?;
    for ship in &ships {
        println!("   {}: {}", ship.symbol, ship.nav.waypoint_symbol);
    }
    
    Ok(())
}