// 🧠💾 PERSISTENT SYSTEM KNOWLEDGE CACHE - NEVER FORGET ANYTHING! 💾🧠
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemKnowledge {
    pub system_symbol: String,
    pub last_updated: String,
    pub waypoints: HashMap<String, WaypointKnowledge>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WaypointKnowledge {
    pub symbol: String,
    pub waypoint_type: String,
    pub traits: Vec<String>,
    pub has_marketplace: bool,
    pub has_shipyard: bool,
    pub has_repair_facility: bool,
    pub has_fuel_depot: bool,
    pub has_mining_site: bool,
    pub market_exports: Vec<String>,
    pub market_imports: Vec<String>,
    pub market_exchange: Vec<String>,
    pub shipyard_ship_types: Vec<String>,
    pub coordinates: (i32, i32),
}

pub struct PersistentKnowledgeSystem {
    client: PriorityApiClient,
    knowledge_cache: HashMap<String, SystemKnowledge>,
    cache_file: String,
}

impl PersistentKnowledgeSystem {
    pub fn new(client: PriorityApiClient) -> Self {
        let cache_file = "storage/system_knowledge_cache.json".to_string();
        let mut system = Self {
            client,
            knowledge_cache: HashMap::new(),
            cache_file,
        };
        
        system.load_cache();
        system
    }

    fn load_cache(&mut self) {
        if Path::new(&self.cache_file).exists() {
            match fs::read_to_string(&self.cache_file) {
                Ok(data) => {
                    match serde_json::from_str::<HashMap<String, SystemKnowledge>>(&data) {
                        Ok(cache) => {
                            self.knowledge_cache = cache;
                            println!("🧠 LOADED SYSTEM KNOWLEDGE: {} systems cached", self.knowledge_cache.len());
                        }
                        Err(e) => println!("⚠️  Failed to parse cache: {}", e),
                    }
                }
                Err(e) => println!("⚠️  Failed to read cache: {}", e),
            }
        } else {
            println!("🆕 NEW KNOWLEDGE CACHE - Will build from scratch");
        }
    }

    fn save_cache(&self) {
        if let Some(parent) = Path::new(&self.cache_file).parent() {
            fs::create_dir_all(parent).ok();
        }
        
        match serde_json::to_string_pretty(&self.knowledge_cache) {
            Ok(data) => {
                match fs::write(&self.cache_file, data) {
                    Ok(_) => println!("💾 KNOWLEDGE CACHE SAVED: {} systems", self.knowledge_cache.len()),
                    Err(e) => println!("❌ Failed to save cache: {}", e),
                }
            }
            Err(e) => println!("❌ Failed to serialize cache: {}", e),
        }
    }

    pub async fn scan_and_cache_system(&mut self, system_symbol: &str) -> Result<&SystemKnowledge, Box<dyn std::error::Error>> {
        println!("🔍 SCANNING SYSTEM {} FOR COMPLETE KNOWLEDGE...", system_symbol);
        
        // First get all waypoints
        let mut waypoints = self.client.get_system_waypoints(system_symbol, None).await?;
        
        // Then get waypoints with specific traits that might be missing
        match self.client.get_system_waypoints_with_traits(system_symbol, "SHIPYARD").await {
            Ok(shipyard_waypoints) => {
                for wp in shipyard_waypoints {
                    if !waypoints.iter().any(|w| w.symbol == wp.symbol) {
                        waypoints.push(wp);
                    }
                }
            }
            Err(e) => println!("   ⚠️  Failed to get shipyard waypoints: {}", e),
        }
        let mut waypoint_knowledge = HashMap::new();
        
        for waypoint in &waypoints {
            let mut wp_knowledge = WaypointKnowledge {
                symbol: waypoint.symbol.clone(),
                waypoint_type: waypoint.waypoint_type.clone(),
                traits: waypoint.traits.iter().map(|t| t.symbol.clone()).collect(),
                has_marketplace: waypoint.traits.iter().any(|t| t.symbol == "MARKETPLACE"),
                has_shipyard: waypoint.traits.iter().any(|t| t.symbol == "SHIPYARD"),
                has_repair_facility: false,
                has_fuel_depot: waypoint.traits.iter().any(|t| t.symbol == "FUEL_DEPOT"),
                has_mining_site: waypoint.traits.iter().any(|t| t.symbol == "MINERAL_DEPOSITS"),
                market_exports: Vec::new(),
                market_imports: Vec::new(),
                market_exchange: Vec::new(),
                shipyard_ship_types: Vec::new(),
                coordinates: (waypoint.x, waypoint.y),
            };
            
            if wp_knowledge.has_shipyard {
                wp_knowledge.has_repair_facility = true;
                
                match self.client.get_shipyard(system_symbol, &waypoint.symbol).await {
                    Ok(shipyard) => {
                        wp_knowledge.shipyard_ship_types = shipyard.ship_types.iter().map(|st| st.ship_type.clone()).collect();
                        println!("   ⚒️  Shipyard {}: {} ship types", waypoint.symbol, wp_knowledge.shipyard_ship_types.len());
                    }
                    Err(e) => println!("   ⚠️  Failed to get shipyard details for {}: {}", waypoint.symbol, e),
                }
                
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
            
            if wp_knowledge.has_marketplace {
                match self.client.get_market(system_symbol, &waypoint.symbol).await {
                    Ok(market) => {
                        wp_knowledge.market_exports = market.exports.iter().map(|e| e.symbol.clone()).collect();
                        wp_knowledge.market_imports = market.imports.iter().map(|i| i.symbol.clone()).collect();
                        wp_knowledge.market_exchange = market.exchange.iter().map(|x| x.symbol.clone()).collect();
                        println!("   🏪 Market {}: {} exports, {} imports, {} exchange", 
                                waypoint.symbol, wp_knowledge.market_exports.len(), 
                                wp_knowledge.market_imports.len(), wp_knowledge.market_exchange.len());
                    }
                    Err(e) => println!("   ⚠️  Failed to get market details for {}: {}", waypoint.symbol, e),
                }
                
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
            
            waypoint_knowledge.insert(waypoint.symbol.clone(), wp_knowledge);
        }
        
        let system_knowledge = SystemKnowledge {
            system_symbol: system_symbol.to_string(),
            last_updated: chrono::Utc::now().to_rfc3339(),
            waypoints: waypoint_knowledge,
        };
        
        self.knowledge_cache.insert(system_symbol.to_string(), system_knowledge);
        self.save_cache();
        
        println!("✅ SYSTEM {} FULLY SCANNED: {} waypoints cached", system_symbol, waypoints.len());
        
        Ok(self.knowledge_cache.get(system_symbol).unwrap())
    }

    pub fn find_shipyards(&self, system_symbol: &str) -> Vec<&WaypointKnowledge> {
        if let Some(system) = self.knowledge_cache.get(system_symbol) {
            system.waypoints.values()
                .filter(|wp| wp.has_shipyard)
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn find_marketplaces(&self, system_symbol: &str) -> Vec<&WaypointKnowledge> {
        if let Some(system) = self.knowledge_cache.get(system_symbol) {
            system.waypoints.values()
                .filter(|wp| wp.has_marketplace)
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn find_fuel_depots(&self, system_symbol: &str) -> Vec<&WaypointKnowledge> {
        if let Some(system) = self.knowledge_cache.get(system_symbol) {
            system.waypoints.values()
                .filter(|wp| wp.has_fuel_depot)
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn find_mining_sites(&self, system_symbol: &str) -> Vec<&WaypointKnowledge> {
        if let Some(system) = self.knowledge_cache.get(system_symbol) {
            system.waypoints.values()
                .filter(|wp| wp.has_mining_site)
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn find_repair_facilities(&self, system_symbol: &str) -> Vec<&WaypointKnowledge> {
        if let Some(system) = self.knowledge_cache.get(system_symbol) {
            system.waypoints.values()
                .filter(|wp| wp.has_repair_facility)
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn find_markets_importing(&self, system_symbol: &str, item: &str) -> Vec<&WaypointKnowledge> {
        if let Some(system) = self.knowledge_cache.get(system_symbol) {
            system.waypoints.values()
                .filter(|wp| wp.market_imports.contains(&item.to_string()))
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn find_markets_exporting(&self, system_symbol: &str, item: &str) -> Vec<&WaypointKnowledge> {
        if let Some(system) = self.knowledge_cache.get(system_symbol) {
            system.waypoints.values()
                .filter(|wp| wp.market_exports.contains(&item.to_string()))
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn is_system_known(&self, system_symbol: &str) -> bool {
        self.knowledge_cache.contains_key(system_symbol)
    }

    pub fn get_knowledge_summary(&self) -> String {
        if self.knowledge_cache.is_empty() {
            return "📊 KNOWLEDGE CACHE EMPTY".to_string();
        }
        
        let mut summary = format!("📊 KNOWLEDGE CACHE SUMMARY ({} systems):\n", self.knowledge_cache.len());
        
        for (system, knowledge) in &self.knowledge_cache {
            let shipyards = knowledge.waypoints.values().filter(|wp| wp.has_shipyard).count();
            let markets = knowledge.waypoints.values().filter(|wp| wp.has_marketplace).count();
            let mining = knowledge.waypoints.values().filter(|wp| wp.has_mining_site).count();
            let fuel = knowledge.waypoints.values().filter(|wp| wp.has_fuel_depot).count();
            
            summary.push_str(&format!(
                "   {} - {} waypoints (⚒️ {}  🏪 {}  ⛏️ {}  ⛽ {})\n",
                system, knowledge.waypoints.len(), shipyards, markets, mining, fuel
            ));
        }
        
        summary
    }

    pub async fn ensure_system_knowledge(&mut self, system_symbol: &str) -> Result<&SystemKnowledge, Box<dyn std::error::Error>> {
        if !self.is_system_known(system_symbol) {
            println!("🔍 SYSTEM {} NOT IN CACHE - SCANNING NOW...", system_symbol);
            self.scan_and_cache_system(system_symbol).await
        } else {
            println!("✅ SYSTEM {} ALREADY CACHED", system_symbol);
            Ok(self.knowledge_cache.get(system_symbol).unwrap())
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("🧠💾🧠 PERSISTENT SYSTEM KNOWLEDGE CACHE 🧠💾🧠");
    println!("===============================================");
    println!("💡 NEVER FORGET SHIPYARDS, MARKETS, OR FACILITIES AGAIN!");
    println!("🎯 CACHE ALL SYSTEM KNOWLEDGE FOR INSTANT GALACTIC NAVIGATION!");
    
    let mut knowledge_system = PersistentKnowledgeSystem::new(priority_client);
    
    println!("\n{}", knowledge_system.get_knowledge_summary());
    
    // Ensure we know about our current system
    let current_system = "X1-N5";
    knowledge_system.ensure_system_knowledge(current_system).await?;
    
    println!("\n🔧 TESTING KNOWLEDGE QUERIES:");
    println!("================================");
    
    let shipyards = knowledge_system.find_shipyards(current_system);
    println!("⚒️  SHIPYARDS IN {}: {}", current_system, shipyards.len());
    for shipyard in &shipyards {
        println!("   {} - Ship types: {:?}", shipyard.symbol, shipyard.shipyard_ship_types);
    }
    
    let markets = knowledge_system.find_marketplaces(current_system);
    println!("\n🏪 MARKETPLACES IN {}: {}", current_system, markets.len());
    for market in markets.iter().take(3) {
        println!("   {} - Exports: {}, Imports: {}", 
                market.symbol, market.market_exports.len(), market.market_imports.len());
    }
    
    let mining_sites = knowledge_system.find_mining_sites(current_system);
    println!("\n⛏️  MINING SITES IN {}: {}", current_system, mining_sites.len());
    
    let fuel_depots = knowledge_system.find_fuel_depots(current_system);
    println!("\n⛽ FUEL DEPOTS IN {}: {}", current_system, fuel_depots.len());
    
    // Test specific item searches
    let iron_buyers = knowledge_system.find_markets_importing(current_system, "IRON_ORE");
    println!("\n💎 MARKETS IMPORTING IRON_ORE: {}", iron_buyers.len());
    for buyer in iron_buyers.iter().take(3) {
        println!("   {}", buyer.symbol);
    }
    
    println!("\n🎯 INSTANT KNOWLEDGE SYSTEM OPERATIONAL!");
    println!("💡 Never rediscover facilities again - all knowledge cached permanently!");
    
    Ok(())
}