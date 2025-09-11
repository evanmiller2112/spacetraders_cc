// 🚨🚨🚨 EMERGENCY FLEET REPAIR PROTOCOL - SAVE GALACTIC DOMINATION! 🚨🚨🚨
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("🚨🚨🚨🚨🚨 EMERGENCY FLEET REPAIR PROTOCOL 🚨🚨🚨🚨🚨");
    println!("=========================================================");
    println!("💥 GALACTIC DOMINATION FLEET IN CRITICAL CONDITION!");
    println!("🔧 INITIATING EMERGENCY REPAIR OPERATIONS!");
    println!("⚡ SAVE THE GALAXY - REPAIR ALL SHIPS!");
    
    // Repair facility locations (detected from fleet analysis)
    let repair_station = "X1-N5-A1"; // Major station with repair capabilities
    
    // Get current fleet status
    let ships = priority_client.get_ships().await?;
    
    println!("\n📊 EMERGENCY FLEET STATUS:");
    println!("============================");
    
    let mut critical_ships = Vec::new();
    let mut ships_at_repair_station = Vec::new();
    let mut ships_needing_relocation = Vec::new();
    
    for ship in &ships {
        let condition = ship.frame.condition.unwrap_or(100.0);
        println!("🚢 {}: {:.0}% condition at {}", ship.symbol, condition, ship.nav.waypoint_symbol);
        
        if condition < 50.0 {
            critical_ships.push(ship.symbol.clone());
            
            if ship.nav.waypoint_symbol == repair_station {
                ships_at_repair_station.push(ship.symbol.clone());
            } else {
                ships_needing_relocation.push(ship.symbol.clone());
            }
        }
    }
    
    println!("\n🎯 EMERGENCY REPAIR PLAN:");
    println!("==========================");
    println!("   Critical ships: {}", critical_ships.len());
    println!("   Already at repair station: {}", ships_at_repair_station.len());
    println!("   Need relocation: {}", ships_needing_relocation.len());
    
    if critical_ships.is_empty() {
        println!("\n✅ NO EMERGENCY REPAIRS NEEDED!");
        return Ok(());
    }
    
    // PHASE 1: RELOCATE SHIPS TO REPAIR STATION
    if !ships_needing_relocation.is_empty() {
        println!("\n🚀🚀🚀 PHASE 1: EMERGENCY RELOCATION TO REPAIR STATION! 🚀🚀🚀");
        println!("===========================================================");
        
        for ship_symbol in &ships_needing_relocation {
            println!("\n🚢 RELOCATING {} to repair station {}", ship_symbol, repair_station);
            
            let ship = priority_client.get_ship(ship_symbol).await?;
            
            // Check if ship can travel (fuel, condition, etc.)
            if ship.fuel.current < 10 {
                println!("   ⛽ WARNING: {} has low fuel ({}/{})", ship_symbol, ship.fuel.current, ship.fuel.capacity);
                println!("   💡 May need fuel before relocation");
            }
            
            // Navigate to repair station
            print!("   🎯 Navigating to {}... ", repair_station);
            match priority_client.navigate_ship(ship_symbol, repair_station).await {
                Ok(nav_data) => {
                    println!("✅ Started! ETA: {}", nav_data.nav.route.arrival);
                    
                    // Wait for arrival (simplified - just wait a fixed time)
                    println!("   ⏱️ Waiting for arrival...");
                    sleep(Duration::from_secs(30)).await;
                }
                Err(e) => {
                    println!("❌ Failed: {}", e);
                    if e.to_string().contains("insufficient fuel") {
                        println!("   🚨 INSUFFICIENT FUEL - Ship stranded!");
                        continue;
                    }
                }
            }
            
            sleep(Duration::from_secs(2)).await;
        }
    }
    
    // PHASE 2: REPAIR ALL CRITICAL SHIPS
    println!("\n🔧🔧🔧 PHASE 2: EMERGENCY REPAIR OPERATIONS! 🔧🔧🔧");
    println!("=================================================");
    
    // Get updated ship list after relocations
    let updated_ships = priority_client.get_ships().await?;
    let mut repair_queue = Vec::new();
    
    for ship in &updated_ships {
        let condition = ship.frame.condition.unwrap_or(100.0);
        if condition < 50.0 && ship.nav.waypoint_symbol == repair_station {
            repair_queue.push(ship.symbol.clone());
        }
    }
    
    if repair_queue.is_empty() {
        println!("❌ NO SHIPS AT REPAIR STATION FOR EMERGENCY REPAIRS!");
        return Ok(());
    }
    
    println!("🔧 Ships ready for repair: {}", repair_queue.len());
    
    // Repair each ship in the queue
    let mut repair_count = 0;
    for ship_symbol in &repair_queue {
        println!("\n🔧 REPAIRING {} ({}/{})", ship_symbol, repair_count + 1, repair_queue.len());
        
        let ship = priority_client.get_ship(ship_symbol).await?;
        let condition_before = ship.frame.condition.unwrap_or(100.0);
        
        // Dock at repair station if not already docked
        if ship.nav.status != "DOCKED" {
            print!("   🚢 Docking at repair station... ");
            match priority_client.dock_ship(ship_symbol).await {
                Ok(_) => println!("✅ Docked"),
                Err(e) => {
                    println!("❌ Failed to dock: {}", e);
                    continue;
                }
            }
        }
        
        // Get repair cost first
        print!("   💰 Checking repair cost... ");
        match priority_client.get_repair_cost(ship_symbol).await {
            Ok(repair_cost) => {
                println!("{}💎", repair_cost.transaction.total_price);
            }
            Err(e) => {
                println!("❌ Cost check failed: {}", e);
            }
        }

        // Attempt repair
        print!("   🔧 Executing emergency repair... ");
        match priority_client.repair_ship(ship_symbol).await {
            Ok(repair_data) => {
                let condition_after = repair_data.ship.frame.condition.unwrap_or(100.0);
                println!("✅ SUCCESS! Cost: {}💎", repair_data.transaction.total_price);
                println!("      Condition: {:.0}% → {:.0}%", condition_before, condition_after);
                
                if condition_after > 80.0 {
                    println!("      🎉 SHIP FULLY OPERATIONAL!");
                } else if condition_after > 50.0 {
                    println!("      ⚡ SHIP OPERATIONALLY RESTORED!");
                } else {
                    println!("      ⚠️ Partial repair - may need additional work");
                }
                
                repair_count += 1;
            }
            Err(e) => {
                println!("❌ Failed: {}", e);
                if e.to_string().contains("insufficient credits") {
                    println!("      💰 INSUFFICIENT CREDITS FOR REPAIR!");
                } else if e.to_string().contains("no repair") {
                    println!("      🔧 REPAIR FACILITY NOT AVAILABLE!");
                } else {
                    println!("      ❓ Unknown repair error");
                }
            }
        }
        
        sleep(Duration::from_secs(3)).await;
    }
    
    // PHASE 3: POST-REPAIR FLEET STATUS
    println!("\n📊📊📊 POST-REPAIR FLEET STATUS 📊📊📊");
    println!("======================================");
    
    let final_ships = priority_client.get_ships().await?;
    let mut operational_ships = 0;
    let mut still_critical = 0;
    
    for ship in &final_ships {
        let condition = ship.frame.condition.unwrap_or(100.0);
        let status = if condition >= 80.0 {
            operational_ships += 1;
            "✅ OPERATIONAL"
        } else if condition >= 50.0 {
            operational_ships += 1;
            "⚡ FUNCTIONAL"
        } else {
            still_critical += 1;
            "🚨 STILL CRITICAL"
        };
        
        println!("   🚢 {}: {:.0}% - {}", ship.symbol, condition, status);
    }
    
    println!("\n🎯 EMERGENCY REPAIR SUMMARY:");
    println!("=============================");
    println!("   Ships repaired: {}", repair_count);
    println!("   Operational ships: {}", operational_ships);
    println!("   Still critical: {}", still_critical);
    
    if still_critical == 0 {
        println!("\n🎉🎉🎉🎉🎉 FLEET EMERGENCY REPAIR COMPLETE! 🎉🎉🎉🎉🎉");
        println!("🚀 GALACTIC DOMINATION FLEET RESTORED!");
        println!("⚡ ALL SHIPS OPERATIONAL!");
        println!("💥 READY TO RESUME TOTAL CONQUEST!");
    } else {
        println!("\n⚠️ PARTIAL REPAIR SUCCESS");
        println!("💡 {} ships still need attention", still_critical);
        println!("🔧 May require additional repair cycles or credits");
    }
    
    println!("\n🔥 GALACTIC DOMINATION STATUS: {}", 
             if still_critical == 0 { "🌌 FULLY RESTORED!" } else { "⚡ PARTIALLY RESTORED" });
    
    Ok(())
}