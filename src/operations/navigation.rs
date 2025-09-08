// Navigation and Fuel Planning Module
// Provides intelligent wayfinding with fuel safety checks

use crate::models::*;
use crate::client::SpaceTradersClient;

pub struct NavigationPlanner {
    client: SpaceTradersClient,
}

impl NavigationPlanner {
    pub fn new(client: SpaceTradersClient) -> Self {
        Self { client }
    }
    
    /// Calculate euclidean distance between two waypoints
    pub fn calculate_distance(from: &ShipRouteWaypoint, to: &Waypoint) -> f64 {
        let dx = (to.x - from.x) as f64;
        let dy = (to.y - from.y) as f64;
        (dx * dx + dy * dy).sqrt()
    }
    
    /// Calculate distance between two waypoints (both as Waypoint structs)
    pub fn calculate_distance_waypoints(from: &Waypoint, to: &Waypoint) -> f64 {
        let dx = (to.x - from.x) as f64;
        let dy = (to.y - from.y) as f64;
        (dx * dx + dy * dy).sqrt()
    }
    
    /// Estimate fuel cost for a journey based on distance
    /// SpaceTraders fuel consumption is roughly 1 unit per unit of distance
    pub fn estimate_fuel_cost(distance: f64) -> i32 {
        // Conservative estimate: ceil the distance and add small buffer
        (distance.ceil() as i32).saturating_add(2)
    }
    
    /// Find nearest waypoints with specific traits (like MARKETPLACE)
    /// WARNING: This uses basic waypoints API which may not have complete trait data.
    /// Prefer using FleetCoordinator.find_nearest_marketplace() which uses scanning API.
    pub async fn find_nearest_waypoints_with_trait(
        &self, 
        system_symbol: &str, 
        from_position: &ShipRouteWaypoint,
        trait_name: &str
    ) -> Result<Vec<(Waypoint, f64)>, Box<dyn std::error::Error>> {
        // TODO: This should use scanning API but needs a ship symbol for scanning
        let waypoints = self.client.get_system_waypoints(system_symbol, None).await?;
        
        let mut matching_waypoints = Vec::new();
        
        for waypoint in waypoints {
            // Check if waypoint has the desired trait (use symbol, not name)
            let has_trait = waypoint.traits.iter().any(|t| t.symbol == trait_name);
            
            if has_trait {
                let distance = Self::calculate_distance(from_position, &waypoint);
                matching_waypoints.push((waypoint, distance));
            }
        }
        
        // Sort by distance
        matching_waypoints.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        
        Ok(matching_waypoints)
    }
    
    /// Check if a ship has enough fuel for a round trip to destination plus reserve for fuel source
    pub async fn can_navigate_safely(
        &self,
        ship: &Ship,
        destination_waypoint: &str
    ) -> Result<NavigationSafetyCheck, Box<dyn std::error::Error>> {
        // Get destination waypoint details
        let system_symbol = &ship.nav.system_symbol;
        let destination = self.client.get_waypoint(system_symbol, destination_waypoint).await?;
        
        // Calculate distance to destination
        let distance_to_dest = Self::calculate_distance(&ship.nav.route.destination, &destination);
        let fuel_needed_to_dest = Self::estimate_fuel_cost(distance_to_dest);
        
        // WARNING: This still uses the old implementation because NavigationPlanner
        // doesn't have access to FleetCoordinator's unified marketplace finder
        // TODO: Refactor to use unified marketplace finding
        let marketplaces = self.find_nearest_waypoints_with_trait(
            system_symbol, 
            &ShipRouteWaypoint {
                symbol: destination.symbol.clone(),
                waypoint_type: destination.waypoint_type.clone(),
                system_symbol: destination.system_symbol.clone(),
                x: destination.x,
                y: destination.y,
            },
            "MARKETPLACE"
        ).await?;
        
        if marketplaces.is_empty() {
            return Ok(NavigationSafetyCheck {
                is_safe: false,
                fuel_needed: fuel_needed_to_dest,
                current_fuel: ship.fuel.current,
                reason: "No marketplace found in system for refueling".to_string(),
                nearest_fuel_source: None,
            });
        }
        
        // Calculate fuel needed to reach nearest marketplace from destination
        let (nearest_marketplace, distance_to_fuel) = &marketplaces[0];
        let fuel_needed_to_marketplace = Self::estimate_fuel_cost(*distance_to_fuel);
        
        // Total fuel needed: to destination + to marketplace + safety buffer
        let total_fuel_needed = fuel_needed_to_dest + fuel_needed_to_marketplace + 50; // 50 unit safety buffer
        
        let is_safe = ship.fuel.current >= total_fuel_needed;
        
        Ok(NavigationSafetyCheck {
            is_safe,
            fuel_needed: total_fuel_needed,
            current_fuel: ship.fuel.current,
            reason: if is_safe {
                format!("Safe navigation: {} fuel available, {} needed", ship.fuel.current, total_fuel_needed)
            } else {
                format!("Insufficient fuel: {} available, {} needed", ship.fuel.current, total_fuel_needed)
            },
            nearest_fuel_source: Some(nearest_marketplace.symbol.clone()),
        })
    }
}

#[derive(Debug)]
pub struct NavigationSafetyCheck {
    pub is_safe: bool,
    pub fuel_needed: i32,
    pub current_fuel: i32,
    pub reason: String,
    pub nearest_fuel_source: Option<String>,
}