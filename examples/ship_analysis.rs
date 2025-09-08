// Ship capability analysis example
use spacetraders_cc::{Admiral, admiral::load_agent_token};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 SpaceTraders Ship Analysis Tool");
    
    // Load agent token
    let token = match load_agent_token() {
        Ok(token) => token,
        Err(_) => {
            println!("❌ No agent token found. Please run the main agent first to authenticate.");
            return Ok(());
        }
    };
    
    // Create Admiral for ship analysis
    let admiral = Admiral::new(token);
    
    // Test authentication
    match admiral.client.get_agent().await {
        Ok(agent) => {
            println!("✅ Successfully authenticated as {}", agent.symbol);
            println!("💰 Credits: {}", agent.credits);
            println!("🚢 Ships: {}\n", agent.ship_count);
        }
        Err(e) => {
            println!("❌ Authentication failed: {}", e);
            return Err(e);
        }
    };
    
    // Run ship capability analysis
    admiral.debug_ship_capabilities().await?;
    
    println!("🎯 Analysis complete! Check above for ship modification opportunities.");
    
    Ok(())
}