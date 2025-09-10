// Goal Manager - Manages goal queue and execution
use crate::goals::{Goal, GoalPriority, GoalStatus, GoalContext, GoalResult, FleetStatus};
use crate::client::PriorityApiClient;
use crate::{o_debug, o_info, o_error};
use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;

struct QueuedGoal {
    goal: Box<dyn Goal>,
    queued_at: std::time::Instant,
}

impl PartialEq for QueuedGoal {
    fn eq(&self, other: &Self) -> bool {
        self.goal.priority() == other.goal.priority() && self.goal.id() == other.goal.id()
    }
}

impl Eq for QueuedGoal {}

impl PartialOrd for QueuedGoal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueuedGoal {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority first, then FIFO for same priority
        self.goal.priority().cmp(&other.goal.priority())
            .then_with(|| other.queued_at.cmp(&self.queued_at))
    }
}

pub struct GoalManager {
    goal_queue: BinaryHeap<QueuedGoal>,
    active_goals: HashMap<String, Box<dyn Goal>>,
    completed_goals: Vec<(String, GoalResult)>,
    failed_goals: Vec<(String, String)>, // goal_id, error_message
    max_concurrent_goals: usize,
}

impl GoalManager {
    pub fn new() -> Self {
        Self {
            goal_queue: BinaryHeap::new(),
            active_goals: HashMap::new(),
            completed_goals: Vec::new(),
            failed_goals: Vec::new(),
            max_concurrent_goals: 3, // Can run up to 3 goals concurrently
        }
    }

    /// Add a goal to the execution queue
    pub fn add_goal(&mut self, goal: Box<dyn Goal>) {
        let goal_id = goal.id();
        let priority = goal.priority();
        
        o_info!("📋 Adding goal to queue: {} [{}] (priority: {})", goal.description(), goal_id, priority as u8);
        
        self.goal_queue.push(QueuedGoal {
            goal,
            queued_at: std::time::Instant::now(),
        });
        
        o_debug!("📊 Goal queue status: {} queued, {} active", 
                self.goal_queue.len(), self.active_goals.len());
    }

    /// Execute the next highest priority goals
    pub async fn execute_goals(&mut self, client: &PriorityApiClient, context: &GoalContext) -> Result<Vec<GoalResult>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();
        
        // Clean up completed active goals first
        self.cleanup_completed_goals();
        
        // Start new goals if we have capacity and queued goals
        while self.active_goals.len() < self.max_concurrent_goals && !self.goal_queue.is_empty() {
            if let Some(queued_goal) = self.goal_queue.pop() {
                let goal = queued_goal.goal;
                let goal_id = goal.id();
                
                o_info!("🚀 Starting goal execution: {}", goal.description());
                
                // Validate goal before execution
                match goal.validate(context).await {
                    Ok(true) => {
                        // Move goal to active execution
                        self.active_goals.insert(goal_id.clone(), goal);
                        o_debug!("✅ Goal validated and activated: {}", goal_id);
                    }
                    Ok(false) => {
                        o_info!("⏸️ Goal validation failed (will retry later): {}", goal_id);
                        // Put goal back in queue for later retry
                        self.goal_queue.push(QueuedGoal {
                            goal,
                            queued_at: std::time::Instant::now(),
                        });
                        break; // Don't try more goals for now
                    }
                    Err(e) => {
                        o_error!("❌ Goal validation error: {} - {}", goal_id, e);
                        self.failed_goals.push((goal_id, e));
                        continue; // Try next goal
                    }
                }
            }
        }
        
        // Execute active goals (in practice, this would be concurrent)
        // For now, we'll execute them sequentially for simplicity
        let active_goal_ids: Vec<String> = self.active_goals.keys().cloned().collect();
        
        for goal_id in active_goal_ids {
            if let Some(mut goal) = self.active_goals.remove(&goal_id) {
                o_debug!("⚡ Executing goal: {}", goal.description());
                
                match goal.execute(client, context).await {
                    Ok(result) => {
                        o_info!("✅ Goal completed: {} - {}", goal_id, result.message);
                        results.push(result.clone());
                        self.completed_goals.push((goal_id, result));
                    }
                    Err(e) => {
                        o_error!("❌ Goal execution failed: {} - {}", goal_id, e);
                        self.failed_goals.push((goal_id, e.to_string()));
                    }
                }
            }
        }
        
        Ok(results)
    }

    /// Check if there are any goals pending or active
    pub fn has_pending_goals(&self) -> bool {
        !self.goal_queue.is_empty() || !self.active_goals.is_empty()
    }

    /// Get queue status for monitoring
    pub fn get_status(&self) -> GoalManagerStatus {
        let queued_priorities: Vec<GoalPriority> = self.goal_queue.iter()
            .map(|qg| qg.goal.priority())
            .collect();
        
        let active_priorities: Vec<GoalPriority> = self.active_goals.values()
            .map(|g| g.priority())
            .collect();
        
        GoalManagerStatus {
            queued_count: self.goal_queue.len(),
            active_count: self.active_goals.len(),
            completed_count: self.completed_goals.len(),
            failed_count: self.failed_goals.len(),
            highest_queued_priority: queued_priorities.iter().max().copied(),
            active_priorities,
        }
    }

    /// Get descriptions of currently active goals
    pub fn get_active_goal_descriptions(&self) -> Vec<String> {
        self.active_goals.values()
            .map(|g| g.description())
            .collect()
    }

    /// Cancel a goal by ID
    pub async fn cancel_goal(&mut self, goal_id: &str) -> Result<bool, String> {
        // Try to cancel from active goals first
        if let Some(mut goal) = self.active_goals.remove(goal_id) {
            goal.cancel().await?;
            o_info!("🛑 Cancelled active goal: {}", goal_id);
            return Ok(true);
        }
        
        // Try to remove from queue
        let mut temp_goals = Vec::new();
        let mut found = false;
        
        while let Some(queued_goal) = self.goal_queue.pop() {
            if queued_goal.goal.id() == goal_id {
                found = true;
                o_info!("🛑 Removed queued goal: {}", goal_id);
                break;
            } else {
                temp_goals.push(queued_goal);
            }
        }
        
        // Restore queue
        for goal in temp_goals {
            self.goal_queue.push(goal);
        }
        
        if !found {
            return Err(format!("Goal not found: {}", goal_id));
        }
        
        Ok(true)
    }

    /// Pause all goals (for system maintenance, etc.)
    pub async fn pause_all_goals(&mut self) -> Result<(), String> {
        o_info!("⏸️ Pausing all active goals");
        
        for goal in self.active_goals.values_mut() {
            goal.pause().await?;
        }
        
        Ok(())
    }

    /// Resume all paused goals
    pub async fn resume_all_goals(&mut self) -> Result<(), String> {
        o_info!("▶️ Resuming all paused goals");
        
        for goal in self.active_goals.values_mut() {
            goal.resume().await?;
        }
        
        Ok(())
    }

    /// Clear completed and failed goals (for cleanup)
    pub fn clear_history(&mut self) {
        let completed_count = self.completed_goals.len();
        let failed_count = self.failed_goals.len();
        
        self.completed_goals.clear();
        self.failed_goals.clear();
        
        o_info!("🧹 Cleared goal history: {} completed, {} failed", completed_count, failed_count);
    }

    /// Build context from current game state for goal execution
    pub async fn build_context(&self, client: &PriorityApiClient) -> Result<GoalContext, Box<dyn std::error::Error>> {
        o_debug!("🔄 Building goal execution context...");
        
        // Get current game state
        let agent = client.get_agent().await?;
        let ships = client.get_ships().await?;
        let contracts = client.get_contracts().await?;
        
        // Analyze fleet status
        let fleet_status = self.analyze_fleet_status(&ships);
        
        let credits = agent.credits as i32;
        Ok(GoalContext {
            ships,
            agent,
            contracts,
            known_waypoints: HashMap::new(), // Would be populated from cache in full implementation
            known_markets: HashMap::new(),   // Would be populated from cache in full implementation
            available_credits: credits,
            fleet_status,
        })
    }

    fn analyze_fleet_status(&self, ships: &[crate::models::Ship]) -> FleetStatus {
        let mut mining_ships = Vec::new();
        let mut hauler_ships = Vec::new();
        let mut probe_ships = Vec::new();
        let mut available_ships = Vec::new();
        let mut busy_ships = HashMap::new();
        
        for ship in ships {
            // Classify ships by type/capability
            if ship.registration.role.contains("EXCAVATOR") || 
               ship.mounts.iter().any(|m| m.symbol.contains("MINING")) {
                mining_ships.push(ship.symbol.clone());
            } else if ship.registration.role.contains("HAULER") || ship.cargo.capacity > 40 {
                hauler_ships.push(ship.symbol.clone());
            } else if ship.registration.role.contains("SATELLITE") || ship.registration.role.contains("PROBE") {
                probe_ships.push(ship.symbol.clone());
            }
            
            // Check if ship is available or busy
            if ship.nav.status == "IN_TRANSIT" {
                busy_ships.insert(ship.symbol.clone(), "navigating".to_string());
            } else if ship.fuel.current < 10 {
                busy_ships.insert(ship.symbol.clone(), "needs_fuel".to_string());
            } else {
                available_ships.push(ship.symbol.clone());
            }
        }
        
        FleetStatus {
            available_ships,
            busy_ships,
            mining_ships,
            hauler_ships,
            probe_ships,
        }
    }

    fn cleanup_completed_goals(&mut self) {
        let completed: Vec<String> = self.active_goals.iter()
            .filter(|(_, goal)| matches!(goal.status(), GoalStatus::Completed | GoalStatus::Failed(_)))
            .map(|(id, _)| id.clone())
            .collect();
        
        for goal_id in completed {
            self.active_goals.remove(&goal_id);
        }
    }
}

#[derive(Debug, Clone)]
pub struct GoalManagerStatus {
    pub queued_count: usize,
    pub active_count: usize,
    pub completed_count: usize,
    pub failed_count: usize,
    pub highest_queued_priority: Option<GoalPriority>,
    pub active_priorities: Vec<GoalPriority>,
}