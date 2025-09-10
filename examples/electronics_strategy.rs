// Strategy for acquiring ELECTRONICS for contract
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    
    println!("🎯 ELECTRONICS Acquisition Strategy");
    
    // Check current situation first
    let ships = client.get_ships().await?;
    let satellite = ships.iter().find(|s| s.registration.role == "SATELLITE").unwrap();
    
    println!("🛰️ SATELLITE Status: {} at {}", satellite.symbol, satellite.nav.waypoint_symbol);
    println!("   Status: {}", satellite.nav.status);
    
    if satellite.nav.status == "IN_TRANSIT" {
        let arrival_time = &satellite.nav.route.arrival;
        println!("   🚀 In transit, arriving at: {}", arrival_time);
        println!("   📍 Destination: {}", satellite.nav.route.destination.symbol);
    }
    
    // Check our current credits for purchasing
    let agent = client.get_agent().await?;
    println!("💰 Current credits: {}", agent.credits);
    
    // Contract analysis
    let contracts = client.get_contracts().await?;
    let electronics_contract = contracts.iter()
        .find(|c| !c.fulfilled && c.terms.deliver.iter().any(|d| d.trade_symbol == "ELECTRONICS"))
        .unwrap();
    
    let delivery_requirement = electronics_contract.terms.deliver.iter()
        .find(|d| d.trade_symbol == "ELECTRONICS")
        .unwrap();
    
    println!("📋 Contract Requirements:");
    println!("   📦 Need: {} ELECTRONICS", delivery_requirement.units_required);
    println!("   📍 Deliver to: {}", delivery_requirement.destination_symbol);
    println!("   💰 Payment: {} credits", electronics_contract.terms.payment.on_fulfilled);
    
    // Calculate budget
    let needed_units = delivery_requirement.units_required as i64;
    let available_budget = agent.credits;
    let max_price_per_unit = available_budget / needed_units;
    
    println!("\n💸 Budget Analysis:");
    println!("   📊 Need: {} units", needed_units);
    println!("   💰 Available budget: {} credits", available_budget);
    println!("   📈 Max price per unit: {} credits", max_price_per_unit);
    
    if max_price_per_unit < 1000 {
        println!("   ⚠️ Budget is tight - may need to sell mining goods first");
    }
    
    // Strategy recommendations
    println!("\n🎯 RECOMMENDED STRATEGY:");
    
    println!("\n1. 🔍 IMMEDIATE ACTIONS:");
    println!("   • Wait for SATELLITE to arrive at X1-N5-A1");
    println!("   • Check X1-N5-A1 marketplace for ELECTRONICS");
    println!("   • If found, calculate purchase cost vs budget");
    
    println!("\n2. 📊 IF ELECTRONICS NOT FOUND IN X1-N5:");
    println!("   • Explore nearby systems (may need to purchase jump drives)");
    println!("   • Check production facilities vs marketplaces");
    println!("   • Consider if contract is worth the exploration cost");
    
    println!("\n3. 💰 BUDGET OPTIMIZATION:");
    println!("   • Continue mining with excavator ships to increase budget");
    println!("   • Sell current cargo to maximize purchase power");
    println!("   • Calculate ROI: contract payment vs acquisition cost");
    
    println!("\n4. 🚢 FLEET DEPLOYMENT:");
    println!("   • SATELLITE: Market reconnaissance");
    println!("   • COMMAND ship: Bulk ELECTRONICS purchasing (40 cargo capacity)");
    println!("   • EXCAVATORS: Continue mining for funding");
    
    // Check if satellite can complete reconnaissance soon
    if satellite.nav.status == "IN_TRANSIT" {
        println!("\n⏳ NEXT STEPS:");
        println!("   1. Wait ~2 minutes for SATELLITE to arrive at X1-N5-A1");
        println!("   2. Dock and check marketplace for ELECTRONICS");
        println!("   3. If found, move COMMAND ship for bulk purchase");
        println!("   4. If not found, plan system exploration");
    }
    
    println!("\n💡 CRITICAL INSIGHT:");
    println!("   ELECTRONICS are manufactured goods - they may be rare");
    println!("   in frontier systems like X1-N5. More likely to find in:");
    println!("   • Central/core systems");
    println!("   • Industrial systems"); 
    println!("   • Systems with advanced civilizations");
    
    Ok(())
}