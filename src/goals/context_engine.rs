// Context Engine - Provides game state awareness for intelligent goal planning
use crate::goals::{GoalContext, FleetStatus};
use crate::client::PriorityApiClient;
use crate::models::*;
use crate::o_debug;
use std::collections::HashMap;

pub struct ContextEngine {
    waypoint_cache: HashMap<String, Vec<Waypoint>>, // system_symbol -> waypoints
    market_cache: HashMap<String, Market>,           // waypoint_symbol -> market
    shipyard_cache: HashMap<String, bool>,           // waypoint_symbol -> has_shipyard
    resource_locations: HashMap<String, Vec<String>>, // resource_type -> waypoint_symbols
    last_update: std::time::Instant,
    cache_ttl: std::time::Duration,
}

impl ContextEngine {
    pub fn new() -> Self {
        Self {
            waypoint_cache: HashMap::new(),
            market_cache: HashMap::new(),
            shipyard_cache: HashMap::new(),
            resource_locations: HashMap::new(),
            last_update: std::time::Instant::now(),
            cache_ttl: std::time::Duration::from_secs(300), // 5 minute cache
        }
    }

    /// Build comprehensive context for goal planning and execution
    pub async fn build_context(&mut self, client: &PriorityApiClient) -> Result<GoalContext, Box<dyn std::error::Error>> {
        o_debug!("🔄 Building comprehensive goal context...");
        
        // Get basic game state
        let agent = client.get_agent().await?;
        let ships = client.get_ships().await?;
        let contracts = client.get_contracts().await?;
        
        // Update caches if needed
        if self.last_update.elapsed() > self.cache_ttl {
            self.update_caches(client, &ships).await?;
        }
        
        // Analyze fleet capabilities
        let fleet_status = self.analyze_fleet_status(&ships);
        
        // Clone cached data for context
        let known_waypoints = self.waypoint_cache.clone();
        let known_markets = self.market_cache.clone();
        
        let credits = agent.credits as i32;
        Ok(GoalContext {
            ships,
            agent,
            contracts,
            known_waypoints,
            known_markets,
            available_credits: credits,
            fleet_status,
        })
    }

    /// Find the best locations for a specific resource
    pub async fn find_resource_locations(&mut self, client: &PriorityApiClient, resource_type: &str, system_symbol: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        o_debug!("🔍 Finding locations for resource: {} in system: {}", resource_type, system_symbol);
        
        // Check cache first
        if let Some(locations) = self.resource_locations.get(resource_type) {
            let system_locations: Vec<_> = locations.iter()
                .filter(|loc| loc.starts_with(system_symbol))
                .cloned()
                .collect();
            
            if !system_locations.is_empty() {
                return Ok(system_locations);
            }
        }
        
        // Fetch waypoints for the system
        let waypoints = match self.get_system_waypoints(client, system_symbol).await {
            Ok(waypoints) => waypoints,
            Err(e) => return Err(e)
        };
        let mut resource_waypoints = Vec::new();
        
        for waypoint in waypoints {
            if self.waypoint_has_resource(&waypoint, resource_type) {
                resource_waypoints.push(waypoint.symbol.clone());
            }
        }
        
        // Update cache
        self.resource_locations.insert(resource_type.to_string(), resource_waypoints.clone());
        
        o_debug!("📍 Found {} locations for {}: {:?}", resource_waypoints.len(), resource_type, resource_waypoints);
        Ok(resource_waypoints)
    }

    /// Find facilities of a specific type (markets, shipyards, refineries)
    pub async fn find_facilities(&mut self, client: &PriorityApiClient, facility_type: &str, system_symbol: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        o_debug!("🏢 Finding {} facilities in system: {}", facility_type, system_symbol);
        
        let waypoints = match self.get_system_waypoints(client, system_symbol).await {
            Ok(waypoints) => waypoints,
            Err(e) => return Err(e)
        };
        let mut facilities = Vec::new();
        
        for waypoint in waypoints {
            match facility_type {
                "marketplace" | "market" => {
                    if waypoint.traits.iter().any(|t| t.symbol == "MARKETPLACE") {
                        facilities.push(waypoint.symbol);
                    }
                }
                "shipyard" => {
                    if waypoint.traits.iter().any(|t| t.symbol == "SHIPYARD") {
                        facilities.push(waypoint.symbol.clone());
                        self.shipyard_cache.insert(waypoint.symbol, true);
                    }
                }
                "refinery" => {
                    if waypoint.traits.iter().any(|t| t.description.to_lowercase().contains("refin") || 
                                                        t.description.to_lowercase().contains("smelt") ||
                                                        t.description.to_lowercase().contains("process")) {
                        facilities.push(waypoint.symbol);
                    }
                }
                "fuel_station" => {
                    // Fuel stations are typically marketplaces
                    if waypoint.traits.iter().any(|t| t.symbol == "MARKETPLACE") {
                        facilities.push(waypoint.symbol);
                    }
                }
                _ => {
                    o_debug!("⚠️ Unknown facility type: {}", facility_type);
                }
            }
        }
        
        o_debug!("🏢 Found {} {} facilities: {:?}", facilities.len(), facility_type, facilities);
        Ok(facilities)
    }

    /// Get optimal ship for a specific task type
    pub fn recommend_ship(&self, task_type: &str, context: &GoalContext) -> Option<String> {
        o_debug!("🚢 Recommending ship for task: {}", task_type);
        
        match task_type {
            "mining" => {
                // Prioritize ships with mining mounts and good cargo capacity
                context.ships.iter()
                    .filter(|ship| !context.fleet_status.busy_ships.contains_key(&ship.symbol))
                    .filter(|ship| {
                        ship.mounts.iter().any(|m| m.symbol.contains("MINING")) ||
                        ship.registration.role.contains("EXCAVATOR")
                    })
                    .max_by_key(|ship| ship.cargo.capacity)
                    .map(|ship| ship.symbol.clone())
            }
            
            "hauling" | "transport" => {
                // Prioritize ships with large cargo capacity
                context.ships.iter()
                    .filter(|ship| !context.fleet_status.busy_ships.contains_key(&ship.symbol))
                    .max_by_key(|ship| ship.cargo.capacity)
                    .map(|ship| ship.symbol.clone())
            }
            
            "exploration" => {
                // Prioritize probe/satellite ships, or ships with good fuel capacity
                context.ships.iter()
                    .filter(|ship| !context.fleet_status.busy_ships.contains_key(&ship.symbol))
                    .find(|ship| {
                        ship.registration.role.contains("SATELLITE") ||
                        ship.registration.role.contains("PROBE")
                    })
                    .or_else(|| {
                        // Fallback to ship with best fuel capacity
                        context.ships.iter()
                            .filter(|ship| !context.fleet_status.busy_ships.contains_key(&ship.symbol))
                            .max_by_key(|ship| ship.fuel.capacity)
                    })
                    .map(|ship| ship.symbol.clone())
            }
            
            "trading" => {
                // Prefer ships that are docked at marketplaces or have cargo to sell
                context.ships.iter()
                    .filter(|ship| !context.fleet_status.busy_ships.contains_key(&ship.symbol))
                    .find(|ship| {
                        ship.cargo.units > 0 || 
                        (ship.nav.status == "DOCKED" && 
                         self.is_marketplace(&ship.nav.waypoint_symbol))
                    })
                    .or_else(|| {
                        // Fallback to ship with largest cargo capacity
                        context.ships.iter()
                            .filter(|ship| !context.fleet_status.busy_ships.contains_key(&ship.symbol))
                            .max_by_key(|ship| ship.cargo.capacity)
                    })
                    .map(|ship| ship.symbol.clone())
            }
            
            _ => {
                // Default: any available ship
                context.fleet_status.available_ships.first().cloned()
            }
        }
    }

    /// Calculate distance between waypoints
    pub fn calculate_distance(&self, from_waypoint: &str, to_waypoint: &str, system_symbol: &str) -> Option<f64> {
        if let Some(waypoints) = self.waypoint_cache.get(system_symbol) {
            let from = waypoints.iter().find(|w| w.symbol == from_waypoint)?;
            let to = waypoints.iter().find(|w| w.symbol == to_waypoint)?;
            
            let dx = (to.x - from.x) as f64;
            let dy = (to.y - from.y) as f64;
            Some((dx * dx + dy * dy).sqrt())
        } else {
            None
        }
    }

    /// Estimate execution time for traveling between waypoints
    pub fn estimate_travel_time(&self, from: &str, to: &str, system: &str, _ship: &Ship) -> Option<f64> {
        let distance = self.calculate_distance(from, to, system)?;
        let fuel_needed = (distance / 100.0).ceil(); // Rough fuel calculation
        let travel_time = fuel_needed * 2.0; // 2 seconds per fuel unit (rough estimate)
        Some(travel_time)
    }

    /// Check if a goal is feasible given current context
    pub fn validate_goal_feasibility(&self, goal_description: &str, context: &GoalContext) -> Result<bool, String> {
        o_debug!("✅ Validating goal feasibility: {}", goal_description);
        
        if goal_description.contains("mine") {
            // Check if we have mining ships
            if context.fleet_status.mining_ships.is_empty() {
                return Err("No mining ships available".to_string());
            }
            
            // Check if mining ships are available (not busy)
            let available_miners: Vec<_> = context.fleet_status.mining_ships.iter()
                .filter(|ship| !context.fleet_status.busy_ships.contains_key(*ship))
                .collect();
            
            if available_miners.is_empty() {
                return Err("All mining ships are busy".to_string());
            }
        }
        
        if goal_description.contains("buy") || goal_description.contains("purchase") {
            // Check if we have sufficient credits (rough estimate)
            if context.available_credits < 1000 {
                return Err("Insufficient credits for purchase".to_string());
            }
        }
        
        if goal_description.contains("refine") && !goal_description.contains("designate") {
            // Check if we have hauler ships for transport (only for actual refining operations, not designation)
            if context.fleet_status.hauler_ships.is_empty() {
                return Err("No hauler ships available for refining operations".to_string());
            }
        }
        
        Ok(true)
    }

    // Private helper methods
    
    async fn update_caches(&mut self, client: &PriorityApiClient, ships: &[Ship]) -> Result<(), Box<dyn std::error::Error>> {
        o_debug!("🔄 Updating context caches...");
        
        // Get unique systems from ship locations
        let mut systems: std::collections::HashSet<String> = std::collections::HashSet::new();
        for ship in ships {
            systems.insert(ship.nav.system_symbol.clone());
        }
        
        // Update waypoint cache for each system
        for system in systems {
            match client.get_system_waypoints(&system, None).await {
                Ok(waypoints) => {
                    self.waypoint_cache.insert(system, waypoints);
                }
                Err(_) => {
                    // Skip failed requests - cache will remain unchanged
                    continue;
                }
            }
        }
        
        self.last_update = std::time::Instant::now();
        o_debug!("✅ Context caches updated");
        Ok(())
    }

    async fn get_system_waypoints(&mut self, client: &PriorityApiClient, system_symbol: &str) -> Result<Vec<Waypoint>, Box<dyn std::error::Error>> {
        // Check cache first
        if let Some(waypoints) = self.waypoint_cache.get(system_symbol) {
            return Ok(waypoints.clone());
        }
        
        // Fetch from API
        match client.get_system_waypoints(system_symbol, None).await {
            Ok(waypoints) => {
                self.waypoint_cache.insert(system_symbol.to_string(), waypoints.clone());
                Ok(waypoints)
            }
            Err(e) => Err(e)
        }
    }

    fn waypoint_has_resource(&self, waypoint: &Waypoint, resource_type: &str) -> bool {
        // Check if waypoint type suggests it has the resource
        if waypoint.waypoint_type.contains("ASTEROID") {
            // Asteroid fields typically have ore resources
            return resource_type.ends_with("_ORE") || resource_type.ends_with("ORE");
        }
        
        // Check waypoint traits for resource indicators
        for trait_info in &waypoint.traits {
            let description = trait_info.description.to_lowercase();
            let resource_lower = resource_type.to_lowercase();
            
            if description.contains(&resource_lower) ||
               description.contains("mining") ||
               description.contains("mineral") ||
               description.contains("ore") {
                return true;
            }
        }
        
        false
    }

    fn analyze_fleet_status(&self, ships: &[Ship]) -> FleetStatus {
        let mut mining_ships = Vec::new();
        let mut hauler_ships = Vec::new();
        let mut probe_ships = Vec::new();
        let mut available_ships = Vec::new();
        let mut busy_ships = HashMap::new();
        
        for ship in ships {
            let ship_symbol = ship.symbol.clone();
            
            // Classify ships by capability
            if ship.registration.role.contains("EXCAVATOR") ||
               ship.mounts.iter().any(|m| m.symbol.contains("MINING")) {
                mining_ships.push(ship_symbol.clone());
            }
            
            if ship.registration.role.contains("HAULER") || ship.cargo.capacity > 40 {
                hauler_ships.push(ship_symbol.clone());
            }
            
            if ship.registration.role.contains("SATELLITE") || 
               ship.registration.role.contains("PROBE") ||
               ship.registration.role.contains("EXPLORER") {
                probe_ships.push(ship_symbol.clone());
            }
            
            // Check availability status
            if ship.nav.status == "IN_TRANSIT" {
                busy_ships.insert(ship_symbol, "in_transit".to_string());
            } else if ship.fuel.current < 5 {
                busy_ships.insert(ship_symbol, "low_fuel".to_string());
            } else if ship.cargo.units >= ship.cargo.capacity {
                busy_ships.insert(ship_symbol, "cargo_full".to_string());
            } else {
                available_ships.push(ship_symbol);
            }
        }
        
        FleetStatus {
            available_ships,
            busy_ships,
            mining_ships,
            hauler_ships,
            probe_ships,
        }
    }

    fn is_marketplace(&self, waypoint_symbol: &str) -> bool {
        // Check if we have market data for this waypoint
        self.market_cache.contains_key(waypoint_symbol) ||
        // Or check cached waypoint traits
        self.waypoint_cache.values()
            .flatten()
            .find(|w| w.symbol == waypoint_symbol)
            .map_or(false, |w| w.traits.iter().any(|t| t.symbol == "MARKETPLACE"))
    }
}