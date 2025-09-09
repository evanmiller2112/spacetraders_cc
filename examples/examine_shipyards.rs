// Examine available shipyards and ships for purchase
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let mut client = SpaceTradersClient::new(token);
    client.set_debug_mode(false);
    
    println!("🏗️ Examining shipyards and available ships...");
    
    // First check our current system
    let waypoints = client.get_system_waypoints("X1-N5", None).await?;
    
    // Find shipyards and show all waypoint types for debugging
    println!("📊 All waypoints in X1-N5 ({} total):", waypoints.len());
    for waypoint in &waypoints {
        let trait_symbols: Vec<String> = waypoint.traits.iter().map(|t| t.symbol.clone()).collect();
        println!("  {} ({}) - Traits: {:?}", waypoint.symbol, waypoint.waypoint_type, trait_symbols);
    }
    
    let shipyards: Vec<_> = waypoints.iter()
        .filter(|w| w.traits.iter().any(|t| t.symbol == "SHIPYARD"))
        .collect();
    
    println!("\n🏭 Found {} shipyards in X1-N5:", shipyards.len());
    
    // If no shipyards in current system, search nearby systems
    if shipyards.is_empty() {
        println!("\n🔍 No shipyards in X1-N5, searching nearby systems...");
        
        // Get a list of systems and check them for shipyards
        println!("   🔍 Getting list of available systems...");
        let systems = client.get_systems(Some(1), Some(5)).await?; // Get first 5 systems only
        println!("   📊 Found {} systems to check", systems.len());
        
        let systems_to_check: Vec<String> = systems.iter().map(|s| s.symbol.clone()).collect();
        
        for system_symbol in systems_to_check {
            println!("\n🌌 Checking system {}...", system_symbol);
            match client.get_system_waypoints(&system_symbol, None).await {
                Ok(system_waypoints) => {
                    let system_shipyards: Vec<_> = system_waypoints.iter()
                        .filter(|w| w.traits.iter().any(|t| t.symbol == "SHIPYARD"))
                        .collect();
                    
                    if !system_shipyards.is_empty() {
                        println!("   🏭 Found {} shipyards in {}!", system_shipyards.len(), system_symbol);
                        for shipyard_wp in &system_shipyards {
                            println!("     • {} ({})", shipyard_wp.symbol, shipyard_wp.waypoint_type);
                        }
                        
                        // Get details for the first shipyard we find
                        if let Some(first_shipyard) = system_shipyards.first() {
                            match client.get_shipyard(&system_symbol, &first_shipyard.symbol).await {
                                Ok(shipyard) => {
                                    println!("     💰 Modification fee: {} credits", shipyard.modifications_fee);
                                    if let Some(ships) = &shipyard.ships {
                                        println!("     🚢 Available ships ({}):", ships.len());
                                        for ship in ships.iter().take(3) { // Show first 3 ships
                                            println!("       • {} - {} credits", ship.ship_type, ship.purchase_price);
                                        }
                                    }
                                }
                                Err(e) => println!("     ❌ Failed to get shipyard details: {}", e),
                            }
                        }
                        break; // Found shipyards, stop searching
                    } else {
                        println!("   ❌ No shipyards in {}", system_symbol);
                    }
                }
                Err(e) => {
                    println!("   ⚠️ Failed to access {}: {}", system_symbol, e);
                }
            }
        }
    }
    
    for shipyard_waypoint in shipyards {
        println!("\n🏭 Shipyard at {} ({})", shipyard_waypoint.symbol, shipyard_waypoint.waypoint_type);
        
        // Get shipyard details
        match client.get_shipyard("X1-N5", &shipyard_waypoint.symbol).await {
            Ok(shipyard) => {
                println!("   💰 Modification fee: {} credits", shipyard.modifications_fee);
                
                if let Some(ships) = &shipyard.ships {
                    println!("   🚢 Available ships ({}):", ships.len());
                    for ship in ships {
                        println!("     • {} - {} credits", ship.ship_type, ship.purchase_price);
                        println!("       📝 {}", ship.description);
                        println!("       🔧 Frame: {}", ship.frame.symbol);
                        println!("       ⚡ Engine: {}", ship.engine.symbol);
                        println!("       🔋 Reactor: {}", ship.reactor.symbol);
                        
                        if !ship.mounts.is_empty() {
                            println!("       🛠️ Mounts:");
                            for mount in &ship.mounts {
                                println!("         - {} ({})", mount.symbol, mount.name);
                            }
                        }
                        
                        if !ship.modules.is_empty() {
                            println!("       📦 Modules:");
                            for module in &ship.modules {
                                println!("         - {} ({})", module.symbol, module.name);
                            }
                        }
                        println!();
                    }
                } else {
                    println!("   ⚠️ No ships available for purchase");
                }
                
                if let Some(transactions) = &shipyard.transactions {
                    if !transactions.is_empty() {
                        println!("   📊 Recent transactions: {}", transactions.len());
                        for (i, tx) in transactions.iter().take(3).enumerate() {
                            println!("     {}. {} bought {} for {} credits", 
                                    i + 1, tx.agent_symbol, tx.ship_symbol, tx.price);
                        }
                    }
                }
            }
            Err(e) => {
                println!("   ❌ Failed to get shipyard details: {}", e);
            }
        }
    }
    
    // Get current ships for comparison
    println!("\n🚢 Current fleet for comparison:");
    let ships = client.get_ships().await?;
    for ship in ships {
        println!("  {} ({}) - Frame: {}", ship.symbol, ship.registration.role, ship.frame.symbol);
        println!("    🔧 Mounts: {:?}", ship.mounts.iter().map(|m| &m.symbol).collect::<Vec<_>>());
        println!("    📦 Modules: {:?}", ship.modules.iter().map(|m| &m.symbol).collect::<Vec<_>>());
        println!("    📊 Cargo: {}/{}, Fuel: {}/{}", 
                ship.cargo.units, ship.cargo.capacity,
                ship.fuel.current, ship.fuel.capacity);
    }
    
    Ok(())
}