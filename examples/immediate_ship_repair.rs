// 🔧 IMMEDIATE SHIP REPAIR - SAVE THE GALACTIC DOMINATION FLEET! 🔧
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("🔧🔧🔧 IMMEDIATE SHIP REPAIR PROTOCOL 🔧🔧🔧");
    println!("==========================================");
    println!("⚡ REPAIR ALL SHIPS AT X1-N5-A1!");
    println!("💥 SAVE GALACTIC DOMINATION!");
    
    let repair_station = "X1-N5-A1";
    
    // Get ships at repair station
    let ships = priority_client.get_ships().await?;
    let ships_at_station: Vec<_> = ships.iter()
        .filter(|ship| ship.nav.waypoint_symbol == repair_station)
        .collect();
    
    println!("\n🚢 SHIPS AT REPAIR STATION {}:", repair_station);
    for ship in &ships_at_station {
        let condition = ship.frame.condition.unwrap_or(100.0);
        println!("   ⚡ {}: {:.0}% condition", ship.symbol, condition);
    }
    
    if ships_at_station.is_empty() {
        println!("❌ NO SHIPS AT REPAIR STATION!");
        return Ok(());
    }
    
    // Repair each ship
    let mut repair_count = 0;
    let mut total_cost = 0;
    
    for ship in &ships_at_station {
        let ship_symbol = &ship.symbol;
        let condition_before = ship.frame.condition.unwrap_or(100.0);
        
        if condition_before >= 90.0 {
            println!("\n✅ {} already in good condition ({:.0}%)", ship_symbol, condition_before);
            continue;
        }
        
        println!("\n🔧 REPAIRING {} ({:.0}% condition)", ship_symbol, condition_before);
        
        // Dock ship first
        if ship.nav.status != "DOCKED" {
            print!("   🚢 Docking... ");
            match priority_client.dock_ship(ship_symbol).await {
                Ok(_) => println!("✅ Docked"),
                Err(e) => {
                    println!("❌ Failed to dock: {}", e);
                    continue;
                }
            }
        }
        
        // Get repair cost
        print!("   💰 Checking repair cost... ");
        let repair_cost = match priority_client.get_repair_cost(ship_symbol).await {
            Ok(cost) => {
                println!("{}💎", cost.transaction.total_price);
                cost.transaction.total_price
            }
            Err(e) => {
                println!("❌ Failed: {}", e);
                continue;
            }
        };
        
        // Execute repair
        print!("   🔧 Repairing... ");
        match priority_client.repair_ship(ship_symbol).await {
            Ok(repair_data) => {
                let condition_after = repair_data.ship.frame.condition.unwrap_or(100.0);
                let actual_cost = repair_data.transaction.total_price;
                
                println!("✅ SUCCESS!");
                println!("      Cost: {}💎", actual_cost);
                println!("      Condition: {:.0}% → {:.0}%", condition_before, condition_after);
                
                if condition_after >= 90.0 {
                    println!("      🎉 SHIP FULLY OPERATIONAL!");
                } else if condition_after >= 70.0 {
                    println!("      ⚡ SHIP OPERATIONALLY RESTORED!");
                } else {
                    println!("      ⚠️ Partial repair - may need additional work");
                }
                
                repair_count += 1;
                total_cost += actual_cost;
            }
            Err(e) => {
                println!("❌ Failed: {}", e);
                if e.to_string().contains("insufficient credits") {
                    println!("      💰 INSUFFICIENT CREDITS!");
                } else if e.to_string().contains("does not exist") {
                    println!("      🔧 REPAIR FACILITY NOT AVAILABLE!");
                } else {
                    println!("      ❓ Error: {}", e);
                }
            }
        }
        
        sleep(Duration::from_secs(2)).await;
    }
    
    // Final status
    println!("\n🎯 REPAIR OPERATION COMPLETE!");
    println!("==============================");
    println!("   Ships repaired: {}", repair_count);
    println!("   Total cost: {}💎", total_cost);
    
    if repair_count > 0 {
        println!("\n🎉 GALACTIC DOMINATION FLEET PARTIALLY RESTORED!");
        println!("⚡ {} ships ready for operations!", repair_count);
    } else {
        println!("\n⚠️ NO REPAIRS COMPLETED");
        println!("💡 Check credits or repair facility availability");
    }
    
    // Get final fleet status
    println!("\n📊 POST-REPAIR FLEET STATUS:");
    let final_ships = priority_client.get_ships().await?;
    let final_ships_at_station: Vec<_> = final_ships.iter()
        .filter(|ship| ship.nav.waypoint_symbol == repair_station)
        .collect();
    
    for ship in &final_ships_at_station {
        let condition = ship.frame.condition.unwrap_or(100.0);
        let status = if condition >= 90.0 {
            "✅ OPERATIONAL"
        } else if condition >= 70.0 {
            "⚡ FUNCTIONAL"
        } else {
            "🚨 STILL CRITICAL"
        };
        
        println!("   🚢 {}: {:.0}% - {}", ship.symbol, condition, status);
    }
    
    Ok(())
}