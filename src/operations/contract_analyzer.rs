// Contract Analyzer - Analyzes contracts to determine required capabilities and trigger ship role changes
use crate::models::Contract;
use crate::client::priority_client::PriorityApiClient;
use crate::goals::goal_types::ShipRoleGoal;
use crate::goals::{Goal, GoalPriority, GoalStatus};
use crate::{o_debug, o_info};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ContractRequirements {
    pub contract_id: String,
    pub requires_mining: bool,
    pub requires_refining: bool,
    pub requires_hauling: bool,
    pub raw_materials: Vec<String>,      // e.g., "IRON_ORE", "COPPER_ORE"
    pub refined_materials: Vec<String>,  // e.g., "IRON", "COPPER"
    pub recommended_ships: RecommendedShips,
}

#[derive(Debug, Clone)]
pub struct RecommendedShips {
    pub miners_needed: i32,
    pub haulers_needed: i32,
    pub refiners_needed: i32,
}

#[derive(Debug, Clone)]
pub struct FleetCapabilities {
    pub active_miners: i32,
    pub active_haulers: i32, 
    pub active_refiners: i32,
    pub total_cargo_capacity: i32,
    pub ships_with_refinery: Vec<String>,
}

pub struct ContractAnalyzer {
    // Mapping of raw materials to their refined counterparts
    refining_pairs: HashMap<String, String>,
}

impl ContractAnalyzer {
    pub fn new() -> Self {
        let mut refining_pairs = HashMap::new();
        
        // Common ore -> metal refining pairs
        refining_pairs.insert("IRON_ORE".to_string(), "IRON".to_string());
        refining_pairs.insert("COPPER_ORE".to_string(), "COPPER".to_string());
        refining_pairs.insert("ALUMINUM_ORE".to_string(), "ALUMINUM".to_string());
        refining_pairs.insert("SILVER_ORE".to_string(), "SILVER".to_string());
        refining_pairs.insert("GOLD_ORE".to_string(), "GOLD".to_string());
        refining_pairs.insert("PLATINUM_ORE".to_string(), "PLATINUM".to_string());
        refining_pairs.insert("TITANIUM_ORE".to_string(), "TITANIUM".to_string());
        refining_pairs.insert("URANIUM_ORE".to_string(), "URANIUM".to_string());
        
        Self {
            refining_pairs,
        }
    }

    pub async fn analyze_contract_requirements(&self, contract: &Contract) -> ContractRequirements {
        o_info!("🔍 Analyzing contract requirements: {}", contract.id);
        
        let mut raw_materials = Vec::new();
        let mut refined_materials = Vec::new();
        let mut requires_mining = false;
        let mut requires_refining = false;
        let mut requires_hauling = false;
        
        let total_units_required: i32 = contract.terms.deliver.iter()
            .map(|d| d.units_required)
            .sum();
        
        for delivery in &contract.terms.deliver {
            let material = &delivery.trade_symbol;
            
            o_debug!("  📦 Required: {} x{}", material, delivery.units_required);
            
            // Check if this is a raw material (ore)
            if material.ends_with("_ORE") {
                raw_materials.push(material.clone());
                requires_mining = true;
                o_debug!("    ⛏️ Raw material detected - mining required");
            }
            // Check if this is a refined material that can be made from ore
            else if self.refining_pairs.values().any(|refined| refined == material) {
                refined_materials.push(material.clone());
                requires_refining = true;
                o_debug!("    🏭 Refined material detected - refining required");
                
                // Also need to mine the corresponding ore
                if let Some(ore) = self.refining_pairs.iter()
                    .find(|(_, refined)| *refined == material)
                    .map(|(ore, _)| ore.clone()) {
                    if !raw_materials.contains(&ore) {
                        raw_materials.push(ore.clone());
                        requires_mining = true;
                        o_debug!("    ⛏️ Will need to mine {} to produce {}", ore, material);
                    }
                }
            }
            // Other materials might require hauling/trading
            else {
                requires_hauling = true;
                o_debug!("    🚛 Trade material detected - hauling/trading required");
            }
        }
        
        // Calculate recommended ship counts
        let recommended_ships = self.calculate_recommended_ships(total_units_required, requires_mining, requires_refining);
        
        o_info!("📊 Contract analysis complete:");
        o_info!("  ⛏️ Mining required: {}", requires_mining);
        o_info!("  🏭 Refining required: {}", requires_refining);
        o_info!("  🚛 Hauling required: {}", requires_hauling);
        o_info!("  📝 Raw materials: {:?}", raw_materials);
        o_info!("  🔧 Refined materials: {:?}", refined_materials);
        o_info!("  🚢 Recommended fleet: {} miners, {} refiners, {} haulers", 
               recommended_ships.miners_needed, recommended_ships.refiners_needed, recommended_ships.haulers_needed);
        
        ContractRequirements {
            contract_id: contract.id.clone(),
            requires_mining,
            requires_refining,
            requires_hauling,
            raw_materials,
            refined_materials,
            recommended_ships,
        }
    }

    fn calculate_recommended_ships(&self, total_units: i32, requires_mining: bool, requires_refining: bool) -> RecommendedShips {
        let mut miners_needed = 0;
        let mut refiners_needed = 0;
        let haulers_needed;
        
        if requires_mining {
            // Rule of thumb: 1 miner per 100 units, minimum 1
            miners_needed = ((total_units / 100).max(1)).min(3); // Cap at 3 miners
        }
        
        if requires_refining {
            // Usually need at least 1 refiner for any refining contract
            refiners_needed = 1;
            
            // For very large contracts, might need more refiners
            if total_units > 500 {
                refiners_needed = 2;
            }
        }
        
        // Always good to have at least 1 hauler for consolidation
        haulers_needed = 1;
        
        RecommendedShips {
            miners_needed,
            haulers_needed,
            refiners_needed,
        }
    }

    pub async fn analyze_current_fleet(&self, client: &PriorityApiClient) -> Result<FleetCapabilities, Box<dyn std::error::Error>> {
        let ships = client.get_ships().await?;
        
        let mut active_miners = 0;
        let mut active_haulers = 0;
        let mut active_refiners = 0;
        let mut total_cargo_capacity = 0;
        let mut ships_with_refinery = Vec::new();
        
        for ship in &ships {
            total_cargo_capacity += ship.cargo.capacity;
            
            // Check ship capabilities
            let has_mining = ship.mounts.iter().any(|m| 
                m.symbol.contains("MINING") || m.symbol.contains("LASER")
            );
            let has_refinery = ship.modules.iter().any(|m| 
                m.symbol.contains("REFINERY")
            );
            let is_hauler = ship.cargo.capacity >= 30 && !has_mining;
            
            if has_refinery {
                active_refiners += 1;
                ships_with_refinery.push(ship.symbol.clone());
            } else if has_mining {
                active_miners += 1;
            } else if is_hauler {
                active_haulers += 1;
            }
        }
        
        Ok(FleetCapabilities {
            active_miners,
            active_haulers,
            active_refiners,
            total_cargo_capacity,
            ships_with_refinery,
        })
    }

    pub async fn generate_required_ship_roles(&self, 
        requirements: &ContractRequirements, 
        fleet_capabilities: &FleetCapabilities
    ) -> Vec<Box<dyn Goal>> {
        let mut goals = Vec::new();
        
        o_info!("🎯 Analyzing fleet gaps for contract {}:", requirements.contract_id);
        o_info!("  Current fleet: {} miners, {} refiners, {} haulers", 
               fleet_capabilities.active_miners, fleet_capabilities.active_refiners, fleet_capabilities.active_haulers);
        o_info!("  Required fleet: {} miners, {} refiners, {} haulers",
               requirements.recommended_ships.miners_needed, requirements.recommended_ships.refiners_needed, 
               requirements.recommended_ships.haulers_needed);
        
        // Check if we need more refiners
        if requirements.requires_refining && fleet_capabilities.active_refiners < requirements.recommended_ships.refiners_needed {
            let refiners_to_create = requirements.recommended_ships.refiners_needed - fleet_capabilities.active_refiners;
            
            o_info!("🏭 REFINER GAP DETECTED: Need {} refiners, have {}", 
                   requirements.recommended_ships.refiners_needed, fleet_capabilities.active_refiners);
            
            for i in 0..refiners_to_create {
                let goal = ShipRoleGoal {
                    id: format!("auto_designate_refiner_{}_{}", requirements.contract_id, i),
                    target_ship: None, // Let system find best candidate
                    desired_role: "refiner".to_string(),
                    priority: GoalPriority::Urgent, // High priority for contract requirements
                    status: GoalStatus::Pending,
                };
                
                o_info!("  📋 Generated goal: Find and designate refiner #{}", i + 1);
                goals.push(Box::new(goal) as Box<dyn Goal>);
            }
        }
        
        // Check if we need more haulers (less critical, lower priority)
        if requirements.requires_hauling && fleet_capabilities.active_haulers < requirements.recommended_ships.haulers_needed {
            let haulers_to_create = requirements.recommended_ships.haulers_needed - fleet_capabilities.active_haulers;
            
            o_info!("🚛 Could use more haulers: Need {}, have {}", 
                   requirements.recommended_ships.haulers_needed, fleet_capabilities.active_haulers);
            
            for i in 0..haulers_to_create.min(1) { // Only suggest 1 hauler max
                let goal = ShipRoleGoal {
                    id: format!("auto_designate_hauler_{}_{}", requirements.contract_id, i),
                    target_ship: None,
                    desired_role: "hauler".to_string(),
                    priority: GoalPriority::Economic,
                    status: GoalStatus::Pending,
                };
                
                o_info!("  📋 Generated goal: Find and designate hauler #{}", i + 1);
                goals.push(Box::new(goal) as Box<dyn Goal>);
            }
        }
        
        if goals.is_empty() {
            o_info!("✅ Fleet composition is adequate for contract requirements");
        } else {
            o_info!("🎯 Generated {} ship role goals to meet contract requirements", goals.len());
        }
        
        goals
    }

    pub async fn auto_analyze_and_suggest(&self, 
        client: &PriorityApiClient, 
        contract: &Contract
    ) -> Result<Vec<Box<dyn Goal>>, Box<dyn std::error::Error>> {
        o_info!("🤖 Auto-analyzing contract and suggesting ship roles...");
        
        // Analyze what the contract requires
        let requirements = self.analyze_contract_requirements(contract).await;
        
        // Analyze current fleet capabilities
        let fleet_capabilities = self.analyze_current_fleet(client).await?;
        
        // Generate goals to fill gaps
        let goals = self.generate_required_ship_roles(&requirements, &fleet_capabilities).await;
        
        Ok(goals)
    }
}