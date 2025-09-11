// Quick check of current iron ore inventory across all ships
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("📊 CURRENT IRON ORE INVENTORY");
    println!("============================");
    
    let ships = priority_client.get_ships().await?;
    let mut total_iron_ore = 0;
    
    for ship in &ships {
        let iron_ore: i32 = ship.cargo.inventory.iter()
            .filter(|item| item.symbol == "IRON_ORE")
            .map(|item| item.units)
            .sum();
        
        if iron_ore > 0 {
            println!("⛏️ {}: {} IRON_ORE", ship.symbol, iron_ore);
            total_iron_ore += iron_ore;
        }
    }
    
    println!("\n🎯 MISSION PROGRESS:");
    println!("   Total iron ore: {} units", total_iron_ore);
    println!("   Target: 100 units");
    println!("   Progress: {}%", (total_iron_ore * 100) / 100);
    println!("   Still needed: {} units", std::cmp::max(0, 100 - total_iron_ore));
    
    if total_iron_ore >= 100 {
        println!("\n🎉🎉🎉 MISSION ACCOMPLISHED! 🎉🎉🎉");
        println!("🏭 READY FOR REFINERY OPERATIONS!");
    } else {
        println!("\n💡 Continue running persistent iron ore hunt!");
        println!("💡 Estimated {} more successful extractions needed", (100 - total_iron_ore + 1) / 2);
    }
    
    Ok(())
}