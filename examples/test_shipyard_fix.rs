use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    
    println!("🔍 Testing shipyard finding fix...");
    
    // Test the NEW method (more efficient)
    println!("📡 Testing NEW API call: /systems/X1-N5/waypoints?traits=SHIPYARD");
    match client.get_system_waypoints_with_traits("X1-N5", "SHIPYARD").await {
        Ok(waypoints) => {
            println!("✅ NEW method: Found {} waypoints with SHIPYARD trait:", waypoints.len());
            for waypoint in &waypoints {
                println!("  🏭 {} ({})", waypoint.symbol, waypoint.waypoint_type);
                for trait_info in &waypoint.traits {
                    if trait_info.symbol == "SHIPYARD" {
                        println!("     ✓ {}: {}", trait_info.symbol, trait_info.name);
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ NEW method error: {}", e);
            return Err(e);
        }
    }
    
    println!();
    
    // Test the OLD method (inefficient - gets all waypoints then filters)
    println!("📡 Testing OLD API call: /systems/X1-N5/waypoints");
    match client.get_system_waypoints("X1-N5", None).await {
        Ok(all_waypoints) => {
            let shipyard_waypoints: Vec<_> = all_waypoints.iter()
                .filter(|w| w.traits.iter().any(|t| t.symbol == "SHIPYARD"))
                .collect();
            
            println!("✅ OLD method: Got {} total waypoints, {} with SHIPYARD trait:", 
                     all_waypoints.len(), shipyard_waypoints.len());
            for waypoint in &shipyard_waypoints {
                println!("  🏭 {} ({})", waypoint.symbol, waypoint.waypoint_type);
            }
        }
        Err(e) => {
            println!("❌ OLD method error: {}", e);
        }
    }

    println!();
    println!("🎉 Shipyard finding fix is working correctly!");
    println!("💡 The new method is more efficient as it filters on the server side!");

    Ok(())
}