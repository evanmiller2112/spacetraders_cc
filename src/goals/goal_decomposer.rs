// Goal Decomposer - Breaks complex goals into sub-goals
use crate::goals::{Goal, GoalPriority};
use crate::goals::goal_types::*;
use crate::{o_debug, o_info};

pub struct GoalDecomposer;

impl GoalDecomposer {
    pub fn new() -> Self {
        Self
    }

    /// Decompose a complex goal into executable sub-goals
    pub async fn decompose(&self, goal: Box<dyn Goal>) -> Vec<Box<dyn Goal>> {
        o_debug!("🔧 Decomposing goal: {}", goal.description());
        
        match goal.id().split('_').next() {
            Some("refine") => self.decompose_refining_goal(goal).await,
            Some("buy") => self.decompose_purchase_goal(goal).await,
            Some("explore") => self.decompose_exploration_goal(goal).await,
            _ => vec![goal], // No decomposition needed
        }
    }

    async fn decompose_refining_goal(&self, goal: Box<dyn Goal>) -> Vec<Box<dyn Goal>> {
        o_info!("🏭 Decomposing refining goal into sub-tasks");
        
        // For "refine iron" goal, we need:
        // 1. Mine iron ore (if not available)
        // 2. Find refinery location
        // 3. Transport ore to refinery
        // 4. Execute refining process
        
        let goal_id = goal.id();
        let parts: Vec<&str> = goal_id.split('_').collect();
        
        if parts.len() >= 3 {
            let resource = parts[1]; // e.g., "iron" from "refine_iron_50"
            let quantity = parts[2].parse::<i32>().unwrap_or(50);
            
            let mut sub_goals: Vec<Box<dyn Goal>> = Vec::new();
            
            // Sub-goal 1: Mine the ore
            sub_goals.push(Box::new(MiningGoal {
                id: format!("mine_{}_ore_for_refining_{}", resource, quantity),
                resource_type: format!("{}_ORE", resource.to_uppercase()),
                target_quantity: quantity,
                priority: GoalPriority::Override,
                status: crate::goals::GoalStatus::Pending,
            }));
            
            // Sub-goal 2: Transport and refine (the original goal, modified)
            sub_goals.push(goal);
            
            o_debug!("🔧 Created {} sub-goals for refining", sub_goals.len());
            return sub_goals;
        }
        
        vec![goal] // Fallback: return original goal
    }

    async fn decompose_purchase_goal(&self, goal: Box<dyn Goal>) -> Vec<Box<dyn Goal>> {
        o_debug!("💳 Decomposing purchase goal");
        
        // For ship purchases, we might need:
        // 1. Find shipyard
        // 2. Ensure sufficient funds
        // 3. Navigate to shipyard
        // 4. Purchase ship
        
        // For resource purchases:
        // 1. Find marketplace with resource
        // 2. Ensure sufficient funds and cargo space
        // 3. Navigate to marketplace
        // 4. Buy resources
        
        // For now, return as-is (would implement complex logic here)
        vec![goal]
    }

    async fn decompose_exploration_goal(&self, goal: Box<dyn Goal>) -> Vec<Box<dyn Goal>> {
        o_debug!("🔍 Decomposing exploration goal");
        
        // Exploration might involve:
        // 1. Assign probe ships
        // 2. Plan exploration route
        // 3. Execute waypoint scanning
        // 4. Report findings
        
        vec![goal] // Simplified for now
    }

    /// Check if a goal needs decomposition based on complexity
    pub fn needs_decomposition(&self, goal: &dyn Goal) -> bool {
        match goal.id().split('_').next() {
            Some("refine") => true,  // Refining usually needs ore mining first
            Some("buy") if goal.id().contains("ship") => true, // Ship buying has multiple steps
            Some("explore") => false, // Exploration is usually atomic
            _ => false,
        }
    }

    /// Estimate total execution time for decomposed goals
    pub fn estimate_total_duration(&self, goals: &[Box<dyn Goal>]) -> f64 {
        goals.iter()
            .map(|g| g.estimated_duration())
            .sum()
    }

    /// Check if sub-goals have dependencies that require sequential execution
    pub fn has_dependencies(&self, goals: &[Box<dyn Goal>]) -> bool {
        // Simple heuristic: if goals share resources or outputs, they likely have dependencies
        for i in 0..goals.len() {
            for j in (i + 1)..goals.len() {
                if self.goals_have_dependency(&*goals[i], &*goals[j]) {
                    return true;
                }
            }
        }
        false
    }

    fn goals_have_dependency(&self, goal1: &dyn Goal, goal2: &dyn Goal) -> bool {
        // Check if goal1's outputs are needed by goal2's inputs
        let goal1_id = goal1.id();
        let goal2_id = goal2.id();
        
        // Example: mining goal output feeds into refining goal input
        if goal1_id.starts_with("mine_") && goal2_id.starts_with("refine_") {
            // Extract resource names and check compatibility
            if let (Some(mine_resource), Some(refine_resource)) = (
                goal1_id.split('_').nth(1),
                goal2_id.split('_').nth(1)
            ) {
                return mine_resource == refine_resource;
            }
        }
        
        false
    }
}