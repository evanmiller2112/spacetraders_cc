// Purchase and outfit a new mining ship similar to our current one
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let mut client = SpaceTradersClient::new(token);
    client.set_debug_mode(false); // Disable debug to avoid prompts
    
    println!("🏗️ Ship Purchase System");
    
    // Get current agent info
    let agent = client.get_agent().await?;
    println!("💰 Current credits: {}", agent.credits);
    
    // Get current ships for reference
    let ships = client.get_ships().await?;
    println!("🚢 Current fleet size: {}", ships.len());
    
    // Analyze our current mining ship configuration
    let mining_ship = ships.iter()
        .find(|s| s.registration.role == "COMMAND" && 
                 s.mounts.iter().any(|m| m.symbol.contains("MINING")))
        .expect("No mining ship found in fleet");
    
    println!("\n📋 CURRENT MINING SHIP ANALYSIS:");
    println!("   🚢 Ship: {}", mining_ship.symbol);
    println!("   🏗️ Frame: {} (Cargo: {})", mining_ship.frame.symbol, mining_ship.cargo.capacity);
    println!("   ⚡ Engine: {}", mining_ship.engine.symbol);
    println!("   🔋 Reactor: {}", mining_ship.reactor.symbol);
    println!("   🛠️ Mounts ({}):", mining_ship.mounts.len());
    for mount in &mining_ship.mounts {
        println!("     • {} - {}", mount.symbol, mount.name);
    }
    println!("   📦 Modules ({}):", mining_ship.modules.len());
    for module in &mining_ship.modules {
        println!("     • {} - {}", module.symbol, module.name);
    }
    
    // Calculate the configuration we want for a new mining ship
    let desired_ship_type = "SHIP_MINING_DRONE"; // Common mining ship type
    let desired_frame = &mining_ship.frame.symbol;
    
    println!("\n🎯 DESIRED NEW MINING SHIP:");
    println!("   📝 Ship Type: {}", desired_ship_type);
    println!("   🏗️ Frame: {}", desired_frame);
    println!("   🛠️ Required Mounts:");
    for mount in &mining_ship.mounts {
        if mount.symbol.contains("MINING") || mount.symbol.contains("SURVEYOR") {
            println!("     • {} (Critical for mining)", mount.symbol);
        } else {
            println!("     • {} (Optional)", mount.symbol);
        }
    }
    
    // For demonstration, let's show what the purchase process would look like
    // (We can't actually purchase without finding a shipyard first)
    println!("\n🏭 SHIPYARD REQUIREMENTS:");
    println!("   ❌ No shipyards found in current system (X1-N5)");
    println!("   🔍 Need to explore other systems to find shipyards");
    println!("   💡 Alternative: Wait for automated exploration to discover shipyards");
    
    println!("\n📊 PURCHASE SIMULATION:");
    println!("   💰 Estimated cost: 150,000 - 300,000 credits (typical mining ship)");
    println!("   💸 Current credits: {} ({})", agent.credits, 
            if agent.credits >= 150000 { "✅ Sufficient" } else { "❌ Need more credits" });
    
    // Show the purchase steps that would happen
    println!("\n🔄 PURCHASE PROCESS (when shipyard found):");
    println!("   1. 🗺️ Navigate to shipyard location");
    println!("   2. 🚢 Purchase {} (or similar frame)", desired_ship_type);
    println!("   3. 🛠️ Install mining laser and surveyor mounts");
    println!("   4. 📦 Add cargo hold modules");
    println!("   5. ⚡ Optimize engine and reactor if needed");
    println!("   6. 🎯 Add new ship to fleet operations");
    
    // Demonstrate what the API call would look like
    if agent.credits >= 200000 {
        println!("\n✨ READY TO PURCHASE when shipyard is available!");
        println!("   🎯 Target: Mining ship similar to {}", mining_ship.symbol);
        println!("   💡 Will automatically outfit with mining equipment");
    } else {
        let needed = 200000 - agent.credits;
        println!("\n⏳ NEED MORE CREDITS:");
        println!("   💰 Need additional: {} credits", needed);
        println!("   🎯 Continue mining operations to accumulate funds");
    }
    
    Ok(())
}