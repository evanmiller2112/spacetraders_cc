// Put test ship back in orbit for normal operations
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    
    // Put CLAUDE_AGENT_2-1 back in orbit for mining
    match client.orbit_ship("CLAUDE_AGENT_2-1").await {
        Ok(_) => println!("✅ CLAUDE_AGENT_2-1 back in orbit for mining operations"),
        Err(e) => {
            if e.to_string().contains("already in orbit") {
                println!("✅ CLAUDE_AGENT_2-1 already in orbit");
            } else {
                println!("⚠️ Could not put ship in orbit: {}", e);
            }
        }
    }
    
    Ok(())
}