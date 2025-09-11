use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    
    println!("💰 CHECKING REPAIR COST WITH GET REQUEST...");
    
    let test_ship = "CLAUDE_AGENT_2-6";
    
    // Test GET request for repair cost
    println!("🔧 GET /my/ships/{}/repair", test_ship);
    
    match client.get_repair_cost(test_ship).await {
        Ok(cost) => {
            println!("✅ SUCCESS!");
            println!("💰 Repair cost: {}💎", cost.transaction.total_price);
            println!("📍 Waypoint: {}", cost.transaction.waypoint_symbol);
            println!("🚢 Ship: {}", cost.transaction.ship_symbol);
            println!("📅 Timestamp: {}", cost.transaction.timestamp);
        }
        Err(e) => {
            println!("❌ FAILED: {}", e);
            
            if e.to_string().contains("422") {
                println!("💡 Status 422: Unprocessable Entity");
                println!("   Possible causes:");
                println!("   - Ship not at a location with repair facilities");
                println!("   - Ship not docked");
                println!("   - Location doesn't support repairs");
            }
        }
    }
    
    // Also check current ship status
    let ship = client.get_ship(test_ship).await?;
    println!("\n📊 CURRENT SHIP STATUS:");
    println!("   Location: {}", ship.nav.waypoint_symbol);
    println!("   Status: {}", ship.nav.status);
    println!("   Condition: {:.0}%", ship.frame.condition.unwrap_or(100.0));
    
    Ok(())
}
