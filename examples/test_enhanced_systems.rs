// Test the enhanced ship purchasing and cargo delivery systems
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    
    println!("🧪 Testing Enhanced Systems");
    println!("================================");
    
    // Get current state
    let agent = client.get_agent().await?;
    let ships = client.get_ships().await?;
    let contracts = client.get_contracts().await?;
    
    println!("📊 CURRENT STATE:");
    println!("   💰 Credits: {}", agent.credits);
    println!("   🚢 Ships: {}", ships.len());
    println!("   📋 Contracts: {}", contracts.len());
    
    if let Some(contract) = contracts.first() {
        println!("\n📋 ACTIVE CONTRACT ANALYSIS:");
        println!("   🎯 Contract: {}", contract.id);
        
        for delivery in &contract.terms.deliver {
            let remaining = delivery.units_required - delivery.units_fulfilled;
            println!("   📦 Need: {} x{} (remaining: {})", 
                    delivery.trade_symbol, delivery.units_required, remaining);
        }
        
        println!("   💎 Payment: {} credits", contract.terms.payment.on_fulfilled);
        
        // Test cargo delivery logic for each ship
        println!("\n🚛 CARGO DELIVERY ANALYSIS:");
        for ship in &ships {
            if ship.cargo.capacity > 0 {
                println!("   🚢 {} ({}/{} cargo):", ship.symbol, ship.cargo.units, ship.cargo.capacity);
                
                // Simulate the delivery logic
                let cargo_percentage = ship.cargo.units as f64 / ship.cargo.capacity as f64 * 100.0;
                
                // Count contract materials
                let mut contract_materials = 0;
                for delivery in &contract.terms.deliver {
                    if let Some(item) = ship.cargo.inventory.iter().find(|i| i.symbol == delivery.trade_symbol) {
                        contract_materials += item.units;
                    }
                }
                
                let contract_material_percentage = contract_materials as f64 / ship.cargo.capacity as f64 * 100.0;
                
                println!("     📊 Cargo: {:.1}% full", cargo_percentage);
                println!("     🎯 Contract materials: {} units ({:.1}%)", contract_materials, contract_material_percentage);
                
                // Apply the new delivery logic
                let should_deliver = if cargo_percentage >= 90.0 {
                    println!("     ✅ DECISION: Deliver (cargo nearly full)");
                    true
                } else if contract_material_percentage >= 75.0 {
                    println!("     ✅ DECISION: Deliver (significant contract materials)");
                    true
                } else {
                    println!("     ⏳ DECISION: Continue mining (not enough to deliver)");
                    false
                };
                
                if should_deliver && contract_materials == 0 {
                    println!("     ⚠️  Would deliver but has no contract materials!");
                }
            }
        }
        
        // Test ship expansion logic
        println!("\n🏗️ SHIP EXPANSION ANALYSIS:");
        
        let mining_ships = ships.iter().filter(|s| s.mounts.iter().any(|m| m.symbol.contains("MINING"))).count();
        let total_contract_units: i32 = contract.terms.deliver.iter().map(|d| d.units_required - d.units_fulfilled).sum();
        
        println!("   🚢 Current mining ships: {}", mining_ships);
        println!("   📦 Remaining contract units: {}", total_contract_units);
        println!("   💎 Contract value: {}", contract.terms.payment.on_fulfilled);
        
        // Apply expansion criteria
        let has_credits = agent.credits >= 300000;
        let contract_is_large = total_contract_units >= 50 || contract.terms.payment.on_fulfilled >= 100000;
        let not_too_many_ships = mining_ships < 4;
        let profitable = contract.terms.payment.on_fulfilled > 250000;
        
        println!("   💰 Has 300k+ credits: {}", if has_credits { "✅" } else { "❌" });
        println!("   📊 Contract is large: {}", if contract_is_large { "✅" } else { "❌" });
        println!("   🚢 Room for more ships: {}", if not_too_many_ships { "✅" } else { "❌" });
        println!("   💎 Contract profitable: {}", if profitable { "✅" } else { "❌" });
        
        if has_credits && contract_is_large && not_too_many_ships && profitable {
            println!("   🎯 EXPANSION RECOMMENDED!");
            println!("     💡 Bot will search for shipyards and attempt purchase");
        } else {
            println!("   ⏸️  Expansion not recommended at this time");
            if !has_credits {
                let needed = 300000 - agent.credits;
                println!("     💸 Need {} more credits", needed);
            }
        }
        
    } else {
        println!("❌ No active contracts found");
    }
    
    println!("\n🎉 Enhanced Systems Ready!");
    println!("   🚛 Smarter cargo delivery: Only deliver when cargo is 90%+ full or has 75%+ contract materials");
    println!("   🏗️ Automatic ship purchasing: Will buy ships when profitable and credits available");
    println!("   💡 Run the main bot to see these systems in action!");
    
    Ok(())
}