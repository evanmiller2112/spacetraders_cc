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
        
        for ship in &ships_for_selling {
            if ship.cargo.units == 0 {
                continue; // Skip empty ships
            }
            
            println!("\n💼 Analyzing cargo on {}...", ship.symbol);
            println!("  📍 Current location: {}", ship.nav.waypoint_symbol);
            println!("  📦 Cargo: {}/{} units", ship.cargo.units, ship.cargo.capacity);
            
            // Separate contract materials from sellable cargo
            let (contract_items, sellable_items) = self.categorize_cargo(ship, contract_materials);
            
            for item in &contract_items {
                println!("  🎯 {} x{} - RESERVED for contract", item.symbol, item.units);
            }
            
            for item in &sellable_items {
                println!("  💰 {} x{} - AVAILABLE for sale", item.symbol, item.units);
            }
            
            if sellable_items.is_empty() {
                println!("  ✅ No sellable cargo (all reserved for contracts)");
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
            
            // Sell all non-contract materials
            println!("  💸 Selling {} different cargo types...", sellable_items.len());
            
            for item in &sellable_items {
                println!("    💰 Selling {} x{} {}...", item.units, item.symbol, item.name);
                
                match self.sell_cargo(&ship.symbol, &item.symbol, item.units).await {
                    Ok(sell_data) => {
                        let transaction = &sell_data.transaction;
                        println!("      ✅ SOLD! {} credits ({} per unit)", 
                                transaction.total_price, transaction.price_per_unit);
                        println!("      📊 Agent credits updated: {}", sell_data.agent.credits);
                        
                        total_revenue += transaction.total_price as i64;
                        items_sold += transaction.units;
                        
                        // Small delay between sales
                        sleep(Duration::from_millis(500)).await;
                    }
                    Err(e) => {
                        println!("      ❌ Sale failed: {}", e);
                        // Continue with other items even if one fails
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
        
        // Sales summary
        println!("\n💰 CARGO SALES COMPLETE!");
        println!("  📦 Items sold: {}", items_sold);
        println!("  💵 Total revenue: {} credits", total_revenue);
        println!("  📈 Average price per unit: {} credits", 
                if items_sold > 0 { total_revenue / items_sold as i64 } else { 0 });
        
        if total_revenue > 0 {
            println!("  🎉 Autonomous cargo selling successful!");
            println!("  💡 Funds available for fleet expansion and operations");
        } else {
            println!("  ℹ️ No cargo sold (all materials reserved for contracts)");
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