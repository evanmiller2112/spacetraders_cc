// 🔍 WAYPOINT FACILITY CHECKER - Find repair stations! 🔍
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("🔍🔍🔍 WAYPOINT FACILITY CHECKER 🔍🔍🔍");
    println!("=====================================");
    println!("🎯 FINDING REPAIR FACILITIES!");
    
    let system = "X1-N5";
    
    // Get all waypoints in the system
    println!("\n📍 SCANNING SYSTEM {} FOR REPAIR FACILITIES:", system);
    let waypoints = priority_client.get_system_waypoints(system, None).await?;
    
    println!("   Found {} waypoints in system", waypoints.len());
    
    for waypoint in &waypoints {
        println!("\n🌟 {} ({})", waypoint.symbol, waypoint.waypoint_type);
        println!("   Traits: {:?}", waypoint.traits.iter().map(|t| &t.symbol).collect::<Vec<_>>());
        
        // Check for repair-related traits
        let has_shipyard = waypoint.traits.iter().any(|t| t.symbol == "SHIPYARD");
        let has_marketplace = waypoint.traits.iter().any(|t| t.symbol == "MARKETPLACE");
        let has_outpost = waypoint.traits.iter().any(|t| t.symbol == "OUTPOST");
        
        if has_shipyard {
            println!("   🔧 SHIPYARD DETECTED - Likely has repair facilities!");
        }
        if has_marketplace {
            println!("   💰 MARKETPLACE - Commercial facilities");
        }
        if has_outpost {
            println!("   🏭 OUTPOST - Basic facilities");
        }
        
        // Check if this is where our ships are
        if waypoint.symbol == "X1-N5-A1" {
            println!("   ⭐ THIS IS WHERE OUR SHIPS ARE CURRENTLY!");
        }
        if waypoint.symbol == "X1-N5-BA5F" {
            println!("   🏭 THIS IS OUR MINING LOCATION!");
        }
    }
    
    // Look specifically for shipyards
    println!("\n🔧 SHIPYARDS IN SYSTEM (likely repair facilities):");
    let shipyards: Vec<_> = waypoints.iter()
        .filter(|w| w.traits.iter().any(|t| t.symbol == "SHIPYARD"))
        .collect();
    
    if shipyards.is_empty() {
        println!("   ❌ NO SHIPYARDS FOUND IN SYSTEM!");
        println!("   💡 May need to travel to another system for repairs");
    } else {
        for shipyard in &shipyards {
            println!("   🔧 {}: {} ({:?})", 
                     shipyard.symbol, 
                     shipyard.waypoint_type,
                     shipyard.traits.iter().map(|t| &t.symbol).collect::<Vec<_>>());
        }
    }
    
    // Check current ship locations vs repair facilities
    println!("\n🚢 SHIP LOCATIONS VS REPAIR FACILITIES:");
    let ships = priority_client.get_ships().await?;
    
    for ship in &ships {
        let condition = ship.frame.condition.unwrap_or(100.0);
        let at_shipyard = shipyards.iter().any(|s| s.symbol == ship.nav.waypoint_symbol);
        
        let facility_status = if at_shipyard {
            "✅ AT SHIPYARD"
        } else {
            "❌ NO REPAIR FACILITY"
        };
        
        println!("   🚢 {}: {:.0}% at {} - {}", 
                 ship.symbol, 
                 condition, 
                 ship.nav.waypoint_symbol, 
                 facility_status);
    }
    
    // Recommendations
    println!("\n💡 REPAIR RECOMMENDATIONS:");
    if !shipyards.is_empty() {
        println!("   🎯 Move ships to shipyard waypoints for repairs:");
        for shipyard in &shipyards {
            println!("      🔧 {}", shipyard.symbol);
        }
    } else {
        println!("   🚀 Consider exploring nearby systems for shipyards");
        println!("   💡 Shipyards typically have SHIPYARD trait and repair facilities");
    }
    
    Ok(())
}