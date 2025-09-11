use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    
    println!("🔧 TESTING SINGLE SHIP REPAIR...");
    
    let test_ship = "CLAUDE_AGENT_2-6"; // Test with one ship
    
    // First check ship status
    let ship = client.get_ship(test_ship).await?;
    let condition = ship.frame.condition.unwrap_or(100.0);
    
    println!("🚢 Ship: {}", test_ship);
    println!("🔧 Condition: {:.0}%", condition);
    println!("📍 Location: {}", ship.nav.waypoint_symbol);
    println!("🚢 Status: {}", ship.nav.status);
    
    // Try repair with detailed error logging
    println!("\n🔧 ATTEMPTING REPAIR...");
    
    match client.repair_ship(test_ship).await {
        Ok(repair_data) => {
            let new_condition = repair_data.ship.frame.condition.unwrap_or(100.0);
            println!("✅ SUCCESS! {:.0}% → {:.0}%", condition, new_condition);
            println!("💰 Cost: {}💎", repair_data.transaction.total_price);
        }
        Err(e) => {
            println!("❌ REPAIR FAILED: {}", e);
            
            // Let's also try to get more details about the error
            if e.to_string().contains("422") {
                println!("💡 Status 422 suggests invalid request or location");
                println!("🔍 Double-checking if X1-N5-A2 supports repairs...");
            }
        }
    }
    
    Ok(())
}
