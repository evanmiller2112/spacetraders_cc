// Debug script to check current contract status
use spacetraders_cc::{Admiral, admiral::load_agent_token};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let admiral = Admiral::new(token);
    
    // Get current contracts and display their status
    let contracts = admiral.client.get_contracts().await?;
    
    println!("📋 Found {} total contracts:", contracts.len());
    
    for (i, contract) in contracts.iter().enumerate() {
        println!("\n{}. Contract ID: {}", i + 1, contract.id);
        println!("   Type: {}", contract.contract_type);
        println!("   Faction: {}", contract.faction_symbol);
        println!("   ✅ ACCEPTED: {}", contract.accepted);
        println!("   ✅ FULFILLED: {}", contract.fulfilled);
        println!("   Payment: {} + {} = {}", 
                contract.terms.payment.on_accepted, 
                contract.terms.payment.on_fulfilled,
                contract.terms.payment.on_accepted + contract.terms.payment.on_fulfilled);
        println!("   Deadline: {}", contract.deadline_to_accept);
        
        println!("   Delivery requirements:");
        for delivery in &contract.terms.deliver {
            println!("     - {} x{} to {} (fulfilled: {}/{})", 
                    delivery.trade_symbol, 
                    delivery.units_required, 
                    delivery.destination_symbol,
                    delivery.units_fulfilled,
                    delivery.units_required);
        }
    }
    
    // Count status
    let accepted_count = contracts.iter().filter(|c| c.accepted).count();
    let fulfilled_count = contracts.iter().filter(|c| c.fulfilled).count();
    let unaccepted_count = contracts.iter().filter(|c| !c.accepted).count();
    
    println!("\n📊 Contract Status Summary:");
    println!("   📝 Unaccepted contracts: {}", unaccepted_count);
    println!("   ✅ Accepted contracts: {}", accepted_count);
    println!("   🎉 Fulfilled contracts: {}", fulfilled_count);
    
    Ok(())
}