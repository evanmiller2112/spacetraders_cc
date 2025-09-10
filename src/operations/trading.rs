// Trading operations module
use crate::client::SpaceTradersClient;
use crate::models::*;
use crate::operations::ShipOperations;
use tokio::time::{sleep, Duration};

pub struct TradingOperations<'a> {
    client: &'a SpaceTradersClient,
    ship_ops: ShipOperations<'a>,
}

impl<'a> TradingOperations<'a> {
    pub fn new(client: &'a SpaceTradersClient) -> Self {
        let ship_ops = ShipOperations::new(client);
        Self { client, ship_ops }
    }

    // Basic trading operations
    pub async fn sell_cargo(&self, ship_symbol: &str, trade_symbol: &str, units: i32) -> Result<SellCargoData, Box<dyn std::error::Error>> {
        self.client.sell_cargo(ship_symbol, trade_symbol, units).await
    }

    /// Check if a market accepts a specific trade good for selling
    pub async fn market_accepts_trade(&self, system_symbol: &str, waypoint_symbol: &str, trade_symbol: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let market = self.client.get_market(system_symbol, waypoint_symbol).await?;
        
        // Market accepts a trade good if it's in imports or exchange lists
        let accepts = market.imports.iter().any(|good| good.symbol == trade_symbol) ||
                     market.exchange.iter().any(|good| good.symbol == trade_symbol);
        
        Ok(accepts)
    }

    /// Get all sellable items from cargo, checking market compatibility
    pub async fn get_sellable_items_at_market<'b>(
        &self, 
        ship: &'b Ship, 
        contract_materials: &[String]
    ) -> Result<Vec<&'b CargoItem>, Box<dyn std::error::Error>> {
        let mut sellable_items = Vec::new();
        
        // Extract system from waypoint (e.g., "X1-N5-BA5F" -> "X1-N5")
        let system_symbol = ship.nav.waypoint_symbol.split('-').take(2).collect::<Vec<&str>>().join("-");
        
        // Check market compatibility for each non-contract item
        for item in &ship.cargo.inventory {
            // Skip contract materials
            if contract_materials.contains(&item.symbol) {
                continue;
            }
            
            // Check if current market accepts this item
            match self.market_accepts_trade(&system_symbol, &ship.nav.waypoint_symbol, &item.symbol).await {
                Ok(true) => {
                    sellable_items.push(item);
                }
                Ok(false) => {
                    // Market doesn't accept this item - we'll handle this in the main function
                    continue;
                }
                Err(e) => {
                    println!("      ⚠️ Could not check market compatibility for {}: {}", item.symbol, e);
                    // Continue trying to sell - let the API tell us if it fails
                    sellable_items.push(item);
                }
            }
        }
        
        Ok(sellable_items)
    }

    // Advanced cargo analysis
    pub fn categorize_cargo<'b>(&self, ship: &'b Ship, contract_materials: &[String]) -> (Vec<&'b CargoItem>, Vec<&'b CargoItem>) {
        let mut contract_items = Vec::new();
        let mut sellable_items = Vec::new();

        for item in &ship.cargo.inventory {
            if contract_materials.contains(&item.symbol) {
                contract_items.push(item);
            } else {
                sellable_items.push(item);
            }
        }

        (contract_items, sellable_items)
    }

    pub async fn execute_autonomous_cargo_selling(
        &self,
        _ships: &[Ship],
        contract_materials: &[String],
    ) -> Result<(i64, i32), Box<dyn std::error::Error>> {
        println!("💰 Starting autonomous cargo selling operations...");
        
        // Get current ships with cargo
        let ships_for_selling = self.client.get_ships().await?;
        
        // Find ships with cargo to sell (exclude contract materials)
        let mut total_revenue = 0i64;
        let mut items_sold = 0;
        let mut total_sale_attempts = 0;
        let mut successful_sales = 0;
        let mut failed_sales = 0;
        
        for ship in &ships_for_selling {
            if ship.cargo.units == 0 {
                continue; // Skip empty ships
            }
            
            println!("\n💼 Analyzing cargo on {}...", ship.symbol);
            println!("  📍 Current location: {}", ship.nav.waypoint_symbol);
            println!("  📦 Cargo: {}/{} units", ship.cargo.units, ship.cargo.capacity);
            
            // Separate contract materials from sellable cargo
            let (contract_items, all_sellable_items) = self.categorize_cargo(ship, contract_materials);
            
            for item in &contract_items {
                println!("  🎯 {} x{} - RESERVED for contract", item.symbol, item.units);
            }
            
            // Check market compatibility for each sellable item
            println!("  🏪 Checking market compatibility at {}...", ship.nav.waypoint_symbol);
            let market_compatible_items = self.get_sellable_items_at_market(ship, contract_materials).await;
            
            let sellable_items = match market_compatible_items {
                Ok(items) => {
                    // Show what can and cannot be sold
                    for item in &all_sellable_items {
                        if items.iter().any(|sellable| sellable.symbol == item.symbol) {
                            println!("  💰 {} x{} - SELLABLE at this market", item.symbol, item.units);
                        } else {
                            println!("  ❌ {} x{} - NOT ACCEPTED at this market", item.symbol, item.units);
                        }
                    }
                    items
                },
                Err(e) => {
                    println!("  ❌ Could not check market compatibility: {}", e);
                    // Fallback to original logic if market check fails
                    for item in &all_sellable_items {
                        println!("  ⚠️ {} x{} - TRYING ANYWAY (compatibility check failed)", item.symbol, item.units);
                    }
                    all_sellable_items
                }
            };
            
            if sellable_items.is_empty() && contract_items.is_empty() {
                println!("  ✅ No cargo to analyze");
                continue;
            } else if sellable_items.is_empty() {
                println!("  ✅ No sellable cargo (all reserved for contracts or not accepted here)");
                continue;
            }
            
            // Dock ship for selling (required by SpaceTraders API)
            if ship.nav.status != "DOCKED" {
                println!("  🛸 Docking {} for cargo sales...", ship.symbol);
                match self.ship_ops.dock(&ship.symbol).await {
                    Ok(_) => println!("    ✅ Ship docked successfully"),
                    Err(e) => {
                        println!("    ❌ Could not dock ship: {}", e);
                        continue;
                    }
                }
            } else {
                println!("  ✅ Ship already docked");
            }
            
            // Sell all market-compatible non-contract materials
            println!("  💸 Selling {} different cargo types...", sellable_items.len());
            
            for item in &sellable_items {
                total_sale_attempts += 1;
                println!("    💰 Selling {} x{} {}...", item.units, item.symbol, item.name);
                
                // Retry logic for rate limits
                let mut retry_count = 0;
                let max_retries = 3;
                
                loop {
                    match self.sell_cargo(&ship.symbol, &item.symbol, item.units).await {
                        Ok(sell_data) => {
                            let transaction = &sell_data.transaction;
                            println!("      ✅ SOLD! {} credits ({} per unit)", 
                                    transaction.total_price, transaction.price_per_unit);
                            println!("      📊 Agent credits updated: {}", sell_data.agent.credits);
                            
                            total_revenue += transaction.total_price as i64;
                            items_sold += transaction.units;
                            successful_sales += 1;
                            
                            // Small delay between sales
                            sleep(Duration::from_millis(500)).await;
                            break;
                        }
                        Err(e) => {
                            let error_str = e.to_string();
                            
                            // Check if it's a rate limit error
                            if error_str.contains("429") && retry_count < max_retries {
                                retry_count += 1;
                                println!("      ⏳ Rate limit hit, retry {}/{} in 2 seconds...", retry_count, max_retries);
                                sleep(Duration::from_secs(2)).await;
                                continue;
                            } else {
                                println!("      ❌ Sale failed: {}", e);
                                failed_sales += 1;
                                break;
                            }
                        }
                    }
                }
            }
            
            // Put ship back in orbit after selling
            if ship.nav.status == "DOCKED" {
                match self.ship_ops.orbit(&ship.symbol).await {
                    Ok(_) => println!("  🚀 {} returned to orbit", ship.symbol),
                    Err(e) => println!("  ⚠️ Could not return {} to orbit: {}", ship.symbol, e),
                }
            }
        }
        
        // Comprehensive sales summary
        println!("\n💰 CARGO SALES COMPLETE!");
        println!("  📊 Sales Summary:");
        println!("    🎯 Total sale attempts: {}", total_sale_attempts);
        println!("    ✅ Successful sales: {}", successful_sales);
        println!("    ❌ Failed sales: {}", failed_sales);
        println!("    📦 Total items sold: {}", items_sold);
        println!("    💵 Total revenue: {} credits", total_revenue);
        
        if items_sold > 0 {
            println!("    📈 Average price per unit: {} credits", total_revenue / items_sold as i64);
        }
        
        // Determine overall success
        if successful_sales > 0 && total_revenue > 0 {
            println!("  🎉 Cargo selling completed with revenue generated!");
            println!("  💡 Funds available for fleet expansion and operations");
            if failed_sales > 0 {
                println!("  ⚠️ {} sales failed (market incompatibility or rate limits)", failed_sales);
            }
        } else if total_sale_attempts > 0 {
            println!("  ❌ All sales failed - no revenue generated");
            if failed_sales > 0 {
                println!("  💡 Check market compatibility and rate limiting");
            }
        } else {
            println!("  ℹ️ No sales attempted (all materials reserved for contracts or no compatible markets)");
        }
        
        Ok((total_revenue, items_sold))
    }

    pub fn analyze_market_opportunities<'b>(&self, waypoints: &'b [Waypoint]) -> Vec<&'b Waypoint> {
        waypoints.iter().filter(|waypoint| {
            waypoint.traits.iter().any(|trait_info| {
                trait_info.name.to_lowercase().contains("marketplace")
            })
        }).collect()
    }

    pub fn calculate_profit_potential(&self, ship: &Ship, distance_to_market: i32) -> i32 {
        let cargo_value = ship.cargo.inventory.len() as i32 * 100; // Rough estimate
        let fuel_cost = distance_to_market * 2; // Rough fuel cost
        cargo_value - fuel_cost
    }
}