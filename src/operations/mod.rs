// Operations module - High-level game operations

pub mod ship;
pub mod mining;
pub mod trading;
pub mod contracts;
pub mod fleet;
pub mod exploration;
pub mod ship_actor;
pub mod fleet_coordinator;
pub mod ship_prioritizer;
pub mod navigation;
pub mod shipyard_operations;
pub mod task_planner;
pub mod product_knowledge;
pub mod ship_role_manager;
pub mod contract_analyzer;
pub mod iron_ore_miner;

pub use ship::*;
pub use mining::*;
pub use trading::*;
pub use contracts::*;
pub use fleet::*;
pub use exploration::*;
pub use ship_actor::*;
pub use fleet_coordinator::*;
pub use ship_prioritizer::*;
pub use navigation::*;
pub use shipyard_operations::*;
pub use task_planner::*;
pub use product_knowledge::*;
pub use ship_role_manager::*;
pub use contract_analyzer::*;
pub use iron_ore_miner::*;