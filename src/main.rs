// SpaceTraders Autonomous Agent - Main Entry Point
// Modular architecture for 100% autonomous gameplay

use spacetraders_cc::{Admiral, admiral::load_agent_token};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 SpaceTraders Autonomous Agent Starting...");
    println!("🏗️  Using new modular architecture!");
    println!("🎯 PRIME DIRECTIVE: 100% autonomous gameplay");
    
    // Load agent token
    let token = load_agent_token()?;
    
    // Create Admiral for autonomous operations
    let admiral = Admiral::new(token);
    
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
    println!("🚀 Starting CONTINUOUS autonomous operations...");
    println!("⚠️  This will run indefinitely - Press Ctrl+C to stop");
    println!("🎯 PRIME DIRECTIVE: 100% autonomous gameplay - no user interaction required");
    
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