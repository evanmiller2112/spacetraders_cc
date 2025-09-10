// Test contract negotiation with detailed error reporting
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    
    println!("🔍 Testing contract negotiation with detailed error analysis...");
    
    // First, check current contracts
    let contracts = client.get_contracts().await?;
    println!("📋 Current Contracts: {}", contracts.len());
    
    for contract in &contracts {
        println!("  - {} ({}): accepted={}, fulfilled={}", 
                contract.id, 
                contract.contract_type,
                contract.accepted, 
                contract.fulfilled);
    }
    
    // Check ships and their locations
    let ships = client.get_ships().await?;
    println!("\n🚢 Ships:");
    
    for ship in &ships {
        println!("  - {} at {}", ship.symbol, ship.nav.waypoint_symbol);
        
        // Check if this waypoint has faction presence
        let waypoint_parts: Vec<&str> = ship.nav.waypoint_symbol.split('-').collect();
        let system_symbol = format!("{}-{}", waypoint_parts[0], waypoint_parts[1]);
        
        match client.get_waypoint(&system_symbol, &ship.nav.waypoint_symbol).await {
            Ok(waypoint) => {
                if let Some(faction) = &waypoint.faction {
                    println!("    ✅ Faction waypoint: {} controlled by {}", waypoint.symbol, faction.symbol);
                    
                    // Try contract negotiation
                    println!("    🤝 Attempting negotiation...");
                    match client.negotiate_contract(&ship.symbol).await {
                        Ok(new_contract) => {
                            println!("    ✅ SUCCESS! New contract: {}", new_contract.id);
                            println!("      Type: {}", new_contract.contract_type);
                            println!("      Faction: {}", new_contract.faction_symbol);
                            return Ok(());
                        }
                        Err(e) => {
                            println!("    ❌ Failed: {}", e);
                            
                            // Parse the error for more details
                            let error_str = e.to_string();
                            if error_str.contains("400") {
                                if error_str.contains("maximum") || error_str.contains("contract") {
                                    println!("      💡 Likely cause: Already at maximum contracts (completed contract blocking slot)");
                                } else if error_str.contains("faction") {
                                    println!("      💡 Likely cause: Ship not at correct faction waypoint");
                                } else {
                                    println!("      💡 Other 400 error - check requirements");
                                }
                            }
                        }
                    }
                } else {
                    println!("    ❌ No faction at this waypoint");
                }
            }
            Err(e) => {
                println!("    ⚠️ Could not check waypoint: {}", e);
            }
        }
        
        println!(); // Empty line for readability
    }
    
    Ok(())
}