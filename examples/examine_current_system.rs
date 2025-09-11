// Examine our current system X1-N5 in detail
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("🔍 EXAMINING CURRENT SYSTEM X1-N5");
    println!("=================================");
    
    // Get waypoints in our system
    let waypoints = priority_client.get_system_waypoints("X1-N5", None).await?;
    
    println!("📍 Found {} waypoints in X1-N5:", waypoints.len());
    
    for waypoint in &waypoints {
        println!("\n🌟 Waypoint: {} ({})", waypoint.symbol, waypoint.waypoint_type);
        println!("   Coordinates: ({}, {})", waypoint.x, waypoint.y);
        
        if !waypoint.traits.is_empty() {
            println!("   🏷️ Traits:");
            for trait_item in &waypoint.traits {
                println!("      - {} ({})", trait_item.name, trait_item.symbol);
                
                // Check for shipyard-related traits
                if trait_item.symbol.contains("SHIPYARD") || trait_item.name.to_lowercase().contains("shipyard") {
                    println!("         ⭐ SHIPYARD DETECTED!");
                }
                if trait_item.symbol.contains("MARKETPLACE") {
                    println!("         🛒 MARKETPLACE DETECTED!");
                }
            }
        }
        
        // Check if this waypoint has a faction
        if let Some(faction) = &waypoint.faction {
            println!("   🏛️ Faction: {}", faction.symbol);
        }
        
        // Check if orbiting anything
        if !waypoint.orbitals.is_empty() {
            println!("   🛰️ Orbitals: {:?}", waypoint.orbitals);
        }
        
        // Special attention to certain types
        match waypoint.waypoint_type.as_str() {
            "ORBITAL_STATION" => println!("   🏗️ This is an orbital station - could have services!"),
            "PLANET" => println!("   🌍 This is a planet - could have surface facilities!"),
            "MOON" => println!("   🌙 This is a moon - could have mining facilities!"),
            "ASTEROID_FIELD" => println!("   ⭐ This is an asteroid field - mining location!"),
            _ => {}
        }
    }
    
    // Show where our ships are currently located
    println!("\n🚢 Current ship locations:");
    let ships = priority_client.get_ships().await?;
    for ship in &ships {
        println!("   {}: {} ({})", ship.symbol, ship.nav.waypoint_symbol, ship.nav.status);
    }
    
    // Look for waypoints that might be shipyards
    println!("\n🔍 Potential service locations:");
    for waypoint in &waypoints {
        let has_shipyard_trait = waypoint.traits.iter().any(|t| 
            t.symbol.contains("SHIPYARD") || 
            t.name.to_lowercase().contains("shipyard") ||
            t.symbol == "SHIPYARD"
        );
        
        let has_marketplace = waypoint.traits.iter().any(|t| t.symbol == "MARKETPLACE");
        let is_station = waypoint.waypoint_type == "ORBITAL_STATION";
        
        if has_shipyard_trait || has_marketplace || is_station {
            println!("🎯 {}: shipyard={}, marketplace={}, station={}", 
                     waypoint.symbol, has_shipyard_trait, has_marketplace, is_station);
        }
    }
    
    Ok(())
}