use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    
    println!("🔧 DOCK ALL SHIPS AND ATTEMPT REPAIRS!");
    
    let ships = client.get_ships().await?;
    
    // First dock all ships
    for ship in &ships {
        if ship.nav.status == "IN_ORBIT" {
            println!("🚢 Docking {}...", ship.symbol);
            match client.dock_ship(&ship.symbol).await {
                Ok(_) => println!("   ✅ DOCKED"),
                Err(e) => println!("   ❌ Dock failed: {}", e),
            }
        }
    }
    
    println!("\n🔧 ATTEMPTING REPAIRS...");
    
    // Then try to repair each ship
    for ship in &ships {
        let condition = ship.frame.condition.unwrap_or(100.0);
        if condition < 70.0 {
            println!("\n🔧 REPAIRING {} ({:.0}% condition)...", ship.symbol, condition);
            
            // Get repair cost
            match client.get_repair_cost(&ship.symbol).await {
                Ok(cost) => {
                    println!("   💰 Repair cost: {}💎", cost.transaction.total_price);
                    
                    // Execute repair
                    match client.repair_ship(&ship.symbol).await {
                        Ok(repair_data) => {
                            let new_condition = repair_data.ship.frame.condition.unwrap_or(100.0);
                            println!("   ✅ REPAIRED: {:.0}% → {:.0}%! Cost: {}💎", 
                                     condition, new_condition, repair_data.transaction.total_price);
                        }
                        Err(e) => {
                            println!("   ❌ Repair failed: {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("   ❌ Cost check failed: {}", e);
                }
            }
        }
    }
    
    Ok(())
}
