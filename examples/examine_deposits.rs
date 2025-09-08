// Quick utility to examine waypoint deposits and traits
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let mut client = SpaceTradersClient::new(token);
    client.set_debug_mode(false);
    
    println!("🔍 Examining waypoint deposits and traits in X1-N5 system...");
    
    // Get waypoints in the system
    let waypoints = client.get_system_waypoints("X1-N5", None).await?;
    
    println!("📊 Found {} waypoints in X1-N5", waypoints.len());
    
    for waypoint in &waypoints {
        if waypoint.waypoint_type == "ASTEROID" || waypoint.waypoint_type == "ENGINEERED_ASTEROID" {
            println!("\n🪨 {} ({})", waypoint.symbol, waypoint.waypoint_type);
            println!("   Traits:");
            for trait_info in &waypoint.traits {
                println!("     • {} - {}", trait_info.symbol, trait_info.name);
                if trait_info.symbol.contains("DEPOSIT") {
                    println!("       ⭐ DEPOSIT: {}", trait_info.description);
                }
            }
        }
    }
    
    println!("\n🎯 Contract needs: COPPER_ORE");
    println!("💡 Looking for deposit traits that might contain metal ores...");
    
    // Show all unique deposit traits across all asteroids
    let mut deposit_traits = std::collections::HashSet::new();
    for waypoint in &waypoints {
        if waypoint.waypoint_type == "ASTEROID" || waypoint.waypoint_type == "ENGINEERED_ASTEROID" {
            for trait_info in &waypoint.traits {
                if trait_info.symbol.contains("DEPOSIT") {
                    deposit_traits.insert(trait_info.symbol.clone());
                }
            }
        }
    }
    
    println!("\n📋 All deposit types found:");
    for deposit in &deposit_traits {
        println!("   • {}", deposit);
    }
    
    Ok(())
}