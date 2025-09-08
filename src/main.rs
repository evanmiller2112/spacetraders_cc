// SpaceTraders Autonomous Agent - Main Entry Point
// Modular architecture for 100% autonomous gameplay

use spacetraders_cc::{Admiral, admiral::load_agent_token};
use clap::Parser;

#[derive(Parser)]
#[command(name = "spacetraders_cc")]
#[command(about = "SpaceTraders Autonomous Agent")]
struct Args {
    #[arg(long, help = "Enable API call approval for debugging")]
    debug_api: bool,
    
    #[arg(long, help = "Log all API calls and responses to a file")]
    debug_api_log: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    println!("🚀 SpaceTraders Autonomous Agent Starting...");
    println!("🏗️  Using new modular architecture!");
    println!("🎯 PRIME DIRECTIVE: 100% autonomous gameplay");
    
    if args.debug_api {
        println!("🐛 DEBUG MODE: API call approval enabled");
    }
    if args.debug_api_log {
        println!("📝 DEBUG MODE: API call logging enabled");
    }
    
    // Load agent token
    let token = load_agent_token()?;
    
    // Create Admiral for autonomous operations
    let mut admiral = Admiral::new(token);
    admiral.set_debug_mode(args.debug_api);
    admiral.set_api_logging(args.debug_api_log);
    
    // Test authentication first
    let agent = match admiral.client.get_agent().await {
        Ok(agent) => {
            println!("✅ Successfully authenticated!");
            println!("📊 Agent Info:");
            println!("  Symbol: {}", agent.symbol);
            println!("  Headquarters: {}", agent.headquarters);
            println!("  Credits: {}", agent.credits);
            println!("  Ships: {}", agent.ship_count);
            agent
        }
        Err(e) => {
            eprintln!("❌ Authentication failed: {}", e);
            return Err(e);
        }
    };
    
    println!("\n🎖️  Admiral ready for autonomous operations!");
    
    
    println!("\n🚀 Starting CONTINUOUS autonomous operations with PROBE exploration...");
    println!("⚠️  This will run indefinitely - Press Ctrl+C to stop");
    println!("🎯 DUAL MISSION: Mining operations + Shipyard exploration");
    
    match admiral.run_continuous_operations().await {
        Ok(()) => {
            println!("\n🎉 AUTONOMOUS OPERATIONS COMPLETED!");
            println!("🎖️  Admiral reporting: Operations terminated by user");
        }
        Err(e) => {
            eprintln!("\n❌ Autonomous operations failed: {}", e);
            eprintln!("🎖️  Admiral reporting: Mission incomplete - system error");
            return Err(e);
        }
    }
    
    Ok(())
}