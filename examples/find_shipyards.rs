// Find shipyards across available systems
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("🔍 SEARCHING FOR SHIPYARDS");
    println!("==========================");
    
    // Get agent information to see starting system
    let agent = priority_client.get_agent().await?;
    println!("🤖 Agent: {} in {}", agent.symbol, agent.headquarters);
    
    // Get systems (use pagination to get more)
    println!("🌌 Scanning systems for shipyards...");
    let systems = priority_client.get_systems(None, Some(20)).await?;
    
    let mut shipyard_locations = Vec::new();
    
    for system in &systems {
        println!("🔍 Checking system: {}", system.symbol);
        
        match priority_client.get_system_waypoints(&system.symbol, None).await {
            Ok(waypoints) => {
                for waypoint in &waypoints {
                    let has_shipyard = waypoint.traits.iter().any(|t| t.symbol == "SHIPYARD");
                    if has_shipyard {
                        shipyard_locations.push(format!("{}", waypoint.symbol));
                        println!("✅ SHIPYARD FOUND: {} ({})", waypoint.symbol, waypoint.waypoint_type);
                        
                        // Show other traits
                        let other_traits: Vec<String> = waypoint.traits.iter()
                            .map(|t| t.symbol.clone())
                            .collect();
                        println!("   Traits: {}", other_traits.join(", "));
                    }
                }
            }
            Err(e) => {
                println!("⚠️ Error getting waypoints for {}: {}", system.symbol, e);
            }
        }
        
        // Brief pause to avoid rate limits
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
    
    println!("\n📊 SHIPYARD SUMMARY:");
    println!("   Found {} shipyards:", shipyard_locations.len());
    for (i, location) in shipyard_locations.iter().enumerate() {
        println!("   {}. {}", i + 1, location);
    }
    
    if shipyard_locations.is_empty() {
        println!("\n❌ No shipyards found in available systems");
        println!("💡 You may need to:");
        println!("   - Explore more systems");
        println!("   - Check if shipyards require discovery first");
    } else {
        println!("\n✅ Shipyards available for module installation!");
        println!("💡 Next: Navigate ships to a shipyard for mining equipment");
        
        // Show current ship locations for reference
        println!("\n🚢 Current ship locations:");
        let ships = priority_client.get_ships().await?;
        for ship in &ships {
            println!("   {}: {}", ship.symbol, ship.nav.waypoint_symbol);
        }
    }
    
    Ok(())
}