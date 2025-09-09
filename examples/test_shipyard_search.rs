// Test the shipyard search functionality
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    
    println!("🔍 Manual search for shipyards...");
    
    // First check our home system specifically
    println!("🏠 Checking home system X1-N5 first...");
    match client.get_system_waypoints("X1-N5", None).await {
        Ok(waypoints) => {
            println!("  📍 Found {} waypoints in home system", waypoints.len());
            for waypoint in &waypoints {
                let has_shipyard = waypoint.traits.iter().any(|t| t.symbol == "SHIPYARD");
                if has_shipyard {
                    println!("  🏭 SHIPYARD FOUND: {} ({})", waypoint.symbol, waypoint.waypoint_type);
                    println!("      Traits: {:?}", waypoint.traits.iter().map(|t| &t.symbol).collect::<Vec<_>>());
                } else {
                    println!("  {} ({}): {:?}", 
                        waypoint.symbol, 
                        waypoint.waypoint_type,
                        waypoint.traits.iter().map(|t| &t.symbol).collect::<Vec<_>>()
                    );
                }
            }
        }
        Err(e) => {
            println!("  ❌ Failed to get waypoints for home system: {}", e);
        }
    }
    
    // Get systems
    let systems = client.get_systems(Some(1), Some(20)).await?;
    println!("\n📊 Got {} systems to check", systems.len());
    
    let mut total_waypoints = 0;
    let mut shipyard_count = 0;
    
    for (i, system) in systems.iter().enumerate() {
        println!("\n🌌 [{}/{}] Checking system {}...", i+1, systems.len(), system.symbol);
        
        match client.get_system_waypoints(&system.symbol, None).await {
            Ok(waypoints) => {
                total_waypoints += waypoints.len();
                println!("  📍 Found {} waypoints", waypoints.len());
                
                // Count charted vs uncharted waypoints
                let charted = waypoints.iter().filter(|w| !w.traits.iter().any(|t| t.symbol == "UNCHARTED")).count();
                let uncharted = waypoints.len() - charted;
                
                if charted > 0 {
                    println!("  📊 Charted: {}, Uncharted: {}", charted, uncharted);
                    
                    // Only check charted waypoints for shipyards
                    let charted_waypoints: Vec<_> = waypoints.iter()
                        .filter(|w| !w.traits.iter().any(|t| t.symbol == "UNCHARTED"))
                        .collect();
                    
                    for waypoint in charted_waypoints {
                        let has_shipyard = waypoint.traits.iter().any(|t| t.symbol == "SHIPYARD");
                        if has_shipyard {
                            shipyard_count += 1;
                            println!("  🏭 SHIPYARD FOUND: {} ({})", waypoint.symbol, waypoint.waypoint_type);
                            println!("      Traits: {:?}", waypoint.traits.iter().map(|t| &t.symbol).collect::<Vec<_>>());
                        }
                    }
                    
                    if waypoints.iter().any(|w| w.traits.iter().any(|t| t.symbol == "SHIPYARD")) {
                        println!("  ✅ System {} has shipyards!", system.symbol);
                    } else {
                        println!("  ❓ System {} is charted but no shipyards found", system.symbol);
                    }
                } else {
                    println!("  ⚫ All waypoints uncharted in {}", system.symbol);
                }
            }
            Err(e) => {
                println!("  ❌ Failed to get waypoints for {}: {}", system.symbol, e);
            }
        }
    }
    
    println!("\n📊 SUMMARY:");
    println!("  🌌 Systems checked: {}", systems.len());
    println!("  📍 Total waypoints: {}", total_waypoints);
    println!("  🏭 Shipyards found: {}", shipyard_count);
    
    if shipyard_count == 0 {
        println!("❌ This is very unusual - there should be shipyards in the game!");
        println!("   Let's check if we're looking for the wrong trait name...");
        
        // Sample some waypoints and show their traits
        if let Ok(waypoints) = client.get_system_waypoints(&systems[0].symbol, None).await {
            println!("\n🔍 Sample traits from {}:", systems[0].symbol);
            for waypoint in waypoints.iter().take(5) {
                println!("  {} ({}): {:?}", 
                    waypoint.symbol, 
                    waypoint.waypoint_type,
                    waypoint.traits.iter().map(|t| &t.symbol).collect::<Vec<_>>()
                );
            }
        }
    }
    
    Ok(())
}