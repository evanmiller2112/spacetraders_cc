// Task Planner - Creates execution plans for ship actions
use crate::client::SpaceTradersClient;
use crate::{o_debug};
use crate::models::*;
use crate::operations::ship_actor::ShipAction;
use crate::operations::navigation::NavigationPlanner;
use crate::config::SpaceTradersConfig;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TaskPlan {
    pub action: ShipAction,
    pub steps: Vec<TaskStep>,
    pub estimated_fuel_required: i32,
    pub estimated_time_seconds: f64,
}

#[derive(Debug, Clone)]
pub struct TaskStep {
    pub step_type: TaskStepType,
    pub location: String,
    pub fuel_cost: i32,
    pub description: String,
}

#[derive(Debug, Clone)]
pub enum TaskStepType {
    Navigate,
    Dock,
    Orbit,
    Mine,
    Refuel,
    SellCargo,
    DeliverCargo,
    Survey,
    JettisonCargo,
}

pub struct TaskPlanner {
    client: SpaceTradersClient,
    waypoint_cache: HashMap<String, Vec<Waypoint>>,
    config: SpaceTradersConfig,
}

impl TaskPlanner {
    pub fn new(client: SpaceTradersClient, config: SpaceTradersConfig) -> Self {
        Self {
            client,
            waypoint_cache: HashMap::new(),
            config,
        }
    }

    /// Update configuration for hot-reloading
    pub fn update_config(&mut self, new_config: SpaceTradersConfig) {
        self.config = new_config;
    }

    /// Create a detailed execution plan for a ship action
    pub async fn create_plan(&mut self, action: &ShipAction, ship: &Ship) -> Result<TaskPlan, Box<dyn std::error::Error>> {
        let mut steps = Vec::new();
        let mut total_fuel = 0;
        let mut estimated_time = 0.0;

        match action {
            ShipAction::Navigate { destination } => {
                let (nav_steps, fuel_needed) = self.plan_navigation(ship, destination).await?;
                steps.extend(nav_steps);
                total_fuel += fuel_needed;
                estimated_time += fuel_needed as f64 * 2.0; // Rough time estimate
            },
            
            ShipAction::Mine { target, .. } => {
                // Plan: Navigate to target -> Orbit -> Mine
                if ship.nav.waypoint_symbol != *target {
                    let (nav_steps, fuel_needed) = self.plan_navigation(ship, target).await?;
                    steps.extend(nav_steps);
                    total_fuel += fuel_needed;
                }
                
                // Add orbit step if needed
                if ship.nav.status == "DOCKED" {
                    steps.push(TaskStep {
                        step_type: TaskStepType::Orbit,
                        location: target.clone(),
                        fuel_cost: 1,
                        description: format!("Orbit at {} for mining", target),
                    });
                    total_fuel += 1;
                }
                
                // Add mining step
                steps.push(TaskStep {
                    step_type: TaskStepType::Mine,
                    location: target.clone(),
                    fuel_cost: 0,
                    description: format!("Extract resources at {}", target),
                });
                
                estimated_time += total_fuel as f64 * 2.0 + 30.0; // Navigation + mining time
            },

            ShipAction::Refuel { station } => {
                // Plan: Navigate to station -> Dock -> Refuel
                if ship.nav.waypoint_symbol != *station {
                    let (nav_steps, fuel_needed) = self.plan_navigation(ship, station).await?;
                    steps.extend(nav_steps);
                    total_fuel += fuel_needed;
                }
                
                // Add dock step if needed
                if ship.nav.status != "DOCKED" {
                    steps.push(TaskStep {
                        step_type: TaskStepType::Dock,
                        location: station.clone(),
                        fuel_cost: 0,
                        description: format!("Dock at {} for refueling", station),
                    });
                }
                
                // Add refuel step
                steps.push(TaskStep {
                    step_type: TaskStepType::Refuel,
                    location: station.clone(),
                    fuel_cost: 0, // Refuel adds fuel, doesn't consume it
                    description: format!("Refuel at {}", station),
                });
                
                estimated_time += total_fuel as f64 * 2.0 + 5.0; // Navigation + refuel time
            },

            ShipAction::SellCargo { marketplace } => {
                // Plan: Navigate to marketplace -> Dock -> Sell cargo
                if ship.nav.waypoint_symbol != *marketplace {
                    let (nav_steps, fuel_needed) = self.plan_navigation(ship, marketplace).await?;
                    steps.extend(nav_steps);
                    total_fuel += fuel_needed;
                }
                
                // Add dock step if needed
                if ship.nav.status != "DOCKED" {
                    steps.push(TaskStep {
                        step_type: TaskStepType::Dock,
                        location: marketplace.clone(),
                        fuel_cost: 0,
                        description: format!("Dock at {} for trading", marketplace),
                    });
                }
                
                // Add sell step
                steps.push(TaskStep {
                    step_type: TaskStepType::SellCargo,
                    location: marketplace.clone(),
                    fuel_cost: 0,
                    description: format!("Sell cargo at {}", marketplace),
                });
                
                estimated_time += total_fuel as f64 * 2.0 + 10.0; // Navigation + trading time
            },

            ShipAction::DeliverCargo { destination, .. } => {
                // Plan: Navigate to destination -> Dock -> Deliver
                if ship.nav.waypoint_symbol != *destination {
                    let (nav_steps, fuel_needed) = self.plan_navigation(ship, destination).await?;
                    steps.extend(nav_steps);
                    total_fuel += fuel_needed;
                }
                
                // Add dock step if needed
                if ship.nav.status != "DOCKED" {
                    steps.push(TaskStep {
                        step_type: TaskStepType::Dock,
                        location: destination.clone(),
                        fuel_cost: 0,
                        description: format!("Dock at {} for delivery", destination),
                    });
                }
                
                // Add delivery step
                steps.push(TaskStep {
                    step_type: TaskStepType::DeliverCargo,
                    location: destination.clone(),
                    fuel_cost: 0,
                    description: format!("Deliver cargo to {}", destination),
                });
                
                estimated_time += total_fuel as f64 * 2.0 + 10.0; // Navigation + delivery time
            },

            ShipAction::Survey { target } => {
                // Plan: Navigate to target -> Survey
                if ship.nav.waypoint_symbol != *target {
                    let (nav_steps, fuel_needed) = self.plan_navigation(ship, target).await?;
                    steps.extend(nav_steps);
                    total_fuel += fuel_needed;
                }
                
                // Add survey step
                steps.push(TaskStep {
                    step_type: TaskStepType::Survey,
                    location: target.clone(),
                    fuel_cost: 0,
                    description: format!("Survey {}", target),
                });
                
                estimated_time += total_fuel as f64 * 2.0 + 15.0; // Navigation + survey time
            },

            _ => {
                // For other actions, create a basic plan
                steps.push(TaskStep {
                    step_type: TaskStepType::Navigate, // Generic placeholder
                    location: ship.nav.waypoint_symbol.clone(),
                    fuel_cost: 10, // Conservative estimate
                    description: format!("Execute {:?}", action),
                });
                total_fuel = 10;
                estimated_time = 30.0;
            }
        }

        Ok(TaskPlan {
            action: action.clone(),
            steps,
            estimated_fuel_required: total_fuel,
            estimated_time_seconds: estimated_time,
        })
    }

    /// Plan navigation from current position to destination with automatic refuel stops
    async fn plan_navigation(&mut self, ship: &Ship, destination: &str) -> Result<(Vec<TaskStep>, i32), Box<dyn std::error::Error>> {
        let mut steps = Vec::new();
        let mut total_fuel = 0;

        // Get destination waypoint to calculate distance
        if let Some(dest_waypoint) = self.get_waypoint_info(destination).await? {
            let distance = self.calculate_distance(ship, &dest_waypoint);
            let fuel_needed = NavigationPlanner::estimate_fuel_cost(distance);

            // Check if ship has enough fuel for direct navigation
            let available_fuel = if ship.nav.status == "DOCKED" { 
                ship.fuel.current - 1 // Account for orbit cost
            } else { 
                ship.fuel.current 
            };

            if available_fuel >= fuel_needed {
                // Direct navigation possible
                return self.plan_direct_navigation(ship, destination, &dest_waypoint).await;
            } else {
                // Need refuel stops - plan multi-hop route
                o_debug!("🗺️ {} needs multi-hop route to {} (fuel: {}/{} needed)", 
                        ship.symbol, destination, available_fuel, fuel_needed);
                return self.plan_multihop_navigation(ship, destination, &dest_waypoint).await;
            }
        } else {
            // Fallback if we can't get waypoint info
            steps.push(TaskStep {
                step_type: TaskStepType::Navigate,
                location: destination.to_string(),
                fuel_cost: 25, // Conservative estimate
                description: format!("Navigate to {} (estimated)", destination),
            });
            total_fuel += 25;
        }

        Ok((steps, total_fuel))
    }

    /// Plan direct navigation (no refuel needed)
    async fn plan_direct_navigation(&self, ship: &Ship, destination: &str, dest_waypoint: &Waypoint) -> Result<(Vec<TaskStep>, i32), Box<dyn std::error::Error>> {
        let mut steps = Vec::new();
        let mut total_fuel = 0;

        let distance = self.calculate_distance(ship, dest_waypoint);
        let fuel_needed = NavigationPlanner::estimate_fuel_cost(distance);

        // Add orbit step if currently docked
        if ship.nav.status == "DOCKED" {
            steps.push(TaskStep {
                step_type: TaskStepType::Orbit,
                location: ship.nav.waypoint_symbol.clone(),
                fuel_cost: 1,
                description: format!("Orbit from {}", ship.nav.waypoint_symbol),
            });
            total_fuel += 1;
        }

        // Add navigation step
        steps.push(TaskStep {
            step_type: TaskStepType::Navigate,
            location: destination.to_string(),
            fuel_cost: fuel_needed,
            description: format!("Navigate to {} ({:.1} units)", destination, distance),
        });
        total_fuel += fuel_needed;

        Ok((steps, total_fuel))
    }

    /// Plan multi-hop navigation with refuel stops
    async fn plan_multihop_navigation(&mut self, ship: &Ship, destination: &str, dest_waypoint: &Waypoint) -> Result<(Vec<TaskStep>, i32), Box<dyn std::error::Error>> {
        let mut steps = Vec::new();
        let mut total_fuel = 0;

        // Add orbit step if currently docked
        if ship.nav.status == "DOCKED" {
            steps.push(TaskStep {
                step_type: TaskStepType::Orbit,
                location: ship.nav.waypoint_symbol.clone(),
                fuel_cost: 1,
                description: format!("Orbit from {}", ship.nav.waypoint_symbol),
            });
            total_fuel += 1;
        }

        // Find fuel stations in the system
        let system_symbol = ship.nav.system_symbol.clone();
        let fuel_stations = self.find_fuel_stations_in_system(&system_symbol).await?;

        if fuel_stations.is_empty() {
            return Err("No fuel stations available for multi-hop navigation".into());
        }

        // Find the best fuel station to reach with current fuel
        let available_fuel = ship.fuel.current - total_fuel;
        let mut best_fuel_station = None;
        let mut best_distance_to_fuel = f64::MAX;

        for station in &fuel_stations {
            let distance_to_station = self.calculate_distance_from_current_position(ship, station);
            let fuel_needed_to_station = NavigationPlanner::estimate_fuel_cost(distance_to_station);

            // Can we reach this fuel station with current fuel?
            if available_fuel >= fuel_needed_to_station + self.config.fuel.fuel_safety_margin {
                // Is this station closer to our destination than our current best?
                let distance_station_to_dest = NavigationPlanner::calculate_distance_waypoints(station, dest_waypoint);
                
                if distance_station_to_dest < best_distance_to_fuel {
                    best_distance_to_fuel = distance_station_to_dest;
                    best_fuel_station = Some(station);
                }
            }
        }

        match best_fuel_station {
            Some(fuel_station) => {
                // Navigate to fuel station first
                let distance_to_fuel = self.calculate_distance_from_current_position(ship, fuel_station);
                let fuel_needed_to_fuel_station = NavigationPlanner::estimate_fuel_cost(distance_to_fuel);

                steps.push(TaskStep {
                    step_type: TaskStepType::Navigate,
                    location: fuel_station.symbol.clone(),
                    fuel_cost: fuel_needed_to_fuel_station,
                    description: format!("Navigate to fuel station {} ({:.1} units)", fuel_station.symbol, distance_to_fuel),
                });
                total_fuel += fuel_needed_to_fuel_station;

                // Dock and refuel
                steps.push(TaskStep {
                    step_type: TaskStepType::Dock,
                    location: fuel_station.symbol.clone(),
                    fuel_cost: 0,
                    description: format!("Dock at {} for refuel", fuel_station.symbol),
                });

                steps.push(TaskStep {
                    step_type: TaskStepType::Refuel,
                    location: fuel_station.symbol.clone(),
                    fuel_cost: 0, // Refuel adds fuel
                    description: format!("Refuel at {}", fuel_station.symbol),
                });

                // Orbit and navigate to final destination
                steps.push(TaskStep {
                    step_type: TaskStepType::Orbit,
                    location: fuel_station.symbol.clone(),
                    fuel_cost: 1,
                    description: format!("Orbit from {}", fuel_station.symbol),
                });
                total_fuel += 1;

                let distance_to_dest = NavigationPlanner::calculate_distance_waypoints(fuel_station, dest_waypoint);
                let fuel_needed_to_dest = NavigationPlanner::estimate_fuel_cost(distance_to_dest);

                steps.push(TaskStep {
                    step_type: TaskStepType::Navigate,
                    location: destination.to_string(),
                    fuel_cost: fuel_needed_to_dest,
                    description: format!("Navigate to {} from fuel station ({:.1} units)", destination, distance_to_dest),
                });
                total_fuel += fuel_needed_to_dest;

                o_debug!("🛣️ Multi-hop route planned: {} -> {} -> {} (total fuel: {})", 
                        ship.nav.waypoint_symbol, fuel_station.symbol, destination, total_fuel);
            }
            None => {
                return Err(format!("No reachable fuel stations found. Available fuel: {}, stations: {}", 
                          available_fuel, fuel_stations.len()).into());
            }
        }

        Ok((steps, total_fuel))
    }

    /// Find fuel stations (marketplaces) in a system
    async fn find_fuel_stations_in_system(&mut self, system_symbol: &str) -> Result<Vec<Waypoint>, Box<dyn std::error::Error>> {
        // Check cache first
        if let Some(waypoints) = self.waypoint_cache.get(system_symbol) {
            let fuel_stations: Vec<_> = waypoints.iter()
                .filter(|w| w.traits.iter().any(|t| t.symbol == "MARKETPLACE"))
                .cloned()
                .collect();
            return Ok(fuel_stations);
        }

        // Fetch from API if not cached
        match self.client.get_system_waypoints(system_symbol, None).await {
            Ok(waypoints) => {
                let fuel_stations: Vec<_> = waypoints.iter()
                    .filter(|w| w.traits.iter().any(|t| t.symbol == "MARKETPLACE"))
                    .cloned()
                    .collect();
                
                self.waypoint_cache.insert(system_symbol.to_string(), waypoints);
                Ok(fuel_stations)
            }
            Err(e) => {
                o_debug!("⚠️ Failed to fetch waypoints for fuel stations in {}: {}", system_symbol, e);
                Ok(Vec::new())
            }
        }
    }

    /// Calculate distance from ship's current position to waypoint
    fn calculate_distance_from_current_position(&self, ship: &Ship, waypoint: &Waypoint) -> f64 {
        let ship_x = ship.nav.route.destination.x as f64;
        let ship_y = ship.nav.route.destination.y as f64;
        let wp_x = waypoint.x as f64;
        let wp_y = waypoint.y as f64;

        ((wp_x - ship_x).powi(2) + (wp_y - ship_y).powi(2)).sqrt()
    }

    /// Calculate distance between ship and waypoint
    fn calculate_distance(&self, ship: &Ship, destination: &Waypoint) -> f64 {
        let ship_x = ship.nav.route.destination.x as f64;
        let ship_y = ship.nav.route.destination.y as f64;
        let dest_x = destination.x as f64;
        let dest_y = destination.y as f64;

        ((dest_x - ship_x).powi(2) + (dest_y - ship_y).powi(2)).sqrt()
    }

    /// Get waypoint information (with caching)
    async fn get_waypoint_info(&mut self, waypoint_symbol: &str) -> Result<Option<Waypoint>, Box<dyn std::error::Error>> {
        // Extract system symbol from waypoint (e.g., "X1-N5-BA5F" -> "X1-N5")
        let system_symbol = waypoint_symbol.split('-').take(2).collect::<Vec<_>>().join("-");

        // Check cache first
        if let Some(waypoints) = self.waypoint_cache.get(&system_symbol) {
            return Ok(waypoints.iter().find(|w| w.symbol == waypoint_symbol).cloned());
        }

        // Fetch from API if not cached
        match self.client.get_system_waypoints(&system_symbol, None).await {
            Ok(waypoints) => {
                let found = waypoints.iter().find(|w| w.symbol == waypoint_symbol).cloned();
                self.waypoint_cache.insert(system_symbol, waypoints);
                Ok(found)
            }
            Err(e) => {
                o_debug!("⚠️ Failed to fetch waypoint info for {}: {}", waypoint_symbol, e);
                Ok(None)
            }
        }
    }
}