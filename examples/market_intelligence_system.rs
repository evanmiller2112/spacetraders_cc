// 🔍💰 MARKET INTELLIGENCE SYSTEM - SCOUT BEFORE YOU SELL! 💰🔍
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct MarketData {
    pub waypoint_symbol: String,
    pub exports: Vec<String>,
    pub imports: Vec<String>,
    pub exchange: Vec<String>,
    pub trade_goods: HashMap<String, TradeInfo>,
}

#[derive(Debug, Clone)]
pub struct TradeInfo {
    pub symbol: String,
    pub supply: String, // SCARCE, LIMITED, MODERATE, HIGH, ABUNDANT
    pub purchase_price: i32,
    pub sell_price: i32,
    pub trade_volume: i32,
}

#[derive(Debug, Clone)]
pub struct TradingOpportunity {
    pub item: String,
    pub buy_at: String,
    pub sell_at: String,
    pub buy_price: i32,
    pub sell_price: i32,
    pub profit_per_unit: i32,
    pub profit_margin: f64,
}

pub struct MarketIntelligenceSystem {
    client: PriorityApiClient,
    market_cache: HashMap<String, MarketData>,
    cargo_inventory: HashMap<String, i32>,
}

impl MarketIntelligenceSystem {
    pub fn new(client: PriorityApiClient) -> Self {
        Self {
            client,
            market_cache: HashMap::new(),
            cargo_inventory: HashMap::new(),
        }
    }

    // Scout market without traveling there
    pub async fn scout_market(&mut self, system: &str, waypoint: &str) -> Result<MarketData, Box<dyn std::error::Error>> {
        println!("🔍 SCOUTING MARKET: {}", waypoint);
        
        let market = self.client.get_market_with_priority(system, waypoint, spacetraders_cc::client::priority_client::ApiPriority::Background).await?;
        
        let mut trade_goods = HashMap::new();
        
        // Process trade goods data
        if let Some(ref goods) = market.trade_goods {
            for good in goods {
                trade_goods.insert(good.symbol.clone(), TradeInfo {
                    symbol: good.symbol.clone(),
                    supply: good.supply.clone(),
                    purchase_price: good.purchase_price,
                    sell_price: good.sell_price,
                    trade_volume: good.trade_volume,
                });
            }
        }
        
        let market_data = MarketData {
            waypoint_symbol: waypoint.to_string(),
            exports: market.exports.iter().map(|e| e.symbol.clone()).collect(),
            imports: market.imports.iter().map(|i| i.symbol.clone()).collect(),
            exchange: market.exchange.iter().map(|x| x.symbol.clone()).collect(),
            trade_goods,
        };
        
        // Cache the data
        self.market_cache.insert(waypoint.to_string(), market_data.clone());
        
        println!("   ✅ Market data cached for {}", waypoint);
        println!("      Exports: {:?}", market_data.exports);
        println!("      Imports: {:?}", market_data.imports);
        println!("      Trade goods: {} items", market_data.trade_goods.len());
        
        Ok(market_data)
    }

    // Update current cargo inventory
    pub async fn update_cargo_inventory(&mut self, ship_symbol: &str) -> Result<(), Box<dyn std::error::Error>> {
        let ship = self.client.get_ship(ship_symbol).await?;
        
        self.cargo_inventory.clear();
        for item in &ship.cargo.inventory {
            self.cargo_inventory.insert(item.symbol.clone(), item.units);
        }
        
        println!("📦 CARGO INVENTORY UPDATED:");
        for (item, units) in &self.cargo_inventory {
            println!("   {} x{}", item, units);
        }
        
        Ok(())
    }

    // Find best selling opportunities for current cargo
    pub fn find_selling_opportunities(&self, cargo_item: &str, _units: i32) -> Vec<(&str, &TradeInfo)> {
        let mut opportunities = Vec::new();
        
        for (waypoint, market_data) in &self.market_cache {
            // Check if this market imports this item or has it in trade goods
            if market_data.imports.contains(&cargo_item.to_string()) || 
               market_data.trade_goods.contains_key(cargo_item) {
                
                if let Some(trade_info) = market_data.trade_goods.get(cargo_item) {
                    opportunities.push((waypoint.as_str(), trade_info));
                }
            }
        }
        
        // Sort by sell price (highest first)
        opportunities.sort_by(|a, b| b.1.sell_price.cmp(&a.1.sell_price));
        
        opportunities
    }

    // Find best buying opportunities for items we want
    pub fn find_buying_opportunities(&self, target_item: &str) -> Vec<(&str, &TradeInfo)> {
        let mut opportunities = Vec::new();
        
        for (waypoint, market_data) in &self.market_cache {
            // Check if this market exports this item or has it in trade goods
            if market_data.exports.contains(&target_item.to_string()) || 
               market_data.trade_goods.contains_key(target_item) {
                
                if let Some(trade_info) = market_data.trade_goods.get(target_item) {
                    opportunities.push((waypoint.as_str(), trade_info));
                }
            }
        }
        
        // Sort by purchase price (lowest first)
        opportunities.sort_by(|a, b| a.1.purchase_price.cmp(&b.1.purchase_price));
        
        opportunities
    }

    // Find profitable trading routes
    pub fn find_trading_routes(&self) -> Vec<TradingOpportunity> {
        let mut opportunities = Vec::new();
        
        // For each item in any market
        let mut all_items: Vec<String> = Vec::new();
        for market_data in self.market_cache.values() {
            for item in market_data.trade_goods.keys() {
                if !all_items.contains(item) {
                    all_items.push(item.clone());
                }
            }
        }
        
        for item in &all_items {
            let buy_opportunities = self.find_buying_opportunities(item);
            let sell_opportunities = self.find_selling_opportunities(item, 1);
            
            if let (Some(buy), Some(sell)) = (buy_opportunities.first(), sell_opportunities.first()) {
                let profit_per_unit = sell.1.sell_price - buy.1.purchase_price;
                let profit_margin = if buy.1.purchase_price > 0 {
                    (profit_per_unit as f64 / buy.1.purchase_price as f64) * 100.0
                } else {
                    0.0
                };
                
                if profit_per_unit > 0 {
                    opportunities.push(TradingOpportunity {
                        item: item.clone(),
                        buy_at: buy.0.to_string(),
                        sell_at: sell.0.to_string(),
                        buy_price: buy.1.purchase_price,
                        sell_price: sell.1.sell_price,
                        profit_per_unit,
                        profit_margin,
                    });
                }
            }
        }
        
        // Sort by profit per unit (highest first)
        opportunities.sort_by(|a, b| b.profit_per_unit.cmp(&a.profit_per_unit));
        
        opportunities
    }

    // Scout all markets in system
    pub async fn scout_system_markets(&mut self, system: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔍🔍🔍 SCOUTING ALL MARKETS IN SYSTEM {} 🔍🔍🔍", system);
        
        // Get all waypoints and filter for marketplaces
        let all_waypoints = self.client.get_system_waypoints(system, None).await?;
        let waypoints: Vec<_> = all_waypoints.into_iter()
            .filter(|w| w.traits.iter().any(|t| t.symbol == "MARKETPLACE"))
            .collect();
        
        println!("   Found {} marketplaces to scout", waypoints.len());
        
        for waypoint in &waypoints {
            match self.scout_market(system, &waypoint.symbol).await {
                Ok(_) => println!("   ✅ Scouted {}", waypoint.symbol),
                Err(e) => println!("   ❌ Failed to scout {}: {}", waypoint.symbol, e),
            }
            
            // Brief delay to avoid API spam
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
        
        Ok(())
    }

    // Recommend best action for ship with current cargo
    pub async fn recommend_action(&mut self, ship_symbol: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n💡💡💡 MARKET INTELLIGENCE RECOMMENDATIONS 💡💡💡");
        println!("======================================================");
        
        self.update_cargo_inventory(ship_symbol).await?;
        
        if self.cargo_inventory.is_empty() {
            println!("📦 CARGO EMPTY - LOOKING FOR BUYING OPPORTUNITIES:");
            
            let routes = self.find_trading_routes();
            let top_routes = routes.into_iter().take(5).collect::<Vec<_>>();
            
            for (i, route) in top_routes.iter().enumerate() {
                println!("   {}. {} - Buy at {} for {}💎, Sell at {} for {}💎",
                         i + 1, route.item, route.buy_at, route.buy_price,
                         route.sell_at, route.sell_price);
                println!("      💰 Profit: {}💎/unit ({:.1}% margin)",
                         route.profit_per_unit, route.profit_margin);
            }
        } else {
            println!("📦 CARGO LOADED - LOOKING FOR SELLING OPPORTUNITIES:");
            
            for (item, units) in &self.cargo_inventory {
                println!("\n   🎯 {} x{} units:", item, units);
                let opportunities = self.find_selling_opportunities(item, *units);
                
                for (i, (waypoint, trade_info)) in opportunities.iter().take(3).enumerate() {
                    let total_value = trade_info.sell_price * units;
                    println!("      {}. {} - {}💎/unit ({}💎 total, supply: {})",
                             i + 1, waypoint, trade_info.sell_price, total_value, trade_info.supply);
                }
            }
        }
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("🔍💰🔍 MARKET INTELLIGENCE SYSTEM 🔍💰🔍");
    println!("=======================================");
    println!("⚡ SCOUT MARKETS BEFORE TRAVELING!");
    println!("💰 FIND PROFITABLE TRADE ROUTES!");
    println!("🎯 AUTONOMOUS TRADING DECISIONS!");
    
    let mut intelligence = MarketIntelligenceSystem::new(priority_client);
    
    // Scout all markets in current system
    intelligence.scout_system_markets("X1-N5").await?;
    
    // Show trading opportunities
    println!("\n💰💰💰 TOP TRADING OPPORTUNITIES 💰💰💰");
    println!("=====================================");
    
    let routes = intelligence.find_trading_routes();
    let top_routes = routes.into_iter().take(10).collect::<Vec<_>>();
    
    for (i, route) in top_routes.iter().enumerate() {
        println!("{}. {} - {}💎 → {}💎 ({}💎 profit, {:.1}% margin)",
                 i + 1, route.item, route.buy_price, route.sell_price,
                 route.profit_per_unit, route.profit_margin);
        println!("   Buy at: {} | Sell at: {}", route.buy_at, route.sell_at);
    }
    
    // Give recommendations for first mining ship
    let ships = intelligence.client.get_ships().await?;
    if let Some(ship) = ships.first() {
        intelligence.recommend_action(&ship.symbol).await?;
    }
    
    println!("\n🤖 MARKET INTELLIGENCE SYSTEM OPERATIONAL!");
    println!("💡 Use this data to make autonomous trading decisions!");
    
    Ok(())
}