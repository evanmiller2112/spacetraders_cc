use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    
    println!("💰 CHECKING AGENT CREDITS AND STATUS...");
    
    match client.get_agent().await {
        Ok(agent) => {
            println!("✅ AGENT: {}", agent.symbol);
            println!("💎 CREDITS: {}", agent.credits);
            println!("🏛️ HEADQUARTERS: {}", agent.headquarters);
            
            // Check if we have enough for repairs (estimate ~50,000 total)
            if agent.credits >= 50000 {
                println!("✅ SUFFICIENT CREDITS for fleet repairs!");
            } else {
                println!("❌ INSUFFICIENT CREDITS! Need ~50,000 for full fleet repair");
                println!("💡 Current: {}, Need: ~50,000", agent.credits);
            }
        }
        Err(e) => {
            println!("❌ Failed to get agent info: {}", e);
        }
    }
    
    Ok(())
}
