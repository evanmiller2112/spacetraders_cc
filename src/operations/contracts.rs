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
        
        println!("🔍 Contract Discovery Debug:");
        println!("  📊 Total contracts returned by API: {}", contracts.len());
        
        if contracts.is_empty() {
            println!("  ⚠️ No contracts available from API");
            println!("  💡 Need to negotiate new contracts with faction waypoints");
            return self.negotiate_new_contract().await;
        }
        
        // Debug: Print details of all contracts
        for (i, contract) in contracts.iter().enumerate() {
            println!("  📝 Contract #{}: {}", i + 1, contract.id);
            println!("    Status: Accepted={}, Fulfilled={}", contract.accepted, contract.fulfilled);
            println!("    Type: {}", contract.contract_type);
            println!("    Faction: {}", contract.faction_symbol);
            if contract.fulfilled {
                println!("    ✅ Already completed");
            } else if contract.accepted {
                println!("    🔄 In progress");
            } else {
                println!("    🆕 Available for acceptance");
            }
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
            // No new contracts to accept - look for active (accepted but not fulfilled) contracts
            println!("  ℹ️ No new contracts to accept - checking for active contracts");
            
            // First, let's categorize all contracts
            let fulfilled_contracts: Vec<_> = contracts.iter()
                .filter(|c| c.fulfilled)
                .cloned()
                .collect();
            
            // Filter contracts: accepted=true AND fulfilled=false
            let active_contracts: Vec<_> = contracts.into_iter()
                .filter(|c| c.accepted && !c.fulfilled)
                .collect();
            
            if active_contracts.is_empty() {
                // Check if we have any fulfilled contracts to report
                
                if !fulfilled_contracts.is_empty() {
                    println!("  🎉 Found {} fulfilled contract(s):", fulfilled_contracts.len());
                    for contract in &fulfilled_contracts {
                        println!("    ✅ {} - COMPLETED", contract.id);
                    }
                    println!("  🔍 No active contracts found - attempting to negotiate new contracts");
                } else {
                    println!("  📋 No active contracts found - attempting to negotiate new contracts");
                }
                
                // All contracts are completed - need to negotiate new ones!
                // This is the key issue: completed contracts block the 1-contract slot
                println!("  🎯 All contracts completed - negotiating new contracts to replace completed ones");
                self.negotiate_new_contract().await
            } else {
                println!("  📋 Found {} active contract(s) to work on:", active_contracts.len());
                for contract in &active_contracts {
                    let progress: i32 = contract.terms.deliver.iter()
                        .map(|d| d.units_fulfilled)
                        .sum();
                    let required: i32 = contract.terms.deliver.iter()
                        .map(|d| d.units_required)
                        .sum();
                    let percentage = if required > 0 { (progress * 100) / required } else { 0 };
                    
                    println!("    🔄 {} - {}% complete ({}/{})", 
                            contract.id, percentage, progress, required);
                }
                
                // For now, return the first active contract
                // TODO: In the future, we could work on multiple contracts simultaneously
                Ok(Some(active_contracts[0].clone()))
            }
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
        
        // First, check if contract is already 100% complete and just needs fulfillment
        println!("🔍 Checking if contract is already ready for fulfillment...");
        let fresh_contract = match self.client.get_contracts().await {
            Ok(contracts) => contracts.into_iter().find(|c| c.id == contract.id),
            Err(e) => {
                println!("  ⚠️ Could not fetch contract status: {}", e);
                None
            }
        };

        if let Some(fresh_contract) = &fresh_contract {
            let total_units_fulfilled: i32 = fresh_contract.terms.deliver.iter()
                .map(|d| d.units_fulfilled)
                .sum();
            let total_units_required: i32 = fresh_contract.terms.deliver.iter()
                .map(|d| d.units_required)
                .sum();
            
            println!("  📊 Contract status: {}/{} units fulfilled ({}%)", 
                    total_units_fulfilled, total_units_required,
                    (total_units_fulfilled * 100) / total_units_required.max(1));
            
            if total_units_fulfilled >= total_units_required {
                println!("🎉 CONTRACT ALREADY 100% COMPLETE! Executing fulfillment...");
                
                match self.fulfill_contract(&contract.id).await {
                    Ok(fulfill_data) => {
                        println!("🎆 CONTRACT FULFILLED SUCCESSFULLY!");
                        println!("  💰 Payment received: {} credits", contract.terms.payment.on_fulfilled);
                        println!("  📊 New agent credits: {}", fulfill_data.agent.credits);
                        println!("  🏆 Contract ID: {} COMPLETED", contract.id);
                        
                        return Ok(true);
                    }
                    Err(e) => {
                        println!("❌ Contract fulfillment failed: {}", e);
                        // Continue with delivery operations in case we need to deliver more
                    }
                }
            } else {
                println!("  📈 Contract needs more deliveries before fulfillment");
            }
        }
        
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
        
        // Get fresh contract status to check actual fulfillment progress
        let fresh_contract = match self.client.get_contracts().await {
            Ok(contracts) => contracts.into_iter().find(|c| c.id == contract.id),
            Err(e) => {
                println!("  ⚠️ Could not fetch contract status: {}", e);
                None
            }
        };

        let contract_ready_for_fulfillment = if let Some(fresh_contract) = fresh_contract {
            let total_units_fulfilled: i32 = fresh_contract.terms.deliver.iter()
                .map(|d| d.units_fulfilled)
                .sum();
            let total_units_required: i32 = fresh_contract.terms.deliver.iter()
                .map(|d| d.units_required)
                .sum();
            
            println!("  📊 Contract status: {}/{} units fulfilled", total_units_fulfilled, total_units_required);
            
            if total_units_fulfilled >= total_units_required {
                println!("  ✅ Contract is 100% complete and ready for fulfillment!");
                true
            } else {
                println!("  📈 Contract progress: {}% complete", 
                        (total_units_fulfilled * 100) / total_units_required.max(1));
                false
            }
        } else {
            // Fallback to old logic if we can't get fresh contract data
            println!("  ⚠️ Using fallback logic - delivered {} units this session", total_delivered);
            total_delivered >= required_materials
        };
        
        if contract_ready_for_fulfillment {
            println!("🎉 CONTRACT READY FOR FULFILLMENT! Executing fulfillment...");
            
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

    /// Negotiate new contracts when needed (e.g., when all current contracts are completed)
    /// 
    /// Requirements for successful contract negotiation:
    /// 1. Ship must be at a faction waypoint
    /// 2. Ship must be DOCKED (will automatically dock if in orbit)
    /// 3. Agent must have available contract slots (max 1 contract at a time)
    /// 4. Ship must not be in transit
    pub async fn negotiate_new_contract(&self) -> Result<Option<Contract>, Box<dyn std::error::Error>> {
        println!("🤝 Starting contract negotiation process...");
        
        // Get ships that are at faction waypoints
        let ships = self.client.get_ships().await?;
        let mut suitable_ships = Vec::new();
        
        for ship in &ships {
            // Skip ships that are in transit
            if ship.nav.status == "IN_TRANSIT" {
                println!("  ⚠️ {} in transit - skipping for contract negotiation", ship.symbol);
                continue;
            }
            
            // Get waypoint info to check for faction presence
            let waypoint_parts: Vec<&str> = ship.nav.waypoint_symbol.split('-').collect();
            let system_symbol = format!("{}-{}", waypoint_parts[0], waypoint_parts[1]);
            
            match self.client.get_waypoint(&system_symbol, &ship.nav.waypoint_symbol).await {
                Ok(waypoint) => {
                    if let Some(faction) = &waypoint.faction {
                        println!("  ✅ {} at faction waypoint {} ({})", 
                                ship.symbol, 
                                waypoint.symbol, 
                                faction.symbol);
                        suitable_ships.push((ship, waypoint));
                    } else {
                        println!("  ❌ {} at {} (no faction)", ship.symbol, ship.nav.waypoint_symbol);
                    }
                }
                Err(e) => {
                    println!("  ⚠️ Could not check waypoint {} for {}: {}", 
                            ship.nav.waypoint_symbol, ship.symbol, e);
                }
            }
        }
        
        if suitable_ships.is_empty() {
            println!("  ❌ No ships at faction waypoints for contract negotiation");
            println!("  💡 Ships need to visit faction-controlled waypoints to negotiate contracts");
            return Ok(None);
        }
        
        // Try to negotiate with the first suitable ship
        let (ship, waypoint) = &suitable_ships[0];
        println!("  🤝 Attempting contract negotiation with {} at {}", ship.symbol, waypoint.symbol);
        
        // CRITICAL: Ship must be docked to negotiate contracts!
        if ship.nav.status != "DOCKED" {
            println!("  🛸 Ship not docked - docking {} at {}...", ship.symbol, waypoint.symbol);
            match self.client.dock_ship(&ship.symbol).await {
                Ok(_) => println!("    ✅ Successfully docked for contract negotiation"),
                Err(e) => {
                    println!("    ❌ Failed to dock {}: {}", ship.symbol, e);
                    println!("    🔄 Trying next ship...");
                    // Try with other ships if docking failed
                    for (ship, waypoint) in suitable_ships.iter().skip(1).take(2) {
                        println!("  🔄 Trying with {} at {}...", ship.symbol, waypoint.symbol);
                        if ship.nav.status != "DOCKED" {
                            if let Err(e) = self.client.dock_ship(&ship.symbol).await {
                                println!("    ❌ Also failed to dock {}: {}", ship.symbol, e);
                                continue;
                            }
                        }
                        // Try to negotiate with this ship now that it's docked
                        match self.client.negotiate_contract(&ship.symbol).await {
                            Ok(new_contract) => {
                                println!("  ✅ Success with docked ship {}! Contract: {}", ship.symbol, new_contract.id);
                                match self.accept_contract(&new_contract.id).await {
                                    Ok(_) => {
                                        println!("  🤝 Contract {} accepted!", new_contract.id);
                                        return Ok(Some(new_contract));
                                    }
                                    Err(e) => {
                                        println!("  ⚠️ Could not accept: {}", e);
                                        return Ok(Some(new_contract));
                                    }
                                }
                            }
                            Err(e) => {
                                println!("  ❌ Still failed with docked {}: {}", ship.symbol, e);
                                continue;
                            }
                        }
                    }
                    return Ok(None);
                }
            }
        }
        
        match self.client.negotiate_contract(&ship.symbol).await {
            Ok(new_contract) => {
                println!("  ✅ Successfully negotiated new contract: {}", new_contract.id);
                println!("    Faction: {}", new_contract.faction_symbol);
                println!("    Type: {}", new_contract.contract_type);
                println!("    Payment: {} on accepted, {} on fulfilled", 
                        new_contract.terms.payment.on_accepted, 
                        new_contract.terms.payment.on_fulfilled);
                
                // Show delivery requirements
                for delivery in &new_contract.terms.deliver {
                    println!("    📦 Deliver: {} x{} to {}", 
                            delivery.trade_symbol, 
                            delivery.units_required,
                            delivery.destination_symbol);
                }
                
                // Automatically accept the newly negotiated contract
                match self.accept_contract(&new_contract.id).await {
                    Ok(_) => {
                        println!("  🤝 Contract {} accepted automatically!", new_contract.id);
                        return Ok(Some(new_contract));
                    }
                    Err(e) => {
                        println!("  ⚠️ Could not accept negotiated contract: {}", e);
                        // Still return the contract even if acceptance failed
                        return Ok(Some(new_contract));
                    }
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("400 Bad Request") {
                    println!("  ❌ Contract negotiation failed: Ship not at faction waypoint or other requirement not met");
                    println!("    Details: {}", error_msg);
                } else if error_msg.contains("409") {
                    println!("  ❌ Contract negotiation failed: Already have maximum contracts (1)");
                    println!("    💡 This suggests the completed contract is still blocking the slot");
                } else {
                    println!("  ❌ Contract negotiation failed: {}", error_msg);
                }
                
                // Try with other ships if available
                for (ship, waypoint) in suitable_ships.iter().skip(1).take(2) {
                    println!("  🔄 Trying with {} at {}...", ship.symbol, waypoint.symbol);
                    
                    // Ensure ship is docked before negotiating
                    if ship.nav.status != "DOCKED" {
                        if let Err(e) = self.client.dock_ship(&ship.symbol).await {
                            println!("    ❌ Failed to dock {}: {}", ship.symbol, e);
                            continue;
                        }
                        println!("    ✅ {} docked for negotiation", ship.symbol);
                    }
                    
                    match self.client.negotiate_contract(&ship.symbol).await {
                        Ok(new_contract) => {
                            println!("  ✅ Success with {}! Contract: {}", ship.symbol, new_contract.id);
                            
                            // Auto-accept
                            match self.accept_contract(&new_contract.id).await {
                                Ok(_) => {
                                    println!("  🤝 Contract {} accepted!", new_contract.id);
                                    return Ok(Some(new_contract));
                                }
                                Err(e) => {
                                    println!("  ⚠️ Could not accept: {}", e);
                                    return Ok(Some(new_contract));
                                }
                            }
                        }
                        Err(e) => {
                            println!("  ❌ Also failed with {}: {}", ship.symbol, e);
                        }
                    }
                }
            }
        }
        
        println!("  ❌ All contract negotiation attempts failed");
        println!("  💡 Will continue autonomous operations without contracts");
        Ok(None)
    }
}