// Debug shipyard access directly via API
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    
    println!("🔍 Direct API test for shipyards...");
    
    // Get our agent info to see starting system
    let agent = client.get_agent().await?;
    println!("🏠 Agent headquarters: {}", agent.headquarters);
    
    // Extract system from headquarters (e.g., X1-N5-A1 -> X1-N5)
    let system_parts: Vec<&str> = agent.headquarters.split('-').collect();
    let home_system = format!("{}-{}", system_parts[0], system_parts[1]);
    println!("🌌 Home system: {}", home_system);
    
    // Try to get waypoints with type filter for specific types that might have shipyards
    let waypoint_types = vec!["PLANET", "ASTEROID_BASE", "ORBITAL_STATION"];
    
    for wp_type in waypoint_types {
        println!("\n🔍 Checking {} waypoints in {}...", wp_type, home_system);
        match client.get_system_waypoints(&home_system, Some(wp_type)).await {
            Ok(waypoints) => {
                println!("  📍 Found {} {} waypoints", waypoints.len(), wp_type);
                for waypoint in &waypoints {
                    println!("    {} - Traits: {:?}", 
                        waypoint.symbol, 
                        waypoint.traits.iter().map(|t| &t.symbol).collect::<Vec<_>>()
                    );
                    
                    // Check if this waypoint has a shipyard
                    if waypoint.traits.iter().any(|t| t.symbol == "SHIPYARD") {
                        println!("    🏭 *** SHIPYARD FOUND! *** {}", waypoint.symbol);
                    }
                }
            }
            Err(e) => {
                println!("  ❌ Failed to get {} waypoints: {}", wp_type, e);
            }
        }
    }
    
    // Also try getting ALL waypoints without filter
    println!("\n🔍 Getting ALL waypoints in {}...", home_system);
    match client.get_system_waypoints(&home_system, None).await {
        Ok(waypoints) => {
            println!("  📍 Found {} total waypoints", waypoints.len());
            let mut found_shipyard = false;
            
            for waypoint in &waypoints {
                if waypoint.traits.iter().any(|t| t.symbol == "SHIPYARD") {
                    println!("  🏭 *** SHIPYARD FOUND! *** {} - Traits: {:?}", 
                        waypoint.symbol, 
                        waypoint.traits.iter().map(|t| &t.symbol).collect::<Vec<_>>()
                    );
                    found_shipyard = true;
                }
            }
            
            if !found_shipyard {
                println!("  ❌ No shipyards found in home system");
                println!("  📊 Sample waypoint traits:");
                for waypoint in waypoints.iter().take(5) {
                    println!("    {} ({}): {:?}", 
                        waypoint.symbol, 
                        waypoint.waypoint_type,
                        waypoint.traits.iter().map(|t| &t.symbol).collect::<Vec<_>>()
                    );
                }
            }
        }
        Err(e) => {
            println!("  ❌ Failed to get waypoints: {}", e);
        }
    }
    
    Ok(())
}