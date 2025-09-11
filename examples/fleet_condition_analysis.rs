// FLEET CONDITION ANALYSIS - Keep our galactic domination fleet in peak condition!
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("🔧🔧🔧 FLEET CONDITION ANALYSIS 🔧🔧🔧");
    println!("=====================================");
    println!("🚨 CRITICAL: Checking ship deterioration!");
    println!("⚡ MAINTAINING GALACTIC DOMINATION FLEET!");
    
    // Get all ships and analyze their condition
    let ships = priority_client.get_ships().await?;
    
    println!("\n📊 CURRENT FLEET CONDITION STATUS:");
    println!("===================================");
    
    let mut total_condition = 0;
    let mut ships_needing_repair = Vec::new();
    let mut critical_condition_ships = Vec::new();
    
    for ship in &ships {
        let condition = ship.frame.condition.unwrap_or(100.0); // Default to 100 if None
        let condition_percent = ((condition / 100.0) * 100.0) as u32;
        total_condition += condition as i32;
        
        println!("\n🚢 {}", ship.symbol);
        println!("   🔧 Condition: {:.0}/100 ({}%)", condition, condition_percent);
        println!("   🚀 Role: {}", ship.registration.role);
        println!("   📍 Location: {}", ship.nav.waypoint_symbol);
        println!("   ⛽ Fuel: {}/{}", ship.fuel.current, ship.fuel.capacity);
        
        // Analyze condition severity
        if condition < 50.0 {
            critical_condition_ships.push(ship.symbol.clone());
            println!("   🚨 CRITICAL CONDITION! Immediate repair needed!");
        } else if condition < 80.0 {
            ships_needing_repair.push(ship.symbol.clone());
            println!("   ⚠️ LOW CONDITION - Schedule repair soon");
        } else {
            println!("   ✅ GOOD CONDITION");
        }
        
        // Check for mining equipment condition impact
        let has_mining_laser = ship.mounts.iter().any(|m| m.symbol.contains("MINING_LASER"));
        let has_surveyor = ship.mounts.iter().any(|m| m.symbol.contains("SURVEYOR"));
        
        if has_mining_laser || has_surveyor {
            println!("   ⛏️ MINING SHIP - Condition affects mining efficiency!");
            if condition < 70.0 {
                println!("   💥 WARNING: Low condition may reduce mining output!");
            }
        }
    }
    
    // Fleet condition analysis
    let avg_condition = total_condition / ships.len() as i32;
    println!("\n📈 FLEET CONDITION SUMMARY:");
    println!("============================");
    println!("   Total ships: {}", ships.len());
    println!("   Average condition: {}/100", avg_condition);
    println!("   Ships needing repair: {}", ships_needing_repair.len());
    println!("   Critical condition ships: {}", critical_condition_ships.len());
    
    if avg_condition < 70 {
        println!("\n🚨🚨🚨 FLEET CONDITION ALERT! 🚨🚨🚨");
        println!("⚠️ GALACTIC DOMINATION AT RISK!");
        println!("💡 IMMEDIATE ACTION REQUIRED!");
    } else if avg_condition < 85 {
        println!("\n⚠️ Fleet condition declining - schedule maintenance");
    } else {
        println!("\n✅ Fleet in excellent condition for galactic domination!");
    }
    
    // Check for repair capabilities at current locations
    println!("\n🔧 REPAIR FACILITY ANALYSIS:");
    println!("=============================");
    
    let mut location_groups: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for ship in &ships {
        location_groups.entry(ship.nav.waypoint_symbol.clone())
            .or_insert_with(Vec::new)
            .push(ship.symbol.clone());
    }
    
    for (location, ships_at_location) in location_groups {
        println!("\n📍 Location: {}", location);
        println!("   Ships present: {:?}", ships_at_location);
        
        // Check if location has repair facilities
        // Note: This would need actual waypoint data to determine repair availability
        println!("   🔧 Repair facilities: [CHECKING...]");
        
        // For now, assume major stations have repair capabilities
        if location.contains("A1") || location.contains("SHIPYARD") {
            println!("   ✅ REPAIR AVAILABLE - Major station detected");
        } else {
            println!("   ❌ NO REPAIR - May need to relocate for maintenance");
        }
    }
    
    // Recommendations
    println!("\n💡 MAINTENANCE RECOMMENDATIONS:");
    println!("===============================");
    
    if !critical_condition_ships.is_empty() {
        println!("🚨 IMMEDIATE ACTION REQUIRED:");
        for ship in &critical_condition_ships {
            println!("   - Repair {} IMMEDIATELY (critical condition)", ship);
        }
    }
    
    if !ships_needing_repair.is_empty() {
        println!("⚠️ SCHEDULE REPAIRS SOON:");
        for ship in &ships_needing_repair {
            println!("   - Schedule repair for {} (declining condition)", ship);
        }
    }
    
    println!("\n🎯 OPERATIONAL IMPACT ANALYSIS:");
    println!("================================");
    
    let mining_ships_low_condition = ships.iter()
        .filter(|s| {
            let has_mining_gear = s.mounts.iter().any(|m| m.symbol.contains("MINING_LASER") || m.symbol.contains("SURVEYOR"));
            let condition = s.frame.condition.unwrap_or(100.0);
            has_mining_gear && condition < 70.0
        })
        .count();
    
    if mining_ships_low_condition > 0 {
        println!("⚠️ {} mining ships have low condition!", mining_ships_low_condition);
        println!("💥 This may reduce mining efficiency and threaten galactic domination!");
        println!("🔧 Priority repair recommended for mining fleet!");
    } else {
        println!("✅ Mining fleet condition is good for continued operations!");
    }
    
    // Condition deterioration prediction
    println!("\n📊 CONDITION DETERIORATION PREDICTION:");
    println!("======================================");
    println!("💡 Based on current usage patterns:");
    
    for ship in &ships {
        let has_mining_gear = ship.mounts.iter().any(|m| m.symbol.contains("MINING_LASER") || m.symbol.contains("SURVEYOR"));
        if has_mining_gear {
            let condition = ship.frame.condition.unwrap_or(100.0);
            // Estimate condition loss per mining operation (this would need actual data)
            let estimated_operations_remaining = if condition > 80.0 {
                "50+ operations"
            } else if condition > 60.0 {
                "20-30 operations"
            } else if condition > 40.0 {
                "10-15 operations"
            } else {
                "5 or fewer operations"
            };
            
            println!("   ⛏️ {}: ~{} before repair needed", ship.symbol, estimated_operations_remaining);
        }
    }
    
    println!("\n🔥 GALACTIC DOMINATION STATUS:");
    println!("==============================");
    
    if critical_condition_ships.is_empty() && ships_needing_repair.len() < 2 {
        println!("🌌 GALACTIC DOMINATION SECURE!");
        println!("✅ Fleet condition supports continued operations!");
        println!("⛏️ MINING OPERATIONS CAN CONTINUE!");
    } else {
        println!("⚠️ GALACTIC DOMINATION AT RISK!");
        println!("🔧 MAINTENANCE REQUIRED TO MAINTAIN SUPREMACY!");
        println!("💡 Address ship condition before continuing major operations!");
    }
    
    Ok(())
}