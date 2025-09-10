// SpaceTraders Autonomous Agent - Main Entry Point
// Modular architecture for 100% autonomous gameplay

use spacetraders_cc::{Admiral, admiral::load_agent_token, output_broker, o_error, o_info, o_debug};
use spacetraders_cc::goals::{GoalManager, GoalInterpreter, GoalDecomposer, ResourceAllocator, ContextEngine};
use spacetraders_cc::client::{PriorityApiClient};
use clap::Parser;

#[derive(Parser)]
#[command(name = "spacetraders_cc")]
#[command(about = "SpaceTraders Autonomous Agent")]
struct Args {
    #[arg(long, help = "Enable API call approval for debugging")]
    debug_api: bool,
    
    #[arg(long, help = "Log all API calls and responses to a file")]
    debug_api_log: bool,
    
    #[arg(long, help = "Enable comprehensive function call logging")]
    full_debug: bool,
    
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count, help = "Increase verbosity (-v basic, -vv full debug)")]
    verbose: u8,
    
    #[arg(short = 'g', long = "goal", help = "Execute a specific goal instead of autonomous operations (e.g., 'mine iron', 'refine copper', 'explore system')")]
    goal: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    // Initialize output broker
    output_broker::init_output_broker();
    
    // Set verbosity level in both old and new systems during transition
    spacetraders_cc::verbosity::set_verbosity_level(args.verbose);
    output_broker::get_output_broker().set_verbosity_level(args.verbose).await;
    
    o_info!("🚀 SpaceTraders Autonomous Agent Starting...");
    o_info!("🏗️  Using new modular architecture!");
    o_info!("🎯 PRIME DIRECTIVE: 100% autonomous gameplay");
    
    if args.debug_api {
        o_debug!("🐛 DEBUG MODE: API call approval enabled");
    }
    if args.debug_api_log {
        o_debug!("📝 DEBUG MODE: API call logging enabled");
    }
    
    // Load agent token
    let token = load_agent_token()?;
    
    // Create Admiral for autonomous operations
    let mut admiral = Admiral::new(token)?;
    admiral.set_debug_mode(args.debug_api);
    admiral.set_api_logging(args.debug_api_log);
    admiral.set_full_debug(args.full_debug);
    
    // Test authentication first
    let _agent = match admiral.client.get_agent().await {
        Ok(agent) => {
            o_info!("✅ Successfully authenticated!");
            o_info!("📊 Agent Info:");
            o_info!("  Symbol: {}", agent.symbol);
            o_info!("  Headquarters: {}", agent.headquarters);
            o_info!("  Credits: {}", agent.credits);
            o_info!("  Ships: {}", agent.ship_count);
            agent
        }
        Err(e) => {
            o_error!("❌ Authentication failed: {}", e);
            return Err(e);
        }
    };
    
    o_info!("\n🎖️  Admiral ready for operations!");
    
    // Check if a specific goal was provided
    if let Some(goal_command) = args.goal {
        o_info!("\n🎯 GOAL MODE: Executing specific goal instead of autonomous operations");
        o_info!("📋 Goal command: '{}'", goal_command);
        
        match execute_goal(&mut admiral, &goal_command).await {
            Ok(()) => {
                o_info!("\n🎉 GOAL COMPLETED!");
                o_info!("🎖️  Admiral reporting: Goal execution successful");
            }
            Err(e) => {
                o_error!("\n❌ Goal execution failed: {}", e);
                o_error!("🎖️  Admiral reporting: Goal incomplete - system error");
                return Err(e);
            }
        }
    } else {
        o_info!("\n🚀 Starting CONTINUOUS autonomous operations with PROBE exploration...");
        o_info!("⚠️  This will run indefinitely - Press Ctrl+C to stop");
        o_info!("🎯 DUAL MISSION: Mining operations + Shipyard exploration");
        o_info!("💡 TIP: Use --goal 'your command' for specific tasks (e.g., --goal 'mine iron')");
        
        match admiral.run_continuous_operations().await {
            Ok(()) => {
                o_info!("\n🎉 AUTONOMOUS OPERATIONS COMPLETED!");
                o_info!("🎖️  Admiral reporting: Operations terminated by user");
            }
            Err(e) => {
                o_error!("\n❌ Autonomous operations failed: {}", e);
                o_error!("🎖️  Admiral reporting: Mission incomplete - system error");
                return Err(e);
            }
        }
    }
    
    Ok(())
}

async fn execute_goal(admiral: &Admiral, goal_command: &str) -> Result<(), Box<dyn std::error::Error>> {
    o_info!("🧠 Initializing goal execution system...");
    
    // Create priority-aware API client
    let priority_client = PriorityApiClient::new(admiral.client.clone());
    
    // Initialize goal system components
    let mut goal_manager = GoalManager::new();
    let goal_interpreter = GoalInterpreter::new();
    let goal_decomposer = GoalDecomposer::new();
    let mut resource_allocator = ResourceAllocator::new();
    let mut context_engine = ContextEngine::new();
    
    // Parse the natural language goal
    o_info!("🎯 Interpreting goal: '{}'", goal_command);
    let goal = goal_interpreter.parse_goal(goal_command).await
        .map_err(|e| format!("Failed to interpret goal '{}': {}", goal_command, e))?;
    
    o_info!("✅ Goal parsed: {}", goal.description());
    
    // Build execution context
    o_info!("🔄 Building execution context...");
    let context = context_engine.build_context(&priority_client).await?;
    
    // Validate goal feasibility
    o_info!("✅ Validating goal feasibility...");
    context_engine.validate_goal_feasibility(goal_command, &context)
        .map_err(|e| format!("Goal validation failed: {}", e))?;
    
    // Decompose complex goals into sub-goals if needed
    let goals = if goal_decomposer.needs_decomposition(&*goal) {
        o_info!("🔧 Decomposing complex goal into sub-goals...");
        let sub_goals = goal_decomposer.decompose(goal).await;
        o_info!("📋 Created {} sub-goals for execution", sub_goals.len());
        sub_goals
    } else {
        vec![goal]
    };
    
    // Add all goals to the manager with resource allocation
    for goal in goals {
        // Try to allocate resources for this goal
        match resource_allocator.allocate_ships(&*goal, &context) {
            Ok(allocated_ships) => {
                o_info!("📦 Allocated {} ships for goal: {}", allocated_ships.len(), goal.description());
                for ship in &allocated_ships {
                    o_debug!("  - {}", ship);
                }
            }
            Err(e) => {
                o_info!("⚠️ Resource allocation warning for goal '{}': {}", goal.description(), e);
                // Continue anyway - the goal execution may handle this gracefully
            }
        }
        goal_manager.add_goal(goal);
    }
    
    // Execute goals
    o_info!("🚀 Starting goal execution...");
    let start_time = std::time::Instant::now();
    
    while goal_manager.has_pending_goals() {
        // Update context for current execution cycle
        let current_context = context_engine.build_context(&priority_client).await?;
        
        // Execute next batch of goals
        let results = goal_manager.execute_goals(&priority_client, &current_context).await?;
        
        // Report results
        for result in results {
            if result.success {
                o_info!("✅ Goal result: {}", result.message);
                if !result.ships_used.is_empty() {
                    o_info!("🚢 Ships used: {:?}", result.ships_used);
                }
                if result.credits_spent != 0 {
                    if result.credits_spent > 0 {
                        o_info!("💸 Credits spent: {}", result.credits_spent);
                    } else {
                        o_info!("💰 Credits earned: {}", -result.credits_spent);
                    }
                }
            } else {
                o_error!("❌ Goal failed: {}", result.message);
            }
        }
        
        // Brief pause between execution cycles
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        // Safety timeout
        if start_time.elapsed().as_secs() > 600 { // 10 minute timeout
            o_error!("⏰ Goal execution timeout - stopping after 10 minutes");
            break;
        }
    }
    
    let execution_time = start_time.elapsed().as_secs_f64();
    let status = goal_manager.get_status();
    
    o_info!("\n📊 Goal Execution Summary:");
    o_info!("  ⏱️  Total execution time: {:.1} seconds", execution_time);
    o_info!("  ✅ Goals completed: {}", status.completed_count);
    o_info!("  ❌ Goals failed: {}", status.failed_count);
    o_info!("  📋 Goals remaining: {}", status.queued_count);
    
    if status.failed_count > 0 {
        return Err("Some goals failed during execution".into());
    }
    
    Ok(())
}