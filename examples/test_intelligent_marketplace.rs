// Test intelligent marketplace selection based on actual cargo compatibility
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token};
use spacetraders_cc::operations::FleetCoordinator;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    
    println!("🧠 Testing intelligent marketplace selection...");
    
    // Get our ships
    let ships = client.get_ships().await?;
    println!("📊 Found {} ships", ships.len());
    
    // Find a ship with cargo to test
    let ship_with_cargo = ships.iter()
        .find(|s| !s.cargo.inventory.is_empty())
        .cloned();
    
    if let Some(ship) = ship_with_cargo {
        println!("\n🚢 Testing with ship: {} (cargo: {}/{})", 
                ship.symbol, ship.cargo.units, ship.cargo.capacity);
        
        // Show current cargo
        println!("📦 Current cargo:");
        for item in &ship.cargo.inventory {
            println!("   • {} x{}", item.symbol, item.units);
        }
        
        // Create sellable items list (exclude contract materials)
        let contract_materials = vec!["COPPER_ORE".to_string()]; // Example contract material
        let sellable_items: Vec<_> = ship.cargo.inventory.iter()
            .filter(|item| !contract_materials.contains(&item.symbol))
            .collect();
            
        if !sellable_items.is_empty() {
            println!("\n💰 Sellable items (non-contract):");
            for item in &sellable_items {
                println!("   • {} x{}", item.symbol, item.units);
            }
            
            // Test our intelligent marketplace finder
            let mut coordinator = FleetCoordinator::new(client.clone());
            coordinator.initialize_fleet().await?;
            
            println!("\n🔍 Finding best marketplace for cargo...");
            match coordinator.find_best_marketplace_for_cargo(&ship, &sellable_items).await {
                Ok(best_market) => {
                    println!("✅ Best marketplace: {}", best_market);
                    
                    // Compare with the old logic that was failing
                    println!("\n📊 COMPARISON:");
                    println!("   ❌ Old logic: Would go to X1-N5-BA5F (fuel-only market)");
                    println!("   ✅ New logic: Going to {} (compatible market)", best_market);
                    
                    // Verify the market can actually buy our items
                    println!("\n🔍 Verifying market compatibility...");
                    match client.get_market(&ship.nav.system_symbol, &best_market).await {
                        Ok(market) => {
                            println!("   🛒 Market imports: {:?}", 
                                    market.imports.iter().map(|i| &i.symbol).collect::<Vec<_>>());
                            println!("   🔄 Market exchange: {:?}", 
                                    market.exchange.iter().map(|e| &e.symbol).collect::<Vec<_>>());
                            
                            let mut compatible_count = 0;
                            for item in &sellable_items {
                                let can_buy = market.imports.iter().any(|i| i.symbol == item.symbol) ||
                                             market.exchange.iter().any(|e| e.symbol == item.symbol);
                                if can_buy {
                                    compatible_count += 1;
                                    println!("   ✅ {} - CAN SELL", item.symbol);
                                } else {
                                    println!("   ❌ {} - cannot sell", item.symbol);
                                }
                            }
                            
                            println!("\n🎯 RESULT: {}/{} items can be sold at {}", 
                                    compatible_count, sellable_items.len(), best_market);
                        }
                        Err(e) => {
                            println!("   ⚠️ Failed to verify market: {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("❌ No compatible marketplace found: {}", e);
                    println!("💡 This means we'd fallback to jettisoning cargo");
                }
            }
        } else {
            println!("\n⚠️ No sellable cargo found (all items are contract materials)");
        }
    } else {
        println!("⚠️ No ships with cargo found for testing");
    }

    Ok(())
}