// Trading operations module
use crate::client::SpaceTradersClient;
use crate::{o_info};
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
                    o_info!("      ⚠️ Could not check market compatibility for {}: {}", item.symbol, e);
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
        o_info!("💰 Starting autonomous cargo selling operations...");
        
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
            
            o_info!("\n💼 Analyzing cargo on {}...", ship.symbol);
            o_info!("  📍 Current location: {}", ship.nav.waypoint_symbol);
            o_info!("  📦 Cargo: {}/{} units", ship.cargo.units, ship.cargo.capacity);
            
            // Separate contract materials from sellable cargo
            let (contract_items, all_sellable_items) = self.categorize_cargo(ship, contract_materials);
            
            for item in &contract_items {
                o_info!("  🎯 {} x{} - RESERVED for contract", item.symbol, item.units);
            }
            
            // Check market compatibility for each sellable item
            o_info!("  🏪 Checking market compatibility at {}...", ship.nav.waypoint_symbol);
            let market_compatible_items = self.get_sellable_items_at_market(ship, contract_materials).await;
            
            let sellable_items = match market_compatible_items {
                Ok(items) => {
                    // Show what can and cannot be sold
                    for item in &all_sellable_items {
                        if items.iter().any(|sellable| sellable.symbol == item.symbol) {
                            o_info!("  💰 {} x{} - SELLABLE at this market", item.symbol, item.units);
                        } else {
                            o_info!("  ❌ {} x{} - NOT ACCEPTED at this market", item.symbol, item.units);
                        }
                    }
                    items
                },
                Err(e) => {
                    o_info!("  ❌ Could not check market compatibility: {}", e);
                    // Fallback to original logic if market check fails
                    for item in &all_sellable_items {
                        o_info!("  ⚠️ {} x{} - TRYING ANYWAY (compatibility check failed)", item.symbol, item.units);
                    }
                    all_sellable_items
                }
            };
            
            if sellable_items.is_empty() && contract_items.is_empty() {
                o_info!("  ✅ No cargo to analyze");
                continue;
            } else if sellable_items.is_empty() {
                o_info!("  ✅ No sellable cargo (all reserved for contracts or not accepted here)");
                continue;
            }
            
            // Dock ship for selling (required by SpaceTraders API)
            if ship.nav.status != "DOCKED" {
                o_info!("  🛸 Docking {} for cargo sales...", ship.symbol);
                match self.ship_ops.dock(&ship.symbol).await {
                    Ok(_) => o_info!("    ✅ Ship docked successfully"),
                    Err(e) => {
                        o_info!("    ❌ Could not dock ship: {}", e);
                        continue;
                    }
                }
            } else {
                o_info!("  ✅ Ship already docked");
            }
            
            // Sell all market-compatible non-contract materials
            o_info!("  💸 Selling {} different cargo types...", sellable_items.len());
            
            for item in &sellable_items {
                total_sale_attempts += 1;
                o_info!("    💰 Selling {} x{} {}...", item.units, item.symbol, item.name);
                
                // Retry logic for rate limits
                let mut retry_count = 0;
                let max_retries = 3;
                
                loop {
                    match self.sell_cargo(&ship.symbol, &item.symbol, item.units).await {
                        Ok(sell_data) => {
                            let transaction = &sell_data.transaction;
                            o_info!("      ✅ SOLD! {} credits ({} per unit)", 
                                    transaction.total_price, transaction.price_per_unit);
                            o_info!("      📊 Agent credits updated: {}", sell_data.agent.credits);
                            
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
                                o_info!("      ⏳ Rate limit hit, retry {}/{} in 2 seconds...", retry_count, max_retries);
                                sleep(Duration::from_secs(2)).await;
                                continue;
                            } else {
                                o_info!("      ❌ Sale failed: {}", e);
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
                    Ok(_) => o_info!("  🚀 {} returned to orbit", ship.symbol),
                    Err(e) => o_info!("  ⚠️ Could not return {} to orbit: {}", ship.symbol, e),
                }
            }
        }
        
        // Comprehensive sales summary
        o_info!("\n💰 CARGO SALES COMPLETE!");
        o_info!("  📊 Sales Summary:");
        o_info!("    🎯 Total sale attempts: {}", total_sale_attempts);
        o_info!("    ✅ Successful sales: {}", successful_sales);
        o_info!("    ❌ Failed sales: {}", failed_sales);
        o_info!("    📦 Total items sold: {}", items_sold);
        o_info!("    💵 Total revenue: {} credits", total_revenue);
        
        if items_sold > 0 {
            o_info!("    📈 Average price per unit: {} credits", total_revenue / items_sold as i64);
        }
        
        // Determine overall success
        if successful_sales > 0 && total_revenue > 0 {
            o_info!("  🎉 Cargo selling completed with revenue generated!");
            o_info!("  💡 Funds available for fleet expansion and operations");
            if failed_sales > 0 {
                o_info!("  ⚠️ {} sales failed (market incompatibility or rate limits)", failed_sales);
            }
        } else if total_sale_attempts > 0 {
            o_info!("  ❌ All sales failed - no revenue generated");
            if failed_sales > 0 {
                o_info!("  💡 Check market compatibility and rate limiting");
            }
        } else {
            o_info!("  ℹ️ No sales attempted (all materials reserved for contracts or no compatible markets)");
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