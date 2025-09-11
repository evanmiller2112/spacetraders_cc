// Debug mining capability and ship modules
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("🔍 DEBUGGING MINING CAPABILITY");
    println!("==============================");
    
    let ships = priority_client.get_ships().await?;
    let mut mining_capable_ships = 0;
    
    for ship in &ships {
        println!("\n🚢 Ship: {}", ship.symbol);
        println!("   Location: {}", ship.nav.waypoint_symbol);
        println!("   Status: {}", ship.nav.status);
        println!("   Cargo: {}/{}", ship.cargo.units, ship.cargo.capacity);
        
        let mut has_mining = false;
        let mut has_surveyor = false;
        
        println!("   🔧 Modules ({}):", ship.modules.len());
        for module in &ship.modules {
            println!("      - {}", module.symbol);
            if module.symbol.contains("MINING_LASER") || module.symbol.contains("MINING") {
                has_mining = true;
            }
            if module.symbol.contains("SURVEYOR") {
                has_surveyor = true;
            }
        }
        
        let mining_capable = has_mining || has_surveyor;
        let has_cargo_space = ship.cargo.units < ship.cargo.capacity;
        
        println!("   ⛏️ Has Mining Equipment: {}", mining_capable);
        println!("   📦 Has Cargo Space: {}", has_cargo_space);
        println!("   ✅ Mining Ready: {}", mining_capable && has_cargo_space);
        
        if mining_capable && has_cargo_space {
            mining_capable_ships += 1;
        }
        
        // Show current cargo
        if !ship.cargo.inventory.is_empty() {
            println!("   📦 Current Cargo:");
            for item in &ship.cargo.inventory {
                println!("      - {} x{}", item.symbol, item.units);
            }
        }
    }
    
    println!("\n📊 SUMMARY:");
    println!("   Total Ships: {}", ships.len());
    println!("   Mining Capable: {}", mining_capable_ships);
    
    if mining_capable_ships == 0 {
        println!("\n❌ NO MINING-CAPABLE SHIPS FOUND!");
        println!("💡 Ships need MINING_LASER or SURVEYOR modules");
        println!("💡 Consider purchasing mining equipment at shipyards");
    } else {
        println!("\n✅ {} ships ready for mining operations", mining_capable_ships);
    }
    
    Ok(())
}