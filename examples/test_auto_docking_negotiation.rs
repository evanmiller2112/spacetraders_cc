// Test automatic docking and contract negotiation
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, operations::contracts::ContractOperations};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    
    println!("🔍 Testing automatic docking and contract negotiation...");
    
    // Check current contract status
    let contracts = client.get_contracts().await?;
    println!("📋 Current Contracts: {}", contracts.len());
    
    let unfulfilled_count = contracts.iter()
        .filter(|c| !c.fulfilled)
        .count();
    
    if unfulfilled_count >= 1 {
        println!("✅ Already have {} unfulfilled contracts - no need to negotiate new ones", unfulfilled_count);
        
        for contract in &contracts {
            if !contract.fulfilled {
                println!("  📝 Active: {} - {} ({})", 
                        contract.id, 
                        contract.contract_type,
                        if contract.accepted { "ACCEPTED" } else { "PENDING ACCEPTANCE" });
            }
        }
        
        println!("💡 To test negotiation, complete all active contracts first");
        return Ok(());
    }
    
    println!("🎯 All contracts completed - testing automatic contract negotiation with docking");
    
    // Create contract operations instance
    let contract_ops = ContractOperations::new(&client);
    
    // Test the negotiation process
    match contract_ops.negotiate_new_contract().await {
        Ok(Some(new_contract)) => {
            println!("🎉 SUCCESS! Automatically negotiated and accepted new contract:");
            println!("  📝 Contract ID: {}", new_contract.id);
            println!("  🏛️ Faction: {}", new_contract.faction_symbol);
            println!("  📦 Type: {}", new_contract.contract_type);
            println!("  💰 Payment: {} + {} = {} total", 
                    new_contract.terms.payment.on_accepted,
                    new_contract.terms.payment.on_fulfilled,
                    new_contract.terms.payment.on_accepted + new_contract.terms.payment.on_fulfilled);
            
            println!("  📋 Deliverables:");
            for delivery in &new_contract.terms.deliver {
                println!("    - {} x{} to {}", 
                        delivery.trade_symbol, 
                        delivery.units_required,
                        delivery.destination_symbol);
            }
        }
        Ok(None) => {
            println!("❌ No contracts could be negotiated");
            println!("💡 This could be due to:");
            println!("   • Contract slot still blocked by completed contracts");
            println!("   • No ships at suitable faction waypoints");
            println!("   • API restrictions or cooldowns");
        }
        Err(e) => {
            println!("❌ Contract negotiation failed: {}", e);
        }
    }
    
    Ok(())
}