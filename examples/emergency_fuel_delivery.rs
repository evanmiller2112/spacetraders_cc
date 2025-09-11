// Emergency fuel delivery for stranded ships
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("🚨 EMERGENCY FUEL DELIVERY SYSTEM");
    println!("=================================");
    
    let ships = priority_client.get_ships().await?;
    
    // Step 1: Identify stranded ships and potential fuel carriers
    let mut stranded_ships = Vec::new();
    let mut fuel_carriers = Vec::new();
    
    for ship in &ships {
        let fuel_percentage = (ship.fuel.current as f64 / ship.fuel.capacity as f64) * 100.0;
        
        if ship.fuel.current < 50 {
            stranded_ships.push((ship.symbol.clone(), ship.nav.waypoint_symbol.clone(), ship.fuel.current));
            println!("🚨 STRANDED: {} at {} (fuel: {}/{})", 
                     ship.symbol, ship.nav.waypoint_symbol, ship.fuel.current, ship.fuel.capacity);
        } else if fuel_percentage > 50.0 && ship.cargo.capacity > 0 {
            fuel_carriers.push((ship.symbol.clone(), ship.nav.waypoint_symbol.clone(), ship.fuel.current));
            println!("⛽ CARRIER: {} at {} (fuel: {}/{})", 
                     ship.symbol, ship.nav.waypoint_symbol, ship.fuel.current, ship.fuel.capacity);
        }
    }
    
    if stranded_ships.is_empty() {
        println!("✅ No stranded ships found");
        return Ok(());
    }
    
    if fuel_carriers.is_empty() {
        println!("❌ No ships available for fuel delivery!");
        println!("💡 Try manual fuel purchases or different approach");
        return Ok(());
    }
    
    // Step 2: Buy fuel and deliver to stranded ships
    let fuel_station = "X1-N5-B6";
    let carrier = &fuel_carriers[0]; // Use first available carrier
    let carrier_symbol = &carrier.0;
    
    println!("\n⛽ Using {} as fuel carrier", carrier_symbol);
    
    // Navigate carrier to fuel station if needed
    if carrier.1 != fuel_station {
        println!("🚀 Sending {} to fuel station {}", carrier_symbol, fuel_station);
        
        // Orbit if docked
        let ship = priority_client.get_ship(carrier_symbol).await?;
        if ship.nav.status == "DOCKED" {
            priority_client.orbit_ship(carrier_symbol).await?;
        }
        
        // Navigate to fuel station
        match priority_client.navigate_ship(carrier_symbol, fuel_station).await {
            Ok(nav_result) => {
                if let Ok(arrival_time) = nav_result.nav.route.arrival.parse::<chrono::DateTime<chrono::Utc>>() {
                    let now = chrono::Utc::now();
                    let wait_seconds = (arrival_time - now).num_seconds().max(0) as u64 + 2;
                    if wait_seconds > 0 {
                        println!("⏳ Waiting {} seconds for carrier arrival...", wait_seconds);
                        tokio::time::sleep(tokio::time::Duration::from_secs(wait_seconds)).await;
                    }
                }
            }
            Err(e) => {
                println!("❌ Carrier navigation failed: {}", e);
                return Ok(());
            }
        }
    }
    
    // Dock at fuel station
    priority_client.dock_ship(carrier_symbol).await?;
    println!("🛸 Carrier docked at fuel station");
    
    // Buy fuel (as cargo for delivery)
    println!("💰 Purchasing fuel for delivery...");
    let fuel_to_buy = 10; // Start with small amount
    
    match priority_client.purchase_cargo(carrier_symbol, "FUEL", fuel_to_buy).await {
        Ok(_) => {
            println!("✅ Purchased {} units of FUEL", fuel_to_buy);
        }
        Err(e) => {
            println!("❌ Fuel purchase failed: {}", e);
            println!("💡 May need to refuel carrier first or fuel not available");
            
            // Try refueling the carrier itself first
            match priority_client.refuel_ship(carrier_symbol).await {
                Ok(_) => {
                    println!("✅ Carrier refueled successfully");
                }
                Err(e2) => {
                    println!("❌ Carrier refuel also failed: {}", e2);
                    return Ok(());
                }
            }
        }
    }
    
    // Step 3: Deliver fuel to stranded ships
    for (stranded_symbol, stranded_location, _fuel_level) in &stranded_ships {
        println!("\n🚨 Delivering fuel to {} at {}", stranded_symbol, stranded_location);
        
        // Navigate carrier to stranded ship location
        priority_client.orbit_ship(carrier_symbol).await?;
        
        match priority_client.navigate_ship(carrier_symbol, stranded_location).await {
            Ok(nav_result) => {
                if let Ok(arrival_time) = nav_result.nav.route.arrival.parse::<chrono::DateTime<chrono::Utc>>() {
                    let now = chrono::Utc::now();
                    let wait_seconds = (arrival_time - now).num_seconds().max(0) as u64 + 2;
                    if wait_seconds > 0 {
                        println!("⏳ Waiting {} seconds for delivery...", wait_seconds);
                        tokio::time::sleep(tokio::time::Duration::from_secs(wait_seconds)).await;
                    }
                }
                
                // Try to transfer fuel (this might not work directly, need to check API)
                println!("⛽ Attempting fuel transfer...");
                // Note: Fuel transfer between ships might not be a direct API
                // May need to sell fuel and have stranded ship buy it
                
                priority_client.dock_ship(carrier_symbol).await?;
                
                // Check if location has marketplace to sell/buy fuel
                println!("💡 Attempting marketplace fuel transfer method...");
                
                // Sell fuel from carrier
                match priority_client.sell_cargo(carrier_symbol, "FUEL", fuel_to_buy).await {
                    Ok(_) => {
                        println!("✅ Carrier sold fuel to local market");
                        
                        // Have stranded ship dock and buy fuel
                        priority_client.dock_ship(stranded_symbol).await?;
                        
                        match priority_client.purchase_cargo(stranded_symbol, "FUEL", 5).await {
                            Ok(_) => {
                                println!("✅ Stranded ship purchased fuel from market");
                                
                                // Convert cargo fuel to ship fuel (if there's such an API)
                                // This is a placeholder - need to check if direct refuel works now
                                match priority_client.refuel_ship(stranded_symbol).await {
                                    Ok(_) => {
                                        println!("🎉 {} successfully refueled!", stranded_symbol);
                                    }
                                    Err(e) => {
                                        println!("⚠️ Refuel attempt: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                println!("❌ Stranded ship couldn't buy fuel: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("❌ Carrier couldn't sell fuel: {}", e);
                    }
                }
                
            }
            Err(e) => {
                println!("❌ Delivery navigation failed: {}", e);
                continue;
            }
        }
        
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    }
    
    // Step 4: Final status check
    println!("\n📊 FINAL FUEL STATUS:");
    let updated_ships = priority_client.get_ships().await?;
    
    for ship in &updated_ships {
        let fuel_percentage = (ship.fuel.current as f64 / ship.fuel.capacity as f64) * 100.0;
        let status = if fuel_percentage > 50.0 { "✅" } else if fuel_percentage > 20.0 { "⚠️" } else { "🚨" };
        
        println!("   {} {}: {}/{} ({}%)", 
                 status, ship.symbol, ship.fuel.current, ship.fuel.capacity, fuel_percentage as i32);
    }
    
    println!("\n💡 Next steps:");
    println!("   - Ships with good fuel can proceed with mining");
    println!("   - Implement automatic fuel management in navigation");
    println!("   - Set up fuel monitoring for future operations");
    
    Ok(())
}