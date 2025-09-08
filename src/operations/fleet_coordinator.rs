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
                        if self.needs_refuel(&ship) {
                            println!("⛽ {} needs fuel ({}/{})", ship_symbol, ship.fuel.current, ship.fuel.capacity);
                            self.assign_refuel_task(&ship).await?;
                        } else if self.has_contract_materials(&ship, &needed_materials) {
                            println!("📦 {} has contract materials - assigning delivery", ship_symbol);
                            self.assign_delivery_task(&ship, contract).await?;
                        } else if self.is_cargo_full(&ship) {
                            println!("🗃️ {} cargo full - need to manage inventory", ship_symbol);
                            self.assign_cargo_management(&ship, contract).await?;
                        } else if metrics.capabilities.can_mine && metrics.contract_contribution >= 0.05 && !self.is_probe(&ship) {
                            println!("⛏️ {} assigned to mining (priority: {:.2})", ship_symbol, metrics.priority_weight);
                            self.assign_mining_task(&ship, &needed_materials, &contract.id).await?;
                        } else if metrics.capabilities.can_explore {
                            println!("🛰️ {} assigned to exploration", ship_symbol);
                            self.assign_exploration_task(&ship).await?;
                        } else if metrics.capabilities.can_trade {
                            println!("🏪 {} assigned to support operations (trading ready)", ship_symbol);
                        }
                    }
                }
            }
        }
        
        Ok(())
    }

    fn can_mine(&self, ship: &Ship) -> bool {
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

    async fn assign_mining_task(&mut self, ship: &Ship, needed_materials: &[String], contract_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Find suitable mining location using cached waypoints
        let system_symbol = ship.nav.waypoint_symbol.split('-').take(2).collect::<Vec<&str>>().join("-");
        let waypoints = self.get_system_waypoints_cached(&system_symbol).await?;
        
        // Find best asteroid field
        let asteroid_fields: Vec<_> = waypoints.into_iter()
            .filter(|w| w.waypoint_type == "ASTEROID" || w.waypoint_type == "ENGINEERED_ASTEROID")
            .collect();
        
        if let Some(target) = asteroid_fields.first() {
            println!("⛏️ Assigning {} to mine at {}", ship.symbol, target.symbol);
            
            let mining_action = ShipAction::Mine {
                target: target.symbol.clone(),
                needed_materials: needed_materials.to_vec(),
                contract_id: contract_id.to_string(),
            };
            
            self.send_action_to_ship(&ship.symbol, mining_action).await?;
        } else {
            println!("⚠️ No mining locations found for {}", ship.symbol);
        }
        
        Ok(())
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
        // Determine what to do with cargo
        let contract_materials: Vec<String> = contract.terms.deliver
            .iter()
            .map(|d| d.trade_symbol.clone())
            .collect();

        // Check what cargo we have
        let mut has_contract_items = false;
        let mut has_sellable_items = false;

        for item in &ship.cargo.inventory {
            if contract_materials.contains(&item.symbol) {
                has_contract_items = true;
            } else {
                has_sellable_items = true;
            }
        }

        if has_contract_items {
            // Deliver contract items first
            println!("📦 {} delivering contract materials", ship.symbol);
            self.assign_delivery_task(ship, contract).await
        } else if has_sellable_items {
            // Sell non-contract items
            println!("💰 {} selling non-contract cargo", ship.symbol);
            self.assign_sell_task(ship).await
        } else {
            // Should not happen, but fallback to mining
            println!("⚠️ {} cargo management fallback - return to mining", ship.symbol);
            let needed_materials: Vec<String> = contract.terms.deliver
                .iter()
                .map(|d| d.trade_symbol.clone())
                .collect();
            self.assign_mining_task(ship, &needed_materials, &contract.id).await
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
        self.find_waypoint_with_trait(system_symbol, from_x, from_y, &["FUEL_STATION", "MARKETPLACE", "SHIPYARD"], scanning_ship_symbol).await
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
    
}