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
    
    /// Check if a ship has enough fuel for navigation to destination
    /// Now supports both direct navigation and suggests intermediate fuel stops
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
        
        // Account for orbit cost if currently docked
        let available_fuel = if ship.nav.status == "DOCKED" {
            ship.fuel.current - 1
        } else {
            ship.fuel.current
        };
        
        // Check if direct navigation is possible
        if available_fuel >= fuel_needed_to_dest {
            return Ok(NavigationSafetyCheck {
                is_safe: true,
                fuel_needed: fuel_needed_to_dest,
                current_fuel: ship.fuel.current,
                reason: format!("Direct navigation possible: {} fuel available, {} needed", available_fuel, fuel_needed_to_dest),
                nearest_fuel_source: None,
            });
        }
        
        // Direct navigation not possible - find nearest fuel station for multi-hop
        let marketplaces = self.find_nearest_waypoints_with_trait(
            system_symbol, 
            &ship.nav.route.destination,
            "MARKETPLACE"
        ).await?;
        
        if marketplaces.is_empty() {
            return Ok(NavigationSafetyCheck {
                is_safe: false,
                fuel_needed: fuel_needed_to_dest,
                current_fuel: ship.fuel.current,
                reason: "No fuel stations found in system for refueling".to_string(),
                nearest_fuel_source: None,
            });
        }
        
        // Find a fuel station we can reach with current fuel
        for (marketplace, distance_to_fuel) in &marketplaces {
            let fuel_needed_to_station = Self::estimate_fuel_cost(*distance_to_fuel);
            
            if available_fuel >= fuel_needed_to_station + 10 { // 10 fuel safety margin
                return Ok(NavigationSafetyCheck {
                    is_safe: false, // Not safe for direct navigation
                    fuel_needed: fuel_needed_to_dest, // Still report the direct fuel needed
                    current_fuel: ship.fuel.current,
                    reason: format!("Multi-hop route needed: {} fuel available, {} needed for direct route. Can refuel at {}", 
                                   available_fuel, fuel_needed_to_dest, marketplace.symbol),
                    nearest_fuel_source: Some(marketplace.symbol.clone()),
                });
            }
        }
        
        // No reachable fuel stations
        let (nearest_marketplace, distance_to_fuel) = &marketplaces[0];
        let fuel_needed_to_station = Self::estimate_fuel_cost(*distance_to_fuel);
        
        Ok(NavigationSafetyCheck {
            is_safe: false,
            fuel_needed: fuel_needed_to_dest,
            current_fuel: ship.fuel.current,
            reason: format!("No reachable fuel stations: {} fuel available, need {} to reach nearest station at {}", 
                           available_fuel, fuel_needed_to_station, nearest_marketplace.symbol),
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