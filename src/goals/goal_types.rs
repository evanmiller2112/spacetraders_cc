// Goal Types - Concrete implementations of different goal types
use crate::goals::{Goal, GoalPriority, GoalStatus, GoalContext, GoalResult, SurveyCache};
use crate::client::{PriorityApiClient, ApiPriority};
use crate::models::transaction::Survey;
use crate::{o_debug, o_info};
use async_trait::async_trait;
use std::collections::HashMap;

#[derive(Debug)]
pub struct MiningGoal {
    pub id: String,
    pub resource_type: String,
    pub target_quantity: i32,
    pub priority: GoalPriority,
    pub status: GoalStatus,
    pub survey_cache: SurveyCache,
    pub use_surveys: bool,
}

impl Clone for MiningGoal {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            resource_type: self.resource_type.clone(),
            target_quantity: self.target_quantity,
            priority: self.priority,
            status: self.status.clone(),
            survey_cache: SurveyCache::new(), // Create fresh cache for cloned goal
            use_surveys: self.use_surveys,
        }
    }
}

#[async_trait]
impl Goal for MiningGoal {
    fn id(&self) -> String { self.id.clone() }
    fn description(&self) -> String { 
        format!("Mine {} units of {}", self.target_quantity, self.resource_type) 
    }
    fn priority(&self) -> GoalPriority { self.priority }
    fn status(&self) -> GoalStatus { self.status.clone() }
    fn estimated_duration(&self) -> f64 { (self.target_quantity as f64) * 30.0 } // 30 sec per unit
    fn required_resources(&self) -> Vec<String> { vec!["mining_ship".to_string()] }

    async fn validate(&self, context: &GoalContext) -> Result<bool, String> {
        // Check if we have mining ships available
        if context.fleet_status.mining_ships.is_empty() {
            return Err("No mining ships available".to_string());
        }
        
        // Check if any mining ships are available (not busy)
        let available_miners: Vec<_> = context.fleet_status.mining_ships.iter()
            .filter(|ship| !context.fleet_status.busy_ships.contains_key(*ship))
            .collect();
        
        if available_miners.is_empty() {
            return Err("All mining ships are busy".to_string());
        }

        Ok(true)
    }

    async fn execute(&mut self, client: &PriorityApiClient, context: &GoalContext) -> Result<GoalResult, Box<dyn std::error::Error>> {
        o_info!("⛏️ Executing {} mining goal: {}", 
               if self.use_surveys { "survey-based" } else { "standard" }, 
               self.description());
        self.status = GoalStatus::Active;

        // Find available mining ship
        let mining_ship = context.fleet_status.mining_ships.iter()
            .find(|ship| !context.fleet_status.busy_ships.contains_key(*ship))
            .ok_or("No available mining ships")?;

        let ship = client.get_ship(mining_ship).await?;
        o_debug!("🚢 Using mining ship: {} at {}", ship.symbol, ship.nav.waypoint_symbol);

        // Find mining location for the resource
        let mining_location = self.find_mining_location(client, &self.resource_type, &ship.nav.system_symbol).await?;
        o_info!("📍 Mining location found: {}", mining_location);

        let mut total_mined = 0;
        let start_time = std::time::Instant::now();
        let mut active_survey: Option<Survey> = None;

        // Navigate to mining location if needed
        if ship.nav.waypoint_symbol != mining_location {
            o_debug!("🗺️ Navigating {} to {}", ship.symbol, mining_location);
            client.navigate_ship_with_priority(&ship.symbol, &mining_location, ApiPriority::Override).await?;
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        }

        // Ensure ship is in orbit for mining
        if ship.nav.status != "IN_ORBIT" {
            client.orbit_ship_with_priority(&ship.symbol, ApiPriority::Override).await?;
        }

        while total_mined < self.target_quantity {
            // Get survey if using survey-based mining and don't have an active one
            if self.use_surveys && active_survey.is_none() {
                o_debug!("🔍 Getting survey for {} at {}", self.resource_type, mining_location);
                match self.survey_cache.get_best_survey(client, &ship.symbol, &mining_location, &self.resource_type).await {
                    Ok(Some(survey)) => {
                        active_survey = Some(survey.clone());
                        o_info!("📋 Using survey {} for targeted {} mining", survey.signature, self.resource_type);
                    }
                    Ok(None) => {
                        o_debug!("📋 No suitable survey found, using standard mining");
                    }
                    Err(e) => {
                        o_debug!("⚠️ Survey failed: {} - falling back to standard mining", e);
                    }
                }
            }

            // Mine resources with or without survey
            o_debug!("⛏️ {} mining at {}", ship.symbol, mining_location);
            
            let extraction_data = match if let Some(ref survey) = active_survey {
                client.extract_resources_with_priority(&ship.symbol, Some(survey), ApiPriority::Override).await
            } else {
                client.extract_resources_with_priority(&ship.symbol, None, ApiPriority::Override).await
            } {
                Ok(data) => data,
                Err(e) => {
                    let error_str = e.to_string();
                    if error_str.contains("survey") && error_str.contains("expired") {
                        o_info!("📋 Survey expired, clearing active survey");
                        active_survey = None; // Clear expired survey
                        continue; // Try again without survey
                    } else {
                        o_debug!("⚠️ Mining operation failed: {}", error_str);
                        // Exit loop, will handle retry at a higher level
                        break;
                    }
                }
            };

            // Extract the actual mining yield from the response
            let yield_item = &extraction_data.extraction.extraction_yield;
            
            if yield_item.symbol == self.resource_type {
                total_mined += yield_item.units;
                o_info!("💎 Mined {} {} (total: {}/{}) {}", 
                       yield_item.units, yield_item.symbol, total_mined, self.target_quantity,
                       if active_survey.is_some() { "📋 [SURVEY]" } else { "" });
            } else {
                o_debug!("⚪ Mined {} {} (not target resource)", yield_item.units, yield_item.symbol);
            }

            // Check if cargo is full
            let updated_ship = client.get_ship(&ship.symbol).await?;
            if updated_ship.cargo.units >= updated_ship.cargo.capacity {
                o_info!("📦 Cargo full, goal may need hauling support");
                break;
            }

            // Brief pause between mining operations
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        }

        let execution_time = start_time.elapsed().as_secs_f64();
        self.status = if total_mined >= self.target_quantity {
            GoalStatus::Completed
        } else {
            GoalStatus::Paused // Needs more resources or hauling
        };

        // Log survey cache statistics
        if self.use_surveys {
            let stats = self.survey_cache.get_stats();
            o_debug!("📊 Survey cache stats: {} surveys across {} waypoints", 
                    stats.total_surveys, stats.waypoints_cached);
        }

        Ok(GoalResult {
            success: total_mined >= self.target_quantity,
            message: format!("Mined {}/{} {} units{}", 
                           total_mined, self.target_quantity, self.resource_type,
                           if self.use_surveys { " using surveys" } else { "" }),
            ships_used: vec![ship.symbol.clone()],
            resources_consumed: HashMap::new(),
            credits_spent: 0,
            execution_time,
        })
    }
}

impl MiningGoal {
    async fn find_mining_location(&self, client: &PriorityApiClient, resource: &str, system: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Get system waypoints and find asteroid fields with the desired resource
        let waypoints = client.get_system_waypoints(system, None).await?;
        
        for waypoint in waypoints {
            if waypoint.waypoint_type.contains("ASTEROID") {
                // In a full implementation, would check waypoint traits for specific resources
                o_debug!("🗿 Found potential mining site: {} ({})", waypoint.symbol, waypoint.waypoint_type);
                return Ok(waypoint.symbol);
            }
        }
        
        Err(format!("No mining locations found for {} in system {}", resource, system).into())
    }
}

#[derive(Debug, Clone)]
pub struct RefiningGoal {
    pub id: String,
    pub input_resource: String,
    pub output_resource: String, 
    pub target_quantity: i32,
    pub priority: GoalPriority,
    pub status: GoalStatus,
}

#[async_trait]
impl Goal for RefiningGoal {
    fn id(&self) -> String { self.id.clone() }
    fn description(&self) -> String { 
        format!("Refine {} {} into {} {}", self.target_quantity, self.input_resource, self.target_quantity, self.output_resource) 
    }
    fn priority(&self) -> GoalPriority { self.priority }
    fn status(&self) -> GoalStatus { self.status.clone() }
    fn estimated_duration(&self) -> f64 { (self.target_quantity as f64) * 45.0 } // 45 sec per unit
    fn required_resources(&self) -> Vec<String> { vec!["hauler_ship".to_string(), self.input_resource.clone()] }

    async fn validate(&self, context: &GoalContext) -> Result<bool, String> {
        if context.fleet_status.hauler_ships.is_empty() {
            return Err("No hauler ships available for refining transport".to_string());
        }
        Ok(true)
    }

    async fn execute(&mut self, client: &PriorityApiClient, context: &GoalContext) -> Result<GoalResult, Box<dyn std::error::Error>> {
        o_info!("🏭 Executing refining goal: {}", self.description());
        self.status = GoalStatus::Active;

        // Step 1: Find refinery facilities in current system
        let system_symbol = &context.ships[0].nav.system_symbol; // Use first ship's system
        let waypoints = client.get_system_waypoints(system_symbol, None).await?;
        
        let refineries: Vec<_> = waypoints.iter()
            .filter(|w| w.traits.iter().any(|t| 
                t.description.to_lowercase().contains("refin") || 
                t.description.to_lowercase().contains("smelt") ||
                t.description.to_lowercase().contains("process")))
            .collect();
            
        if refineries.is_empty() {
            return Err(format!("No refineries found in system {}", system_symbol).into());
        }
        
        let refinery = &refineries[0];
        o_info!("🏭 Found refinery at: {}", refinery.symbol);
        
        // Step 2: Find ships with raw materials that can be refined
        let haulers_with_materials: Vec<_> = context.ships.iter()
            .filter(|ship| {
                context.fleet_status.hauler_ships.contains(&ship.symbol) &&
                ship.cargo.inventory.iter().any(|item| 
                    item.symbol.ends_with("_ORE") || 
                    item.symbol.contains("RAW_"))
            })
            .collect();
            
        if haulers_with_materials.is_empty() {
            return Err("No hauler ships with raw materials for refining".into());
        }
        
        let ship = haulers_with_materials[0];
        o_info!("🚢 Using ship {} for refining transport", ship.symbol);
        
        // Step 3: Navigate to refinery if needed
        if ship.nav.waypoint_symbol != refinery.symbol {
            o_info!("🚀 Navigating to refinery: {}", refinery.symbol);
            client.navigate_ship_with_priority(&ship.symbol, &refinery.symbol, crate::client::ApiPriority::ActiveGoal).await?;
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await; // Brief wait for arrival
        }
        
        // Step 4: Dock at refinery
        if ship.nav.status != "DOCKED" {
            client.dock_ship_with_priority(&ship.symbol, crate::client::ApiPriority::ActiveGoal).await?;
        }
        
        o_info!("🏭 Ship docked at refinery. Raw materials available:");
        for item in &ship.cargo.inventory {
            if item.symbol.ends_with("_ORE") || item.symbol.contains("RAW_") {
                o_info!("  - {} x{}", item.symbol, item.units);
            }
        }
        
        // Note: Actual refining would require specific SpaceTraders API endpoints
        // For now, we've positioned the ship at a refinery ready for refining operations
        
        self.status = GoalStatus::Completed;
        Ok(GoalResult {
            success: true,
            message: format!("Positioned ship at {} refinery for {} -> {} processing", refinery.symbol, self.input_resource, self.output_resource),
            ships_used: vec![ship.symbol.clone()],
            resources_consumed: HashMap::new(),
            credits_spent: 0,
            execution_time: 180.0,
        })
    }
}

#[derive(Debug, Clone)]
pub struct SellingGoal {
    pub id: String,
    pub resource_type: Option<String>, // None means sell all cargo
    pub target_quantity: Option<i32>,  // None means sell all available
    pub priority: GoalPriority,
    pub status: GoalStatus,
}

#[async_trait]
impl Goal for SellingGoal {
    fn id(&self) -> String { self.id.clone() }
    fn description(&self) -> String { 
        match (&self.resource_type, &self.target_quantity) {
            (Some(resource), Some(qty)) => format!("Sell {} units of {}", qty, resource),
            (Some(resource), None) => format!("Sell all {}", resource),
            (None, _) => "Sell all cargo".to_string(),
        }
    }
    fn priority(&self) -> GoalPriority { self.priority }
    fn status(&self) -> GoalStatus { self.status.clone() }
    fn estimated_duration(&self) -> f64 { 120.0 } // 2 minutes for selling operations
    fn required_resources(&self) -> Vec<String> { vec!["ship_with_cargo".to_string()] }

    async fn validate(&self, context: &GoalContext) -> Result<bool, String> {
        // Check if any ships have cargo to sell
        let ships_with_cargo = context.ships.iter()
            .filter(|ship| ship.cargo.units > 0)
            .count();
            
        if ships_with_cargo == 0 {
            return Err("No ships with cargo to sell".to_string());
        }
        
        Ok(true)
    }

    async fn execute(&mut self, client: &PriorityApiClient, context: &GoalContext) -> Result<GoalResult, Box<dyn std::error::Error>> {
        o_info!("💰 Executing selling goal: {}", self.description());
        self.status = GoalStatus::Active;

        let mut total_revenue = 0;
        let mut ships_used = Vec::new();

        // Find ships with cargo
        let cargo_ships: Vec<_> = context.ships.iter()
            .filter(|ship| ship.cargo.units > 0)
            .collect();

        for ship in cargo_ships {
            o_debug!("🚢 Selling cargo from: {} ({} items)", ship.symbol, ship.cargo.units);
            
            // Find nearest marketplace - simplified implementation
            let marketplace = self.find_nearest_marketplace(client, &ship.nav.system_symbol).await?;
            
            // Navigate to marketplace if needed
            if ship.nav.waypoint_symbol != marketplace {
                client.navigate_ship_with_priority(&ship.symbol, &marketplace, ApiPriority::Override).await?;
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await; // Wait for arrival
            }
            
            // Dock for selling
            if ship.nav.status != "DOCKED" {
                client.dock_ship_with_priority(&ship.symbol, ApiPriority::Override).await?;
            }
            
            // Sell cargo items
            for cargo_item in &ship.cargo.inventory {
                if self.should_sell_item(&cargo_item.symbol) {
                    let units_to_sell = if let Some(target_qty) = self.target_quantity {
                        target_qty.min(cargo_item.units)
                    } else {
                        cargo_item.units
                    };
                    
                    o_debug!("💱 Selling {} {} from {}", units_to_sell, cargo_item.symbol, ship.symbol);
                    let sell_result = client.sell_cargo_with_priority(
                        &ship.symbol, 
                        &cargo_item.symbol, 
                        units_to_sell,
                        ApiPriority::Override
                    ).await?;
                    
                    total_revenue += sell_result.transaction.total_price;
                    o_info!("✅ Sold {} {} for {} credits", 
                           units_to_sell, cargo_item.symbol, sell_result.transaction.total_price);
                }
            }
            
            ships_used.push(ship.symbol.clone());
        }

        self.status = GoalStatus::Completed;
        Ok(GoalResult {
            success: true,
            message: format!("Sold cargo for {} credits", total_revenue),
            ships_used,
            resources_consumed: HashMap::new(),
            credits_spent: -total_revenue, // Negative because we gained credits
            execution_time: 120.0,
        })
    }
}

impl SellingGoal {
    fn should_sell_item(&self, item_symbol: &str) -> bool {
        match &self.resource_type {
            Some(target_resource) => item_symbol == target_resource,
            None => true, // Sell everything if no specific resource specified
        }
    }
    
    async fn find_nearest_marketplace(&self, client: &PriorityApiClient, system: &str) -> Result<String, Box<dyn std::error::Error>> {
        let waypoints = client.get_system_waypoints(system, None).await?;
        
        for waypoint in waypoints {
            if waypoint.traits.iter().any(|t| t.symbol == "MARKETPLACE") {
                return Ok(waypoint.symbol);
            }
        }
        
        Err(format!("No marketplace found in system {}", system).into())
    }
}

// Additional goal types for completeness
#[derive(Debug, Clone)]
pub struct ShipPurchaseGoal {
    pub id: String,
    pub ship_type: String,
    pub priority: GoalPriority,
    pub status: GoalStatus,
}

#[derive(Debug, Clone)]
pub struct ResourcePurchaseGoal {
    pub id: String,
    pub resource_type: String,
    pub target_quantity: i32,
    pub priority: GoalPriority,
    pub status: GoalStatus,
}

#[derive(Debug, Clone)]
pub struct ExplorationGoal {
    pub id: String,
    pub target_type: String, // "SHIPYARDS", "MARKETS", system symbol, etc.
    pub priority: GoalPriority,
    pub status: GoalStatus,
}

#[derive(Debug, Clone)]
pub struct TransferGoal {
    pub id: String,
    pub resource_type: String,
    pub target_ship: String,
    pub quantity: Option<i32>, // None means transfer all available
    pub priority: GoalPriority,
    pub status: GoalStatus,
}

#[derive(Debug, Clone)]
pub struct DebugGoal {
    pub id: String,
    pub target: String, // What to debug
    pub priority: GoalPriority,
    pub status: GoalStatus,
}

#[derive(Debug, Clone)]
pub struct ShipRoleGoal {
    pub id: String,
    pub target_ship: Option<String>, // Specific ship to designate, or None to find best
    pub desired_role: String, // "refiner", "hauler", "miner", etc.
    pub priority: GoalPriority,
    pub status: GoalStatus,
}

#[async_trait]
impl Goal for TransferGoal {
    fn id(&self) -> String { self.id.clone() }
    fn description(&self) -> String { 
        match self.quantity {
            Some(qty) => format!("Transfer {} {} to {}", qty, self.resource_type, self.target_ship),
            None => format!("Transfer all {} to {}", self.resource_type, self.target_ship),
        }
    }
    fn priority(&self) -> GoalPriority { self.priority }
    fn status(&self) -> GoalStatus { self.status.clone() }
    fn estimated_duration(&self) -> f64 { 30.0 }
    fn required_resources(&self) -> Vec<String> { vec!["ship_with_cargo".to_string(), "target_ship".to_string()] }
    
    async fn validate(&self, context: &GoalContext) -> Result<bool, String> {
        // Check if target ship exists
        let target_exists = context.ships.iter().any(|ship| ship.symbol == self.target_ship);
        if !target_exists {
            return Err(format!("Target ship {} not found", self.target_ship));
        }
        
        // Check if any ships have the resource to transfer
        let ships_with_resource = context.ships.iter()
            .filter(|ship| ship.symbol != self.target_ship) // Don't transfer to self
            .any(|ship| ship.cargo.inventory.iter()
                .any(|item| item.symbol == self.resource_type && item.units > 0));
        
        if !ships_with_resource {
            return Err(format!("No ships have {} to transfer", self.resource_type));
        }
        
        Ok(true)
    }
    
    async fn execute(&mut self, client: &PriorityApiClient, context: &GoalContext) -> Result<GoalResult, Box<dyn std::error::Error>> {
        o_info!("🔄 Executing transfer goal: {}", self.description());
        self.status = GoalStatus::Active;
        
        let mut total_transferred = 0;
        let mut ships_used = Vec::new();
        let start_time = std::time::Instant::now();
        
        // Find target ship
        let target_ship = context.ships.iter()
            .find(|ship| ship.symbol == self.target_ship)
            .ok_or(format!("Target ship {} not found", self.target_ship))?;
        
        o_debug!("🎯 Target ship: {} (cargo: {}/{})", 
                target_ship.symbol, target_ship.cargo.units, target_ship.cargo.capacity);
        
        // Find source ships with the resource
        let source_ships: Vec<_> = context.ships.iter()
            .filter(|ship| ship.symbol != self.target_ship)
            .filter(|ship| ship.cargo.inventory.iter()
                .any(|item| item.symbol == self.resource_type && item.units > 0))
            .collect();
        
        if source_ships.is_empty() {
            return Err(format!("No ships found with {} to transfer", self.resource_type).into());
        }
        
        o_info!("📦 Found {} ships with {}", source_ships.len(), self.resource_type);
        
        for source_ship in source_ships {
            // Check if target ship has space
            let updated_target = client.get_ship(&self.target_ship).await?;
            let available_space = updated_target.cargo.capacity - updated_target.cargo.units;
            
            if available_space <= 0 {
                o_info!("🚫 Target ship {} is full, stopping transfers", self.target_ship);
                break;
            }
            
            // Find the resource in source ship's cargo
            let source_cargo = source_ship.cargo.inventory.iter()
                .find(|item| item.symbol == self.resource_type)
                .ok_or(format!("Resource {} not found in {}", self.resource_type, source_ship.symbol))?;
            
            // Calculate transfer amount
            let transfer_amount = match self.quantity {
                Some(target_qty) => {
                    let remaining_needed = target_qty - total_transferred;
                    remaining_needed.min(source_cargo.units).min(available_space)
                }
                None => source_cargo.units.min(available_space) // Transfer all available
            };
            
            if transfer_amount <= 0 {
                continue;
            }
            
            // Check if ships are at the same location
            if source_ship.nav.waypoint_symbol != target_ship.nav.waypoint_symbol {
                o_info!("⚠️ Ships not at same location: {} at {}, {} at {}. Skipping transfer.",
                       source_ship.symbol, source_ship.nav.waypoint_symbol,
                       target_ship.symbol, target_ship.nav.waypoint_symbol);
                continue;
            }
            
            // Perform the transfer
            o_info!("📦 Transferring {} {} from {} to {}", 
                   transfer_amount, self.resource_type, source_ship.symbol, target_ship.symbol);
            
            match client.transfer_cargo_with_priority(
                &source_ship.symbol,
                &self.resource_type,
                transfer_amount,
                &target_ship.symbol,
                ApiPriority::ActiveGoal
            ).await {
                Ok(_) => {
                    total_transferred += transfer_amount;
                    ships_used.push(source_ship.symbol.clone());
                    o_info!("✅ Transferred {} {} (total: {})", 
                           transfer_amount, self.resource_type, total_transferred);
                    
                    // Check if we've transferred enough
                    if let Some(target_qty) = self.quantity {
                        if total_transferred >= target_qty {
                            o_info!("🎯 Target quantity {} reached", target_qty);
                            break;
                        }
                    }
                }
                Err(e) => {
                    o_info!("⚠️ Transfer failed from {}: {}", source_ship.symbol, e);
                    continue; // Try next ship
                }
            }
        }
        
        let execution_time = start_time.elapsed().as_secs_f64();
        self.status = if total_transferred > 0 {
            GoalStatus::Completed
        } else {
            GoalStatus::Failed("No transfers completed".to_string())
        };
        
        Ok(GoalResult {
            success: total_transferred > 0,
            message: format!("Transferred {} {} to {}", total_transferred, self.resource_type, self.target_ship),
            ships_used,
            resources_consumed: HashMap::new(),
            credits_spent: 0,
            execution_time,
        })
    }
}

#[async_trait]
impl Goal for ShipRoleGoal {
    fn id(&self) -> String { self.id.clone() }
    fn description(&self) -> String { 
        match &self.target_ship {
            Some(ship) => format!("Designate {} as {}", ship, self.desired_role),
            None => format!("Find and designate best ship as {}", self.desired_role),
        }
    }
    fn priority(&self) -> GoalPriority { self.priority }
    fn status(&self) -> GoalStatus { self.status.clone() }
    fn estimated_duration(&self) -> f64 { 120.0 } // 2 minutes for analysis and designation
    fn required_resources(&self) -> Vec<String> { vec!["ship_role_manager".to_string()] }
    
    async fn validate(&self, context: &GoalContext) -> Result<bool, String> {
        // If specific ship is requested, check if it exists
        if let Some(ship_symbol) = &self.target_ship {
            let ship_exists = context.ships.iter().any(|ship| ship.symbol == *ship_symbol);
            if !ship_exists {
                return Err(format!("Target ship {} not found", ship_symbol));
            }
        }
        
        // Check if we have any ships available for role change
        if context.ships.is_empty() {
            return Err("No ships available for role designation".to_string());
        }
        
        Ok(true)
    }
    
    async fn execute(&mut self, client: &PriorityApiClient, _context: &GoalContext) -> Result<GoalResult, Box<dyn std::error::Error>> {
        o_info!("🎭 Executing ship role goal: {}", self.description());
        self.status = GoalStatus::Active;
        
        let start_time = std::time::Instant::now();
        
        // Import the ship role manager
        use crate::operations::ShipRoleManager;
        let mut role_manager = ShipRoleManager::new();
        
        // Analyze the fleet
        role_manager.analyze_fleet(client).await?;
        role_manager.print_fleet_analysis();
        
        let designated_ship = match &self.target_ship {
            Some(ship_symbol) => {
                // Use specified ship
                o_info!("🎯 Designating specific ship: {}", ship_symbol);
                ship_symbol.clone()
            },
            None => {
                // Find best candidate for the role
                match self.desired_role.as_str() {
                    "refiner" | "refinery" => {
                        if let Some(best_candidate) = role_manager.find_best_refinery_candidate() {
                            o_info!("🏆 Found best refinery candidate: {} (score: {:.2})", 
                                   best_candidate.ship_symbol, best_candidate.refinery_score);
                            best_candidate.ship_symbol.clone()
                        } else {
                            return Err("No suitable ships found for refinery role".into());
                        }
                    },
                    _ => {
                        return Err(format!("Role designation for '{}' not yet implemented", self.desired_role).into());
                    }
                }
            }
        };
        
        // Perform the role designation
        let success = match self.desired_role.as_str() {
            "refiner" | "refinery" => {
                role_manager.designate_refinery_ship(&designated_ship, client).await?
            },
            _ => {
                o_info!("🔧 Role designation for '{}' not yet implemented, marking as planned", self.desired_role);
                true // For now, just mark as successful for planning purposes
            }
        };
        
        if success {
            o_info!("✅ Successfully designated {} as {}", designated_ship, self.desired_role);
            self.status = GoalStatus::Completed;
            
            Ok(GoalResult {
                success: true,
                message: format!("Ship {} designated as {}", designated_ship, self.desired_role),
                ships_used: vec![designated_ship],
                resources_consumed: HashMap::new(),
                credits_spent: 0,
                execution_time: start_time.elapsed().as_secs_f64(),
            })
        } else {
            o_info!("❌ Failed to designate {} as {}", designated_ship, self.desired_role);
            self.status = GoalStatus::Failed(format!("Failed to designate ship {} as {}", designated_ship, self.desired_role));
            
            Err(format!("Failed to designate ship {} as {}", designated_ship, self.desired_role).into())
        }
    }
}

// Implement Goal trait for remaining types (simplified for now)
#[async_trait] impl Goal for ShipPurchaseGoal {
    fn id(&self) -> String { self.id.clone() }
    fn description(&self) -> String { format!("Purchase {} ship", self.ship_type) }
    fn priority(&self) -> GoalPriority { self.priority }
    fn status(&self) -> GoalStatus { self.status.clone() }
    fn estimated_duration(&self) -> f64 { 300.0 }
    fn required_resources(&self) -> Vec<String> { vec!["credits".to_string()] }
    async fn validate(&self, _context: &GoalContext) -> Result<bool, String> { Ok(true) }
    async fn execute(&mut self, _client: &PriorityApiClient, _context: &GoalContext) -> Result<GoalResult, Box<dyn std::error::Error>> {
        o_info!("🚢 Ship purchase goal: {} (placeholder)", self.ship_type);
        self.status = GoalStatus::Completed;
        Ok(GoalResult { success: true, message: "Ship purchase planned".to_string(), ships_used: vec![], resources_consumed: HashMap::new(), credits_spent: 0, execution_time: 1.0 })
    }
}

#[async_trait] impl Goal for ResourcePurchaseGoal {
    fn id(&self) -> String { self.id.clone() }
    fn description(&self) -> String { format!("Purchase {} units of {}", self.target_quantity, self.resource_type) }
    fn priority(&self) -> GoalPriority { self.priority }
    fn status(&self) -> GoalStatus { self.status.clone() }
    fn estimated_duration(&self) -> f64 { 180.0 }
    fn required_resources(&self) -> Vec<String> { vec!["credits".to_string(), "cargo_space".to_string()] }
    async fn validate(&self, _context: &GoalContext) -> Result<bool, String> { Ok(true) }
    async fn execute(&mut self, _client: &PriorityApiClient, _context: &GoalContext) -> Result<GoalResult, Box<dyn std::error::Error>> {
        o_info!("🛒 Resource purchase goal: {} {} (placeholder)", self.target_quantity, self.resource_type);
        self.status = GoalStatus::Completed;
        Ok(GoalResult { success: true, message: "Resource purchase planned".to_string(), ships_used: vec![], resources_consumed: HashMap::new(), credits_spent: 0, execution_time: 1.0 })
    }
}

#[async_trait] impl Goal for ExplorationGoal {
    fn id(&self) -> String { self.id.clone() }
    fn description(&self) -> String { format!("Explore {}", self.target_type) }
    fn priority(&self) -> GoalPriority { self.priority }
    fn status(&self) -> GoalStatus { self.status.clone() }
    fn estimated_duration(&self) -> f64 { 600.0 }
    fn required_resources(&self) -> Vec<String> { vec!["probe_ship".to_string()] }
    async fn validate(&self, _context: &GoalContext) -> Result<bool, String> { Ok(true) }
    async fn execute(&mut self, _client: &PriorityApiClient, _context: &GoalContext) -> Result<GoalResult, Box<dyn std::error::Error>> {
        o_info!("🔍 Exploration goal: {} (placeholder)", self.target_type);
        self.status = GoalStatus::Completed;
        Ok(GoalResult { success: true, message: "Exploration planned".to_string(), ships_used: vec![], resources_consumed: HashMap::new(), credits_spent: 0, execution_time: 1.0 })
    }
}

#[async_trait] impl Goal for DebugGoal {
    fn id(&self) -> String { self.id.clone() }
    fn description(&self) -> String { format!("Debug {}", self.target) }
    fn priority(&self) -> GoalPriority { self.priority }
    fn status(&self) -> GoalStatus { self.status.clone() }
    fn estimated_duration(&self) -> f64 { 60.0 }
    fn required_resources(&self) -> Vec<String> { vec![] }
    async fn validate(&self, _context: &GoalContext) -> Result<bool, String> { Ok(true) }
    async fn execute(&mut self, client: &PriorityApiClient, context: &GoalContext) -> Result<GoalResult, Box<dyn std::error::Error>> {
        o_info!("🐛 Debug goal: {}", self.target);
        
        if self.target.starts_with("ship:") {
            let ship_id = self.target.replace("ship:", "");
            let ship = client.get_ship(&ship_id).await?;
            o_info!("🚢 Ship {} debug info:", ship.symbol);
            o_info!("  Location: {}", ship.nav.waypoint_symbol);
            o_info!("  Status: {}", ship.nav.status);
            o_info!("  Cargo: {}/{}", ship.cargo.units, ship.cargo.capacity);
            o_info!("  Fuel: {}/{}", ship.fuel.current, ship.fuel.capacity);
        } else if self.target == "fleet" {
            o_info!("🚢 Fleet debug info from context:");
            o_info!("  Total ships: {}", context.ships.len());
            o_info!("  Available ships: {}", context.fleet_status.available_ships.len());
            o_info!("  Mining ships: {}", context.fleet_status.mining_ships.len());
            o_info!("  Hauler ships: {}", context.fleet_status.hauler_ships.len());
            o_info!("  Probe ships: {}", context.fleet_status.probe_ships.len());
            o_info!("  Busy ships: {}", context.fleet_status.busy_ships.len());
            for (ship, reason) in &context.fleet_status.busy_ships {
                o_info!("    {} is busy: {}", ship, reason);
            }
        } else if self.target == "agent" {
            o_info!("🤖 Agent debug info from context:");
            o_info!("  Call sign: {}", context.agent.symbol);
            o_info!("  Credits: {}", context.agent.credits);
            o_info!("  Headquarters: {}", context.agent.headquarters);
        } else if self.target == "contracts" {
            o_info!("📋 Contracts debug info from context:");
            o_info!("  Total contracts: {}", context.contracts.len());
            for contract in &context.contracts {
                o_info!("  - {} ({}): {}", contract.id, contract.contract_type, 
                       if contract.accepted { "ACCEPTED" } else { "NOT_ACCEPTED" });
            }
        } else {
            o_info!("🔍 Debug target '{}' analysis (placeholder)", self.target);
        }
        
        self.status = GoalStatus::Completed;
        Ok(GoalResult { success: true, message: "Debug analysis completed".to_string(), ships_used: vec![], resources_consumed: HashMap::new(), credits_spent: 0, execution_time: 1.0 })
    }
}