// Find waypoints with faction presence for contract negotiation
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    
    println!("🔍 Looking for faction waypoints in system X1-N5...");
    
    // Get all waypoints in the system
    let waypoints = client.get_system_waypoints("X1-N5", None).await?;
    
    println!("📍 Found {} waypoints total", waypoints.len());
    
    // Check each waypoint for faction presence
    let mut faction_waypoints = Vec::new();
    
    for waypoint in &waypoints {
        if let Some(faction) = &waypoint.faction {
            faction_waypoints.push(waypoint);
            println!("🏛️ {} - {} controlled by {}", 
                    waypoint.symbol, 
                    waypoint.waypoint_type, 
                    faction.symbol);
        }
    }
    
    if faction_waypoints.is_empty() {
        println!("❌ No faction waypoints found in X1-N5");
        println!("💡 Ships need to be at faction waypoints to negotiate contracts");
    } else {
        println!("\n✅ Found {} faction waypoints:", faction_waypoints.len());
        for wp in &faction_waypoints {
            if let Some(faction) = &wp.faction {
                println!("  🏛️ {} ({}): {} faction", 
                        wp.symbol, 
                        wp.waypoint_type, 
                        faction.symbol);
                
                // Check if it has useful traits like marketplace
                let traits: Vec<_> = wp.traits.iter().map(|t| &t.symbol).collect();
                println!("    Traits: {:?}", traits);
            }
        }
        
        // Check current ship positions
        println!("\n🚢 Current ship positions:");
        let ships = client.get_ships().await?;
        for ship in &ships {
            let at_faction_waypoint = faction_waypoints.iter()
                .any(|wp| wp.symbol == ship.nav.waypoint_symbol);
            
            if at_faction_waypoint {
                println!("  ✅ {} at {} (FACTION WAYPOINT)", ship.symbol, ship.nav.waypoint_symbol);
            } else {
                println!("  ❌ {} at {} (not a faction waypoint)", ship.symbol, ship.nav.waypoint_symbol);
            }
        }
    }
    
    Ok(())
}