// Install surveyor mounts on ships for targeted mining
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("🔍 INSTALLING SURVEYOR MOUNTS");
    println!("=============================");
    
    let agent = priority_client.get_agent().await?;
    println!("💰 Credits: {}", agent.credits);
    
    // Focus on our mining ship at BA5F
    let miner_symbol = "CLAUDE_AGENT_2-6";
    let shipyard_location = "X1-N5-A2"; // Known shipyard
    
    println!("\n🔍 Checking current miner status...");
    let miner = priority_client.get_ship(miner_symbol).await?;
    println!("   Location: {} (fuel: {})", miner.nav.waypoint_symbol, miner.fuel.current);
    
    // Check current mounts
    println!("   Current mounts:");
    if miner.mounts.is_empty() {
        println!("      (no mounts installed)");
    } else {
        for mount in &miner.mounts {
            println!("      - {}", mount.symbol);
        }
    }
    
    // Navigate to shipyard if not there
    if miner.nav.waypoint_symbol != shipyard_location {
        println!("\n🚀 Navigating to shipyard {}...", shipyard_location);
        
        if miner.nav.status == "DOCKED" {
            priority_client.orbit_ship(miner_symbol).await?;
        }
        
        // Check if we have enough fuel
        if miner.fuel.current < 100 {
            println!("⚠️ Low fuel ({}) - may not reach shipyard", miner.fuel.current);
            println!("💡 Try using a closer ship or refueling first");
            return Ok(());
        }
        
        match priority_client.navigate_ship(miner_symbol, shipyard_location).await {
            Ok(nav_result) => {
                if let Ok(arrival_time) = nav_result.nav.route.arrival.parse::<chrono::DateTime<chrono::Utc>>() {
                    let now = chrono::Utc::now();
                    let wait_seconds = (arrival_time - now).num_seconds().max(0) as u64 + 2;
                    if wait_seconds > 0 && wait_seconds < 300 {
                        println!("⏳ Waiting {} seconds for arrival...", wait_seconds);
                        tokio::time::sleep(tokio::time::Duration::from_secs(wait_seconds)).await;
                    }
                }
            }
            Err(e) => {
                println!("❌ Navigation failed: {}", e);
                return Ok(());
            }
        }
    } else {
        println!("✅ Already at shipyard location");
    }
    
    // Dock at shipyard
    println!("🛸 Docking at shipyard...");
    priority_client.dock_ship(miner_symbol).await?;
    
    // Try to install surveyor mount
    println!("\n🔧 Installing surveyor mount...");
    let mount_types = vec![
        ("MOUNT_SURVEYOR_I", "Basic Surveyor"),
        ("MOUNT_SURVEYOR_II", "Advanced Surveyor"),
    ];
    
    let mut installed = false;
    
    for (mount_symbol, mount_name) in &mount_types {
        println!("🔧 Attempting to install {}...", mount_name);
        
        // Try to install the mount (this might use a different API than modules)
        match priority_client.install_ship_module(miner_symbol, mount_symbol).await {
            Ok(_) => {
                println!("✅ Successfully installed {} on {}", mount_name, miner_symbol);
                installed = true;
                break;
            }
            Err(e) => {
                println!("⚠️ Failed to install {}: {}", mount_name, e);
                
                // Check if error gives us clues about the correct API
                if e.to_string().contains("mount") {
                    println!("💡 This might need a different API endpoint for mounts vs modules");
                }
            }
        }
        
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
    
    if !installed {
        println!("\n❌ Could not install surveyor mounts");
        println!("💡 Possible issues:");
        println!("   - Mounts may use different API than modules");
        println!("   - Ship may not have mount slots available");
        println!("   - Surveyor mounts may not be available at this shipyard");
        println!("   - Insufficient credits");
        
        // Let's check what's available at the shipyard
        println!("\n🔍 Checking shipyard capabilities...");
        match priority_client.get_shipyard("X1-N5", "X1-N5-A2").await {
            Ok(shipyard) => {
                println!("✅ Shipyard accessed");
                println!("   Modification fee: {} credits", shipyard.modifications_fee);
                
                if let Some(ships) = &shipyard.ships {
                    println!("   {} ship types available for purchase", ships.len());
                    
                    // Check if any ships come with surveyor mounts
                    for ship_type in ships {
                        if !ship_type.mounts.is_empty() {
                            println!("   Ship {}: {} mounts", ship_type.ship_type, ship_type.mounts.len());
                            for mount in &ship_type.mounts {
                                if mount.symbol.contains("SURVEYOR") {
                                    println!("      🎯 Has surveyor: {}", mount.symbol);
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                println!("❌ Could not access shipyard details: {}", e);
            }
        }
    }
    
    // Check final mount status
    println!("\n🔍 Final mount verification...");
    let updated_miner = priority_client.get_ship(miner_symbol).await?;
    
    let has_surveyor = updated_miner.mounts.iter().any(|m| m.symbol.contains("SURVEYOR"));
    
    if has_surveyor {
        println!("✅ {} now has surveyor capability!", miner_symbol);
        println!("💡 Can now create targeted surveys for iron ore mining");
        println!("💡 Next: Run intensive mining with survey targeting");
    } else {
        println!("❌ {} still lacks surveyor capability", miner_symbol);
        println!("💡 Alternative approaches:");
        println!("   - Continue basic mining (slower but works)");
        println!("   - Purchase a new ship with surveyor mounts");
        println!("   - Research correct API for mount installation");
    }
    
    // Show current mining capability
    println!("\n📊 CURRENT MINING STATUS:");
    println!("   Ship: {} at {}", miner_symbol, updated_miner.nav.waypoint_symbol);
    println!("   Can mine: {} (at ENGINEERED_ASTEROID)", 
             updated_miner.nav.waypoint_symbol == "X1-N5-BA5F");
    println!("   Has surveyor: {}", has_surveyor);
    println!("   Cargo space: {}/{}", updated_miner.cargo.units, updated_miner.cargo.capacity);
    
    Ok(())
}