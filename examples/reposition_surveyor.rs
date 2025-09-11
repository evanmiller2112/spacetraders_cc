// Quick fix: Get surveyor back to mining location
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("🚀 SURVEYOR REPOSITIONING");
    println!("========================");
    
    let surveyor = "CLAUDE_AGENT_2-1";
    let target = "X1-N5-BA5F";
    
    // Check current status
    let surveyor_ship = priority_client.get_ship(surveyor).await?;
    println!("📍 Current location: {}", surveyor_ship.nav.waypoint_symbol);
    println!("⛽ Fuel: {}/{}", surveyor_ship.fuel.current, surveyor_ship.fuel.capacity);
    println!("🚢 Status: {}", surveyor_ship.nav.status);
    
    if surveyor_ship.nav.waypoint_symbol == target {
        println!("✅ Already at target location!");
        return Ok(());
    }
    
    // Orbit if docked
    if surveyor_ship.nav.status == "DOCKED" {
        println!("🛸 Moving to orbit...");
        priority_client.orbit_ship(surveyor).await?;
    }
    
    // Navigate to mining location
    println!("🚀 Navigating to {}...", target);
    match priority_client.navigate_ship(surveyor, target).await {
        Ok(nav_result) => {
            println!("✅ Navigation successful!");
            
            if let Ok(arrival_time) = nav_result.nav.route.arrival.parse::<chrono::DateTime<chrono::Utc>>() {
                let now = chrono::Utc::now();
                let wait_seconds = (arrival_time - now).num_seconds().max(0) as u64;
                println!("⏳ Arrival in {} seconds", wait_seconds);
                
                if wait_seconds > 0 && wait_seconds < 300 { // Wait up to 5 minutes
                    println!("⌛ Waiting for arrival...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(wait_seconds + 3)).await;
                    
                    // Check final position
                    let final_ship = priority_client.get_ship(surveyor).await?;
                    println!("📍 Final location: {}", final_ship.nav.waypoint_symbol);
                    println!("⛽ Final fuel: {}/{}", final_ship.fuel.current, final_ship.fuel.capacity);
                    
                    if final_ship.nav.waypoint_symbol == target {
                        println!("🎯 Surveyor successfully positioned at mining location!");
                        println!("💡 Ready to run iron ore blitz campaign!");
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Navigation failed: {}", e);
            println!("💡 This might be the multi-hop routing issue");
        }
    }
    
    Ok(())
}