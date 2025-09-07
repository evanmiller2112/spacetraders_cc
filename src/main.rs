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
    println!("Choose operation mode:");
    println!("  1. Single autonomous cycle (default)");
    println!("  2. Continuous autonomous operations (runs forever)");
    
    // For now, run single cycle - could be enhanced with command line args
    println!("\n🎖️  Starting single autonomous cycle...");
    
    match admiral.run_autonomous_cycle().await {
        Ok(()) => {
            println!("\n🎉 AUTONOMOUS OPERATIONS COMPLETED SUCCESSFULLY!");
            println!("🎖️  Admiral reporting: Mission accomplished!");
            println!("💡 To run continuous operations, modify main.rs or add command line options");
        }
        Err(e) => {
            eprintln!("\n❌ Autonomous operations failed: {}", e);
            eprintln!("🎖️  Admiral reporting: Mission incomplete - retry recommended");
            return Err(e);
        }
    }
    
    Ok(())
}