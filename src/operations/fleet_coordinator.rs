// Fleet Coordinator - Manages ship actors and task assignment
use crate::client::SpaceTradersClient;
use crate::models::*;
use crate::operations::ship_actor::*;
use crate::operations::ship_prioritizer::*;
use crate::storage::{ShipStateStore, SurveyCache};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use std::collections::HashMap;

pub struct FleetCoordinator {
    client: SpaceTradersClient,
    ship_queues: HashMap<String, mpsc::UnboundedSender<ShipAction>>,
    ship_states: HashMap<String, ShipState>,
    status_receiver: mpsc::UnboundedReceiver<(String, ShipState)>,
    status_sender: mpsc::UnboundedSender<(String, ShipState)>,
    prioritizer: ShipPrioritizer,
    fleet_metrics: Vec<ShipPerformanceMetrics>,
    ship_cache: ShipStateStore,
    survey_cache: SurveyCache,
}

impl FleetCoordinator {
    pub fn new(client: SpaceTradersClient) -> Self {
        let (status_sender, status_receiver) = mpsc::unbounded_channel();
        let prioritizer = ShipPrioritizer::new(client.clone());
        let ship_cache = ShipStateStore::new("storage/ship_states.json", 5); // 5 minute staleness threshold
        let survey_cache = SurveyCache::new("storage/survey_cache.json", 12); // 12 hour cache duration
        
        Self {
            client,
            ship_queues: HashMap::new(),
            ship_states: HashMap::new(),
            status_receiver,
            status_sender,
            prioritizer,
            fleet_metrics: Vec::new(),
            ship_cache,
            survey_cache,
        }
    }

    pub async fn initialize_fleet(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🚀 Initializing fleet with cached state system...");
        
        // Print cache status first
        self.ship_cache.print_cache_status();
        self.survey_cache.print_cache_status();
        
        // Get all ships from API (we need this once to know what ships exist)
        let ships = self.client.get_ships().await?;
        println!("📡 Fetched {} ships from API", ships.len());
        
        // Cache all ships
        for ship in &ships {
            self.ship_cache.cache_ship(ship.clone())?;
        }
        
        // Spawn actors for all ships
        for ship in ships {
            self.spawn_ship_actor(ship).await?;
        }
        
        println!("✅ Fleet initialization complete - {} ship actors spawned with cached states", self.ship_queues.len());
        self.ship_cache.print_cache_status();
        Ok(())
    }

    async fn spawn_ship_actor(&mut self, ship: Ship) -> Result<(), Box<dyn std::error::Error>> {
        let ship_symbol = ship.symbol.clone();
        
        // Create action channel for this ship
        let (action_sender, action_receiver) = mpsc::unbounded_channel();
        
        // Clone client for the actor
        let client_clone = SpaceTradersClient::new(self.client.token.clone());
        let status_sender_clone = self.status_sender.clone();
        
        // Create and spawn the ship actor
        let mut actor = ShipActor::new(
            ship_symbol.clone(),
            action_receiver,
            status_sender_clone,
            client_clone,
        );
        
        // Initialize ship state
        let initial_state = ShipState {
            ship: ship.clone(),
            cooldown_until: None,
            current_action: None,
            status: ShipActorStatus::Idle,
        };
        
        // Store references
        self.ship_queues.insert(ship_symbol.clone(), action_sender);
        self.ship_states.insert(ship_symbol.clone(), initial_state);
        
        // Spawn the actor task
        tokio::spawn(async move {
            actor.run().await;
        });
        
        println!("🤖 Spawned actor for {}", ship_symbol);
        Ok(())
    }

    pub async fn run_autonomous_operations(&mut self, contract: &Contract) -> Result<(), Box<dyn std::error::Error>> {
        println!("🎖️ Fleet Coordinator starting autonomous operations...");
        println!("🎯 Contract: {} - Materials: {:?}", contract.id, 
                contract.terms.deliver.iter().map(|d| &d.trade_symbol).collect::<Vec<_>>());
        
        // Start the main coordination loop
        let mut cycle_count = 0;
        
        loop {
            cycle_count += 1;
            println!("\n🔄 ═══ COORDINATION CYCLE #{} ═══", cycle_count);
            
            // Process status updates from ships
            self.process_status_updates().await;
            
            // Check for new ships that might have been added
            self.discover_new_ships().await?;
            
            // Check if we should purchase additional ships
            self.check_ship_expansion(contract).await?;
            
            // Assign tasks based on current fleet state
            self.assign_tasks(contract).await?;
            
            // Wait before next cycle
            sleep(Duration::from_secs(10)).await;
            
            // Check if contract is complete
            if self.is_contract_complete(contract).await? {
                println!("🎉 Contract {} completed!", contract.id);
                break;
            }
        }
        
        Ok(())
    }

    async fn process_status_updates(&mut self) {
        // Process all pending status updates
        while let Ok((ship_symbol, new_state)) = self.status_receiver.try_recv() {
            println!("📡 Status update from {}: {:?}", ship_symbol, new_state.status);
            self.ship_states.insert(ship_symbol, new_state);
        }
    }

    async fn assign_tasks(&mut self, contract: &Contract) -> Result<(), Box<dyn std::error::Error>> {
        let needed_materials: Vec<String> = contract.terms.deliver
            .iter()
            .map(|d| d.trade_symbol.clone())
            .collect();
        
        println!("🎯 Assigning tasks - needed materials: {:?}", needed_materials);
        
        // Get ships from cache (refresh stale ones)
        let cached_ships = self.ship_cache.list_cached_ships();
        let mut ships = Vec::new();
        
        for ship_symbol in cached_ships {
            if let Ok(cached_state) = self.ship_cache.get_fresh_ship_state(&ship_symbol, &self.client).await {
                ships.push(cached_state.ship.clone());
            }
        }
        
        if ships.is_empty() {
            println!("⚠️ No ships available in cache, fetching from API...");
            let api_ships = self.client.get_ships().await?;
            for ship in &api_ships {
                self.ship_cache.cache_ship(ship.clone())?;
            }
            ships = api_ships;
        }
        
        println!("📊 Using {} ships ({} from cache)", ships.len(), self.ship_cache.list_cached_ships().len());
        
        // Analyze fleet performance and get prioritized metrics
        self.fleet_metrics = self.prioritizer.analyze_fleet_performance(&ships, contract).await?;
        
        // Update ship statuses in metrics from our state tracking
        for metrics in &mut self.fleet_metrics {
            if let Some(ship_state) = self.ship_states.get(&metrics.ship_symbol) {
                metrics.status = ship_state.status.clone();
            }
        }
        
        // Get idle ships in priority order
        let idle_ships = self.prioritizer.get_idle_ships(&self.fleet_metrics);
        
        if idle_ships.is_empty() {
            println!("📊 All ships are busy. Current fleet status:");
            for metrics in &self.fleet_metrics {
                println!("  🚢 {} - Priority: {:.2} - Status: {:?}", 
                        metrics.ship_symbol, metrics.priority_weight, metrics.status);
            }
        } else {
            println!("🎯 Assigning tasks to {} idle ships in priority order", idle_ships.len());
            
            for ship_symbol in idle_ships {
                if let Some(ship) = ships.iter().find(|s| s.symbol == ship_symbol) {
                    if let Some(metrics) = self.fleet_metrics.iter().find(|m| m.ship_symbol == ship_symbol) {
                        let recommended_task = self.prioritizer.recommend_optimal_task(metrics, contract);
                        println!("🎖️ {} (Priority: {:.2}) -> {}", ship_symbol, metrics.priority_weight, recommended_task);
                        
                        // Priority-based task assignment
                        // Check if this is a probe/satellite first - they can't move and need special handling
                        if self.is_probe(&ship) {
                            // Probes can only scan, they cannot move
                            if ship.fuel.capacity == 0 {
                                // Stationary satellite - skip entirely to reduce console noise
                                // TODO: Implement useful satellite functionality later
                                continue;
                            } else {
                                println!("🔭 {} is a probe - assigning exploration", ship_symbol);
                                self.assign_exploration_task(&ship).await?;
                            }
                        } else if self.needs_refuel(&ship) {
                            println!("⛽ {} needs fuel ({}/{})", ship_symbol, ship.fuel.current, ship.fuel.capacity);
                            self.assign_refuel_task(&ship).await?;
                        } else if self.should_deliver_cargo(&ship, contract) {
                            println!("📦 {} ready for delivery - assigning cargo delivery", ship_symbol);
                            self.assign_delivery_task(&ship, contract).await?;
                        } else if self.is_cargo_full(&ship) {
                            println!("🗃️ {} cargo full - need to manage inventory", ship_symbol);
                            self.assign_cargo_management(&ship, contract).await?;
                        } else if metrics.capabilities.can_mine && metrics.contract_contribution >= 0.05 {
                            println!("⛏️ {} assigned to mining (priority: {:.2})", ship_symbol, metrics.priority_weight);
                            self.assign_mining_task(&ship, &needed_materials, &contract.id).await?;
                        } else if metrics.capabilities.can_trade {
                            println!("🏪 {} assigned to support operations (trading ready)", ship_symbol);
                        }
                    }
                }
            }
        }
        
        Ok(())
    }

    fn _can_mine(&self, ship: &Ship) -> bool {
        ship.mounts.iter().any(|mount| {
            mount.symbol.contains("MINING") || mount.symbol.contains("EXTRACTOR")
        })
    }

    fn is_probe(&self, ship: &Ship) -> bool {
        ship.registration.role == "SATELLITE" || ship.frame.symbol.contains("PROBE")
    }

    fn has_contract_materials(&self, ship: &Ship, needed_materials: &[String]) -> bool {
        ship.cargo.inventory.iter().any(|item| needed_materials.contains(&item.symbol))
    }
    
    /// Determine if ship should deliver cargo (either full cargo or enough for contract)
    fn should_deliver_cargo(&self, ship: &Ship, contract: &Contract) -> bool {
        let needed_materials: Vec<String> = contract.terms.deliver
            .iter()
            .map(|d| d.trade_symbol.clone())
            .collect();
        
        // Check if ship has any contract materials
        let has_contract_items = self.has_contract_materials(ship, &needed_materials);
        if !has_contract_items {
            return false;
        }
        
        // Calculate how much contract material we have
        let mut contract_material_count = 0;
        let mut total_contract_required = 0;
        
        for delivery in &contract.terms.deliver {
            let remaining_needed = delivery.units_required - delivery.units_fulfilled;
            total_contract_required += remaining_needed;
            
            if let Some(cargo_item) = ship.cargo.inventory.iter().find(|item| item.symbol == delivery.trade_symbol) {
                contract_material_count += cargo_item.units.min(remaining_needed);
            }
        }
        
        // Deliver if:
        // 1. Cargo is full (>= 90% capacity)
        // 2. We have enough to fulfill entire contract
        // 3. We have a significant amount (>= 75% of cargo) of contract materials
        let cargo_full = ship.cargo.units as f64 / ship.cargo.capacity as f64 >= 0.9;
        let can_fulfill_contract = contract_material_count >= total_contract_required;
        let significant_amount = contract_material_count as f64 / ship.cargo.capacity as f64 >= 0.75;
        
        if cargo_full {
            println!("🗃️ {} cargo nearly full ({}/{}), should deliver", ship.symbol, ship.cargo.units, ship.cargo.capacity);
            true
        } else if can_fulfill_contract {
            println!("🎯 {} has enough to fulfill contract ({} units), should deliver", ship.symbol, contract_material_count);
            true
        } else if significant_amount {
            println!("📦 {} has significant contract materials ({} units = {:.1}%), should deliver", 
                    ship.symbol, contract_material_count, (contract_material_count as f64 / ship.cargo.capacity as f64) * 100.0);
            true
        } else {
            println!("⏳ {} should continue mining ({} contract materials, {}/{})", 
                    ship.symbol, contract_material_count, ship.cargo.units, ship.cargo.capacity);
            false
        }
    }

    async fn assign_mining_task(&mut self, ship: &Ship, needed_materials: &[String], contract_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Find suitable mining location using cached waypoints
        let system_symbol = ship.nav.waypoint_symbol.split('-').take(2).collect::<Vec<&str>>().join("-");
        let client_for_nav = self.client.clone(); // Clone before mutable borrow
        let waypoints = self.get_system_waypoints_cached(&system_symbol).await?;
        
        // Determine what deposit type we need based on the materials
        let needed_deposit_trait = Self::determine_needed_deposit_type(needed_materials);
        println!("🎯 CONTRACT ANALYSIS: Need materials {:?}", needed_materials);
        println!("   📋 DEPOSIT REQUIREMENT: Looking for asteroids with {}", needed_deposit_trait);
        
        // Show the logic behind deposit selection
        let material_category = if needed_materials.iter().any(|m| ["IRON_ORE", "COPPER_ORE", "ALUMINUM_ORE", "GOLD_ORE", "PLATINUM_ORE", "SILVER_ORE", "URANIUM_ORE", "TITANIUM_ORE", "ZINC_ORE"].contains(&m.as_str())) {
            "metal ores"
        } else if needed_materials.iter().any(|m| ["PRECIOUS_STONES", "DIAMONDS", "RARE_EARTH_ELEMENTS"].contains(&m.as_str())) {
            "precious materials"
        } else if needed_materials.iter().any(|m| ["QUARTZ_SAND", "SILICON_CRYSTALS", "CRYSTALLIZED_SULFUR", "SALT", "GRAPHITE", "LIMESTONE", "CLAY"].contains(&m.as_str())) {
            "industrial minerals"
        } else {
            "unknown materials"
        };
        println!("   🧭 LOGIC: {} are {} → target deposit type: {}", needed_materials.join(", "), material_category, needed_deposit_trait);
        
        // Find asteroids with the right deposit type
        let suitable_asteroids: Vec<_> = waypoints.into_iter()
            .filter(|w| {
                (w.waypoint_type == "ASTEROID" || w.waypoint_type == "ENGINEERED_ASTEROID") &&
                w.traits.iter().any(|t| t.symbol == needed_deposit_trait)
            })
            .collect();
        
        // Filter asteroids by fuel safety - check if ship can reach them
        let total_suitable = suitable_asteroids.len();
        println!("🔍 Checking fuel safety for {} potential mining targets", total_suitable);
        let mut fuel_safe_asteroids = Vec::new();
        
        for asteroid in suitable_asteroids {
            // Create navigation planner for fuel checks
            let nav_planner = crate::operations::NavigationPlanner::new(client_for_nav.clone());
            
            match nav_planner.can_navigate_safely(ship, &asteroid.symbol).await {
                Ok(safety_check) => {
                    if safety_check.is_safe {
                        println!("✅ {} is fuel-safe: {}", asteroid.symbol, safety_check.reason);
                        fuel_safe_asteroids.push(asteroid);
                    } else {
                        println!("⛽ {} is too far: {}", asteroid.symbol, safety_check.reason);
                    }
                }
                Err(e) => {
                    println!("⚠️ {} fuel check failed: {}, including anyway", asteroid.symbol, e);
                    fuel_safe_asteroids.push(asteroid); // Include if check fails
                }
            }
        }
        
        // Sort by preference: Engineered asteroids with marketplaces first, then others
        fuel_safe_asteroids.sort_by(|a, b| {
            let a_score = Self::calculate_mining_preference_score(a);
            let b_score = Self::calculate_mining_preference_score(b);
            b_score.cmp(&a_score) // Descending order (higher score first)
        });
        
        if let Some(target) = fuel_safe_asteroids.first() {
            let deposit_types: Vec<String> = target.traits.iter()
                .filter(|t| t.symbol.contains("DEPOSIT"))
                .map(|t| t.symbol.clone())
                .collect();
            
            let score = Self::calculate_mining_preference_score(target);
            let has_marketplace = target.traits.iter().any(|t| t.symbol == "MARKETPLACE");
            let has_fuel = target.traits.iter().any(|t| t.symbol == "FUEL_STATION");
            
            println!("⛏️ Assigning {} to mine at {} ({})", ship.symbol, target.symbol, target.waypoint_type);
            println!("   🎯 REASON: Need {:?} → requires {} → this asteroid has it", needed_materials, needed_deposit_trait);
            println!("   💎 Deposit types: {:?}", deposit_types);
            println!("   📊 Selection score: {} {}{}", score, 
                    if has_marketplace { "🏪" } else { "" },
                    if has_fuel { "⛽" } else { "" });
            if score > 0 {
                println!("   ⭐ PRIORITY: {}", 
                    if score >= 1100 { "Engineered asteroid with marketplace - optimal!" }
                    else if score >= 200 { "Has fuel station - convenient refueling" }
                    else if score >= 100 { "Engineered asteroid - better yields" }
                    else { "Standard asteroid" });
            }
            
            let mining_action = ShipAction::Mine {
                target: target.symbol.clone(),
                needed_materials: needed_materials.to_vec(),
                contract_id: contract_id.to_string(),
            };
            
            self.send_action_to_ship(&ship.symbol, mining_action).await?;
        } else {
            let total_fuel_safe = fuel_safe_asteroids.len();
            println!("⚠️ No fuel-safe mining locations found for {} (need deposit type: {})", ship.symbol, needed_deposit_trait);
            println!("   📊 Analysis summary:");
            println!("     • Suitable deposit type: {} asteroids", total_suitable);
            println!("     • Fuel-safe: {} asteroids", total_fuel_safe);
            println!("   Available asteroids:");
            for waypoint in waypoints.iter().filter(|w| w.waypoint_type == "ASTEROID" || w.waypoint_type == "ENGINEERED_ASTEROID") {
                let deposits: Vec<String> = waypoint.traits.iter()
                    .filter(|t| t.symbol.contains("DEPOSIT"))
                    .map(|t| t.symbol.clone())
                    .collect();
                println!("     • {} - {:?}", waypoint.symbol, deposits);
            }
        }
        
        Ok(())
    }
    
    /// Determine what deposit type is needed based on the materials we're looking for
    fn determine_needed_deposit_type(needed_materials: &[String]) -> &'static str {
        // Check if we need metal ores (iron, copper, aluminum, etc.)
        let metal_ores = [
            "IRON_ORE", "COPPER_ORE", "ALUMINUM_ORE", "GOLD_ORE", "PLATINUM_ORE", 
            "SILVER_ORE", "URANIUM_ORE", "TITANIUM_ORE", "ZINC_ORE"
        ];
        
        // Check if we need precious materials  
        let precious_materials = [
            "PRECIOUS_STONES", "DIAMONDS", "RARE_EARTH_ELEMENTS"
        ];
        
        // Check if we need industrial minerals
        let industrial_minerals = [
            "QUARTZ_SAND", "SILICON_CRYSTALS", "CRYSTALLIZED_SULFUR", "SALT",
            "GRAPHITE", "LIMESTONE", "CLAY"
        ];
        
        for material in needed_materials {
            if metal_ores.iter().any(|&ore| material.contains(ore)) {
                return "COMMON_METAL_DEPOSITS";
            }
            if precious_materials.iter().any(|&precious| material.contains(precious)) {
                return "PRECIOUS_METAL_DEPOSITS"; // If this exists
            }
            if industrial_minerals.iter().any(|&mineral| material.contains(mineral)) {
                return "MINERAL_DEPOSITS";
            }
        }
        
        // Default to common metal deposits for unknown materials that might be ores
        "COMMON_METAL_DEPOSITS"
    }
    
    /// Calculate mining preference score (higher = better)
    fn calculate_mining_preference_score(waypoint: &Waypoint) -> i32 {
        let mut score = 0;
        
        // Prefer engineered asteroids (usually better yields)
        if waypoint.waypoint_type == "ENGINEERED_ASTEROID" {
            score += 100;
        }
        
        // Huge bonus for having a marketplace (can sell immediately)
        if waypoint.traits.iter().any(|t| t.symbol == "MARKETPLACE") {
            score += 1000;
        }
        
        // Bonus for fuel stations (can refuel on-site)  
        if waypoint.traits.iter().any(|t| t.symbol == "FUEL_STATION") {
            score += 200;
        }
        
        // Small penalty for dangerous traits
        if waypoint.traits.iter().any(|t| t.symbol == "EXPLOSIVE_GASES") {
            score -= 10;
        }
        
        score
    }

    async fn assign_exploration_task(&mut self, ship: &Ship) -> Result<(), Box<dyn std::error::Error>> {
        println!("🛰️ Assigning {} to explore for shipyards", ship.symbol);
        
        let system_symbol = ship.nav.waypoint_symbol.split('-').take(2).collect::<Vec<&str>>().join("-");
        let systems_to_explore = vec![system_symbol];
        
        let exploration_action = ShipAction::Explore {
            systems: systems_to_explore,
        };
        
        self.send_action_to_ship(&ship.symbol, exploration_action).await?;
        Ok(())
    }


    async fn assign_delivery_task(&mut self, ship: &Ship, contract: &Contract) -> Result<(), Box<dyn std::error::Error>> {
        // Find deliverable items
        for delivery in &contract.terms.deliver {
            if let Some(cargo_item) = ship.cargo.inventory.iter().find(|item| item.symbol == delivery.trade_symbol) {
                let units_to_deliver = std::cmp::min(cargo_item.units, delivery.units_required - delivery.units_fulfilled);
                
                if units_to_deliver > 0 {
                    println!("📦 Assigning {} to deliver {} x{} to {}", 
                            ship.symbol, delivery.trade_symbol, units_to_deliver, delivery.destination_symbol);
                    
                    // First navigate to destination if not there
                    if ship.nav.waypoint_symbol != delivery.destination_symbol {
                        let nav_action = ShipAction::Navigate {
                            destination: delivery.destination_symbol.clone(),
                        };
                        self.send_action_to_ship(&ship.symbol, nav_action).await?;
                    }
                    
                    // Then deliver cargo
                    let delivery_action = ShipAction::DeliverCargo {
                        contract_id: contract.id.clone(),
                        destination: delivery.destination_symbol.clone(),
                        trade_symbol: delivery.trade_symbol.clone(),
                        units: units_to_deliver,
                    };
                    
                    self.send_action_to_ship(&ship.symbol, delivery_action).await?;
                    break;
                }
            }
        }
        
        Ok(())
    }

    async fn send_action_to_ship(&mut self, ship_symbol: &str, action: ShipAction) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(sender) = self.ship_queues.get(ship_symbol) {
            // Mark ship state as stale since we're sending it an action
            let action_description = format!("{:?}", action);
            if let Err(e) = self.ship_cache.mark_ship_action(ship_symbol, &action_description) {
                println!("⚠️ Failed to mark ship as stale: {}", e);
            }
            
            sender.send(action)
                .map_err(|e| format!("Failed to send action to {}: {}", ship_symbol, e))?;
        } else {
            return Err(format!("No action queue for ship {}", ship_symbol).into());
        }
        Ok(())
    }

    async fn is_contract_complete(&self, contract: &Contract) -> Result<bool, Box<dyn std::error::Error>> {
        // Check if all deliveries are fulfilled
        let updated_contracts = self.client.get_contracts().await?;
        
        if let Some(updated_contract) = updated_contracts.iter().find(|c| c.id == contract.id) {
            return Ok(updated_contract.fulfilled);
        }
        
        Ok(false)
    }

    pub fn print_fleet_status(&self) {
        println!("\n📊 FLEET STATUS:");
        self.ship_cache.print_cache_status();
        
        for (ship_symbol, state) in &self.ship_states {
            if let Some(metrics) = self.fleet_metrics.iter().find(|m| m.ship_symbol == *ship_symbol) {
                let cache_info = if let Some(cached) = self.ship_cache.get_ship_state(ship_symbol) {
                    if cached.should_refresh(5) {
                        "🔄 STALE"
                    } else {
                        "✅ CACHED"
                    }
                } else {
                    "❌ NOT CACHED"
                };
                
                println!("  🚢 {} (Priority: {:.2}) [{}]: {:?} - Contract: {:.1}% - Income: {:.0}/hr", 
                        ship_symbol, 
                        metrics.priority_weight,
                        cache_info,
                        state.status,
                        metrics.contract_contribution * 100.0,
                        metrics.income_generation);
            } else {
                println!("  🚢 {}: {:?}", ship_symbol, state.status);
            }
        }
    }

    // Cargo and fuel management helper methods
    fn needs_refuel(&self, ship: &Ship) -> bool {
        // Probes don't need fuel management - they're designed for long-range exploration
        if self.is_probe(ship) {
            return false;
        }
        
        // Need refuel if less than 20% fuel or less than 400 units
        let fuel_percentage = ship.fuel.current as f64 / ship.fuel.capacity as f64;
        fuel_percentage < 0.2 || ship.fuel.current < 400
    }

    fn is_cargo_full(&self, ship: &Ship) -> bool {
        ship.cargo.units >= ship.cargo.capacity
    }

    async fn assign_refuel_task(&mut self, ship: &Ship) -> Result<(), Box<dyn std::error::Error>> {
        // Find nearest station with fuel
        let refuel_station = self.find_nearest_refuel_station(ship).await?;
        println!("⛽ {} assigned to refuel at {} (closest available)", ship.symbol, refuel_station);
        
        let action = ShipAction::Refuel {
            station: refuel_station
        };
        
        self.send_action_to_ship(&ship.symbol, action).await
    }

    async fn find_nearest_refuel_station(&mut self, ship: &Ship) -> Result<String, Box<dyn std::error::Error>> {
        // Use the unified fuel station finder
        let system_symbol = &ship.nav.system_symbol;
        let ship_x = ship.nav.route.destination.x;
        let ship_y = ship.nav.route.destination.y;
        
        match self.find_fuel_station(system_symbol, ship_x, ship_y, &ship.symbol).await? {
            Some(station) => Ok(station),
            None => {
                // Fallback to headquarters if no fuel stations found
                println!("⚠️ No fuel stations found in {}, using headquarters", system_symbol);
                Ok("X1-DC46-A1".to_string())
            }
        }
    }

    async fn assign_cargo_management(&mut self, ship: &Ship, contract: &Contract) -> Result<(), Box<dyn std::error::Error>> {
        println!("🗃️ {} cargo management - analyzing full cargo hold", ship.symbol);
        
        let contract_materials: Vec<String> = contract.terms.deliver
            .iter()
            .map(|d| d.trade_symbol.clone())
            .collect();

        // Categorize cargo
        let mut contract_items = Vec::new();
        let mut sellable_items = Vec::new();
        
        for item in &ship.cargo.inventory {
            if contract_materials.contains(&item.symbol) {
                contract_items.push(item);
                println!("   🎯 Contract: {} x{}", item.symbol, item.units);
            } else {
                sellable_items.push(item);
                println!("   💰 Sellable: {} x{}", item.symbol, item.units);
            }
        }

        // Strategy: Prioritize contract items, then sell/jettison non-contract items
        if !contract_items.is_empty() {
            // Deliver contract items first if we have enough or cargo is very full
            if self.should_deliver_cargo(ship, contract) {
                println!("📦 {} delivering contract materials first", ship.symbol);
                return self.assign_delivery_task(ship, contract).await;
            }
        }
        
        if !sellable_items.is_empty() {
            // Try to sell non-contract items to make room
            println!("💰 {} attempting to sell non-contract cargo", ship.symbol);
            self.assign_smart_sell_or_jettison(ship, &sellable_items, &contract_materials).await
        } else if !contract_items.is_empty() {
            // Only contract items - deliver them
            println!("📦 {} only has contract items - delivering", ship.symbol);
            self.assign_delivery_task(ship, contract).await
        } else {
            // Empty cargo (should not happen) - return to mining
            println!("⚠️ {} empty cargo - return to mining", ship.symbol);
            let needed_materials: Vec<String> = contract.terms.deliver
                .iter()
                .map(|d| d.trade_symbol.clone())
                .collect();
            self.assign_mining_task(ship, &needed_materials, &contract.id).await
        }
    }
    
    /// Smart sell or jettison: try to sell first, jettison if that fails
    async fn assign_smart_sell_or_jettison(&mut self, ship: &Ship, sellable_items: &[&crate::models::CargoItem], contract_materials: &[String]) -> Result<(), Box<dyn std::error::Error>> {
        // Find best marketplace that can actually buy our cargo
        match self.find_best_marketplace_for_cargo(ship, sellable_items).await {
            Ok(marketplace) => {
                println!("🏪 {} will sell at {} (compatible market)", ship.symbol, marketplace);
                
                // Create smart sell action that includes jettison fallback
                let action = ShipAction::SmartSellOrJettison {
                    marketplace,
                    contract_materials: contract_materials.to_vec(),
                };
                
                self.send_action_to_ship(&ship.symbol, action).await
            }
            Err(e) => {
                println!("⚠️ {} no marketplace found ({}), will jettison directly", ship.symbol, e);
                
                // No marketplace available - jettison directly
                let action = ShipAction::JettisonCargo {
                    contract_materials: contract_materials.to_vec(),
                };
                
                self.send_action_to_ship(&ship.symbol, action).await
            }
        }
    }

    async fn assign_sell_task(&mut self, ship: &Ship) -> Result<(), Box<dyn std::error::Error>> {
        // Find nearest marketplace to sell cargo
        let marketplace = self.find_nearest_marketplace(ship).await?;
        println!("💰 {} assigned to sell cargo at {} (nearest marketplace)", ship.symbol, marketplace);

        // Create sell action for non-contract items
        let action = ShipAction::SellCargo {
            marketplace,
        };
        
        self.send_action_to_ship(&ship.symbol, action).await
    }

    async fn find_nearest_marketplace(&mut self, ship: &Ship) -> Result<String, Box<dyn std::error::Error>> {
        // Use the unified marketplace finder
        let system_symbol = &ship.nav.system_symbol;
        let ship_x = ship.nav.route.destination.x;
        let ship_y = ship.nav.route.destination.y;
        
        match self.find_marketplace(system_symbol, ship_x, ship_y, &ship.symbol).await? {
            Some(marketplace) => Ok(marketplace),
            None => {
                // Fallback to headquarters if no marketplaces found
                println!("⚠️ No marketplaces found in {}, using headquarters", system_symbol);
                Ok("X1-DC46-A1".to_string())
            }
        }
    }

    // Helper method to get system waypoints - checks cache first
    async fn get_system_waypoints_cached(&mut self, system_symbol: &str) -> Result<&Vec<Waypoint>, Box<dyn std::error::Error>> {
        // Check if we need to scan the system
        if self.survey_cache.should_scan_system(system_symbol) {
            println!("📡 Scanning system {} (not in cache or stale)", system_symbol);
            let waypoints = self.client.get_system_waypoints(system_symbol, None).await?;
            self.survey_cache.cache_system_waypoints(system_symbol, waypoints)?;
        }
        
        // Return cached waypoints (guaranteed to exist after the above check)
        Ok(self.survey_cache.get_cached_waypoints(system_symbol).unwrap())
    }

    // Helper methods using cached data
    pub fn find_nearest_fuel_station_cached(&mut self, system_symbol: &str, from_x: i32, from_y: i32) -> Option<String> {
        if let Some(waypoint) = self.survey_cache.find_nearest_waypoint_with_trait(
            system_symbol, "FUEL_STATION", from_x, from_y
        ) {
            Some(waypoint.symbol.clone())
        } else if let Some(waypoint) = self.survey_cache.find_nearest_waypoint_with_trait(
            system_symbol, "MARKETPLACE", from_x, from_y
        ) {
            Some(waypoint.symbol.clone())
        } else {
            None
        }
    }

    /// UNIFIED WAYPOINT FINDER - Single source of truth for finding waypoints with specific traits
    /// Cache-first approach: checks cache -> scans if needed -> returns nearest matching waypoint
    pub async fn find_waypoint_with_trait(&mut self, system_symbol: &str, from_x: i32, from_y: i32, trait_symbols: &[&str], scanning_ship_symbol: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        // 1. Check cache first (if not stale)
        if !self.survey_cache.should_scan_system(system_symbol) {
            if let Some(cached_waypoints) = self.survey_cache.get_cached_waypoints(system_symbol) {
                // Check if cached data has the requested traits
                let has_requested_traits = cached_waypoints.iter().any(|w| 
                    w.traits.iter().any(|t| trait_symbols.contains(&t.symbol.as_str()))
                );
                
                if has_requested_traits {
                    // Use cached data to find nearest waypoint with requested trait
                    for trait_symbol in trait_symbols {
                        if let Some(waypoint) = self.survey_cache.find_nearest_waypoint_with_trait(
                            system_symbol, trait_symbol, from_x, from_y
                        ) {
                            println!("📋 Found {} in cache: {}", trait_symbol, waypoint.symbol);
                            return Ok(Some(waypoint.symbol.clone()));
                        }
                    }
                }
            }
        }
        
        // 2. Cache miss or stale - scan for fresh data
        println!("🔍 Cache miss for {}, scanning for waypoints with traits: {:?}", system_symbol, trait_symbols);
        let waypoints = self.scan_and_cache_waypoints(system_symbol, scanning_ship_symbol).await?;
        
        // 3. Find nearest waypoint with requested traits from fresh data
        let mut nearest_waypoint: Option<&crate::models::Waypoint> = None;
        let mut min_distance = f64::MAX;
        
        for waypoint in &waypoints {
            if waypoint.traits.iter().any(|trait_| 
                trait_symbols.contains(&trait_.symbol.as_str())
            ) {
                let dx = (waypoint.x - from_x) as f64;
                let dy = (waypoint.y - from_y) as f64;
                let distance = (dx * dx + dy * dy).sqrt();
                
                if distance < min_distance {
                    min_distance = distance;
                    nearest_waypoint = Some(waypoint);
                }
            }
        }
        
        if let Some(waypoint) = nearest_waypoint {
            let trait_names: Vec<_> = waypoint.traits.iter()
                .filter(|t| trait_symbols.contains(&t.symbol.as_str()))
                .map(|t| &t.symbol)
                .collect();
            println!("🏪 Found nearest waypoint with traits {:?}: {} (distance: {:.1})", 
                    trait_names, waypoint.symbol, min_distance);
            Ok(Some(waypoint.symbol.clone()))
        } else {
            println!("❌ No waypoints with traits {:?} found in {}", trait_symbols, system_symbol);
            Ok(None)
        }
    }
    
    /// Convenience method for finding marketplaces
    pub async fn find_marketplace(&mut self, system_symbol: &str, from_x: i32, from_y: i32, scanning_ship_symbol: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        self.find_waypoint_with_trait(system_symbol, from_x, from_y, &["MARKETPLACE", "SHIPYARD"], scanning_ship_symbol).await
    }
    
    /// Convenience method for finding fuel stations
    pub async fn find_fuel_station(&mut self, system_symbol: &str, from_x: i32, from_y: i32, scanning_ship_symbol: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        self.find_waypoint_with_trait(system_symbol, from_x, from_y, &["MARKETPLACE"], scanning_ship_symbol).await
    }
    
    /// Scan waypoints for detailed trait data and cache the results
    /// This ensures we get full trait information including MARKETPLACE
    async fn scan_and_cache_waypoints(&mut self, system_symbol: &str, scanning_ship: &str) -> Result<Vec<crate::models::Waypoint>, Box<dyn std::error::Error>> {
        println!("🔬 {} scanning waypoints in {} for detailed trait data", scanning_ship, system_symbol);
        
        // Use the scanning API to get full waypoint data with traits
        let scanned_waypoints = self.client.scan_waypoints(scanning_ship).await?;
        
        // Convert scanned waypoints to regular waypoints for caching
        let waypoints: Vec<crate::models::Waypoint> = scanned_waypoints.into_iter().map(|sw| {
            crate::models::Waypoint {
                symbol: sw.symbol,
                waypoint_type: sw.waypoint_type,
                system_symbol: sw.system_symbol,
                x: sw.x,
                y: sw.y,
                orbitals: sw.orbitals,
                traits: sw.traits,
                chart: sw.chart,
                faction: sw.faction,
            }
        }).collect();
        
        // Validate scanned data quality
        let waypoints_with_traits = waypoints.iter().filter(|w| !w.traits.is_empty()).count();
        println!("📊 Scanned {} waypoints, {} have trait data", waypoints.len(), waypoints_with_traits);
        
        // Cache the waypoint data
        self.survey_cache.cache_system_waypoints(system_symbol, waypoints.clone())?;
        
        let marketplace_count = waypoints.iter().filter(|w| 
            w.traits.iter().any(|t| t.symbol == "MARKETPLACE")
        ).count();
        
        println!("📡 Scanned {} waypoints in {}, found {} with MARKETPLACE trait", 
                waypoints.len(), system_symbol, marketplace_count);
        
        Ok(waypoints)
    }
    
    /// Find the best marketplace that can actually buy the ship's cargo
    pub async fn find_best_marketplace_for_cargo(&mut self, ship: &Ship, sellable_items: &[&crate::models::CargoItem]) -> Result<String, Box<dyn std::error::Error>> {
        if sellable_items.is_empty() {
            return Err("No sellable items to find market for".into());
        }
        
        // Get all marketplaces in the current system using cached waypoints
        let system = ship.nav.system_symbol.clone();
        let waypoints = self.get_system_waypoints_cached(&system).await?.clone();
        
        let marketplaces: Vec<_> = waypoints.iter()
            .filter(|w| w.traits.iter().any(|t| t.symbol == "MARKETPLACE"))
            .collect();
        
        if marketplaces.is_empty() {
            return Err("No marketplaces found in system".into());
        }
        
        // Get cargo symbols we want to sell
        let cargo_symbols: Vec<&str> = sellable_items.iter()
            .map(|item| item.symbol.as_str())
            .collect();
        
        println!("🔍 {} checking {} marketplaces for compatibility with cargo: {:?}", 
                ship.symbol, marketplaces.len(), cargo_symbols);
        
        let mut best_market = None;
        let mut best_compatibility_score = 0;
        
        // Check each marketplace
        for marketplace_waypoint in marketplaces {
            match self.client.get_market(&system, &marketplace_waypoint.symbol).await {
                Ok(market) => {
                    // Count how many of our items this market can buy
                    let mut compatibility_score = 0;
                    let mut compatible_items = Vec::new();
                    
                    for cargo_item in &cargo_symbols {
                        let can_buy = market.imports.iter().any(|import| import.symbol == *cargo_item) ||
                                     market.exchange.iter().any(|exchange| exchange.symbol == *cargo_item);
                        
                        if can_buy {
                            compatibility_score += 1;
                            compatible_items.push(*cargo_item);
                        }
                    }
                    
                    println!("   📊 {}: can buy {}/{} items {:?}", 
                            marketplace_waypoint.symbol, 
                            compatibility_score, 
                            cargo_symbols.len(),
                            compatible_items);
                    
                    if compatibility_score > best_compatibility_score {
                        best_compatibility_score = compatibility_score;
                        best_market = Some(marketplace_waypoint.symbol.clone());
                    }
                }
                Err(e) => {
                    println!("   ⚠️ Failed to get market data for {}: {}", marketplace_waypoint.symbol, e);
                }
            }
        }
        
        if let Some(market) = best_market {
            println!("✅ {} found best market: {} (compatibility: {}/{})", 
                    ship.symbol, market, best_compatibility_score, cargo_symbols.len());
            Ok(market)
        } else {
            Err(format!("No compatible marketplaces found for cargo: {:?}", cargo_symbols).into())
        }
    }
    
    /// Discover and integrate any new ships that aren't currently managed
    async fn discover_new_ships(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Only check periodically to avoid excessive API calls
        use std::sync::Mutex;
        use std::time::{Instant, Duration};
        
        static LAST_DISCOVERY_CHECK: Mutex<Option<Instant>> = Mutex::new(None);
        const CHECK_INTERVAL: Duration = Duration::from_secs(60); // Check every minute
        
        {
            let mut last_check = LAST_DISCOVERY_CHECK.lock().unwrap();
            if let Some(last_time) = *last_check {
                if last_time.elapsed() < CHECK_INTERVAL {
                    return Ok(()); // Too soon since last check
                }
            }
            *last_check = Some(Instant::now());
        }
        
        // Get current ships from API
        let current_ships = match self.client.get_ships().await {
            Ok(ships) => ships,
            Err(e) => {
                println!("⚠️ Failed to check for new ships: {}", e);
                return Ok(()); // Don't fail the main loop over this
            }
        };
        
        // Find ships that aren't in our management system
        let mut new_ships = Vec::new();
        for ship in current_ships {
            if !self.ship_queues.contains_key(&ship.symbol) {
                new_ships.push(ship);
            }
        }
        
        if !new_ships.is_empty() {
            println!("🔍 Discovered {} new ships not in fleet management:", new_ships.len());
            for ship in new_ships {
                println!("   🚢 Adding {} ({}) to active fleet", ship.symbol, ship.registration.role);
                
                if let Err(e) = self.spawn_ship_actor(ship.clone()).await {
                    println!("   ⚠️ Failed to spawn actor for {}: {}", ship.symbol, e);
                } else {
                    // Cache the ship state
                    if let Err(e) = self.ship_cache.cache_ship(ship.clone()) {
                        println!("   ⚠️ Failed to cache ship {}: {}", ship.symbol, e);
                    }
                    println!("   ✅ {} now under fleet management", ship.symbol);
                }
            }
        }
        
        Ok(())
    }
    
    /// Check if we should purchase additional ships for fleet expansion
    async fn check_ship_expansion(&mut self, contract: &Contract) -> Result<(), Box<dyn std::error::Error>> {
        // Only check for expansion periodically to avoid spam
        use std::sync::Mutex;
        use std::time::{Instant, Duration};
        
        static LAST_CHECK: Mutex<Option<Instant>> = Mutex::new(None);
        
        {
            let mut last_check = LAST_CHECK.lock().unwrap();
            if let Some(last_time) = *last_check {
                if last_time.elapsed() < Duration::from_secs(300) { // Check every 5 minutes
                    return Ok(());
                }
            }
            *last_check = Some(Instant::now());
        }
        
        println!("🏗️ Checking fleet expansion opportunities...");
        
        // Get current agent info to check credits
        let agent = self.client.get_agent().await?;
        let current_ships = self.client.get_ships().await?;
        
        // Find our mining ships
        let mining_ships: Vec<_> = current_ships.iter()
            .filter(|s| s.mounts.iter().any(|m| m.symbol.contains("MINING")))
            .collect();
        
        println!("📊 Current fleet: {} ships ({} miners)", current_ships.len(), mining_ships.len());
        println!("💰 Available credits: {}", agent.credits);
        
        // Check if we should buy another mining ship
        let should_expand = self.should_expand_fleet(&agent, &current_ships, contract).await;
        
        if should_expand && agent.credits >= 150000 { // Minimum for a decent mining ship
            println!("🎯 Fleet expansion recommended - searching for shipyards...");
            
            // Try to find and purchase a ship
            match self.attempt_ship_purchase(&agent, &mining_ships).await {
                Ok(new_ship) => {
                    println!("🎉 Successfully purchased new ship: {}", new_ship.symbol);
                    println!("   🚢 Type: {} Frame: {}", new_ship.registration.role, new_ship.frame.symbol);
                    println!("   ⛏️ Ready for mining operations!");
                    
                    // CRITICAL: Add the new ship to the fleet by spawning its actor
                    if let Err(e) = self.spawn_ship_actor(new_ship.clone()).await {
                        println!("⚠️ Failed to add new ship to fleet: {}", e);
                        println!("   💡 Ship purchased but won't be active until next restart");
                    } else {
                        println!("✅ New ship {} added to active fleet management", new_ship.symbol);
                        // Cache the new ship state
                        if let Err(e) = self.ship_cache.cache_ship(new_ship) {
                            println!("⚠️ Failed to cache new ship state: {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("⚠️ Ship purchase failed: {}", e);
                }
            }
        } else if should_expand {
            let needed = 150000 - agent.credits;
            println!("💸 Want to expand fleet but need {} more credits", needed);
        }
        
        Ok(())
    }
    
    /// Determine if we should expand the fleet
    async fn should_expand_fleet(&self, agent: &crate::models::Agent, ships: &[crate::models::Ship], contract: &Contract) -> bool {
        // Expansion criteria:
        // 1. Have enough credits (150k+)
        // 2. Contract is large enough to benefit from more ships
        // 3. Don't have too many ships already
        
        let mining_ships = ships.iter().filter(|s| s.mounts.iter().any(|m| m.symbol.contains("MINING"))).count();
        
        // Calculate contract workload
        let total_contract_units: i32 = contract.terms.deliver.iter().map(|d| d.units_required).sum();
        let contract_value = contract.terms.payment.on_fulfilled;
        
        // Expansion logic (optimized for current situation)
        let has_credits = agent.credits >= 150000; // Lower threshold to enable expansion sooner
        let contract_is_large = total_contract_units >= 30 || contract_value >= 10000; // More accessible thresholds
        let not_too_many_ships = mining_ships < 4; // Cap at 4 mining ships for now
        let profitable = contract_value > 10000; // Lower profit threshold for smaller contracts
        
        if has_credits && contract_is_large && not_too_many_ships && profitable {
            println!("✅ Fleet expansion criteria met:");
            println!("   💰 Credits: {} >= 150,000", agent.credits);
            println!("   📦 Contract size: {} units", total_contract_units);
            println!("   💎 Contract value: {} credits", contract_value);
            println!("   🚢 Current miners: {}/4", mining_ships);
            true
        } else {
            println!("❌ Fleet expansion not recommended:");
            if !has_credits { println!("   💸 Need more credits ({} < 150,000)", agent.credits); }
            if !contract_is_large { println!("   📦 Contract too small ({} units, {} value)", total_contract_units, contract_value); }
            if !not_too_many_ships { println!("   🚢 Already have enough miners ({})", mining_ships); }
            if !profitable { println!("   💎 Contract not profitable enough ({} < 15,000)", contract_value); }
            false
        }
    }
    
    /// Attempt to purchase a mining ship
    async fn attempt_ship_purchase(&self, _agent: &crate::models::Agent, reference_ships: &[&crate::models::Ship]) -> Result<crate::models::Ship, Box<dyn std::error::Error>> {
        // Use our shipyard operations system
        let shipyard_ops = crate::operations::ShipyardOperations::new(self.client.clone());
        
        // Find shipyards
        let shipyards = shipyard_ops.find_shipyards().await?;
        
        if shipyards.is_empty() {
            return Err("No shipyards found - need to explore more systems".into());
        }
        
        // Get reference mining ship configuration
        let reference_ship = reference_ships.first()
            .ok_or("No reference mining ship available")?;
        
        // Try each shipyard until we find one with suitable ships
        for shipyard in shipyards {
            println!("🏭 Checking shipyard at {}", shipyard.waypoint_symbol);
            
            match shipyard_ops.purchase_mining_ship(&shipyard, reference_ship).await {
                Ok(new_ship) => {
                    // Attempt to outfit the ship (may not have all required APIs yet)
                    if let Err(e) = shipyard_ops.outfit_mining_ship(&new_ship, reference_ship).await {
                        println!("⚠️ Ship outfitting incomplete: {}", e);
                        println!("   💡 Ship can still be used for basic mining");
                    }
                    
                    return Ok(new_ship);
                }
                Err(e) => {
                    println!("   ❌ Purchase failed at {}: {}", shipyard.waypoint_symbol, e);
                }
            }
        }
        
        Err("No suitable ships found at any available shipyard".into())
    }
    
}