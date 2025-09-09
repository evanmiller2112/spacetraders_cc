// Ship Actor System - Per-ship asynchronous action queues
use crate::client::SpaceTradersClient;
use crate::models::*;
use crate::operations::NavigationPlanner;
use crate::storage::CooldownStore;
use tokio::sync::mpsc;
use tokio::time::{Duration, Instant, sleep};
use chrono;
#[derive(Debug)]
pub struct ShipActorError(pub String);

impl std::fmt::Display for ShipActorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ShipActorError {}

impl From<Box<dyn std::error::Error>> for ShipActorError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        ShipActorError(err.to_string())
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for ShipActorError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        ShipActorError(err.to_string())
    }
}

unsafe impl Send for ShipActorError {}
unsafe impl Sync for ShipActorError {}

#[derive(Debug, Clone)]
pub enum ShipAction {
    Mine { 
        target: String,
        needed_materials: Vec<String>,
        contract_id: String,
    },
    Navigate { 
        destination: String 
    },
    Explore { 
        systems: Vec<String> 
    },
    Trade { 
        items: Vec<String>,
        marketplace: String,
    },
    Refuel { 
        station: String 
    },
    SellCargo { 
        marketplace: String 
    },
    Dock,
    Orbit,
    Survey { target: String },
    DeliverCargo {
        contract_id: String,
        destination: String,
        trade_symbol: String,
        units: i32,
    },
    SmartSellOrJettison {
        marketplace: String,
        contract_materials: Vec<String>,
    },
    JettisonCargo {
        contract_materials: Vec<String>,
    },
}

#[derive(Debug, Clone)]
pub struct ShipState {
    pub ship: Ship,
    pub cooldown_until: Option<Instant>,
    pub current_action: Option<ShipAction>,
    pub current_plan: Option<crate::operations::task_planner::TaskPlan>,
    pub status: ShipActorStatus,
}

#[derive(Debug, Clone)]
pub enum ShipActorStatus {
    Idle,
    Working,
    OnCooldown,
    Navigating,
    Error(String),
}

pub struct ShipActor {
    ship_symbol: String,
    action_receiver: mpsc::UnboundedReceiver<ShipAction>,
    status_sender: mpsc::UnboundedSender<(String, ShipState)>,
    client: SpaceTradersClient,
    navigation_planner: NavigationPlanner,
    cooldown_until: Option<Instant>,
    cooldown_store: CooldownStore,
}

impl ShipActor {
    pub fn new(
        ship_symbol: String,
        action_receiver: mpsc::UnboundedReceiver<ShipAction>,
        status_sender: mpsc::UnboundedSender<(String, ShipState)>,
        client: SpaceTradersClient,
    ) -> Self {        
        let storage_path = format!("storage/cooldowns_{}.json", ship_symbol);
        let cooldown_store = CooldownStore::new(&storage_path);
        let navigation_planner = NavigationPlanner::new(client.clone());
        
        Self {
            ship_symbol,
            action_receiver,
            status_sender,
            client,
            navigation_planner,
            cooldown_until: None,
            cooldown_store,
        }
    }

    pub async fn run(&mut self) {
        println!("🤖 {} actor started", self.ship_symbol);
        
        // Check for persisted cooldowns
        if let Some(remaining) = self.cooldown_store.get_remaining_cooldown(&self.ship_symbol) {
            self.cooldown_until = Some(Instant::now() + Duration::from_secs_f64(remaining));
            println!("💾 {} restored cooldown: {:.1}s remaining", self.ship_symbol, remaining);
        }
        
        self.cooldown_store.print_status();
        
        loop {
            // Check if we're still on cooldown
            if let Some(cooldown_end) = self.cooldown_until {
                if Instant::now() < cooldown_end {
                    let remaining = cooldown_end.duration_since(Instant::now());
                    println!("⏳ {} on cooldown for {:.1}s", self.ship_symbol, remaining.as_secs_f64());
                    
                    // Update status to on cooldown
                    self.send_status(ShipActorStatus::OnCooldown).await;
                    
                    // Wait for cooldown or new action (whichever comes first)
                    tokio::select! {
                        _ = sleep(remaining) => {
                            self.cooldown_until = None;
                            println!("✅ {} cooldown complete", self.ship_symbol);
                            
                            // Clear persisted cooldown
                            if let Err(e) = self.cooldown_store.clear_cooldown(&self.ship_symbol) {
                                println!("⚠️ Failed to clear cooldown for {}: {}", self.ship_symbol, e);
                            }
                            
                            self.send_status(ShipActorStatus::Idle).await;
                        }
                        action = self.action_receiver.recv() => {
                            if let Some(action) = action {
                                // Queue action for after cooldown
                                println!("📥 {} queued action during cooldown: {:?}", self.ship_symbol, action);
                                continue;
                            } else {
                                break; // Channel closed
                            }
                        }
                    }
                    continue;
                }
            }

            // Wait for next action
            match self.action_receiver.recv().await {
                Some(action) => {
                    println!("🚀 {} executing: {:?}", self.ship_symbol, action);
                    self.send_status(ShipActorStatus::Working).await;
                    self.execute_action(action).await;
                }
                None => {
                    println!("🛑 {} actor stopping - channel closed", self.ship_symbol);
                    break;
                }
            }
        }
    }

    async fn execute_action(&mut self, action: ShipAction) {
        let result = match &action {
            ShipAction::Mine { target, needed_materials, contract_id: _ } => {
                self.execute_mining(target, needed_materials).await
            }
            ShipAction::Navigate { destination } => {
                self.execute_navigation(destination).await
            }
            ShipAction::Explore { systems } => {
                self.execute_exploration(systems).await
            }
            ShipAction::Trade { items: _, marketplace: _ } => {
                // TODO: Implement trading
                println!("🏪 {} trading not yet implemented", self.ship_symbol);
                Ok(())
            }
            ShipAction::Survey { target } => {
                self.execute_survey(target).await
            }
            ShipAction::Refuel { station } => {
                self.execute_refuel_at_station(station).await
            }
            ShipAction::SellCargo { marketplace } => {
                self.execute_sell_cargo(marketplace).await
            }
            ShipAction::Dock => {
                self.execute_dock().await
            }
            ShipAction::Orbit => {
                self.execute_orbit().await
            }
            ShipAction::DeliverCargo { contract_id, destination, trade_symbol, units } => {
                self.execute_cargo_delivery(contract_id, destination, trade_symbol, *units).await
            }
            ShipAction::SmartSellOrJettison { marketplace, contract_materials } => {
                self.execute_smart_sell_or_jettison(marketplace, contract_materials).await
            }
            ShipAction::JettisonCargo { contract_materials } => {
                self.execute_jettison_cargo(contract_materials).await
            }
        };

        let status = match result {
            Ok(()) => {
                println!("✅ {} completed: {:?}", self.ship_symbol, action);
                
                // Pretty-print current ship status after action
                self.print_ship_status().await;
                
                ShipActorStatus::Idle
            }
            Err(e) => {
                let error_message = e.to_string();
                println!("❌ {} failed: {:?} - Error: {}", self.ship_symbol, action, error_message);
                
                // Try to extract cooldown from error
                if let Some(cooldown_seconds) = self.extract_cooldown_from_error(&error_message) {
                    println!("⏳ {} detected cooldown: {:.1}s", self.ship_symbol, cooldown_seconds);
                    self.cooldown_until = Some(Instant::now() + Duration::from_secs_f64(cooldown_seconds));
                    
                    // Persist cooldown
                    if let Err(e) = self.cooldown_store.set_cooldown(&self.ship_symbol, cooldown_seconds) {
                        println!("⚠️ Failed to save cooldown for {}: {}", self.ship_symbol, e);
                    }
                }
                
                ShipActorStatus::Error(error_message)
            }
        };
        
        self.send_status(status).await;
    }

    async fn execute_mining(&mut self, target: &str, needed_materials: &[String]) -> Result<(), ShipActorError> {
        // First, check if we're at the mining location
        let ship = match self.client.get_ship(&self.ship_symbol).await {
            Ok(ship) => ship,
            Err(e) => return Err(ShipActorError(format!("Could not get ship data: {}", e)))
        };
        
        // Navigate to target if we're not already there
        if ship.nav.waypoint_symbol != target {
            println!("🧭 {} navigating to mining target {}", self.ship_symbol, target);
            if let Err(e) = self.execute_navigation(target).await {
                return Err(ShipActorError(format!("Failed to navigate to mining location {}: {}", target, e)));
            }
            
            // Wait for arrival if in transit
            loop {
                let current_ship = self.client.get_ship(&self.ship_symbol).await
                    .map_err(|e| ShipActorError(format!("Failed to check ship status: {}", e)))?;
                
                if current_ship.nav.status == "IN_TRANSIT" {
                    // Parse arrival time and calculate wait
                    if let Ok(arrival_time) = chrono::DateTime::parse_from_rfc3339(&current_ship.nav.route.arrival) {
                        let now = chrono::Utc::now();
                        let wait_seconds = (arrival_time.timestamp() - now.timestamp()).max(0);
                        if wait_seconds > 0 {
                            println!("⏳ {} in transit, arriving in {} seconds", self.ship_symbol, wait_seconds);
                            tokio::time::sleep(Duration::from_secs(wait_seconds.min(5) as u64)).await;
                            continue;
                        }
                    }
                }
                break;
            }
            
            // Need to orbit for mining
            match self.client.orbit_ship(&self.ship_symbol).await {
                Ok(_) => println!("🛸 {} in orbit for mining", self.ship_symbol),
                Err(e) => {
                    if !e.to_string().contains("already in orbit") {
                        println!("⚠️ {} orbit failed: {}", self.ship_symbol, e);
                    }
                }
            }
        } else {
            println!("✅ {} already at mining target {}", self.ship_symbol, target);
            
            // Ensure we're in orbit even if already at location
            let current_ship = self.client.get_ship(&self.ship_symbol).await
                .map_err(|e| ShipActorError(format!("Failed to check ship status: {}", e)))?;
            
            if current_ship.nav.status == "DOCKED" {
                match self.client.orbit_ship(&self.ship_symbol).await {
                    Ok(_) => println!("🛸 {} now in orbit for mining", self.ship_symbol),
                    Err(e) => {
                        if !e.to_string().contains("already in orbit") {
                            return Err(ShipActorError(format!("Failed to orbit for mining: {}", e)));
                        }
                    }
                }
            }
        }
        
        println!("⛏️ {} performing extraction at {}", self.ship_symbol, target);
        
        // Wait for any in-transit status before attempting extraction
        self.wait_for_arrival().await?;
        
        self.attempt_extraction_with_retry(needed_materials).await
    }

    async fn execute_exploration(&mut self, systems: &[String]) -> Result<(), ShipActorError> {
        println!("🛰️ {} exploring systems: {:?}", self.ship_symbol, systems);
        
        for system in systems {
            // Get system waypoints and look for shipyards
            let waypoints_result = self.client.get_system_waypoints(system, None).await;
            
            match waypoints_result {
                Ok(waypoints) => {
                    for waypoint in waypoints {
                        let has_shipyard = waypoint.traits.iter().any(|t| 
                            t.name.to_lowercase().contains("shipyard"));
                        
                        if has_shipyard {
                            println!("🚢 {} found shipyard at {}!", self.ship_symbol, waypoint.symbol);
                        }
                    }
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    println!("⚠️ {} failed to explore {}: {}", self.ship_symbol, system, error_msg);
                }
            }
        }
        
        Ok(())
    }

    async fn execute_survey(&mut self, target: &str) -> Result<(), ShipActorError> {
        // Ensure ship is in orbit before surveying
        let ship = self.client.get_ship(&self.ship_symbol).await
            .map_err(|e| ShipActorError(format!("Failed to get ship status: {}", e)))?;
            
        if ship.nav.status == "DOCKED" {
            println!("🛸 {} needs to orbit before surveying", self.ship_symbol);
            match self.client.orbit_ship(&self.ship_symbol).await {
                Ok(_) => {
                    println!("🌌 {} now in orbit, ready to survey", self.ship_symbol);
                }
                Err(e) => {
                    if !e.to_string().contains("already in orbit") {
                        return Err(ShipActorError(format!("Failed to orbit for survey: {}", e)));
                    }
                }
            }
        }
        
        match self.client.create_survey(&self.ship_symbol).await {
            Ok(survey_data) => {
                println!("🔍 {} surveyed {} - found {} deposits", self.ship_symbol, target, survey_data.surveys.len());
                
                // Set cooldown
                if survey_data.cooldown.remaining_seconds > 0.0 {
                    self.cooldown_until = Some(Instant::now() + Duration::from_secs_f64(survey_data.cooldown.remaining_seconds));
                    
                    // Persist cooldown
                    if let Err(e) = self.cooldown_store.set_cooldown(&self.ship_symbol, survey_data.cooldown.remaining_seconds) {
                        println!("⚠️ Failed to save cooldown for {}: {}", self.ship_symbol, e);
                    }
                }
                
                Ok(())
            }
            Err(e) => {
                let error_msg = e.to_string();
                println!("❌ {} survey failed: {}", self.ship_symbol, error_msg);
                Err(ShipActorError(error_msg))
            }
        }
    }

    async fn _execute_refuel(&mut self) -> Result<(), ShipActorError> {
        match self.client.refuel_ship(&self.ship_symbol).await {
            Ok(refuel_data) => {
                println!("⛽ {} refueled - {}/{} fuel", self.ship_symbol, refuel_data.fuel.current, refuel_data.fuel.capacity);
                Ok(())
            }
            Err(e) => Err(ShipActorError(e.to_string()))
        }
    }

    async fn execute_dock(&mut self) -> Result<(), ShipActorError> {
        match self.client.dock_ship(&self.ship_symbol).await {
            Ok(_) => {
                println!("🛸 {} docked", self.ship_symbol);
                Ok(())
            }
            Err(e) => Err(ShipActorError(e.to_string()))
        }
    }

    async fn execute_orbit(&mut self) -> Result<(), ShipActorError> {
        match self.client.orbit_ship(&self.ship_symbol).await {
            Ok(_) => {
                println!("🌌 {} in orbit", self.ship_symbol);
                Ok(())
            }
            Err(e) => Err(ShipActorError(e.to_string()))
        }
    }

    async fn execute_cargo_delivery(&mut self, contract_id: &str, destination: &str, trade_symbol: &str, units: i32) -> Result<(), ShipActorError> {
        // First check if ship is in transit and wait for arrival
        let mut ship = self.client.get_ship(&self.ship_symbol).await
            .map_err(|e| ShipActorError(format!("Failed to get ship status: {}", e)))?;
        
        if ship.nav.status == "IN_TRANSIT" {
            println!("⏳ {} waiting for transit completion before cargo delivery", self.ship_symbol);
            
            // Wait for transit to complete
            loop {
                let current_ship = self.client.get_ship(&self.ship_symbol).await
                    .map_err(|e| ShipActorError(format!("Failed to check ship status: {}", e)))?;
                
                if current_ship.nav.status == "IN_TRANSIT" {
                    if let Ok(arrival_time) = chrono::DateTime::parse_from_rfc3339(&current_ship.nav.route.arrival) {
                        let now = chrono::Utc::now();
                        let wait_seconds = (arrival_time.timestamp() - now.timestamp()).max(0);
                        if wait_seconds > 0 {
                            println!("⏳ {} in transit, arriving in {} seconds", self.ship_symbol, wait_seconds);
                            tokio::time::sleep(Duration::from_secs(wait_seconds.min(5) as u64)).await;
                            continue;
                        }
                    }
                } else {
                    ship = current_ship;
                    break;
                }
            }
            println!("✅ {} arrived at destination", self.ship_symbol);
        }
        
        // Check if ship is at the correct destination for delivery
        if ship.nav.waypoint_symbol != destination {
            println!("🧭 {} not at delivery destination ({} -> {}), navigating...", 
                    self.ship_symbol, ship.nav.waypoint_symbol, destination);
            
            // Try to navigate to the destination
            match self.execute_navigation(destination).await {
                Ok(_) => {
                    println!("✅ {} arrived at delivery destination: {}", self.ship_symbol, destination);
                }
                Err(nav_error) => {
                    // Check if this is a fuel issue
                    if nav_error.0.contains("Insufficient fuel") {
                        println!("⛽ {} needs refuel before delivery navigation", self.ship_symbol);
                        
                        // Check fuel safety to get nearest fuel source suggestion
                        let fuel_station = match self.navigation_planner.can_navigate_safely(&ship, destination).await {
                            Ok(safety_check) => {
                                safety_check.nearest_fuel_source
                                    .ok_or_else(|| ShipActorError(format!("Insufficient fuel for delivery and no fuel station suggested")))?
                            }
                            Err(e) => {
                                return Err(ShipActorError(format!("Failed to get fuel suggestions: {}", e)));
                            }
                        };
                        
                        println!("⛽ {} refueling at {} (suggested by navigation planner)", self.ship_symbol, fuel_station);
                        self.execute_refuel_at_station(&fuel_station).await?;
                        
                        // Try navigation again after refueling
                        println!("🧭 {} retrying navigation to {} after refuel", self.ship_symbol, destination);
                        self.execute_navigation(destination).await?;
                        
                        println!("✅ {} arrived at delivery destination after refuel: {}", self.ship_symbol, destination);
                    } else {
                        // Re-throw non-fuel navigation errors
                        return Err(nav_error);
                    }
                }
            }
        } else {
            println!("📍 {} already at delivery destination: {}", self.ship_symbol, destination);
        }
        
        // Ensure ship is docked for cargo delivery
        match self.client.dock_ship(&self.ship_symbol).await {
            Ok(_) => {
                println!("🚢 {} docked for cargo delivery", self.ship_symbol);
            }
            Err(e) => {
                if !e.to_string().contains("already docked") {
                    return Err(ShipActorError(format!("Failed to dock for delivery: {}", e)));
                }
            }
        }
        
        match self.client.deliver_cargo(&self.ship_symbol, contract_id, trade_symbol, units).await {
            Ok(_) => {
                println!("📦 {} delivered {} x{} to {}", self.ship_symbol, trade_symbol, units, destination);
                Ok(())
            }
            Err(e) => Err(ShipActorError(format!("Contract delivery failed: {}", e)))
        }
    }

    async fn execute_navigation(&mut self, destination: &str) -> Result<(), ShipActorError> {
        // Get current ship data for fuel check
        let ship = match self.client.get_ship(&self.ship_symbol).await {
            Ok(ship) => ship,
            Err(e) => return Err(ShipActorError(format!("Could not get ship data: {}", e)))
        };
        
        // Check if ship needs to be in orbit before navigating
        if ship.nav.status == "DOCKED" {
            println!("🛸 {} needs to orbit before navigating", self.ship_symbol);
            match self.client.orbit_ship(&self.ship_symbol).await {
                Ok(_) => {
                    println!("🌌 {} now in orbit, ready to navigate", self.ship_symbol);
                }
                Err(e) => {
                    // Check if already in orbit
                    if !e.to_string().contains("already in orbit") {
                        return Err(ShipActorError(format!("Failed to orbit before navigation: {}", e)));
                    }
                }
            }
        }
        
        // Check fuel safety before navigation
        match self.navigation_planner.can_navigate_safely(&ship, destination).await {
            Ok(safety_check) => {
                if !safety_check.is_safe {
                    println!("⛽ {} navigation BLOCKED: {}", self.ship_symbol, safety_check.reason);
                    if let Some(fuel_source) = safety_check.nearest_fuel_source {
                        println!("💡 {} should refuel at {} first", self.ship_symbol, fuel_source);
                    }
                    return Err(ShipActorError(format!("Insufficient fuel: {}", safety_check.reason)));
                }
                
                println!("✅ {} fuel check passed: {}", self.ship_symbol, safety_check.reason);
            }
            Err(e) => {
                println!("⚠️ {} fuel safety check failed: {}, proceeding with caution", self.ship_symbol, e);
                // Continue but warn - this might be due to API issues
            }
        }
        
        // Proceed with navigation
        match self.client.navigate_ship(&self.ship_symbol, destination).await {
            Ok(_) => {
                println!("🧭 {} navigating to {}", self.ship_symbol, destination);
                Ok(())
            }
            Err(e) => Err(ShipActorError(e.to_string()))
        }
    }

    async fn send_status(&self, status: ShipActorStatus) {
        // Get current ship state - this is a simplified version
        // In a real implementation, we'd fetch the current ship data
        let ship_state = ShipState {
            ship: Ship {
                symbol: self.ship_symbol.clone(),
                registration: ShipRegistration {
                    name: self.ship_symbol.clone(),
                    faction_symbol: "UNKNOWN".to_string(),
                    role: "UNKNOWN".to_string(),
                },
                nav: ShipNav {
                    system_symbol: "UNKNOWN".to_string(),
                    waypoint_symbol: "UNKNOWN".to_string(),
                    route: ShipRoute {
                        destination: ShipRouteWaypoint {
                            symbol: "UNKNOWN".to_string(),
                            waypoint_type: "UNKNOWN".to_string(),
                            system_symbol: "UNKNOWN".to_string(),
                            x: 0,
                            y: 0,
                        },
                        origin: ShipRouteWaypoint {
                            symbol: "UNKNOWN".to_string(),
                            waypoint_type: "UNKNOWN".to_string(),
                            system_symbol: "UNKNOWN".to_string(),
                            x: 0,
                            y: 0,
                        },
                        departure_time: "UNKNOWN".to_string(),
                        arrival: "UNKNOWN".to_string(),
                    },
                    status: "UNKNOWN".to_string(),
                    flight_mode: "CRUISE".to_string(),
                },
                crew: ShipCrew {
                    current: 0,
                    required: 0,
                    capacity: 0,
                    rotation: "STRICT".to_string(),
                    morale: 100,
                    wages: 0,
                },
                frame: ShipFrame {
                    symbol: "UNKNOWN".to_string(),
                    name: "Unknown".to_string(),
                    description: "Unknown".to_string(),
                    condition: Some(1.0),
                    integrity: Some(1.0),
                    module_slots: 0,
                    mounting_points: 0,
                    fuel_capacity: 0,
                    requirements: ShipRequirements {
                        power: Some(0),
                        crew: Some(0),
                        slots: Some(0),
                    },
                },
                reactor: ShipModule {
                    symbol: "UNKNOWN".to_string(),
                    capacity: None,
                    range: None,
                    name: "Unknown".to_string(),
                    description: "Unknown".to_string(),
                    requirements: ShipRequirements {
                        power: Some(0),
                        crew: Some(0),
                        slots: Some(0),
                    },
                },
                engine: ShipModule {
                    symbol: "UNKNOWN".to_string(),
                    capacity: None,
                    range: None,
                    name: "Unknown".to_string(),
                    description: "Unknown".to_string(),
                    requirements: ShipRequirements {
                        power: Some(0),
                        crew: Some(0),
                        slots: Some(0),
                    },
                },
                cooldown: ShipCooldown {
                    ship_symbol: self.ship_symbol.clone(),
                    total_seconds: 0.0,
                    remaining_seconds: 0.0,
                    expiration: None,
                },
                modules: vec![],
                mounts: vec![],
                cargo: ShipCargo {
                    capacity: 0,
                    units: 0,
                    inventory: vec![],
                },
                fuel: ShipFuel {
                    current: 0,
                    capacity: 0,
                    consumed: None,
                },
            },
            cooldown_until: self.cooldown_until,
            current_action: None,
            current_plan: None,
            status,
        };
        
        if let Err(_) = self.status_sender.send((self.ship_symbol.clone(), ship_state)) {
            // Channel closed - coordinator is shutting down
        }
    }

    fn extract_cooldown_from_error(&self, error_str: &str) -> Option<f64> {
        // Look for pattern like "cooldown for 27 second(s)"
        if let Some(start) = error_str.find("cooldown for ") {
            let after_cooldown = &error_str[start + 13..]; // Skip "cooldown for "
            if let Some(end) = after_cooldown.find(" second") {
                let number_str = &after_cooldown[..end];
                if let Ok(seconds) = number_str.parse::<f64>() {
                    return Some(seconds);
                }
            }
        }
        None
    }
    
    async fn print_ship_status(&self) {
        match self.client.get_ship(&self.ship_symbol).await {
            Ok(ship) => {
                println!("🚢 ═══ {} STATUS ═══", self.ship_symbol);
                println!("📍 Location: {} ({})", ship.nav.waypoint_symbol, ship.nav.status);
                println!("⛽ Fuel: {}/{}", ship.fuel.current, ship.fuel.capacity);
                println!("📦 Cargo: {}/{}", ship.cargo.units, ship.cargo.capacity);
                
                if !ship.cargo.inventory.is_empty() {
                    println!("📋 Inventory:");
                    for item in &ship.cargo.inventory {
                        println!("   • {} x{}", item.symbol, item.units);
                    }
                } else {
                    println!("📋 Inventory: Empty");
                }
                
                if ship.cooldown.remaining_seconds > 0.0 {
                    println!("⏳ Cooldown: {:.1}s remaining", ship.cooldown.remaining_seconds);
                } else {
                    println!("✅ Cooldown: Ready");
                }
                
                println!("🔧 Mounts:");
                for mount in &ship.mounts {
                    println!("   • {} (Strength: {})", mount.symbol, mount.strength.unwrap_or(0));
                }
                println!("════════════════════════════════");
            }
            Err(e) => {
                println!("❌ Failed to get {} status: {}", self.ship_symbol, e);
            }
        }
    }

    async fn execute_refuel_at_station(&self, station: &str) -> Result<(), ShipActorError> {
        println!("⛽ {} attempting to refuel at {}", self.ship_symbol, station);
        
        // First navigate to the station if not there
        let current_ship = self.client.get_ship(&self.ship_symbol).await
            .map_err(|e| ShipActorError(format!("Failed to get ship status: {}", e)))?;
        
        if current_ship.nav.waypoint_symbol != station {
            println!("🚀 {} navigating to {} for refuel", self.ship_symbol, station);
            
            // Ensure ship is in orbit before navigating
            if current_ship.nav.status == "DOCKED" {
                println!("🛸 {} needs to orbit before navigating", self.ship_symbol);
                match self.client.orbit_ship(&self.ship_symbol).await {
                    Ok(_) => {
                        println!("🌌 {} now in orbit, ready to navigate", self.ship_symbol);
                    }
                    Err(e) => {
                        if !e.to_string().contains("already in orbit") {
                            return Err(ShipActorError(format!("Failed to orbit before navigation: {}", e)));
                        }
                    }
                }
            }
            
            // Navigate to station  
            self.client.navigate_ship(&self.ship_symbol, station).await
                .map_err(|e| ShipActorError(format!("Navigation to {} failed: {}", station, e)))?;
            
            println!("✅ {} navigation started to {}", self.ship_symbol, station);
            
            // Wait for transit to complete
            loop {
                let transit_ship = self.client.get_ship(&self.ship_symbol).await
                    .map_err(|e| ShipActorError(format!("Failed to check ship status: {}", e)))?;
                
                if transit_ship.nav.status == "IN_TRANSIT" {
                    if let Ok(arrival_time) = chrono::DateTime::parse_from_rfc3339(&transit_ship.nav.route.arrival) {
                        let now = chrono::Utc::now();
                        let wait_seconds = (arrival_time.timestamp() - now.timestamp()).max(0);
                        if wait_seconds > 0 {
                            println!("⏳ {} in transit, arriving in {} seconds", self.ship_symbol, wait_seconds);
                            tokio::time::sleep(Duration::from_secs(wait_seconds.min(5) as u64)).await;
                            continue;
                        }
                    }
                }
                break;
            }
            println!("✅ {} arrived at {}", self.ship_symbol, station);
        }
        
        // Dock at station
        match self.client.dock_ship(&self.ship_symbol).await {
            Ok(_) => {
                println!("🚢 {} docked at {}", self.ship_symbol, station);
            }
            Err(e) => {
                let error_str = e.to_string();
                // Handle various acceptable docking failures gracefully
                if error_str.contains("already docked") {
                    println!("🚢 {} already docked at {}", self.ship_symbol, station);
                } else if error_str.contains("429 Too Many Requests") {
                    // Extract retry delay from the rate limit error
                    let retry_after = if let Some(retry_match) = error_str.find("\"retryAfter\":") {
                        let start = retry_match + 13; // Length of "retryAfter":
                        let end = error_str[start..].find(',').unwrap_or(10) + start;
                        error_str[start..end].trim().parse::<f64>().unwrap_or(1.0)
                    } else {
                        1.0 // Default 1 second retry
                    };
                    println!("🕐 {} hit API rate limit - waiting {:.1}s before retry", self.ship_symbol, retry_after);
                    return Err(ShipActorError(format!("Rate limited - retry in {:.1}s", retry_after)));
                } else if error_str.contains("400 Bad Request") {
                    // Common 400 errors for docking - handle gracefully
                    if error_str.contains("not at waypoint") || error_str.contains("must be at") {
                        println!("⚠️  {} cannot dock - not at correct location for {}", self.ship_symbol, station);
                        return Err(ShipActorError(format!("Cannot refuel - ship not at station location: {}", station)));
                    } else if error_str.contains("cannot dock") || error_str.contains("docking not allowed") {
                        println!("⚠️  {} cannot dock at {} - docking not allowed", self.ship_symbol, station);
                        return Err(ShipActorError(format!("Docking not allowed at station: {}", station)));
                    } else {
                        println!("⚠️  {} docking failed at {} with 400 error: {}", self.ship_symbol, station, error_str);
                        return Err(ShipActorError(format!("Docking failed: {}", e)));
                    }
                } else {
                    println!("⚠️  {} unexpected docking error at {}: {}", self.ship_symbol, station, error_str);
                    return Err(ShipActorError(format!("Docking failed: {}", e)));
                }
            }
        }
        
        // Refuel
        match self.client.refuel_ship(&self.ship_symbol).await {
            Ok(refuel_data) => {
                println!("⛽ {} refueled: {} units for {} credits", 
                        self.ship_symbol, 
                        refuel_data.transaction.units.unwrap_or(0),
                        refuel_data.transaction.total_price);
                Ok(())
            }
            Err(e) => {
                Err(ShipActorError(format!("Refuel failed: {}", e)))
            }
        }
    }

    async fn execute_sell_cargo(&self, marketplace: &str) -> Result<(), ShipActorError> {
        println!("💰 {} attempting to sell cargo at {}", self.ship_symbol, marketplace);
        
        // Get current ship status
        let current_ship = self.client.get_ship(&self.ship_symbol).await
            .map_err(|e| ShipActorError(format!("Failed to get ship status: {}", e)))?;
        
        if current_ship.cargo.inventory.is_empty() {
            println!("📦 {} has no cargo to sell", self.ship_symbol);
            return Ok(());
        }
        
        // Navigate to marketplace if not there
        if current_ship.nav.waypoint_symbol != marketplace {
            println!("🚀 {} navigating to {} to sell cargo", self.ship_symbol, marketplace);
            
            match self.client.navigate_ship(&self.ship_symbol, marketplace).await {
                Ok(_) => {
                    println!("✅ {} arrived at {}", self.ship_symbol, marketplace);
                }
                Err(e) => {
                    return Err(ShipActorError(format!("Navigation to {} failed: {}", marketplace, e)));
                }
            }
        }
        
        // Dock at marketplace
        match self.client.dock_ship(&self.ship_symbol).await {
            Ok(_) => {
                println!("🚢 {} docked at {}", self.ship_symbol, marketplace);
            }
            Err(e) => {
                let error_str = e.to_string();
                // Handle various acceptable docking failures gracefully
                if error_str.contains("already docked") {
                    println!("🚢 {} already docked at {}", self.ship_symbol, marketplace);
                } else if error_str.contains("429 Too Many Requests") {
                    // Extract retry delay from the rate limit error
                    let retry_after = if let Some(retry_match) = error_str.find("\"retryAfter\":") {
                        let start = retry_match + 13; // Length of "retryAfter":
                        let end = error_str[start..].find(',').unwrap_or(10) + start;
                        error_str[start..end].trim().parse::<f64>().unwrap_or(1.0)
                    } else {
                        1.0 // Default 1 second retry
                    };
                    println!("🕐 {} hit API rate limit - waiting {:.1}s before retry", self.ship_symbol, retry_after);
                    return Err(ShipActorError(format!("Rate limited - retry in {:.1}s", retry_after)));
                } else if error_str.contains("400 Bad Request") {
                    // Common 400 errors for docking - handle gracefully
                    if error_str.contains("not at waypoint") || error_str.contains("must be at") {
                        println!("⚠️  {} cannot dock - not at correct location for {}", self.ship_symbol, marketplace);
                        return Err(ShipActorError(format!("Cannot sell cargo - ship not at marketplace location: {}", marketplace)));
                    } else if error_str.contains("cannot dock") || error_str.contains("docking not allowed") {
                        println!("⚠️  {} cannot dock at {} - docking not allowed", self.ship_symbol, marketplace);
                        return Err(ShipActorError(format!("Docking not allowed at marketplace: {}", marketplace)));
                    } else {
                        println!("⚠️  {} docking failed at {} with 400 error: {}", self.ship_symbol, marketplace, error_str);
                        return Err(ShipActorError(format!("Docking failed: {}", e)));
                    }
                } else {
                    println!("⚠️  {} unexpected docking error at {}: {}", self.ship_symbol, marketplace, error_str);
                    return Err(ShipActorError(format!("Docking failed: {}", e)));
                }
            }
        }
        
        // Sell all cargo items
        for item in &current_ship.cargo.inventory {
            println!("💰 {} selling {} x{}", self.ship_symbol, item.symbol, item.units);
            
            match self.client.sell_cargo(&self.ship_symbol, &item.symbol, item.units).await {
                Ok(sell_data) => {
                    println!("💵 {} sold {} x{} for {} credits each", 
                            self.ship_symbol, 
                            item.symbol,
                            sell_data.transaction.units,
                            sell_data.transaction.price_per_unit);
                }
                Err(e) => {
                    println!("⚠️ {} failed to sell {}: {}", self.ship_symbol, item.symbol, e);
                    // Continue trying to sell other items
                }
            }
        }
        
        println!("💰 {} finished selling cargo", self.ship_symbol);
        Ok(())
    }
    
    /// Smart sell or jettison: try to sell first, then jettison if selling fails
    async fn execute_smart_sell_or_jettison(&mut self, marketplace: &str, contract_materials: &[String]) -> Result<(), ShipActorError> {
        println!("🏪 {} attempting smart sell/jettison at {}", self.ship_symbol, marketplace);
        
        // First, try to sell at the marketplace
        let sell_result = self.execute_sell_cargo_at_marketplace(marketplace).await;
        
        match sell_result {
            Ok(()) => {
                println!("✅ {} successfully sold cargo at marketplace", self.ship_symbol);
                Ok(())
            }
            Err(e) => {
                println!("⚠️ {} selling failed: {}", self.ship_symbol, e);
                println!("🗑️ {} falling back to jettisoning non-contract cargo", self.ship_symbol);
                
                // Fallback to jettisoning
                self.execute_jettison_cargo(contract_materials).await
            }
        }
    }
    
    /// Execute selling cargo at a specific marketplace
    async fn execute_sell_cargo_at_marketplace(&mut self, marketplace: &str) -> Result<(), ShipActorError> {
        // Navigate to marketplace first
        let current_ship = self.client.get_ship(&self.ship_symbol).await
            .map_err(|e| ShipActorError(format!("Failed to get ship status: {}", e)))?;
        
        if current_ship.nav.waypoint_symbol != marketplace {
            println!("🚀 {} navigating to marketplace {}", self.ship_symbol, marketplace);
            
            // Ensure ship is in orbit before navigating
            if current_ship.nav.status == "DOCKED" {
                match self.client.orbit_ship(&self.ship_symbol).await {
                    Ok(_) => println!("🌌 {} in orbit for navigation", self.ship_symbol),
                    Err(e) if !e.to_string().contains("already in orbit") => {
                        return Err(ShipActorError(format!("Failed to orbit: {}", e)));
                    }
                    _ => {}
                }
            }
            
            self.client.navigate_ship(&self.ship_symbol, marketplace).await
                .map_err(|e| ShipActorError(format!("Navigation to {} failed: {}", marketplace, e)))?;
                
            println!("✅ {} arrived at marketplace {}", self.ship_symbol, marketplace);
        }
        
        // Dock at marketplace
        match self.client.dock_ship(&self.ship_symbol).await {
            Ok(_) => {
                println!("🚢 {} docked at marketplace", self.ship_symbol);
            }
            Err(e) => {
                let error_str = e.to_string();
                // Handle various acceptable docking failures gracefully
                if error_str.contains("already docked") {
                    println!("🚢 {} already docked at marketplace", self.ship_symbol);
                } else if error_str.contains("429 Too Many Requests") {
                    // Extract retry delay from the rate limit error
                    let retry_after = if let Some(retry_match) = error_str.find("\"retryAfter\":") {
                        let start = retry_match + 13; // Length of "retryAfter":
                        let end = error_str[start..].find(',').unwrap_or(10) + start;
                        error_str[start..end].trim().parse::<f64>().unwrap_or(1.0)
                    } else {
                        1.0 // Default 1 second retry
                    };
                    println!("🕐 {} hit API rate limit - waiting {:.1}s before retry", self.ship_symbol, retry_after);
                    return Err(ShipActorError(format!("Rate limited - retry in {:.1}s", retry_after)));
                } else if error_str.contains("400 Bad Request") {
                    // Common 400 errors for docking - handle gracefully
                    if error_str.contains("not at waypoint") || error_str.contains("must be at") {
                        println!("⚠️  {} cannot dock - not at correct location for marketplace", self.ship_symbol);
                        return Err(ShipActorError(format!("Cannot dock - ship not at marketplace location: {}", marketplace)));
                    } else if error_str.contains("cannot dock") || error_str.contains("docking not allowed") {
                        println!("⚠️  {} cannot dock at marketplace - docking not allowed", self.ship_symbol);
                        return Err(ShipActorError(format!("Docking not allowed at marketplace: {}", marketplace)));
                    } else {
                        println!("⚠️  {} docking failed at marketplace with 400 error: {}", self.ship_symbol, error_str);
                        return Err(ShipActorError(format!("Docking failed: {}", e)));
                    }
                } else {
                    println!("⚠️  {} unexpected docking error at marketplace: {}", self.ship_symbol, error_str);
                    return Err(ShipActorError(format!("Docking failed: {}", e)));
                }
            }
        }
        
        // Now sell all cargo
        self.execute_sell_cargo(marketplace).await
    }
    
    /// Check if an error message indicates the ship is in transit (error 4214)
    fn is_transit_error(&self, error_msg: &str) -> bool {
        error_msg.contains("4214") || 
        error_msg.contains("in-transit") || 
        error_msg.contains("arrives in") ||
        error_msg.contains("secondsToArrival")
    }
    
    /// Attempt resource extraction with automatic retry for transit errors
    async fn attempt_extraction_with_retry(&mut self, needed_materials: &[String]) -> Result<(), ShipActorError> {
        // Try extraction first
        let extraction_result = self.try_extraction().await;
        
        match extraction_result {
            Ok(extraction_data) => {
                self.process_extraction_success(&extraction_data, needed_materials).await
            }
            Err(error_msg) => {
                if self.is_transit_error(&error_msg) {
                    // Handle transit error with retry
                    println!("⏳ {} still in transit, waiting for arrival...", self.ship_symbol);
                    self.wait_for_arrival().await?;
                    
                    // Retry after arrival
                    match self.try_extraction().await {
                        Ok(extraction_data) => {
                            println!("⛏️ {} extraction successful after waiting for transit", self.ship_symbol);
                            self.process_extraction_success(&extraction_data, needed_materials).await
                        }
                        Err(retry_error_msg) => {
                            println!("❌ {} extraction failed even after waiting: {}", self.ship_symbol, retry_error_msg);
                            Err(ShipActorError(retry_error_msg))
                        }
                    }
                } else {
                    println!("❌ {} extraction failed: {}", self.ship_symbol, error_msg);
                    Err(ShipActorError(error_msg))
                }
            }
        }
    }
    
    /// Try extraction and return either success data or error message string
    async fn try_extraction(&self) -> Result<crate::models::ExtractionData, String> {
        match self.client.extract_resources(&self.ship_symbol).await {
            Ok(extraction_data) => Ok(extraction_data),
            Err(e) => Err(e.to_string())
        }
    }
    
    /// Process successful extraction data
    async fn process_extraction_success(&mut self, extraction_data: &crate::models::ExtractionData, needed_materials: &[String]) -> Result<(), ShipActorError> {
        let yield_info = &extraction_data.extraction.extraction_yield;
        println!("⛏️ {} extracted {} x{}", self.ship_symbol, yield_info.symbol, yield_info.units);
        
        // Set cooldown from extraction
        if extraction_data.cooldown.remaining_seconds > 0.0 {
            self.cooldown_until = Some(Instant::now() + Duration::from_secs_f64(extraction_data.cooldown.remaining_seconds));
            
            // Persist cooldown
            if let Err(e) = self.cooldown_store.set_cooldown(&self.ship_symbol, extraction_data.cooldown.remaining_seconds) {
                println!("⚠️ Failed to save cooldown for {}: {}", self.ship_symbol, e);
            }
        }
        
        // Check if it's contract material
        if needed_materials.contains(&yield_info.symbol) {
            println!("🎯 {} found CONTRACT MATERIAL: {}! ✨", self.ship_symbol, yield_info.symbol);
        }
        
        Ok(())
    }

    /// Wait for ship to arrive if it's currently in transit
    async fn wait_for_arrival(&self) -> Result<(), ShipActorError> {
        loop {
            let ship = self.client.get_ship(&self.ship_symbol).await
                .map_err(|e| ShipActorError(format!("Failed to check ship status: {}", e)))?;
            
            if ship.nav.status != "IN_TRANSIT" {
                break; // Ship has arrived
            }
            
            // Parse arrival time and calculate wait
            if let Ok(arrival_time) = chrono::DateTime::parse_from_rfc3339(&ship.nav.route.arrival) {
                let now = chrono::Utc::now();
                let wait_seconds = (arrival_time.timestamp() - now.timestamp()).max(0);
                if wait_seconds > 0 {
                    println!("⏳ {} in transit to {}, arriving in {} seconds", 
                            self.ship_symbol, ship.nav.route.destination.symbol, wait_seconds);
                    
                    // Wait for a reasonable amount (max 10 seconds at a time to avoid blocking)
                    let sleep_duration = wait_seconds.min(10) as u64;
                    tokio::time::sleep(Duration::from_secs(sleep_duration)).await;
                    continue;
                }
            }
            
            // If we can't parse the time, wait a short amount and check again
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        
        println!("✅ {} has arrived and is ready for operations", self.ship_symbol);
        Ok(())
    }

    async fn execute_jettison_cargo(&mut self, contract_materials: &[String]) -> Result<(), ShipActorError> {
        println!("🗑️ {} jettisoning non-contract cargo", self.ship_symbol);
        
        // Get current cargo
        let ship = self.client.get_ship(&self.ship_symbol).await
            .map_err(|e| ShipActorError(format!("Failed to get ship status: {}", e)))?;
        
        let mut jettisoned_items = 0;
        let mut kept_items = 0;
        
        for item in &ship.cargo.inventory {
            if contract_materials.contains(&item.symbol) {
                println!("   🎯 Keeping contract item: {} x{}", item.symbol, item.units);
                kept_items += 1;
            } else {
                println!("   🗑️ Jettisoning: {} x{}", item.symbol, item.units);
                
                match self.client.jettison_cargo(&self.ship_symbol, &item.symbol, item.units).await {
                    Ok(_jettison_data) => {
                        println!("   ✅ Jettisoned {} x{}", item.symbol, item.units);
                        jettisoned_items += 1;
                    }
                    Err(e) => {
                        println!("   ⚠️ Failed to jettison {}: {}", item.symbol, e);
                    }
                }
            }
        }
        
        if jettisoned_items > 0 {
            println!("🗑️ {} jettisoned {} items, kept {} contract items", 
                    self.ship_symbol, jettisoned_items, kept_items);
        } else {
            println!("⚠️ {} no items jettisoned", self.ship_symbol);
        }
        
        Ok(())
    }
}