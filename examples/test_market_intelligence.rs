// Test market intelligence - see what markets actually buy and sell
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    
    println!("🔍 Testing market intelligence...");
    
    // Test the market that's failing to buy our cargo
    let system = "X1-N5";
    let waypoint = "X1-N5-BA5F";
    
    println!("📡 Checking market data for {}", waypoint);
    
    match client.get_market(system, waypoint).await {
        Ok(market) => {
            println!("✅ Market data retrieved for {}", market.symbol);
            
            println!("\n🛒 IMPORTS (What this market BUYS):");
            if market.imports.is_empty() {
                println!("   ❌ No imports - this market doesn't buy anything!");
            } else {
                for import in &market.imports {
                    println!("   • {} - {}", import.symbol, import.name);
                }
            }
            
            println!("\n🏭 EXPORTS (What this market SELLS):");
            if market.exports.is_empty() {
                println!("   ❌ No exports");
            } else {
                for export in &market.exports {
                    println!("   • {} - {}", export.symbol, export.name);
                }
            }
            
            println!("\n🔄 EXCHANGE (What this market TRADES):");
            if market.exchange.is_empty() {
                println!("   ❌ No exchange items");
            } else {
                for exchange in &market.exchange {
                    println!("   • {} - {}", exchange.symbol, exchange.name);
                }
            }
            
            if let Some(trade_goods) = &market.trade_goods {
                println!("\n💰 LIVE PRICES ({} items):", trade_goods.len());
                for good in trade_goods {
                    println!("   • {} - Buy: {} | Sell: {} | Supply: {}", 
                            good.symbol, good.purchase_price, good.sell_price, good.supply);
                }
            }
            
            // Test what our ship is carrying vs what this market buys
            println!("\n🚢 CARGO COMPATIBILITY CHECK:");
            let cargo_items = vec!["SILICON_CRYSTALS", "IRON_ORE", "ICE_WATER", "QUARTZ_SAND", "ALUMINUM_ORE"];
            
            for cargo_item in &cargo_items {
                let can_sell = market.imports.iter().any(|i| i.symbol == *cargo_item) ||
                              market.exchange.iter().any(|e| e.symbol == *cargo_item);
                
                if can_sell {
                    println!("   ✅ {} - CAN SELL HERE", cargo_item);
                } else {
                    println!("   ❌ {} - CANNOT SELL HERE", cargo_item);
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to get market data: {}", e);
        }
    }
    
    // Also check if we can find other markets that might buy these items
    println!("\n🔍 Scanning other markets in system {}...", system);
    
    // Get waypoints with MARKETPLACE trait  
    match client.get_system_waypoints_with_traits(system, "MARKETPLACE").await {
        Ok(waypoints) => {
            println!("📊 Found {} marketplaces in system", waypoints.len());
            
            for waypoint in &waypoints {
                if waypoint.symbol != "X1-N5-BA5F" { // Skip the one we already checked
                    println!("\n🏪 Checking market at {}...", waypoint.symbol);
                    
                    match client.get_market(system, &waypoint.symbol).await {
                        Ok(market) => {
                            let import_symbols: Vec<&str> = market.imports.iter().map(|i| i.symbol.as_str()).collect();
                            let exchange_symbols: Vec<&str> = market.exchange.iter().map(|e| e.symbol.as_str()).collect();
                            
                            println!("   🛒 Imports: {:?}", import_symbols);
                            println!("   🔄 Exchange: {:?}", exchange_symbols);
                            
                            // Check our cargo
                            let cargo_items = vec!["SILICON_CRYSTALS", "IRON_ORE", "ICE_WATER", "QUARTZ_SAND", "ALUMINUM_ORE"];
                            let mut can_sell_items = Vec::new();
                            
                            for cargo_item in &cargo_items {
                                if import_symbols.contains(cargo_item) || exchange_symbols.contains(cargo_item) {
                                    can_sell_items.push(*cargo_item);
                                }
                            }
                            
                            if !can_sell_items.is_empty() {
                                println!("   ✅ CAN SELL: {:?}", can_sell_items);
                            } else {
                                println!("   ❌ Cannot sell our cargo here either");
                            }
                        }
                        Err(e) => {
                            println!("   ⚠️ Failed to get market data: {}", e);
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to get waypoints: {}", e);
        }
    }

    Ok(())
}