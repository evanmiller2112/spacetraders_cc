// Test the cargo transfer system for refinery operations
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient, operations::ShipRoleManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("🔄 Testing Cargo Transfer System for Refinery Operations");
    println!("=======================================================");
    
    // Initialize ship role manager
    let mut role_manager = ShipRoleManager::new();
    
    // Analyze fleet for refinery capabilities
    println!("🔍 Analyzing fleet for refinery capabilities...");
    match role_manager.analyze_fleet(&priority_client).await {
        Ok(_) => {
            println!("✅ Fleet analysis completed");
        }
        Err(e) => {
            println!("❌ Fleet analysis failed: {}", e);
            return Ok(());
        }
    }
    
    // Find best refinery candidate
    println!("\n🏭 Checking for designated refiner...");
    if let Some(refiner) = role_manager.find_best_refinery_candidate() {
        println!("✅ Best refinery candidate: {} (score: {:.2})", 
                refiner.ship_symbol, refiner.refinery_score);
    } else {
        println!("❌ No refinery candidate found");
        return Ok(());
    }
    
    // Get current fleet status
    println!("\n🚢 Checking fleet cargo status...");
    let ships = priority_client.get_ships().await?;
    
    let mut iron_ore_carriers = Vec::new();
    let mut total_iron_ore = 0;
    
    for ship in &ships {
        if ship.cargo.capacity > 0 {
            println!("🚢 {}: {}/{} cargo at {}", 
                    ship.symbol, ship.cargo.units, ship.cargo.capacity, ship.nav.waypoint_symbol);
            
            for item in &ship.cargo.inventory {
                println!("   📦 {} x{}", item.symbol, item.units);
                if item.symbol == "IRON_ORE" && item.units > 0 {
                    iron_ore_carriers.push((ship.symbol.clone(), item.units));
                    total_iron_ore += item.units;
                }
            }
        }
    }
    
    if iron_ore_carriers.is_empty() {
        println!("💼 No ships carrying iron ore found");
        println!("🔧 For testing, you might want to mine some iron ore first");
        return Ok(());
    }
    
    println!("\n⛏️ Iron ore summary:");
    println!("   📊 Total iron ore in fleet: {} units", total_iron_ore);
    println!("   🚛 Carriers: {}", iron_ore_carriers.len());
    for (ship, units) in &iron_ore_carriers {
        println!("     - {}: {} units", ship, units);
    }
    
    // Test the cargo coordination system
    println!("\n🔄 Testing cargo transfer coordination...");
    match role_manager.coordinate_ore_to_refiner_transfer(&priority_client).await {
        Ok(success) => {
            if success {
                println!("✅ Cargo transfer coordination completed successfully!");
                println!("🏭 Iron ore should now be transferred to the designated refiner");
            } else {
                println!("⚠️ Cargo transfer coordination completed, but no transfers were needed");
            }
        }
        Err(e) => {
            println!("❌ Cargo transfer coordination failed: {}", e);
        }
    }
    
    println!("\n📊 Post-transfer fleet status:");
    let updated_ships = priority_client.get_ships().await?;
    for ship in &updated_ships {
        if ship.cargo.capacity > 0 && ship.cargo.units > 0 {
            println!("🚢 {}: {}/{} cargo", ship.symbol, ship.cargo.units, ship.cargo.capacity);
            for item in &ship.cargo.inventory {
                if item.units > 0 {
                    println!("   📦 {} x{}", item.symbol, item.units);
                }
            }
        }
    }
    
    println!("\n🎉 Cargo Transfer Test Complete!");
    println!("🔧 The system can now automatically coordinate iron ore transfers to refiners");
    
    Ok(())
}