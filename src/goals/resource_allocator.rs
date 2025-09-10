// Resource Allocator - Assigns ships and resources to goals
use crate::goals::{GoalContext, Goal};
use crate::{o_debug, o_info};
use std::collections::HashMap;

pub struct ResourceAllocator {
    ship_assignments: HashMap<String, String>, // ship_id -> goal_id
}

impl ResourceAllocator {
    pub fn new() -> Self {
        Self {
            ship_assignments: HashMap::new(),
        }
    }

    /// Allocate ships to a goal based on its requirements
    pub fn allocate_ships(&mut self, goal: &dyn Goal, context: &GoalContext) -> Result<Vec<String>, String> {
        let required_resources = goal.required_resources();
        let mut allocated_ships = Vec::new();
        
        o_debug!("🎯 Allocating resources for goal: {}", goal.description());
        o_debug!("📋 Required resources: {:?}", required_resources);
        
        for resource_type in required_resources {
            match resource_type.as_str() {
                "mining_ship" => {
                    if let Some(ship) = self.find_available_mining_ship(context) {
                        allocated_ships.push(ship.clone());
                        self.ship_assignments.insert(ship.clone(), goal.id());
                        o_debug!("⛏️ Allocated mining ship: {}", ship);
                    } else {
                        return Err("No available mining ships".to_string());
                    }
                }
                
                "hauler_ship" => {
                    if let Some(ship) = self.find_available_hauler_ship(context) {
                        allocated_ships.push(ship.clone());
                        self.ship_assignments.insert(ship.clone(), goal.id());
                        o_debug!("🚛 Allocated hauler ship: {}", ship);
                    } else {
                        return Err("No available hauler ships".to_string());
                    }
                }
                
                "probe_ship" => {
                    if let Some(ship) = self.find_available_probe_ship(context) {
                        allocated_ships.push(ship.clone());
                        self.ship_assignments.insert(ship.clone(), goal.id());
                        o_debug!("🛰️ Allocated probe ship: {}", ship);
                    } else {
                        return Err("No available probe ships".to_string());
                    }
                }
                
                "ship_with_cargo" => {
                    if let Some(ship) = self.find_ship_with_cargo(context) {
                        allocated_ships.push(ship.clone());
                        self.ship_assignments.insert(ship.clone(), goal.id());
                        o_debug!("📦 Allocated cargo ship: {}", ship);
                    } else {
                        return Err("No ships with cargo available".to_string());
                    }
                }
                
                "credits" => {
                    o_debug!("💰 Credits available: {}", context.available_credits);
                    // Credits are handled at the goal level, not ship allocation
                }
                
                "cargo_space" => {
                    // Find ship with available cargo space
                    if let Some(ship) = self.find_ship_with_cargo_space(context) {
                        if !allocated_ships.contains(&ship) {
                            allocated_ships.push(ship.clone());
                            self.ship_assignments.insert(ship.clone(), goal.id());
                            o_debug!("📦 Allocated ship with cargo space: {}", ship);
                        }
                    } else {
                        return Err("No ships with available cargo space".to_string());
                    }
                }
                
                _ => {
                    o_debug!("⚠️ Unknown resource type: {}", resource_type);
                }
            }
        }
        
        if allocated_ships.is_empty() {
            return Err(format!("Could not allocate any ships for goal: {}", goal.description()));
        }
        
        o_info!("✅ Allocated {} ships to goal: {}", allocated_ships.len(), goal.description());
        Ok(allocated_ships)
    }

    /// Release ships from a completed or failed goal
    pub fn release_ships(&mut self, goal_id: &str) -> Vec<String> {
        let released_ships: Vec<String> = self.ship_assignments.iter()
            .filter(|(_, assigned_goal_id)| *assigned_goal_id == goal_id)
            .map(|(ship_id, _)| ship_id.clone())
            .collect();
        
        for ship_id in &released_ships {
            self.ship_assignments.remove(ship_id);
        }
        
        o_debug!("🔓 Released {} ships from goal: {}", released_ships.len(), goal_id);
        released_ships
    }

    /// Check if a ship is available for allocation
    pub fn is_ship_available(&self, ship_id: &str, context: &GoalContext) -> bool {
        // Check if already assigned to another goal
        if self.ship_assignments.contains_key(ship_id) {
            return false;
        }
        
        // Check if ship is busy in fleet status
        if context.fleet_status.busy_ships.contains_key(ship_id) {
            return false;
        }
        
        // Check if ship is available in fleet status
        context.fleet_status.available_ships.contains(&ship_id.to_string())
    }

    /// Get current ship assignments for monitoring
    pub fn get_assignments(&self) -> &HashMap<String, String> {
        &self.ship_assignments
    }

    /// Get allocation statistics
    pub fn get_allocation_stats(&self) -> AllocationStats {
        AllocationStats {
            total_assigned_ships: self.ship_assignments.len(),
            assignments: self.ship_assignments.clone(),
        }
    }

    // Private helper methods
    
    fn find_available_mining_ship(&self, context: &GoalContext) -> Option<String> {
        context.fleet_status.mining_ships.iter()
            .find(|ship_id| self.is_ship_available(ship_id, context))
            .cloned()
    }
    
    fn find_available_hauler_ship(&self, context: &GoalContext) -> Option<String> {
        context.fleet_status.hauler_ships.iter()
            .find(|ship_id| self.is_ship_available(ship_id, context))
            .cloned()
    }
    
    fn find_available_probe_ship(&self, context: &GoalContext) -> Option<String> {
        context.fleet_status.probe_ships.iter()
            .find(|ship_id| self.is_ship_available(ship_id, context))
            .cloned()
    }
    
    fn find_ship_with_cargo(&self, context: &GoalContext) -> Option<String> {
        context.ships.iter()
            .find(|ship| {
                ship.cargo.units > 0 && self.is_ship_available(&ship.symbol, context)
            })
            .map(|ship| ship.symbol.clone())
    }
    
    fn find_ship_with_cargo_space(&self, context: &GoalContext) -> Option<String> {
        context.ships.iter()
            .find(|ship| {
                ship.cargo.units < ship.cargo.capacity && 
                self.is_ship_available(&ship.symbol, context)
            })
            .map(|ship| ship.symbol.clone())
    }

    /// Find the best ship for a specific goal type
    pub fn find_optimal_ship(&self, goal_type: &str, context: &GoalContext) -> Option<String> {
        match goal_type {
            "mining" => {
                // Prefer mining ships, but any ship with mining capability works
                context.ships.iter()
                    .filter(|ship| {
                        self.is_ship_available(&ship.symbol, context) &&
                        (ship.registration.role.contains("EXCAVATOR") ||
                         ship.mounts.iter().any(|m| m.symbol.contains("MINING")))
                    })
                    .max_by_key(|ship| ship.cargo.capacity) // Prefer larger cargo
                    .map(|ship| ship.symbol.clone())
            }
            
            "trading" => {
                // Prefer ships with large cargo capacity
                context.ships.iter()
                    .filter(|ship| self.is_ship_available(&ship.symbol, context))
                    .max_by_key(|ship| ship.cargo.capacity)
                    .map(|ship| ship.symbol.clone())
            }
            
            "exploration" => {
                // Prefer probe/satellite ships, or fast ships
                context.ships.iter()
                    .filter(|ship| self.is_ship_available(&ship.symbol, context))
                    .find(|ship| {
                        ship.registration.role.contains("SATELLITE") ||
                        ship.registration.role.contains("PROBE")
                    })
                    .or_else(|| {
                        // Fallback to any available ship
                        context.ships.iter()
                            .find(|ship| self.is_ship_available(&ship.symbol, context))
                    })
                    .map(|ship| ship.symbol.clone())
            }
            
            _ => {
                // Default: find any available ship
                context.fleet_status.available_ships.iter()
                    .find(|ship_id| self.is_ship_available(ship_id, context))
                    .cloned()
            }
        }
    }

    /// Calculate resource utilization efficiency
    pub fn calculate_efficiency(&self, context: &GoalContext) -> f64 {
        if context.ships.is_empty() {
            return 0.0;
        }
        
        let total_ships = context.ships.len();
        let assigned_ships = self.ship_assignments.len();
        
        (assigned_ships as f64) / (total_ships as f64) * 100.0
    }
}

#[derive(Debug, Clone)]
pub struct AllocationStats {
    pub total_assigned_ships: usize,
    pub assignments: HashMap<String, String>, // ship_id -> goal_id
}