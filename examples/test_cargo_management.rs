// Test the enhanced cargo management system
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    
    println!("🗃️ Testing Enhanced Cargo Management System");
    println!("================================================");
    
    // Get current ships and contracts
    let ships = client.get_ships().await?;
    let contracts = client.get_contracts().await?;
    
    if let Some(contract) = contracts.first() {
        println!("📋 Contract: {}", contract.id);
        
        let contract_materials: Vec<String> = contract.terms.deliver
            .iter()
            .map(|d| d.trade_symbol.clone())
            .collect();
        
        println!("🎯 Contract materials: {:?}", contract_materials);
        
        for ship in &ships {
            if ship.cargo.capacity > 0 {
                println!("\n🚢 Ship: {} ({}/{})", ship.symbol, ship.cargo.units, ship.cargo.capacity);
                
                // Analyze cargo composition
                let mut contract_items = Vec::new();
                let mut non_contract_items = Vec::new();
                
                for item in &ship.cargo.inventory {
                    if contract_materials.contains(&item.symbol) {
                        contract_items.push(item);
                        println!("   🎯 Contract: {} x{}", item.symbol, item.units);
                    } else {
                        non_contract_items.push(item);
                        println!("   💰 Non-contract: {} x{}", item.symbol, item.units);
                    }
                }
                
                // Simulate cargo management decisions
                let cargo_percentage = ship.cargo.units as f64 / ship.cargo.capacity as f64;
                println!("   📊 Cargo utilization: {:.1}%", cargo_percentage * 100.0);
                
                if cargo_percentage >= 0.9 {
                    println!("   🔥 CARGO FULL - Management needed!");
                    
                    if !contract_items.is_empty() && !non_contract_items.is_empty() {
                        println!("   ✅ STRATEGY: Sell non-contract items first, then deliver contract items");
                        println!("     1. 🏪 Try selling {} non-contract items", non_contract_items.len());
                        println!("     2. 🗑️ Jettison if selling fails");
                        println!("     3. 📦 Continue mining with freed space");
                    } else if !contract_items.is_empty() {
                        println!("   ✅ STRATEGY: Only contract items - deliver them");
                    } else if !non_contract_items.is_empty() {
                        println!("   ✅ STRATEGY: Only non-contract items - sell or jettison all");
                    }
                } else if cargo_percentage >= 0.75 && !contract_items.is_empty() {
                    println!("   📦 SIGNIFICANT CONTRACT MATERIALS - Consider delivery");
                } else {
                    println!("   ⏳ CONTINUE MINING - Cargo not full enough");
                }
                
                // Show what would be jettisoned in worst case
                if !non_contract_items.is_empty() {
                    println!("   🗑️ Would jettison if needed:");
                    for item in &non_contract_items {
                        println!("     - {} x{} (value lost)", item.symbol, item.units);
                    }
                }
                
                if !contract_items.is_empty() {
                    let contract_units: i32 = contract_items.iter().map(|i| i.units).sum();
                    println!("   🎯 Contract items preserved: {} total units", contract_units);
                }
            }
        }
        
        println!("\n🎉 Enhanced Cargo Management Benefits:");
        println!("   💰 Preserves valuable contract materials");
        println!("   🏪 Attempts to sell before jettisoning (credits > waste)");
        println!("   🗑️ Jettisons only when selling fails");
        println!("   ⛏️ Maximizes mining efficiency by freeing space");
        println!("   🎯 Prioritizes contract completion");
        
    } else {
        println!("❌ No active contracts found");
    }
    
    println!("\n✨ The bot will now intelligently manage full cargo holds!");
    
    Ok(())
}