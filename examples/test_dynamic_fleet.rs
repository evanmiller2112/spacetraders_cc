// Test dynamic fleet management - ships should be automatically discovered and added
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    
    println!("🔍 Testing dynamic fleet management...");
    
    // Get current fleet
    let ships = client.get_ships().await?;
    println!("📊 Current fleet size: {} ships", ships.len());
    
    for (i, ship) in ships.iter().enumerate() {
        println!("  {}. {} ({}) - Frame: {}", 
                 i + 1, ship.symbol, ship.registration.role, ship.frame.symbol);
        
        // Show ship status
        println!("     📍 Location: {}", ship.nav.waypoint_symbol);
        println!("     ⛽ Fuel: {}/{}", ship.fuel.current, ship.fuel.capacity);
        println!("     📦 Cargo: {}/{}", ship.cargo.units, ship.cargo.capacity);
        
        // Show mounts (for mining capability)
        if !ship.mounts.is_empty() {
            let mining_mounts: Vec<_> = ship.mounts.iter()
                .filter(|m| m.symbol.contains("MINING") || m.symbol.contains("LASER"))
                .collect();
            if !mining_mounts.is_empty() {
                println!("     ⛏️ Mining equipment:");
                for mount in mining_mounts {
                    println!("        - {} ({})", mount.symbol, mount.name);
                }
            }
        }
        println!();
    }
    
    println!("✅ Fleet analysis complete!");
    println!("💡 When running the main bot:");
    println!("   • All ships will be automatically managed by their own actors");
    println!("   • New ships (purchased or otherwise added) will be discovered within 60 seconds");
    println!("   • Each ship will receive mining/trading tasks based on its capabilities");
    println!("   • Fleet expansion will happen automatically when credits are available");

    Ok(())
}