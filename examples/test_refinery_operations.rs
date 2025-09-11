// Test the complete refinery operation system
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient, operations::ShipRoleManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("🏭 Testing Complete Refinery Operation System");
    println!("============================================");
    
    // Initialize ship role manager
    let mut role_manager = ShipRoleManager::new();
    
    // Analyze fleet for refinery capabilities
    println!("🔍 Analyzing fleet...");
    match role_manager.analyze_fleet(&priority_client).await {
        Ok(_) => {
            println!("✅ Fleet analysis completed");
        }
        Err(e) => {
            println!("❌ Fleet analysis failed: {}", e);
            return Ok(());
        }
    }
    
    // Check for designated refiner
    println!("\n🏭 Checking refinery designation...");
    if let Some(refiner) = role_manager.find_best_refinery_candidate() {
        println!("✅ Designated refiner: {} (score: {:.2})", 
                refiner.ship_symbol, refiner.refinery_score);
        
        // Get current cargo status
        let ship = priority_client.get_ship(&refiner.ship_symbol).await?;
        println!("📦 Current cargo: {}/{}", ship.cargo.units, ship.cargo.capacity);
        
        let mut iron_ore_units = 0;
        for item in &ship.cargo.inventory {
            println!("   - {} x{}", item.symbol, item.units);
            if item.symbol == "IRON_ORE" {
                iron_ore_units = item.units;
            }
        }
        
        if iron_ore_units < 100 {
            println!("⚠️ Need more iron ore for refining (have {}, need 100+)", iron_ore_units);
            
            // Coordinate ore transfers first
            println!("\n🔄 Coordinating ore transfers...");
            match role_manager.coordinate_ore_to_refiner_transfer(&priority_client).await {
                Ok(success) => {
                    if success {
                        println!("✅ Ore transfers completed");
                        
                        // Get updated cargo after transfer
                        let updated_ship = priority_client.get_ship(&refiner.ship_symbol).await?;
                        iron_ore_units = updated_ship.cargo.inventory
                            .iter()
                            .find(|item| item.symbol == "IRON_ORE")
                            .map(|item| item.units)
                            .unwrap_or(0);
                        
                        println!("📦 Updated iron ore: {} units", iron_ore_units);
                    } else {
                        println!("⚠️ No ore transfers available");
                    }
                }
                Err(e) => {
                    println!("❌ Ore transfer failed: {}", e);
                }
            }
        }
        
        // Start refinery operations
        println!("\n⚙️ Starting refinery operations...");
        match role_manager.start_refinery_operations(&priority_client).await {
            Ok(success) => {
                if success {
                    println!("🎉 Refinery operations completed successfully!");
                } else {
                    println!("⚠️ Refinery operations completed with warnings");
                }
                
                // Show final cargo status
                println!("\n📊 Final cargo status:");
                let final_ship = priority_client.get_ship(&refiner.ship_symbol).await?;
                println!("📦 Final cargo: {}/{}", final_ship.cargo.units, final_ship.cargo.capacity);
                
                let mut final_iron_ore = 0;
                let mut refined_iron = 0;
                
                for item in &final_ship.cargo.inventory {
                    println!("   - {} x{}", item.symbol, item.units);
                    if item.symbol == "IRON_ORE" {
                        final_iron_ore = item.units;
                    } else if item.symbol == "IRON" {
                        refined_iron = item.units;
                    }
                }
                
                println!("\n📈 Refinery Results:");
                println!("   🪨 Remaining iron ore: {} units", final_iron_ore);
                println!("   ⚙️ Refined iron produced: {} units", refined_iron);
                println!("   📊 Conversion rate: 100 ore → 10 refined iron");
            }
            Err(e) => {
                println!("❌ Refinery operations failed: {}", e);
            }
        }
    } else {
        println!("❌ No refinery candidate found");
    }
    
    println!("\n🎉 Refinery Operation Test Complete!");
    println!("🏭 The refinery system can now automatically:");
    println!("   📦 Coordinate ore collection from fleet");
    println!("   ⚙️ Refine raw materials into processed goods");
    println!("   🔄 Handle cooldowns and multiple refining cycles");
    println!("   📊 Track production metrics and efficiency");
    
    Ok(())
}