// Test the complete contract fulfillment strategy with refinery
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::PriorityApiClient, operations::ShipRoleManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("📋 Testing Contract Fulfillment Strategy with Refinery");
    println!("====================================================");
    
    // Check current contracts
    println!("🔍 Checking active contracts...");
    let contracts = priority_client.get_contracts().await?;
    
    for contract in &contracts {
        if contract.accepted && !contract.fulfilled {
            println!("📋 Active Contract: {}", contract.id);
            println!("   🏢 Faction: {}", contract.faction_symbol);
            println!("   💰 Payment: {} on accept, {} on fulfill", contract.terms.payment.on_accepted, contract.terms.payment.on_fulfilled);
            
            for delivery in &contract.terms.deliver {
                let progress = format!("{}/{}", delivery.units_fulfilled, delivery.units_required);
                let remaining = delivery.units_required - delivery.units_fulfilled;
                
                println!("   📦 Delivery: {} x{} to {} ({})", 
                        delivery.trade_symbol, delivery.units_required, 
                        delivery.destination_symbol, progress);
                
                if delivery.trade_symbol == "IRON" && remaining > 0 {
                    println!("   🏭 IRON delivery detected - perfect for refinery strategy!");
                    println!("   📊 Strategy Analysis:");
                    println!("      - Need {} refined IRON units", remaining);
                    println!("      - Requires {} iron ore (10:1 ratio)", remaining * 10);
                    println!("      - {} refining cycles needed", (remaining + 9) / 10);
                }
            }
        }
    }
    
    // Initialize ship role manager
    let mut role_manager = ShipRoleManager::new();
    
    // Analyze fleet
    println!("\n🔍 Analyzing fleet capabilities...");
    match role_manager.analyze_fleet(&priority_client).await {
        Ok(_) => {
            println!("✅ Fleet analysis completed");
        }
        Err(e) => {
            println!("❌ Fleet analysis failed: {}", e);
            return Ok(());
        }
    }
    
    // Check refiner designation
    if let Some(refiner) = role_manager.find_best_refinery_candidate() {
        println!("🏭 Designated refiner: {} (score: {:.2})", 
                refiner.ship_symbol, refiner.refinery_score);
        
        let refiner_ship = priority_client.get_ship(&refiner.ship_symbol).await?;
        println!("📦 Refiner cargo capacity: {}", refiner_ship.cargo.capacity);
        
        // Determine strategy
        if refiner_ship.cargo.capacity >= 83 { // 73 + 10 buffer
            println!("🏭 BATCH STRATEGY RECOMMENDED:");
            println!("   ✅ Sufficient cargo capacity for all 73 iron units");
            println!("   🔧 May need to expand cargo capacity with modules");
            println!("   📦 Single large refining session");
            println!("   🚚 One delivery trip with all 73 units");
        } else {
            println!("🔄 INCREMENTAL STRATEGY RECOMMENDED:");
            println!("   📦 Limited cargo capacity ({})", refiner_ship.cargo.capacity);
            println!("   🔄 8 cycles: mine 100 ore → refine 10 iron → deliver");
            println!("   🚚 8 separate delivery trips (7×10 + 1×3 iron)");
            println!("   ⚡ No cargo expansion needed");
        }
    } else {
        println!("❌ No refinery candidate found");
        return Ok(());
    }
    
    // Test the strategy execution
    println!("\n🚀 Testing contract strategy execution...");
    match role_manager.execute_refinery_contract_strategy(&priority_client).await {
        Ok(success) => {
            if success {
                println!("🎉 Contract strategy execution completed successfully!");
                println!("📋 Contract should now be fulfilled or in progress");
            } else {
                println!("⚠️ Contract strategy completed with warnings");
                println!("📋 May need manual intervention or more resources");
            }
        }
        Err(e) => {
            println!("❌ Contract strategy failed: {}", e);
        }
    }
    
    println!("\n📊 Strategy Summary:");
    println!("🏭 The system intelligently chooses between:");
    println!("   📦 BATCH: Expand cargo → refine all → deliver once");  
    println!("   🔄 INCREMENTAL: Small batches → refine → deliver → repeat");
    println!("   🎯 Based on refiner cargo capacity vs contract requirements");
    println!("   ⚡ Automatic fallback if expansion fails");
    
    println!("\n🎉 Contract Strategy Test Complete!");
    
    Ok(())
}