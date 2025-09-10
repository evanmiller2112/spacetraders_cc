// SpaceTraders Autonomous Agent - Main Entry Point
// Modular architecture for 100% autonomous gameplay

use spacetraders_cc::{Admiral, admiral::load_agent_token, output_broker, o_error, o_info, o_debug};
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
    
    o_info!("\n🎖️  Admiral ready for autonomous operations!");
    
    
    o_info!("\n🚀 Starting CONTINUOUS autonomous operations with PROBE exploration...");
    o_info!("⚠️  This will run indefinitely - Press Ctrl+C to stop");
    o_info!("🎯 DUAL MISSION: Mining operations + Shipyard exploration");
    
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
    
    Ok(())
}