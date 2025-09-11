use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = env::var("SPACETRADERS_TOKEN").expect("SPACETRADERS_TOKEN not set");
    let client = reqwest::Client::new();
    
    println!("🔧 FINDING SHIPYARDS IN X1-N5...");
    
    let url = "https://api.spacetraders.io/v2/systems/X1-N5/waypoints?traits=SHIPYARD";
    let response = client
        .get(url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;
        
    let text = response.text().await?;
    println!("{}", text);
    
    Ok(())
}
