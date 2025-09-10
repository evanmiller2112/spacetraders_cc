// Analyze marketplace options for buying ELECTRONICS
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    
    println!("🔍 Analyzing ELECTRONICS contract and marketplace options...");
    
    // First, get contract details
    println!("\n📋 Contract Analysis:");
    let contracts = client.get_contracts().await?;
    let electronics_contract = contracts.iter()
        .find(|c| !c.fulfilled && c.terms.deliver.iter().any(|d| d.trade_symbol == "ELECTRONICS"));
    
    if let Some(contract) = electronics_contract {
        println!("✅ Found ELECTRONICS contract: {}", contract.id);
        for delivery in &contract.terms.deliver {
            if delivery.trade_symbol == "ELECTRONICS" {
                println!("  📦 Need: {} x{} ELECTRONICS", delivery.units_required, delivery.units_required);
                println!("  📍 Deliver to: {}", delivery.destination_symbol);
                println!("  📊 Progress: {}/{} ({:.1}%)", 
                        delivery.units_fulfilled, 
                        delivery.units_required,
                        (delivery.units_fulfilled as f64 / delivery.units_required as f64) * 100.0);
            }
        }
        println!("  💰 Payment: {} + {} = {} total credits", 
                contract.terms.payment.on_accepted,
                contract.terms.payment.on_fulfilled,
                contract.terms.payment.on_accepted + contract.terms.payment.on_fulfilled);
    } else {
        println!("❌ No active ELECTRONICS contract found");
        return Ok(());
    }
    
    // Get all waypoints in the system to find marketplaces
    println!("\n🏪 Marketplace Analysis:");
    let waypoints = client.get_system_waypoints("X1-N5", None).await?;
    
    let marketplaces: Vec<_> = waypoints.iter()
        .filter(|w| w.traits.iter().any(|t| t.symbol == "MARKETPLACE"))
        .collect();
    
    println!("Found {} marketplaces in X1-N5:", marketplaces.len());
    
    for marketplace in &marketplaces {
        println!("\n🏪 {} ({})", marketplace.symbol, marketplace.waypoint_type);
        println!("  📍 Coordinates: ({}, {})", marketplace.x, marketplace.y);
        println!("  🏛️ Faction: {}", 
                marketplace.faction.as_ref().map(|f| f.symbol.as_str()).unwrap_or("None"));
        
        // Check if we can get market data
        match client.get_market("X1-N5", &marketplace.symbol).await {
            Ok(market) => {
                if let Some(trade_goods) = &market.trade_goods {
                    let electronics = trade_goods.iter()
                        .find(|good| good.symbol == "ELECTRONICS");
                    
                    if let Some(electronics) = electronics {
                        println!("  ✅ ELECTRONICS AVAILABLE!");
                        println!("    💰 Buy Price: {} credits/unit", electronics.purchase_price);
                        println!("    📦 Volume: {} units available", electronics.trade_volume);
                        println!("    📊 Supply: {}", electronics.supply);
                        
                        // Calculate cost for contract
                        if let Some(contract) = electronics_contract {
                            let needed = contract.terms.deliver.iter()
                                .find(|d| d.trade_symbol == "ELECTRONICS")
                                .map(|d| d.units_required - d.units_fulfilled)
                                .unwrap_or(0);
                            let total_cost = needed * electronics.purchase_price;
                            println!("    💸 Cost for {} units: {} credits", needed, total_cost);
                        }
                    } else {
                        println!("  ❌ ELECTRONICS not available here");
                    }
                    
                    // Show other available goods
                    let goods_list: Vec<String> = trade_goods.iter()
                        .map(|g| g.symbol.clone())
                        .collect();
                    println!("  📋 Other goods: {}", goods_list.join(", "));
                } else {
                    println!("  ⚠️ Market data not available (need to dock to see prices)");
                }
            }
            Err(e) => {
                println!("  ❌ Could not access market: {}", e);
            }
        }
    }
    
    // Show our current fleet status
    println!("\n🚢 Fleet Status for Trading:");
    let ships = client.get_ships().await?;
    
    for ship in &ships {
        println!("  🚢 {} ({}):", ship.symbol, ship.registration.role);
        println!("    📍 Location: {}", ship.nav.waypoint_symbol);
        println!("    📦 Cargo: {}/{} units", ship.cargo.units, ship.cargo.capacity);
        println!("    💰 Can carry: {} ELECTRONICS units", ship.cargo.capacity - ship.cargo.units);
        
        // Check if this ship is good for trading
        if ship.registration.role == "HAULER" || ship.registration.role == "COMMAND" {
            println!("    ✅ Good for marketplace trading");
        } else if ship.registration.role == "SATELLITE" {
            println!("    🔍 Good for market reconnaissance"); 
        } else {
            println!("    ⛏️ Better for mining operations");
        }
    }
    
    println!("\n💡 Recommended Strategy:");
    println!("  1. Use SATELLITE/PROBE ships to scout all marketplaces for ELECTRONICS");
    println!("  2. Use COMMAND/HAULER ships with high cargo capacity for purchasing");
    println!("  3. Continue mining with excavator ships to fund purchases");
    println!("  4. Calculate most cost-effective marketplace for bulk purchase");
    
    Ok(())
}