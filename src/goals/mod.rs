// Goals module - Intelligent goal system for development and autonomous operations
pub mod goal_types;
pub mod goal_manager;
pub mod goal_interpreter;
pub mod goal_decomposer;
pub mod resource_allocator;
pub mod context_engine;

pub use goal_types::*;
pub use goal_manager::GoalManager;
pub use goal_interpreter::GoalInterpreter;
pub use goal_decomposer::GoalDecomposer;
pub use resource_allocator::ResourceAllocator;
pub use context_engine::ContextEngine;

use crate::client::{PriorityApiClient, ApiPriority};
use crate::models::*;
use std::collections::HashMap;
use async_trait::async_trait;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GoalPriority {
    Deferred = 10,
    Exploration = 20, 
    Maintenance = 30,
    Economic = 40,
    Contract = 60,
    Urgent = 80,
    Override = 100,
}

impl From<GoalPriority> for ApiPriority {
    fn from(goal_priority: GoalPriority) -> Self {
        match goal_priority {
            GoalPriority::Deferred => ApiPriority::Deferred,
            GoalPriority::Exploration => ApiPriority::Background,
            GoalPriority::Maintenance => ApiPriority::Background,
            GoalPriority::Economic => ApiPriority::Normal,
            GoalPriority::Contract => ApiPriority::ActiveGoal,
            GoalPriority::Urgent => ApiPriority::Urgent,
            GoalPriority::Override => ApiPriority::Override,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoalStatus {
    Pending,
    Active,
    Paused,
    Completed,
    Failed(String),
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct GoalContext {
    pub ships: Vec<Ship>,
    pub agent: Agent,
    pub contracts: Vec<Contract>,
    pub known_waypoints: HashMap<String, Vec<Waypoint>>,
    pub known_markets: HashMap<String, Market>,
    pub available_credits: i32,
    pub fleet_status: FleetStatus,
}

#[derive(Debug, Clone)]
pub struct FleetStatus {
    pub available_ships: Vec<String>,
    pub busy_ships: HashMap<String, String>, // ship_symbol -> current_task
    pub mining_ships: Vec<String>,
    pub hauler_ships: Vec<String>,
    pub probe_ships: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct GoalResult {
    pub success: bool,
    pub message: String,
    pub ships_used: Vec<String>,
    pub resources_consumed: HashMap<String, i32>,
    pub credits_spent: i32,
    pub execution_time: f64,
}

#[async_trait]
pub trait Goal: Send + Sync {
    fn id(&self) -> String;
    fn description(&self) -> String;
    fn priority(&self) -> GoalPriority;
    fn status(&self) -> GoalStatus;
    fn estimated_duration(&self) -> f64; // seconds
    fn required_resources(&self) -> Vec<String>; // ship types, materials, etc.
    
    async fn validate(&self, context: &GoalContext) -> Result<bool, String>;
    async fn execute(&mut self, client: &PriorityApiClient, context: &GoalContext) -> Result<GoalResult, Box<dyn std::error::Error>>;
    async fn can_interrupt(&self) -> bool { false }
    async fn pause(&mut self) -> Result<(), String> { Ok(()) }
    async fn resume(&mut self) -> Result<(), String> { Ok(()) }
    async fn cancel(&mut self) -> Result<(), String> { Ok(()) }
}