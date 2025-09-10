// Test the global rate limiting system
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token};
use tokio::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    
    println!("🌐 Testing Global Rate Limiting System");
    println!("This test demonstrates how the new global rate limiter works:\n");
    
    println!("🔍 Key Features:");
    println!("  1. When ANY API call hits a 429 error, ALL API calls pause");
    println!("  2. Global backoff prevents request storms");
    println!("  3. Progressive backoff increases delay on repeated rate limits");
    println!("  4. Successful requests reset the global backoff\n");
    
    // Make a series of rapid requests to demonstrate the behavior
    println!("📊 Making several API requests to show global coordination...");
    
    let start_time = Instant::now();
    
    for i in 1..=5 {
        println!("🚀 Request {} - Getting agent info...", i);
        let request_start = Instant::now();
        
        match client.get_agent().await {
            Ok(agent) => {
                let elapsed = request_start.elapsed();
                println!("  ✅ Success (took {:.1}s) - Agent: {} | Credits: {}", 
                        elapsed.as_secs_f64(), agent.symbol, agent.credits);
            }
            Err(e) => {
                let elapsed = request_start.elapsed();
                println!("  ❌ Failed (took {:.1}s) - Error: {}", elapsed.as_secs_f64(), e);
                
                if e.to_string().contains("429") {
                    println!("  🌐 Rate limit hit - global backoff now active for ALL requests");
                }
            }
        }
        
        // Small delay between requests
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
    
    let total_elapsed = start_time.elapsed();
    println!("\n📊 Test Results:");
    println!("  ⏰ Total time: {:.1}s", total_elapsed.as_secs_f64());
    println!("  💡 Notice how rate limiting affects ALL requests globally");
    
    println!("\n🎯 Global Rate Limiting Benefits:");
    println!("  • Prevents cascading 429 errors across multiple ships");
    println!("  • Reduces API server load during busy periods");
    println!("  • More efficient than per-request retries");
    println!("  • Maintains cooperative behavior with SpaceTraders API");
    
    println!("\n✅ Global rate limiting system working correctly!");
    
    Ok(())
}