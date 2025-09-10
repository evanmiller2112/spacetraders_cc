// Goal Interpreter - Parse natural language commands into executable goals
use crate::goals::{Goal, GoalPriority};
use crate::goals::goal_types::*;
use crate::{o_debug, o_info};
use std::collections::HashMap;

pub struct GoalInterpreter {
    // Keywords for different goal types
    resource_keywords: HashMap<String, Vec<String>>,
    action_keywords: HashMap<String, Vec<String>>,
    ship_type_keywords: HashMap<String, Vec<String>>,
}

impl GoalInterpreter {
    pub fn new() -> Self {
        let mut resource_keywords = HashMap::new();
        resource_keywords.insert("iron".to_string(), vec!["IRON".to_string(), "IRON_ORE".to_string()]);
        resource_keywords.insert("copper".to_string(), vec!["COPPER".to_string(), "COPPER_ORE".to_string()]);
        resource_keywords.insert("aluminum".to_string(), vec!["ALUMINUM".to_string(), "ALUMINUM_ORE".to_string()]);
        resource_keywords.insert("silver".to_string(), vec!["SILVER".to_string(), "SILVER_ORE".to_string()]);
        resource_keywords.insert("gold".to_string(), vec!["GOLD".to_string(), "GOLD_ORE".to_string()]);
        resource_keywords.insert("platinum".to_string(), vec!["PLATINUM".to_string(), "PLATINUM_ORE".to_string()]);
        
        let mut action_keywords = HashMap::new();
        action_keywords.insert("mine".to_string(), vec!["extract".to_string(), "dig".to_string(), "harvest".to_string()]);
        action_keywords.insert("refine".to_string(), vec!["process".to_string(), "smelt".to_string(), "manufacture".to_string()]);
        action_keywords.insert("sell".to_string(), vec!["trade".to_string(), "market".to_string()]);
        action_keywords.insert("buy".to_string(), vec!["purchase".to_string(), "acquire".to_string()]);
        action_keywords.insert("explore".to_string(), vec!["scout".to_string(), "discover".to_string(), "survey".to_string()]);
        action_keywords.insert("debug".to_string(), vec!["analyze".to_string(), "inspect".to_string(), "examine".to_string()]);
        
        let mut ship_type_keywords = HashMap::new();
        ship_type_keywords.insert("mining".to_string(), vec!["excavator".to_string(), "miner".to_string()]);
        ship_type_keywords.insert("hauler".to_string(), vec!["cargo".to_string(), "transport".to_string(), "freighter".to_string()]);
        ship_type_keywords.insert("probe".to_string(), vec!["scout".to_string(), "satellite".to_string(), "explorer".to_string()]);
        
        Self {
            resource_keywords,
            action_keywords,
            ship_type_keywords,
        }
    }

    pub async fn parse_goal(&self, input: &str) -> Result<Box<dyn Goal>, String> {
        let input = input.to_lowercase().trim().to_string();
        o_info!("🧠 Interpreting goal: '{}'", input);
        
        let tokens: Vec<&str> = input.split_whitespace().collect();
        if tokens.is_empty() {
            return Err("Empty goal command".to_string());
        }

        // Detect action type
        let action = self.detect_action(&tokens)?;
        o_debug!("🎯 Detected action: {}", action);

        match action.as_str() {
            "mine" => self.parse_mining_goal(&tokens).await,
            "refine" => self.parse_refining_goal(&tokens).await,
            "sell" => self.parse_selling_goal(&tokens).await,
            "buy" => self.parse_buying_goal(&tokens).await,
            "explore" => self.parse_exploration_goal(&tokens).await,
            "debug" => self.parse_debug_goal(&tokens).await,
            _ => Err(format!("Unknown action: {}", action)),
        }
    }

    fn detect_action(&self, tokens: &[&str]) -> Result<String, String> {
        for token in tokens {
            for (action, synonyms) in &self.action_keywords {
                if action == token || synonyms.contains(&token.to_string()) {
                    return Ok(action.clone());
                }
            }
        }
        
        // Try to infer action from context
        if self.contains_resource(tokens) {
            if tokens.iter().any(|&t| t.contains("station") || t.contains("refinery") || t.contains("factory")) {
                return Ok("refine".to_string());
            }
            return Ok("mine".to_string()); // Default for resource mentions
        }

        if tokens.iter().any(|&t| t.contains("ship") || t.contains("buy") || t.contains("purchase")) {
            return Ok("buy".to_string());
        }

        if tokens.iter().any(|&t| t.contains("system") || t.contains("waypoint") || t.contains("explore")) {
            return Ok("explore".to_string());
        }

        Err("Could not detect action from command".to_string())
    }

    async fn parse_mining_goal(&self, tokens: &[&str]) -> Result<Box<dyn Goal>, String> {
        let resource = self.extract_resource(tokens)?;
        let quantity = self.extract_quantity(tokens).unwrap_or(100); // Default 100 units
        
        o_debug!("⛏️ Parsing mining goal: {} units of {}", quantity, resource);
        
        Ok(Box::new(MiningGoal {
            id: format!("mine_{}_{}", resource.to_lowercase(), quantity),
            resource_type: resource,
            target_quantity: quantity,
            priority: GoalPriority::Override, // Development goals get highest priority
            status: crate::goals::GoalStatus::Pending,
        }))
    }

    async fn parse_refining_goal(&self, tokens: &[&str]) -> Result<Box<dyn Goal>, String> {
        let resource = self.extract_resource(tokens)?;
        let quantity = self.extract_quantity(tokens).unwrap_or(50); // Default 50 units
        
        o_debug!("🏭 Parsing refining goal: {} units of {}", quantity, resource);
        
        Ok(Box::new(RefiningGoal {
            id: format!("refine_{}_{}", resource.to_lowercase(), quantity),
            input_resource: format!("{}_ORE", resource.to_uppercase()),
            output_resource: resource.to_uppercase(),
            target_quantity: quantity,
            priority: GoalPriority::Override,
            status: crate::goals::GoalStatus::Pending,
        }))
    }

    async fn parse_selling_goal(&self, tokens: &[&str]) -> Result<Box<dyn Goal>, String> {
        let resource = self.extract_resource(tokens).unwrap_or_else(|_| "ALL".to_string());
        let quantity = self.extract_quantity(tokens);
        
        o_debug!("💰 Parsing selling goal: {:?} units of {}", quantity, resource);
        
        Ok(Box::new(SellingGoal {
            id: format!("sell_{}_{:?}", resource.to_lowercase(), quantity),
            resource_type: if resource == "ALL" { None } else { Some(resource) },
            target_quantity: quantity,
            priority: GoalPriority::Override,
            status: crate::goals::GoalStatus::Pending,
        }))
    }

    async fn parse_buying_goal(&self, tokens: &[&str]) -> Result<Box<dyn Goal>, String> {
        // Check if buying a ship or resource
        if tokens.iter().any(|&t| t.contains("ship") || self.ship_type_keywords.keys().any(|k| t.contains(k))) {
            let ship_type = self.extract_ship_type(tokens)?;
            
            o_debug!("🚢 Parsing ship buying goal: {}", ship_type);
            
            Ok(Box::new(ShipPurchaseGoal {
                id: format!("buy_ship_{}", ship_type.to_lowercase()),
                ship_type,
                priority: GoalPriority::Override,
                status: crate::goals::GoalStatus::Pending,
            }))
        } else {
            let resource = self.extract_resource(tokens)?;
            let quantity = self.extract_quantity(tokens).unwrap_or(50);
            
            o_debug!("🛒 Parsing resource buying goal: {} units of {}", quantity, resource);
            
            Ok(Box::new(ResourcePurchaseGoal {
                id: format!("buy_{}_{}", resource.to_lowercase(), quantity),
                resource_type: resource,
                target_quantity: quantity,
                priority: GoalPriority::Override,
                status: crate::goals::GoalStatus::Pending,
            }))
        }
    }

    async fn parse_exploration_goal(&self, tokens: &[&str]) -> Result<Box<dyn Goal>, String> {
        // Extract target (system, waypoints, shipyards, etc.)
        let target = if let Some(system) = tokens.iter().find(|&t| t.starts_with("x1-") || t.starts_with("X1-")) {
            system.to_string().to_uppercase()
        } else if tokens.iter().any(|&t| t.contains("shipyard")) {
            "SHIPYARDS".to_string()
        } else if tokens.iter().any(|&t| t.contains("market")) {
            "MARKETS".to_string()
        } else {
            "NEARBY".to_string()
        };
        
        o_debug!("🔍 Parsing exploration goal: {}", target);
        
        Ok(Box::new(ExplorationGoal {
            id: format!("explore_{}", target.to_lowercase()),
            target_type: target,
            priority: GoalPriority::Override,
            status: crate::goals::GoalStatus::Pending,
        }))
    }

    async fn parse_debug_goal(&self, tokens: &[&str]) -> Result<Box<dyn Goal>, String> {
        // Extract what to debug (ship, waypoint, contracts, etc.)
        let debug_target = if let Some(ship_id) = tokens.iter().find(|&t| t.contains("-")) {
            format!("ship:{}", ship_id.to_uppercase())
        } else if tokens.iter().any(|&t| t.contains("contract")) {
            "contracts".to_string()
        } else if tokens.iter().any(|&t| t.contains("fleet")) {
            "fleet".to_string()
        } else if tokens.iter().any(|&t| t.contains("waypoint")) {
            "waypoints".to_string()
        } else {
            "system".to_string()
        };
        
        o_debug!("🐛 Parsing debug goal: {}", debug_target);
        
        Ok(Box::new(DebugGoal {
            id: format!("debug_{}", debug_target.replace(":", "_")),
            target: debug_target,
            priority: GoalPriority::Override,
            status: crate::goals::GoalStatus::Pending,
        }))
    }

    fn contains_resource(&self, tokens: &[&str]) -> bool {
        for token in tokens {
            if self.resource_keywords.contains_key(*token) {
                return true;
            }
        }
        false
    }

    fn extract_resource(&self, tokens: &[&str]) -> Result<String, String> {
        for token in tokens {
            if let Some(resource_variants) = self.resource_keywords.get(*token) {
                return Ok(resource_variants[0].clone()); // Return canonical form
            }
        }
        Err("No resource found in command".to_string())
    }

    fn extract_quantity(&self, tokens: &[&str]) -> Option<i32> {
        for token in tokens {
            if let Ok(num) = token.parse::<i32>() {
                return Some(num);
            }
        }
        None
    }

    fn extract_ship_type(&self, tokens: &[&str]) -> Result<String, String> {
        for token in tokens {
            if let Some(_) = self.ship_type_keywords.get(*token) {
                return Ok(token.to_uppercase());
            }
        }
        
        // Default ship type inference
        if tokens.iter().any(|&t| t.contains("haul") || t.contains("cargo") || t.contains("transport")) {
            return Ok("HAULER".to_string());
        }
        if tokens.iter().any(|&t| t.contains("mine") || t.contains("excavat")) {
            return Ok("MINING".to_string());
        }
        
        Err("Could not determine ship type from command".to_string())
    }
}