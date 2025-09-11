// Test the survey-based iron ore mining system
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient, operations::IronOreMiner};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("🎯 TESTING SURVEY-BASED IRON ORE MINING");
    println!("=====================================");
    
    // Step 1: Create iron ore miner targeting 100 units
    let mut iron_miner = IronOreMiner::new(100);
    
    // Step 2: Execute mining campaign
    println!("🚀 Starting iron ore mining campaign...");
    match iron_miner.execute_mining_campaign(&priority_client).await {
        Ok(success) => {
            if success {
                println!("🎉 MINING CAMPAIGN SUCCESSFUL!");
                println!("✅ Target iron ore amount achieved");
            } else {
                println!("⚠️ Mining campaign completed but target not fully reached");
            }
        }
        Err(e) => {
            println!("❌ Mining campaign failed: {}", e);
        }
    }
    
    // Step 3: Check final iron ore inventory across fleet
    println!("\n📊 FINAL IRON ORE INVENTORY:");
    let ships = priority_client.get_ships().await?;
    let mut total_iron_ore = 0;
    
    for ship in &ships {
        let ship_iron_ore: i32 = ship.cargo.inventory.iter()
            .filter(|item| item.symbol == "IRON_ORE")
            .map(|item| item.units)
            .sum();
            
        if ship_iron_ore > 0 {
            println!("   ⛏️ {}: {} IRON_ORE", ship.symbol, ship_iron_ore);
            total_iron_ore += ship_iron_ore;
        }
    }
    
    println!("📊 TOTAL IRON ORE ACROSS FLEET: {} units", total_iron_ore);
    
    if total_iron_ore >= 100 {
        println!("🎉 SUCCESS: Ready for refinery operations!");
        println!("💡 Next: Run cargo run --example fix_refiner_and_mine_ore");
    } else {
        println!("⚠️ Need {} more iron ore units", 100 - total_iron_ore);
        println!("💡 Consider running mining campaign again");
    }
    
    Ok(())
}