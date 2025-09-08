// Ship prioritization system for optimal task assignment
use crate::client::SpaceTradersClient;
use crate::models::*;
use crate::operations::ship_actor::ShipActorStatus;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ShipPerformanceMetrics {
    pub ship_symbol: String,
    pub contract_contribution: f64,  // How much this ship contributes to contract completion
    pub income_generation: f64,      // Credits earned per hour
    pub efficiency_score: f64,       // Overall efficiency rating
    pub priority_weight: f64,        // Final priority for task assignment
    pub status: ShipActorStatus,
    pub capabilities: ShipCapabilities,
}

#[derive(Debug, Clone)]
pub struct ShipCapabilities {
    pub can_mine: bool,
    pub can_trade: bool,
    pub can_explore: bool,
    pub can_haul: bool,
    pub mining_power: i32,         // Mining laser strength or 0
    pub cargo_capacity: i32,
    pub fuel_capacity: i32,
}

pub struct ShipPrioritizer {
    client: SpaceTradersClient,
    performance_history: HashMap<String, Vec<ShipPerformanceMetrics>>,
}

impl ShipPrioritizer {
    pub fn new(client: SpaceTradersClient) -> Self {
        Self {
            client,
            performance_history: HashMap::new(),
        }
    }

    pub async fn analyze_fleet_performance(&mut self, ships: &[Ship], contract: &Contract) -> Result<Vec<ShipPerformanceMetrics>, Box<dyn std::error::Error>> {
        println!("📊 Analyzing fleet performance for optimal task assignment...");
        
        let needed_materials: Vec<String> = contract.terms.deliver
            .iter()
            .map(|d| d.trade_symbol.clone())
            .collect();
        
        let mut metrics = Vec::new();
        
        for ship in ships {
            let capabilities = self.analyze_ship_capabilities(ship);
            let contract_contribution = self.calculate_contract_contribution(ship, &needed_materials, contract);
            let income_generation = self.estimate_income_generation(ship, &capabilities);
            let efficiency_score = self.calculate_efficiency_score(ship, &capabilities);
            
            let ship_metrics = ShipPerformanceMetrics {
                ship_symbol: ship.symbol.clone(),
                contract_contribution,
                income_generation,
                efficiency_score,
                priority_weight: 0.0, // Will be calculated later
                status: ShipActorStatus::Idle, // Will be updated by coordinator
                capabilities,
            };
            
            println!("📈 {} Analysis:", ship.symbol);
            println!("   Contract Contribution: {:.1}%", ship_metrics.contract_contribution * 100.0);
            println!("   Income Generation: {:.0} credits/hour", ship_metrics.income_generation);
            println!("   Efficiency Score: {:.2}", ship_metrics.efficiency_score);
            
            metrics.push(ship_metrics);
        }
        
        // Calculate relative priority weights
        self.calculate_priority_weights(&mut metrics);
        
        // Sort by priority (highest first)
        metrics.sort_by(|a, b| b.priority_weight.partial_cmp(&a.priority_weight).unwrap());
        
        println!("\n🎯 Fleet Priority Ranking:");
        for (i, metric) in metrics.iter().enumerate() {
            println!("   {}. {} - Priority: {:.2} ({})", 
                    i + 1, 
                    metric.ship_symbol, 
                    metric.priority_weight,
                    self.get_primary_role(&metric.capabilities));
        }
        
        Ok(metrics)
    }

    fn analyze_ship_capabilities(&self, ship: &Ship) -> ShipCapabilities {
        // Probes/satellites are designed for exploration, not mining
        let is_probe = ship.registration.role == "SATELLITE" || ship.frame.symbol.contains("PROBE");
        let can_mine = !is_probe && ship.mounts.iter().any(|mount| {
            mount.symbol.contains("MINING") || mount.symbol.contains("EXTRACTOR")
        });
        
        let mining_power = if is_probe {
            0 // Probes have no mining power regardless of mounts
        } else {
            ship.mounts.iter()
                .filter(|mount| mount.symbol.contains("MINING") || mount.symbol.contains("EXTRACTOR"))
                .map(|mount| mount.strength.unwrap_or(0))
                .sum()
        };
        
        let can_trade = ship.cargo.capacity >= 10; // Minimum cargo for meaningful trading
        let can_haul = ship.cargo.capacity >= 20 && !can_mine; // Large cargo, not primarily a miner
        let can_explore = ship.registration.role == "SATELLITE" || ship.frame.symbol.contains("PROBE");
        
        ShipCapabilities {
            can_mine,
            can_trade,
            can_explore,
            can_haul,
            mining_power,
            cargo_capacity: ship.cargo.capacity,
            fuel_capacity: ship.fuel.capacity,
        }
    }

    fn calculate_contract_contribution(&self, ship: &Ship, needed_materials: &[String], contract: &Contract) -> f64 {
        let capabilities = self.analyze_ship_capabilities(ship);
        
        // Determine if this contract requires mining or can be fulfilled through other means
        let requires_mining = self.contract_requires_mining(needed_materials);
        
        if requires_mining && !capabilities.can_mine {
            // This is a mining contract but ship can't mine - zero contribution
            return 0.0;
        }
        
        if !requires_mining {
            // Non-mining contract - all ships can potentially contribute through trading/transport
            // Base contribution on cargo capacity and trading capability
            if capabilities.can_trade || capabilities.can_haul {
                return (capabilities.cargo_capacity as f64 / 100.0).min(0.8);
            } else if capabilities.can_explore {
                // Probes can still transport small amounts or scout markets
                return 0.1;
            }
            return 0.0;
        }
        
        // This is a mining contract and ship can mine - calculate mining contribution
        let total_needed: i32 = contract.terms.deliver.iter().map(|d| d.units_required).sum();
        let ship_capacity = capabilities.cargo_capacity;
        
        // Estimate cycles needed to fill contract
        let cycles_per_ship = (total_needed as f64 / ship_capacity as f64).ceil();
        
        // Higher mining power = more contribution per cycle
        let mining_efficiency = capabilities.mining_power as f64 / 10.0; // Normalize to 0-1 scale roughly
        
        // Contract contribution score (0.0 to 1.0)
        let contribution = (mining_efficiency * ship_capacity as f64) / (cycles_per_ship * 100.0);
        contribution.min(1.0).max(0.0)
    }
    
    /// Determine if contract materials are typically obtained through mining
    fn contract_requires_mining(&self, needed_materials: &[String]) -> bool {
        // List of commonly mined materials in SpaceTraders
        let mineable_materials = [
            "IRON_ORE", "COPPER_ORE", "ALUMINUM_ORE", "GOLD_ORE", "PLATINUM_ORE",
            "SILVER_ORE", "URANIUM_ORE", "PRECIOUS_STONES", "QUARTZ_SAND",
            "SILICON_CRYSTALS", "DIAMONDS", "HYDROCARBON", "LIQUID_HYDROGEN"
        ];
        
        // If any needed material is mineable, this is likely a mining contract
        needed_materials.iter().any(|material| {
            mineable_materials.iter().any(|mineable| material.contains(mineable))
        })
    }

    fn estimate_income_generation(&self, ship: &Ship, capabilities: &ShipCapabilities) -> f64 {
        if capabilities.can_mine {
            // Mining income: mining power * cargo capacity * estimated cycles per hour
            let cycles_per_hour = 60.0 / 90.0; // Assume 90 seconds per mining cycle
            let estimated_value_per_unit = 50.0; // Rough estimate for mined materials
            capabilities.mining_power as f64 * capabilities.cargo_capacity as f64 * cycles_per_hour * estimated_value_per_unit
        } else if capabilities.can_trade {
            // Trading income: cargo capacity * estimated profit margin
            capabilities.cargo_capacity as f64 * 20.0 // Rough profit estimate
        } else {
            // Exploration/utility ships generate indirect value
            100.0 // Base utility value
        }
    }

    fn calculate_efficiency_score(&self, ship: &Ship, capabilities: &ShipCapabilities) -> f64 {
        let mut score = 0.0;
        
        // Cargo efficiency
        if capabilities.cargo_capacity > 0 {
            score += (capabilities.cargo_capacity as f64 / 100.0).min(1.0);
        }
        
        // Mining efficiency
        if capabilities.can_mine {
            score += (capabilities.mining_power as f64 / 20.0).min(1.0);
        }
        
        // Multi-role versatility bonus
        let roles = [capabilities.can_mine, capabilities.can_trade, capabilities.can_haul, capabilities.can_explore]
            .iter().filter(|&&x| x).count();
        score += (roles as f64 * 0.1);
        
        // Fuel efficiency
        if capabilities.fuel_capacity > 0 {
            score += (capabilities.fuel_capacity as f64 / 1000.0).min(0.5);
        }
        
        score
    }

    fn calculate_priority_weights(&self, metrics: &mut Vec<ShipPerformanceMetrics>) {
        // First pass: calculate max income for normalization
        let max_income = metrics.iter().map(|m| m.income_generation).fold(0.0f64, f64::max);
        
        // Second pass: update priority weights
        for metric in metrics.iter_mut() {
            // Weighted priority calculation
            // Contract contribution is most important (60%)
            // Income generation is secondary (25%) 
            // Efficiency score provides fine-tuning (15%)
            
            let normalized_contract = metric.contract_contribution;
            let normalized_income = if max_income > 0.0 { metric.income_generation / max_income } else { 0.0 };
            let normalized_efficiency = metric.efficiency_score / 5.0; // Assume max efficiency of 5.0
            
            metric.priority_weight = (normalized_contract * 0.6) + 
                                   (normalized_income * 0.25) + 
                                   (normalized_efficiency * 0.15);
        }
    }

    pub fn get_idle_ships(&self, fleet_metrics: &[ShipPerformanceMetrics]) -> Vec<String> {
        fleet_metrics
            .iter()
            .filter(|metrics| matches!(metrics.status, ShipActorStatus::Idle))
            .map(|metrics| metrics.ship_symbol.clone())
            .collect()
    }

    pub fn get_lowest_priority_active_ship(&self, fleet_metrics: &[ShipPerformanceMetrics]) -> Option<String> {
        fleet_metrics
            .iter()
            .filter(|metrics| matches!(metrics.status, ShipActorStatus::Working))
            .min_by(|a, b| a.priority_weight.partial_cmp(&b.priority_weight).unwrap())
            .map(|metrics| metrics.ship_symbol.clone())
    }

    pub fn should_reassign_task(&self, fleet_metrics: &[ShipPerformanceMetrics], new_task_priority: f64) -> Option<String> {
        // Find the lowest priority active ship
        if let Some(lowest_priority_ship) = self.get_lowest_priority_active_ship(fleet_metrics) {
            if let Some(lowest_metrics) = fleet_metrics.iter().find(|m| m.ship_symbol == lowest_priority_ship) {
                // If the new task is significantly more important than what the lowest priority ship is doing
                if new_task_priority > lowest_metrics.priority_weight * 1.5 { // 50% threshold
                    println!("🔄 Task reassignment recommended: {} (priority {:.2}) should yield to new task (priority {:.2})",
                            lowest_priority_ship, lowest_metrics.priority_weight, new_task_priority);
                    return Some(lowest_priority_ship);
                }
            }
        }
        None
    }


    fn get_primary_role(&self, capabilities: &ShipCapabilities) -> &str {
        if capabilities.can_mine && capabilities.mining_power > 0 {
            "Primary Miner"
        } else if capabilities.can_haul {
            "Hauler"
        } else if capabilities.can_trade {
            "Trader"
        } else if capabilities.can_explore {
            "Explorer"
        } else {
            "Utility"
        }
    }

    pub fn recommend_optimal_task(&self, ship_metrics: &ShipPerformanceMetrics, _contract: &Contract) -> String {
        if ship_metrics.capabilities.can_mine && ship_metrics.contract_contribution >= 0.05 {
            "High-priority mining".to_string()
        } else if ship_metrics.capabilities.can_haul && ship_metrics.capabilities.cargo_capacity > 30 {
            "Cargo transport".to_string()
        } else if ship_metrics.capabilities.can_explore {
            "System exploration".to_string()
        } else {
            "Support operations".to_string()
        }
    }
}