// Debug contract status and availability
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    
    println!("🔍 Debug contract status...");
    
    // Get current contracts
    let contracts = client.get_contracts().await?;
    
    println!("📋 Current Contracts ({})", contracts.len());
    for (i, contract) in contracts.iter().enumerate() {
        println!("  {}. {} - {}", i + 1, contract.id, contract.contract_type);
        println!("     Faction: {}", contract.faction_symbol);
        println!("     Accepted: {} | Fulfilled: {}", contract.accepted, contract.fulfilled);
        println!("     Payment: {} on accept, {} on fulfill", 
                contract.terms.payment.on_accepted, contract.terms.payment.on_fulfilled);
        
        if !contract.accepted {
            println!("     ⭐ AVAILABLE FOR ACCEPTANCE");
        } else if contract.accepted && !contract.fulfilled {
            println!("     🔄 ACTIVE - IN PROGRESS");
        } else if contract.fulfilled {
            println!("     ✅ COMPLETED");
        }
        
        println!("     Deadline to accept: {}", contract.deadline_to_accept);
        println!("     Deliveries:");
        for delivery in &contract.terms.deliver {
            println!("       - {} x{} to {} ({}/{})", 
                    delivery.trade_symbol, 
                    delivery.units_required,
                    delivery.destination_symbol,
                    delivery.units_fulfilled,
                    delivery.units_required);
        }
        println!();
    }
    
    // Check if we can negotiate a new contract
    println!("🤝 Testing contract negotiation...");
    let ships = client.get_ships().await?;
    
    for ship in ships.iter().take(1) { // Just test with one ship
        println!("  Testing with ship: {} at {}", ship.symbol, ship.nav.waypoint_symbol);
        
        match client.negotiate_contract(&ship.symbol).await {
            Ok(new_contract) => {
                println!("  ✅ Successfully negotiated contract: {}", new_contract.id);
                println!("     Faction: {}", new_contract.faction_symbol);
                println!("     Payment: {} + {}", 
                        new_contract.terms.payment.on_accepted,
                        new_contract.terms.payment.on_fulfilled);
            }
            Err(e) => {
                println!("  ❌ Contract negotiation failed: {}", e);
            }
        }
        break;
    }
    
    Ok(())
}