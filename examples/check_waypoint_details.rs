use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    
    println!("🔍 CHECKING X1-N5-A2 WAYPOINT DETAILS...");
    
    match client.get_waypoint("X1-N5", "X1-N5-A2").await {
        Ok(waypoint) => {
            println!("✅ WAYPOINT: {}", waypoint.symbol);
            println!("📍 TYPE: {}", waypoint.waypoint_type);
            println!("🏷️ TRAITS:");
            for trait_info in &waypoint.traits {
                println!("   - {} ({})", trait_info.symbol, trait_info.name);
            }
            
            // Check for repair-related traits
            let has_shipyard = waypoint.traits.iter().any(|t| t.symbol == "SHIPYARD");
            let has_marketplace = waypoint.traits.iter().any(|t| t.symbol == "MARKETPLACE");
            
            println!("\n🔧 REPAIR CAPABILITIES:");
            if has_shipyard {
                println!("   ✅ HAS SHIPYARD TRAIT");
            } else {
                println!("   ❌ NO SHIPYARD TRAIT");
            }
            
            if has_marketplace {
                println!("   ✅ HAS MARKETPLACE");
            }
            
            // Check if waypoint has shipyard data
            if let Some(ref shipyard) = waypoint.shipyard {
                println!("\n🏭 SHIPYARD SERVICES:");
                println!("   Ship types: {:?}", shipyard.ship_types);
                // Note: Repair services might not be directly listed in shipyard data
            } else {
                println!("\n❌ NO SHIPYARD DATA AVAILABLE");
            }
        }
        Err(e) => {
            println!("❌ Failed to get waypoint details: {}", e);
        }
    }
    
    println!("\n🔍 Checking other shipyards for comparison...");
    let other_shipyards = ["X1-N5-C37", "X1-N5-H49"];
    
    for shipyard_symbol in &other_shipyards {
        println!("\n🔧 CHECKING {}:", shipyard_symbol);
        match client.get_waypoint("X1-N5", shipyard_symbol).await {
            Ok(waypoint) => {
                let traits: Vec<_> = waypoint.traits.iter().map(|t| &t.symbol).collect();
                println!("   Traits: {:?}", traits);
                
                if waypoint.shipyard.is_some() {
                    println!("   ✅ Has shipyard data");
                } else {
                    println!("   ❌ No shipyard data");
                }
            }
            Err(e) => {
                println!("   ❌ Failed: {}", e);
            }
        }
    }
    
    Ok(())
}
