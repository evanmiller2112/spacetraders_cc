// Debug stuck ships and their refueling options
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    
    println!("🔍 Debug stuck ships...");
    
    // Check stuck ships
    let ships = client.get_ships().await?;
    let stuck_ships: Vec<_> = ships.iter()
        .filter(|ship| ship.symbol == "CLAUDE_AGENT_2-4" || ship.symbol == "CLAUDE_AGENT_2-5")
        .collect();
    
    for ship in &stuck_ships {
        println!("🚢 {} Status:", ship.symbol);
        println!("  📍 Location: {}", ship.nav.waypoint_symbol);
        println!("  ⛽ Fuel: {}/{} units", ship.fuel.current, ship.fuel.capacity);
        println!("  📦 Cargo: {}/{} units", ship.cargo.units, ship.cargo.capacity);
        println!("  🚀 Status: {}", ship.nav.status);
        
        // Check current waypoint details
        println!("  🔍 Checking waypoint {}...", ship.nav.waypoint_symbol);
        
        // Extract system from waypoint (e.g., X1-N5-H48 -> X1-N5)
        let waypoint_parts: Vec<&str> = ship.nav.waypoint_symbol.split('-').collect();
        let system_symbol = format!("{}-{}", waypoint_parts[0], waypoint_parts[1]);
        
        match client.get_waypoint(&system_symbol, &ship.nav.waypoint_symbol).await {
            Ok(waypoint) => {
                println!("    Type: {}", waypoint.waypoint_type);
                println!("    Traits: {:?}", waypoint.traits.iter().map(|t| &t.symbol).collect::<Vec<_>>());
                
                // Check if it has a marketplace
                let has_marketplace = waypoint.traits.iter().any(|t| t.symbol == "MARKETPLACE");
                if has_marketplace {
                    println!("    🏪 ✅ HAS MARKETPLACE - can refuel here!");
                    
                    // Check marketplace for fuel
                    match client.get_market(&system_symbol, &ship.nav.waypoint_symbol).await {
                        Ok(market) => {
                            if let Some(trade_goods) = &market.trade_goods {
                                let fuel_available = trade_goods.iter()
                                    .find(|good| good.symbol == "FUEL");
                                if let Some(fuel) = fuel_available {
                                    println!("    ⛽ Fuel available: {} units at {} credits/unit", 
                                            fuel.trade_volume, fuel.purchase_price);
                                } else {
                                    println!("    ❌ No fuel for sale at this marketplace");
                                }
                            } else {
                                println!("    ⚠️ Market data not available (need to dock first)");
                            }
                        }
                        Err(e) => {
                            println!("    ⚠️ Could not fetch market data: {}", e);
                        }
                    }
                } else {
                    println!("    ❌ NO MARKETPLACE - cannot refuel here");
                }
            }
            Err(e) => {
                println!("    ❌ Could not fetch waypoint details: {}", e);
            }
        }
        
        println!();
    }
    
    // Find nearest refueling stations
    println!("🔍 Finding refueling options in system X1-N5...");
    let waypoints = client.get_system_waypoints("X1-N5", None).await?;
    
    let refuel_stations: Vec<_> = waypoints.iter()
        .filter(|wp| wp.traits.iter().any(|t| t.symbol == "MARKETPLACE"))
        .collect();
    
    println!("📍 Available refueling stations:");
    for station in &refuel_stations {
        println!("  🏪 {} ({})", station.symbol, station.waypoint_type);
        println!("    Traits: {:?}", station.traits.iter().map(|t| &t.symbol).collect::<Vec<_>>());
    }
    
    Ok(())
}