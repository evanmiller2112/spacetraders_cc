// Contract operations module
use crate::client::SpaceTradersClient;
use crate::models::*;
use crate::operations::ShipOperations;
use tokio::time::{sleep, Duration};

pub struct ContractOperations<'a> {
    client: &'a SpaceTradersClient,
    ship_ops: ShipOperations<'a>,
}

impl<'a> ContractOperations<'a> {
    pub fn new(client: &'a SpaceTradersClient) -> Self {
        let ship_ops = ShipOperations::new(client);
        Self { client, ship_ops }
    }

    // Basic contract operations
    pub async fn get_contracts(&self) -> Result<Vec<Contract>, Box<dyn std::error::Error>> {
        self.client.get_contracts().await
    }

    pub async fn accept_contract(&self, contract_id: &str) -> Result<ContractAcceptData, Box<dyn std::error::Error>> {
        self.client.accept_contract(contract_id).await
    }

    pub async fn deliver_cargo(&self, ship_symbol: &str, contract_id: &str, trade_symbol: &str, units: i32) -> Result<DeliverCargoData, Box<dyn std::error::Error>> {
        self.client.deliver_cargo(ship_symbol, contract_id, trade_symbol, units).await
    }

    pub async fn fulfill_contract(&self, contract_id: &str) -> Result<FulfillContractData, Box<dyn std::error::Error>> {
        self.client.fulfill_contract(contract_id).await
    }

    // Advanced contract operations
    pub fn get_required_materials(&self, contract: &Contract) -> Vec<String> {
        contract.terms.deliver.iter()
            .map(|delivery| delivery.trade_symbol.clone())
            .collect()
    }

    pub async fn analyze_and_accept_best_contract(&self) -> Result<Option<Contract>, Box<dyn std::error::Error>> {
        println!("📋 Checking available contracts...");
        
        let contracts = self.get_contracts().await?;
        
        if contracts.is_empty() {
            println!("⚠️ No contracts available");
            return Ok(None);
        }

        // Find the best unaccepted contract
        let mut best_contract = None;
        let mut best_score = 0i64;

        for contract in &contracts {
            if !contract.accepted {
                let score = self.score_contract(contract);
                println!("📝 Found contract: {} (Type: {})", contract.id, contract.contract_type);
                println!("  Faction: {}", contract.faction_symbol);
                println!("  Payment: {} on accepted, {} on fulfilled", 
                        contract.terms.payment.on_accepted, contract.terms.payment.on_fulfilled);
                println!("  Deadline to Accept: {}", contract.deadline_to_accept);
                println!("  Delivery Requirements:");
                
                for delivery in &contract.terms.deliver {
                    println!("    - {} x{} to {}", 
                            delivery.trade_symbol, delivery.units_required, delivery.destination_symbol);
                }
                println!("  Contract Score: {}", score);

                if score > best_score {
                    best_score = score;
                    best_contract = Some(contract);
                }
            }
        }

        if let Some(contract) = best_contract {
            println!("🤝 Accepting contract {}...", contract.id);
            match self.accept_contract(&contract.id).await {
                Ok(_) => {
                    println!("  ✅ Contract accepted successfully!");
                    Ok(Some((*contract).clone()))
                }
                Err(e) => {
                    println!("  ⚠️ Could not accept contract (might already be accepted): {}", e);
                    println!("  Continuing with mission analysis...");
                    Ok(Some((*contract).clone()))
                }
            }
        } else {
            println!("  ℹ️ All contracts already accepted");
            Ok(contracts.into_iter().find(|c| c.accepted))
        }
    }

    fn score_contract(&self, contract: &Contract) -> i64 {
        let total_payment = contract.terms.payment.on_accepted + contract.terms.payment.on_fulfilled;
        let total_units_required: i32 = contract.terms.deliver.iter()
            .map(|delivery| delivery.units_required)
            .sum();
        
        // Score based on credits per unit required
        if total_units_required > 0 {
            total_payment / total_units_required as i64
        } else {
            total_payment
        }
    }

    pub async fn execute_autonomous_contract_delivery(
        &self,
        contract: &Contract,
        needed_materials: &[String],
    ) -> Result<bool, Box<dyn std::error::Error>> {
        println!("📦 Starting autonomous contract delivery operations...");
        
        // Check if any ships have enough contract materials for delivery
        let ships_for_delivery = self.client.get_ships().await?;
        
        // Analyze contract completion status
        let mut total_contract_materials = 0;
        let mut delivery_ready_ships = Vec::new();
        
        for ship in &ships_for_delivery {
            if ship.cargo.units == 0 {
                continue;
            }
            
            let mut ship_contract_materials = 0;
            for item in &ship.cargo.inventory {
                if needed_materials.contains(&item.symbol) {
                    ship_contract_materials += item.units;
                    total_contract_materials += item.units;
                }
            }
            
            if ship_contract_materials > 0 {
                delivery_ready_ships.push((ship, ship_contract_materials));
            }
        }
        
        let required_materials: i32 = contract.terms.deliver.iter()
            .map(|d| d.units_required)
            .sum();
        
        println!("📈 Contract Progress Analysis:");
        println!("  🎯 Required: {} {}", required_materials, 
                contract.terms.deliver[0].trade_symbol);
        println!("  📦 Collected: {} {}", total_contract_materials, 
                contract.terms.deliver[0].trade_symbol);
        println!("  🚚 Ships with contract materials: {}", delivery_ready_ships.len());
        
        if total_contract_materials < required_materials {
            println!("🔄 Contract delivery pending - need more materials");
            println!("  📊 Progress: {}/{} {} collected ({}%)", 
                    total_contract_materials, required_materials, 
                    contract.terms.deliver[0].trade_symbol,
                    (total_contract_materials * 100 / required_materials.max(1)));
            println!("  💡 Continuing mining operations to complete contract");
            return Ok(false);
        }

        println!("🎉 CONTRACT READY FOR DELIVERY!");
        
        // Navigate ships to delivery destination
        let delivery_destination = &contract.terms.deliver[0].destination_symbol;
        println!("\n🚀 Deploying delivery fleet to {}...", delivery_destination);
        
        for (ship, materials_count) in &delivery_ready_ships {
            println!("  📦 {} carrying {} contract materials", ship.symbol, materials_count);
            
            // Navigate to delivery destination if not already there
            if ship.nav.waypoint_symbol != *delivery_destination {
                println!("    🗺️ Navigating to {}...", delivery_destination);
                
                // Put in orbit first if docked
                if ship.nav.status == "DOCKED" {
                    match self.ship_ops.orbit(&ship.symbol).await {
                        Ok(_) => println!("      ✅ Ship put into orbit"),
                        Err(e) => {
                            println!("      ❌ Could not orbit: {}", e);
                            continue;
                        }
                    }
                }
                
                // Navigate to destination
                match self.ship_ops.navigate(&ship.symbol, delivery_destination).await {
                    Ok(nav_data) => {
                        println!("      ✅ Navigation started (fuel: {}/{})", 
                                nav_data.fuel.current, nav_data.fuel.capacity);
                    }
                    Err(e) => {
                        println!("      ❌ Navigation failed: {}", e);
                        continue;
                    }
                }
                
                // Wait for arrival
                println!("      ⏳ Waiting for arrival (30 seconds)...");
                sleep(Duration::from_secs(30)).await;
            } else {
                println!("    ✅ Already at delivery destination");
            }
        }
        
        // Get updated ship positions
        let delivery_ships = self.client.get_ships().await?;
        
        // Dock ships and deliver cargo
        let mut total_delivered = 0;
        
        for (original_ship, _) in &delivery_ready_ships {
            if let Some(current_ship) = delivery_ships.iter().find(|s| s.symbol == original_ship.symbol) {
                if current_ship.nav.waypoint_symbol != *delivery_destination {
                    println!("  ⚠️ {} not at delivery destination", current_ship.symbol);
                    continue;
                }
                
                // Dock for delivery
                if current_ship.nav.status != "DOCKED" {
                    println!("  🛸 Docking {} for cargo delivery...", current_ship.symbol);
                    match self.ship_ops.dock(&current_ship.symbol).await {
                        Ok(_) => println!("    ✅ Ship docked"),
                        Err(e) => {
                            println!("    ❌ Could not dock: {}", e);
                            continue;
                        }
                    }
                }
                
                // Deliver each contract material
                for item in &current_ship.cargo.inventory {
                    if needed_materials.contains(&item.symbol) {
                        println!("  📦 Delivering {} x{} {}...", 
                                item.units, item.symbol, item.name);
                        
                        match self.deliver_cargo(&current_ship.symbol, &contract.id, 
                                                &item.symbol, item.units).await {
                            Ok(delivery_data) => {
                                println!("    ✅ DELIVERED! Contract updated");
                                total_delivered += item.units;
                                
                                // Show updated contract progress
                                let updated_delivered = delivery_data.contract.terms.deliver
                                    .iter()
                                    .find(|d| d.trade_symbol == item.symbol)
                                    .map(|d| d.units_fulfilled)
                                    .unwrap_or(0);
                                    
                                let required = delivery_data.contract.terms.deliver
                                    .iter()
                                    .find(|d| d.trade_symbol == item.symbol)
                                    .map(|d| d.units_required)
                                    .unwrap_or(0);
                                    
                                println!("    📈 Progress: {}/{} {} delivered", 
                                        updated_delivered, required, item.symbol);
                            }
                            Err(e) => {
                                println!("    ❌ Delivery failed: {}", e);
                            }
                        }
                        
                        // Small delay between deliveries
                        sleep(Duration::from_millis(500)).await;
                    }
                }
            }
        }
        
        // Check if contract can be fulfilled
        println!("\n📋 Checking contract fulfillment status...");
        
        if total_delivered >= required_materials {
            println!("🎉 ALL MATERIALS DELIVERED! Fulfilling contract...");
            
            match self.fulfill_contract(&contract.id).await {
                Ok(fulfill_data) => {
                    println!("🎆 CONTRACT FULFILLED SUCCESSFULLY!");
                    println!("  💰 Payment received: {} credits", contract.terms.payment.on_fulfilled);
                    println!("  📊 New agent credits: {}", fulfill_data.agent.credits);
                    println!("  🏆 Contract ID: {} COMPLETED", contract.id);
                    
                    // Update our agent credits for ship purchasing decisions
                    let _updated_credits = fulfill_data.agent.credits;
                    println!("  📈 Credit gain: +{}", 
                            contract.terms.payment.on_accepted + contract.terms.payment.on_fulfilled);
                    
                    return Ok(true);
                }
                Err(e) => {
                    println!("❌ Contract fulfillment failed: {}", e);
                }
            }
        } else {
            println!("⚠️ Contract not ready for fulfillment yet");
            println!("  Need to deliver {} more units", required_materials - total_delivered);
        }
        
        println!("\n🎉 AUTONOMOUS CONTRACT MANAGEMENT COMPLETE!");
        Ok(false)
    }
}