// Use SATELLITE ship to scout marketplaces for ELECTRONICS
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    
    println!("🛰️ ELECTRONICS Market Reconnaissance Mission");
    println!("Using SATELLITE ship to scout all marketplaces...\n");
    
    // Find our satellite ship
    let ships = client.get_ships().await?;
    let satellite = ships.iter().find(|s| s.registration.role == "SATELLITE");
    
    let satellite = match satellite {
        Some(ship) => {
            println!("✅ Found SATELLITE: {} at {}", ship.symbol, ship.nav.waypoint_symbol);
            ship
        }
        None => {
            println!("❌ No SATELLITE ship found - using COMMAND ship instead");
            ships.iter().find(|s| s.registration.role == "COMMAND").unwrap()
        }
    };
    
    // Get all marketplaces
    let waypoints = client.get_system_waypoints("X1-N5", None).await?;
    let marketplaces: Vec<_> = waypoints.iter()
        .filter(|w| w.traits.iter().any(|t| t.symbol == "MARKETPLACE"))
        .collect();
    
    println!("📊 Scouting {} marketplaces for ELECTRONICS availability...\n", marketplaces.len());
    
    let mut electronics_sources = Vec::new();
    
    for marketplace in &marketplaces {
        println!("🏪 Scouting {} ({})...", marketplace.symbol, marketplace.waypoint_type);
        
        // Navigate to marketplace if not there
        if satellite.nav.waypoint_symbol != marketplace.symbol {
            println!("  🚀 Navigating {} to {}...", satellite.symbol, marketplace.symbol);
            
            // Ensure in orbit before navigation
            if satellite.nav.status == "DOCKED" {
                match client.orbit_ship(&satellite.symbol).await {
                    Ok(_) => println!("    ✅ Ship in orbit"),
                    Err(e) => {
                        if !e.to_string().contains("already in orbit") {
                            println!("    ⚠️ Could not orbit: {}", e);
                        }
                    }
                }
            }
            
            match client.navigate_ship(&satellite.symbol, &marketplace.symbol).await {
                Ok(_) => {
                    println!("    ✅ Navigation started to {}", marketplace.symbol);
                    
                    // Wait for arrival (simplified - in real implementation we'd check arrival time)
                    println!("    ⏳ Waiting for arrival...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                    
                    // Update ship status
                    let updated_ship = client.get_ship(&satellite.symbol).await?;
                    if updated_ship.nav.status == "IN_TRANSIT" {
                        println!("    ⏳ Still in transit, checking anyway...");
                    }
                }
                Err(e) => {
                    println!("    ❌ Navigation failed: {}", e);
                    continue;
                }
            }
        } else {
            println!("  ✅ Already at {}", marketplace.symbol);
        }
        
        // Dock and check market
        println!("  🛸 Docking for market reconnaissance...");
        match client.dock_ship(&satellite.symbol).await {
            Ok(_) => println!("    ✅ Docked successfully"),
            Err(e) => {
                if e.to_string().contains("already docked") {
                    println!("    ✅ Already docked");
                } else {
                    println!("    ❌ Docking failed: {}", e);
                    continue;
                }
            }
        }
        
        // Check market for ELECTRONICS
        match client.get_market("X1-N5", &marketplace.symbol).await {
            Ok(market) => {
                if let Some(trade_goods) = &market.trade_goods {
                    let electronics = trade_goods.iter()
                        .find(|good| good.symbol == "ELECTRONICS");
                    
                    if let Some(electronics) = electronics {
                        println!("    ✅ ELECTRONICS FOUND!");
                        println!("      💰 Price: {} credits/unit", electronics.purchase_price);
                        println!("      📦 Available: {} units", electronics.trade_volume);
                        println!("      📊 Supply level: {}", electronics.supply);
                        
                        electronics_sources.push((
                            marketplace.symbol.clone(),
                            electronics.purchase_price,
                            electronics.trade_volume,
                            electronics.supply.clone()
                        ));
                    } else {
                        println!("    ❌ No ELECTRONICS available");
                    }
                    
                    println!("    📋 Available goods: {}", 
                            trade_goods.iter()
                                .map(|g| g.symbol.clone())
                                .collect::<Vec<_>>()
                                .join(", "));
                } else {
                    println!("    ⚠️ No trade goods data available");
                }
            }
            Err(e) => {
                println!("    ❌ Market access failed: {}", e);
            }
        }
        
        println!(); // Blank line for readability
        
        // Small delay to respect rate limits
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
    
    // Summary of findings
    println!("📊 RECONNAISSANCE SUMMARY:");
    if electronics_sources.is_empty() {
        println!("❌ No ELECTRONICS found in any marketplace in X1-N5 system");
        println!("💡 May need to explore other systems or check production facilities");
    } else {
        println!("✅ Found {} sources of ELECTRONICS:", electronics_sources.len());
        
        // Sort by price (cheapest first)
        electronics_sources.sort_by_key(|(_location, price, _volume, _supply)| *price);
        
        for (i, (location, price, volume, supply)) in electronics_sources.iter().enumerate() {
            println!("  {}. {} - {} credits/unit ({} available, supply: {})", 
                    i + 1, location, price, volume, supply);
            
            // Calculate total cost for contract
            let needed = 21; // Contract requirement
            let available_for_contract = (*volume).min(needed);
            let total_cost = available_for_contract * price;
            println!("     💸 Cost for {} units: {} credits", available_for_contract, total_cost);
        }
        
        // Recommendation
        if let Some((best_location, best_price, best_volume, _)) = electronics_sources.first() {
            let total_needed = 21;
            let total_cost = total_needed * best_price;
            println!("\n💡 RECOMMENDATION:");
            println!("  🎯 Best option: {} at {} credits/unit", best_location, best_price);
            println!("  💰 Total cost for contract: {} credits", total_cost);
            println!("  📦 Available volume: {} units (need {})", best_volume, total_needed);
            
            if *best_volume >= total_needed {
                println!("  ✅ Sufficient supply available for full contract");
            } else {
                println!("  ⚠️ Insufficient supply - may need multiple sources");
            }
        }
    }
    
    Ok(())
}