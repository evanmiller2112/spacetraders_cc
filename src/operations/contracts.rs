// Contract operations module
use crate::client::SpaceTradersClient;
use crate::models::*;
use crate::operations::{ShipOperations, ProductKnowledge};
use crate::{o_error, o_summary, o_info, o_debug};
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
        o_info!("📋 Checking available contracts...");
        
        let contracts = self.get_contracts().await?;
        
        o_debug!("🔍 Contract Discovery Debug:");
        o_debug!("  📊 Total contracts returned by API: {}", contracts.len());
        
        if contracts.is_empty() {
            o_info!("  ⚠️ No contracts available from API");
            o_info!("  💡 Need to negotiate new contracts with faction waypoints");
            return self.negotiate_new_contract().await;
        }
        
        // Debug: Print details of all contracts
        for (i, contract) in contracts.iter().enumerate() {
            o_debug!("  📝 Contract #{}: {}", i + 1, contract.id);
            o_debug!("    Status: Accepted={}, Fulfilled={}", contract.accepted, contract.fulfilled);
            o_info!("    Type: {}", contract.contract_type);
            o_info!("    Faction: {}", contract.faction_symbol);
            if contract.fulfilled {
                o_debug!("    ✅ Already completed");
            } else if contract.accepted {
                o_debug!("    🔄 In progress");
            } else {
                o_debug!("    🆕 Available for acceptance");
            }
        }

        // Find the best unaccepted contract
        let mut best_contract = None;
        let mut best_score = 0i64;

        for contract in &contracts {
            if !contract.accepted {
                let score = self.score_contract(contract);
                o_info!("📝 Found contract: {} (Type: {})", contract.id, contract.contract_type);
                o_info!("  Faction: {}", contract.faction_symbol);
                o_info!("  Payment: {} on accepted, {} on fulfilled", 
                        contract.terms.payment.on_accepted, contract.terms.payment.on_fulfilled);
                o_info!("  Deadline to Accept: {}", contract.deadline_to_accept);
                o_info!("  Delivery Requirements:");
                
                for delivery in &contract.terms.deliver {
                    o_info!("    - {} x{} to {}", 
                            delivery.trade_symbol, delivery.units_required, delivery.destination_symbol);
                }
                o_debug!("  Contract Score: {}", score);

                if score > best_score {
                    best_score = score;
                    best_contract = Some(contract);
                }
            }
        }

        if let Some(contract) = best_contract {
            o_info!("🤝 Accepting contract {}...", contract.id);
            match self.accept_contract(&contract.id).await {
                Ok(_) => {
                    o_summary!("  ✅ Contract accepted successfully!");
                    Ok(Some((*contract).clone()))
                }
                Err(e) => {
                    o_info!("  ⚠️ Could not accept contract (might already be accepted): {}", e);
                    o_info!("  Continuing with mission analysis...");
                    Ok(Some((*contract).clone()))
                }
            }
        } else {
            // No new contracts to accept - look for active (accepted but not fulfilled) contracts
            o_info!("  ℹ️ No new contracts to accept - checking for active contracts");
            
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
                    o_summary!("  🎉 Found {} fulfilled contract(s):", fulfilled_contracts.len());
                    for contract in &fulfilled_contracts {
                        o_summary!("    ✅ {} - COMPLETED", contract.id);
                    }
                    o_info!("  🔍 No active contracts found - attempting to negotiate new contracts");
                } else {
                    o_info!("  📋 No active contracts found - attempting to negotiate new contracts");
                }
                
                // All contracts are completed - need to negotiate new ones!
                // This is the key issue: completed contracts block the 1-contract slot
                o_info!("  🎯 All contracts completed - negotiating new contracts to replace completed ones");
                self.negotiate_new_contract().await
            } else {
                o_info!("  📋 Found {} active contract(s) to work on:", active_contracts.len());
                for contract in &active_contracts {
                    let progress: i32 = contract.terms.deliver.iter()
                        .map(|d| d.units_fulfilled)
                        .sum();
                    let required: i32 = contract.terms.deliver.iter()
                        .map(|d| d.units_required)
                        .sum();
                    let percentage = if required > 0 { (progress * 100) / required } else { 0 };
                    
                    o_info!("    🔄 {} - {}% complete ({}/{})", 
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
        o_info!("📦 Starting autonomous contract delivery operations...");
        
        // First, check if contract is already 100% complete and just needs fulfillment
        o_info!("🔍 Checking if contract is already ready for fulfillment...");
        let fresh_contract = match self.client.get_contracts().await {
            Ok(contracts) => contracts.into_iter().find(|c| c.id == contract.id),
            Err(e) => {
                o_error!("  ⚠️ Could not fetch contract status: {}", e);
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
            
            o_info!("  📊 Contract status: {}/{} units fulfilled ({}%)", 
                    total_units_fulfilled, total_units_required,
                    (total_units_fulfilled * 100) / total_units_required.max(1));
            
            if total_units_fulfilled >= total_units_required {
                o_summary!("🎉 CONTRACT ALREADY 100% COMPLETE! Executing fulfillment...");
                
                match self.fulfill_contract(&contract.id).await {
                    Ok(fulfill_data) => {
                        o_summary!("🎆 CONTRACT FULFILLED SUCCESSFULLY!");
                        o_summary!("  💰 Payment received: {} credits", contract.terms.payment.on_fulfilled);
                        o_summary!("  📊 New agent credits: {}", fulfill_data.agent.credits);
                        o_summary!("  🏆 Contract ID: {} COMPLETED", contract.id);
                        
                        return Ok(true);
                    }
                    Err(e) => {
                        o_error!("❌ Contract fulfillment failed: {}", e);
                        // Continue with delivery operations in case we need to deliver more
                    }
                }
            } else {
                o_info!("  📈 Contract needs more deliveries before fulfillment");
            }
        }
        
        // Check if we need to use marketplace trading for manufactured goods
        let manufactured_goods = ["ELECTRONICS", "MACHINERY", "MEDICINE", "DRUGS", "CLOTHING", "FOOD", "JEWELRY", "TOOLS", "WEAPONS", "EQUIPMENT"];
        let needs_marketplace_trading = needed_materials.iter()
            .any(|material| manufactured_goods.contains(&material.as_str()));
        
        if needs_marketplace_trading {
            o_info!("🏭 Contract requires manufactured goods: {:?}", needed_materials);
            o_info!("🏪 Attempting marketplace trading...");
            
            match self.handle_marketplace_trading(contract).await {
                Ok(trading_initiated) => {
                    if trading_initiated {
                        o_info!("✅ Marketplace trading operations initiated");
                        return Ok(false); // Return false to continue normal cycle
                    } else {
                        o_info!("⚠️ No marketplace trading opportunities found");
                    }
                }
                Err(e) => {
                    o_error!("❌ Marketplace trading failed: {}", e);
                }
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
        
        o_info!("📈 Contract Progress Analysis:");
        o_info!("  🎯 Required: {} {}", required_materials, 
                contract.terms.deliver[0].trade_symbol);
        o_info!("  📦 Collected: {} {}", total_contract_materials, 
                contract.terms.deliver[0].trade_symbol);
        o_info!("  🚚 Ships with contract materials: {}", delivery_ready_ships.len());
        
        if total_contract_materials < required_materials {
            o_info!("🔄 Contract delivery pending - need more materials");
            o_info!("  📊 Progress: {}/{} {} collected ({}%)", 
                    total_contract_materials, required_materials, 
                    contract.terms.deliver[0].trade_symbol,
                    (total_contract_materials * 100 / required_materials.max(1)));
            o_info!("  💡 Continuing mining operations to complete contract");
            return Ok(false);
        }

        o_summary!("🎉 CONTRACT READY FOR DELIVERY!");
        
        // Navigate ships to delivery destination
        let delivery_destination = &contract.terms.deliver[0].destination_symbol;
        o_info!("\n🚀 Deploying delivery fleet to {}...", delivery_destination);
        
        for (ship, materials_count) in &delivery_ready_ships {
            o_info!("  📦 {} carrying {} contract materials", ship.symbol, materials_count);
            
            // Navigate to delivery destination if not already there
            if ship.nav.waypoint_symbol != *delivery_destination {
                o_info!("    🗺️ Navigating to {}...", delivery_destination);
                
                // Put in orbit first if docked
                if ship.nav.status == "DOCKED" {
                    match self.ship_ops.orbit(&ship.symbol).await {
                        Ok(_) => o_info!("      ✅ Ship put into orbit"),
                        Err(e) => {
                            o_error!("      ❌ Could not orbit: {}", e);
                            continue;
                        }
                    }
                }
                
                // Navigate to destination
                match self.ship_ops.navigate(&ship.symbol, delivery_destination).await {
                    Ok(nav_data) => {
                        o_info!("      ✅ Navigation started (fuel: {}/{})", 
                                nav_data.fuel.current, nav_data.fuel.capacity);
                    }
                    Err(e) => {
                        o_error!("      ❌ Navigation failed: {}", e);
                        continue;
                    }
                }
                
                // Wait for arrival
                o_info!("      ⏳ Waiting for arrival (30 seconds)...");
                sleep(Duration::from_secs(30)).await;
            } else {
                o_info!("    ✅ Already at delivery destination");
            }
        }
        
        // Get updated ship positions
        let delivery_ships = self.client.get_ships().await?;
        
        // Dock ships and deliver cargo
        let mut total_delivered = 0;
        
        for (original_ship, _) in &delivery_ready_ships {
            if let Some(current_ship) = delivery_ships.iter().find(|s| s.symbol == original_ship.symbol) {
                if current_ship.nav.waypoint_symbol != *delivery_destination {
                    o_info!("  ⚠️ {} not at delivery destination", current_ship.symbol);
                    continue;
                }
                
                // Dock for delivery
                if current_ship.nav.status != "DOCKED" {
                    o_info!("  🛸 Docking {} for cargo delivery...", current_ship.symbol);
                    match self.ship_ops.dock(&current_ship.symbol).await {
                        Ok(_) => o_info!("    ✅ Ship docked"),
                        Err(e) => {
                            o_error!("    ❌ Could not dock: {}", e);
                            continue;
                        }
                    }
                }
                
                // Deliver each contract material
                for item in &current_ship.cargo.inventory {
                    if needed_materials.contains(&item.symbol) {
                        o_info!("  📦 Delivering {} x{} {}...", 
                                item.units, item.symbol, item.name);
                        
                        match self.deliver_cargo(&current_ship.symbol, &contract.id, 
                                                &item.symbol, item.units).await {
                            Ok(delivery_data) => {
                                o_summary!("    ✅ DELIVERED! Contract updated");
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
                                    
                                o_info!("    📈 Progress: {}/{} {} delivered", 
                                        updated_delivered, required, item.symbol);
                            }
                            Err(e) => {
                                o_error!("    ❌ Delivery failed: {}", e);
                            }
                        }
                        
                        // Small delay between deliveries
                        sleep(Duration::from_millis(500)).await;
                    }
                }
            }
        }
        
        // Check if contract can be fulfilled
        o_info!("\n📋 Checking contract fulfillment status...");
        
        // Get fresh contract status to check actual fulfillment progress
        let fresh_contract = match self.client.get_contracts().await {
            Ok(contracts) => contracts.into_iter().find(|c| c.id == contract.id),
            Err(e) => {
                o_error!("  ⚠️ Could not fetch contract status: {}", e);
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
            
            o_info!("  📊 Contract status: {}/{} units fulfilled", total_units_fulfilled, total_units_required);
            
            if total_units_fulfilled >= total_units_required {
                o_summary!("  ✅ Contract is 100% complete and ready for fulfillment!");
                true
            } else {
                o_info!("  📈 Contract progress: {}% complete", 
                        (total_units_fulfilled * 100) / total_units_required.max(1));
                false
            }
        } else {
            // Fallback to old logic if we can't get fresh contract data
            o_debug!("  ⚠️ Using fallback logic - delivered {} units this session", total_delivered);
            total_delivered >= required_materials
        };
        
        if contract_ready_for_fulfillment {
            o_summary!("🎉 CONTRACT READY FOR FULFILLMENT! Executing fulfillment...");
            
            match self.fulfill_contract(&contract.id).await {
                Ok(fulfill_data) => {
                    o_summary!("🎆 CONTRACT FULFILLED SUCCESSFULLY!");
                    o_summary!("  💰 Payment received: {} credits", contract.terms.payment.on_fulfilled);
                    o_summary!("  📊 New agent credits: {}", fulfill_data.agent.credits);
                    o_summary!("  🏆 Contract ID: {} COMPLETED", contract.id);
                    
                    // Update our agent credits for ship purchasing decisions
                    let _updated_credits = fulfill_data.agent.credits;
                    o_summary!("  📈 Credit gain: +{}", 
                            contract.terms.payment.on_accepted + contract.terms.payment.on_fulfilled);
                    
                    return Ok(true);
                }
                Err(e) => {
                    o_error!("❌ Contract fulfillment failed: {}", e);
                }
            }
        } else {
            o_info!("⚠️ Contract not ready for fulfillment yet");
            o_info!("  Need to deliver {} more units", required_materials - total_delivered);
        }
        
        o_summary!("\n🎉 AUTONOMOUS CONTRACT MANAGEMENT COMPLETE!");
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
        o_info!("🤝 Starting contract negotiation process...");
        
        // Get ships that are at faction waypoints
        let ships = self.client.get_ships().await?;
        let mut suitable_ships = Vec::new();
        
        for ship in &ships {
            // Skip ships that are in transit
            if ship.nav.status == "IN_TRANSIT" {
                o_info!("  ⚠️ {} in transit - skipping for contract negotiation", ship.symbol);
                continue;
            }
            
            // Get waypoint info to check for faction presence
            let waypoint_parts: Vec<&str> = ship.nav.waypoint_symbol.split('-').collect();
            let system_symbol = format!("{}-{}", waypoint_parts[0], waypoint_parts[1]);
            
            match self.client.get_waypoint(&system_symbol, &ship.nav.waypoint_symbol).await {
                Ok(waypoint) => {
                    if let Some(faction) = &waypoint.faction {
                        o_info!("  ✅ {} at faction waypoint {} ({})", 
                                ship.symbol, 
                                waypoint.symbol, 
                                faction.symbol);
                        suitable_ships.push((ship, waypoint));
                    } else {
                        o_error!("  ❌ {} at {} (no faction)", ship.symbol, ship.nav.waypoint_symbol);
                    }
                }
                Err(e) => {
                    o_info!("  ⚠️ Could not check waypoint {} for {}: {}", 
                            ship.nav.waypoint_symbol, ship.symbol, e);
                }
            }
        }
        
        if suitable_ships.is_empty() {
            o_error!("  ❌ No ships at faction waypoints for contract negotiation");
            o_info!("  💡 Ships need to visit faction-controlled waypoints to negotiate contracts");
            return Ok(None);
        }
        
        // Try to negotiate with the first suitable ship
        let (ship, waypoint) = &suitable_ships[0];
        o_info!("  🤝 Attempting contract negotiation with {} at {}", ship.symbol, waypoint.symbol);
        
        // CRITICAL: Ship must be docked to negotiate contracts!
        if ship.nav.status != "DOCKED" {
            o_info!("  🛸 Ship not docked - docking {} at {}...", ship.symbol, waypoint.symbol);
            match self.client.dock_ship(&ship.symbol).await {
                Ok(_) => o_info!("    ✅ Successfully docked for contract negotiation"),
                Err(e) => {
                    o_error!("    ❌ Failed to dock {}: {}", ship.symbol, e);
                    o_info!("    🔄 Trying next ship...");
                    // Try with other ships if docking failed
                    for (ship, waypoint) in suitable_ships.iter().skip(1).take(2) {
                        o_info!("  🔄 Trying with {} at {}...", ship.symbol, waypoint.symbol);
                        if ship.nav.status != "DOCKED" {
                            if let Err(e) = self.client.dock_ship(&ship.symbol).await {
                                o_error!("    ❌ Also failed to dock {}: {}", ship.symbol, e);
                                continue;
                            }
                        }
                        // Try to negotiate with this ship now that it's docked
                        match self.client.negotiate_contract(&ship.symbol).await {
                            Ok(new_contract) => {
                                o_summary!("  ✅ Success with docked ship {}! Contract: {}", ship.symbol, new_contract.id);
                                match self.accept_contract(&new_contract.id).await {
                                    Ok(_) => {
                                        o_summary!("  🤝 Contract {} accepted!", new_contract.id);
                                        return Ok(Some(new_contract));
                                    }
                                    Err(e) => {
                                        o_info!("  ⚠️ Could not accept: {}", e);
                                        return Ok(Some(new_contract));
                                    }
                                }
                            }
                            Err(e) => {
                                o_error!("  ❌ Still failed with docked {}: {}", ship.symbol, e);
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
                o_summary!("  ✅ Successfully negotiated new contract: {}", new_contract.id);
                o_info!("    Faction: {}", new_contract.faction_symbol);
                o_info!("    Type: {}", new_contract.contract_type);
                o_info!("    Payment: {} on accepted, {} on fulfilled", 
                        new_contract.terms.payment.on_accepted, 
                        new_contract.terms.payment.on_fulfilled);
                
                // Show delivery requirements
                for delivery in &new_contract.terms.deliver {
                    o_info!("    📦 Deliver: {} x{} to {}", 
                            delivery.trade_symbol, 
                            delivery.units_required,
                            delivery.destination_symbol);
                }
                
                // Automatically accept the newly negotiated contract
                match self.accept_contract(&new_contract.id).await {
                    Ok(_) => {
                        o_summary!("  🤝 Contract {} accepted automatically!", new_contract.id);
                        return Ok(Some(new_contract));
                    }
                    Err(e) => {
                        o_info!("  ⚠️ Could not accept negotiated contract: {}", e);
                        // Still return the contract even if acceptance failed
                        return Ok(Some(new_contract));
                    }
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("400 Bad Request") {
                    o_error!("  ❌ Contract negotiation failed: Ship not at faction waypoint or other requirement not met");
                    o_info!("    Details: {}", error_msg);
                } else if error_msg.contains("409") {
                    o_error!("  ❌ Contract negotiation failed: Already have maximum contracts (1)");
                    o_info!("    💡 This suggests the completed contract is still blocking the slot");
                } else {
                    o_error!("  ❌ Contract negotiation failed: {}", error_msg);
                }
                
                // Try with other ships if available
                for (ship, waypoint) in suitable_ships.iter().skip(1).take(2) {
                    o_info!("  🔄 Trying with {} at {}...", ship.symbol, waypoint.symbol);
                    
                    // Ensure ship is docked before negotiating
                    if ship.nav.status != "DOCKED" {
                        if let Err(e) = self.client.dock_ship(&ship.symbol).await {
                            o_error!("    ❌ Failed to dock {}: {}", ship.symbol, e);
                            continue;
                        }
                        o_info!("    ✅ {} docked for negotiation", ship.symbol);
                    }
                    
                    match self.client.negotiate_contract(&ship.symbol).await {
                        Ok(new_contract) => {
                            o_summary!("  ✅ Success with {}! Contract: {}", ship.symbol, new_contract.id);
                            
                            // Auto-accept
                            match self.accept_contract(&new_contract.id).await {
                                Ok(_) => {
                                    o_summary!("  🤝 Contract {} accepted!", new_contract.id);
                                    return Ok(Some(new_contract));
                                }
                                Err(e) => {
                                    o_info!("  ⚠️ Could not accept: {}", e);
                                    return Ok(Some(new_contract));
                                }
                            }
                        }
                        Err(e) => {
                            o_error!("  ❌ Also failed with {}: {}", ship.symbol, e);
                        }
                    }
                }
            }
        }
        
        o_error!("  ❌ All contract negotiation attempts failed");
        o_info!("  💡 Will continue autonomous operations without contracts");
        Ok(None)
    }
    
    /// Handle marketplace trading for contract materials that can't be mined
    /// Returns true if trading operations were initiated
    pub async fn handle_marketplace_trading(&self, contract: &Contract) -> Result<bool, Box<dyn std::error::Error>> {
        o_info!("🏪 Analyzing contract for marketplace trading opportunities...");
        
        // Check each delivery requirement
        for delivery in &contract.terms.deliver {
            let needed = delivery.units_required - delivery.units_fulfilled;
            
            if needed <= 0 {
                o_info!("  ✅ {} already fulfilled", delivery.trade_symbol);
                continue;
            }
            
            o_info!("  📦 Need {} units of {}", needed, delivery.trade_symbol);
            
            // Check if this is a manufactured good that requires marketplace purchase
            let manufactured_goods = [
                "ELECTRONICS", "MACHINERY", "MEDICINE", "DRUGS", "CLOTHING", 
                "FOOD", "JEWELRY", "TOOLS", "WEAPONS", "EQUIPMENT"
            ];
            
            if manufactured_goods.contains(&delivery.trade_symbol.as_str()) {
                o_info!("  🏭 {} is a manufactured good - requires marketplace purchase", delivery.trade_symbol);
                
                // Find marketplaces and trading ships
                match self.find_trading_opportunities(&delivery.trade_symbol, needed as i64, Some(&contract.id), Some(&delivery.destination_symbol)).await {
                    Ok(trading_plan) => {
                        if trading_plan.is_some() {
                            o_info!("  ✅ Trading opportunities found for {}", delivery.trade_symbol);
                            return Ok(true);
                        } else {
                            o_error!("  ❌ No trading opportunities found for {}", delivery.trade_symbol);
                        }
                    }
                    Err(e) => {
                        o_error!("  ⚠️ Error finding trading opportunities: {}", e);
                    }
                }
            } else {
                o_info!("  ⛏️ {} can be mined - continuing with mining operations", delivery.trade_symbol);
            }
        }
        
        Ok(false)
    }
    
    /// Find trading opportunities for a specific good using product knowledge
    async fn find_trading_opportunities(&self, good: &str, needed: i64, contract_id: Option<&str>, delivery_destination: Option<&str>) -> Result<Option<TradingPlan>, Box<dyn std::error::Error>> {
        o_info!("  🔍 Searching for {} trading opportunities using product knowledge...", good);
        
        let product_db = ProductKnowledge::new();
        
        // Get agent budget
        let agent = self.client.get_agent().await?;
        let budget = agent.credits;
        
        // Use product-specific reasonable pricing instead of budget/needed
        let max_reasonable_price = product_db.get_max_reasonable_price(good);
        
        o_info!("    💰 Budget: {} credits", budget);
        o_info!("    📈 Max reasonable price per unit: {} credits", max_reasonable_price);
        
        // Check if we have enough budget for at least some units
        let min_purchase_size = 10; // Try to buy at least 10 units
        let min_required_budget = min_purchase_size * max_reasonable_price;
        
        if budget < min_required_budget {
            o_info!("    ⚠️ Budget too low for marketplace trading - need {} credits minimum", min_required_budget);
            return Ok(None);
        }
        
        // Get preferred waypoint traits for this product
        let preferred_traits = product_db.get_preferred_traits(good);
        o_info!("    🎯 Looking for waypoints with traits: {:?}", preferred_traits);
        
        // Find ships to determine which systems to search
        let ships = self.client.get_ships().await?;
        let mut target_systems = std::collections::HashSet::new();
        
        // Collect systems where our ships are located
        for ship in &ships {
            if let Some(system) = self.extract_system_from_waypoint(&ship.nav.waypoint_symbol) {
                target_systems.insert(system);
            }
        }
        
        if target_systems.is_empty() {
            target_systems.insert("X1-N5".to_string()); // Fallback to default system
        }
        
        o_info!("    🗺️ Searching systems: {:?}", target_systems);
        
        // Find marketplaces with preferred traits in target systems
        let mut candidate_marketplaces = Vec::new();
        
        for system_symbol in target_systems {
            match self.client.get_system_waypoints(&system_symbol, None).await {
                Ok(waypoints) => {
                    let system_marketplaces: Vec<_> = waypoints.iter()
                        .filter(|w| {
                            // Must have MARKETPLACE trait
                            let has_marketplace = w.traits.iter().any(|t| t.symbol == "MARKETPLACE");
                            // Prefer waypoints with product-specific traits
                            let has_preferred_trait = preferred_traits.iter()
                                .any(|&pref_trait| w.traits.iter().any(|t| t.symbol == pref_trait));
                            
                            has_marketplace && (has_preferred_trait || preferred_traits.contains(&"MARKETPLACE"))
                        })
                        .cloned()
                        .collect();
                    
                    o_info!("    🏪 Found {} suitable marketplaces in {}", system_marketplaces.len(), system_symbol);
                    candidate_marketplaces.extend(system_marketplaces);
                }
                Err(e) => {
                    o_error!("    ⚠️ Failed to get waypoints for system {}: {}", system_symbol, e);
                }
            }
        }
        
        o_info!("    🏪 Found {} candidate marketplaces to check", candidate_marketplaces.len());
        
        if candidate_marketplaces.is_empty() {
            o_info!("    ❌ No suitable marketplaces found");
            return Ok(None);
        }
        
        // Find a suitable scout ship (prefer SATELLITE)
        let ships = self.client.get_ships().await?;
        let scout_ship = ships.iter()
            .find(|ship| ship.registration.role == "SATELLITE" && ship.nav.status != "IN_TRANSIT")
            .or_else(|| ships.iter().find(|ship| ship.registration.role == "COMMAND" && ship.nav.status != "IN_TRANSIT"));
        
        let scout_ship = match scout_ship {
            Some(ship) => ship,
            None => {
                o_info!("    ⚠️ No available scout ships for market reconnaissance");
                return Ok(None);
            }
        };
        
        o_info!("    🛰️ Using {} for market scouting", scout_ship.symbol);
        
        // Scout each marketplace for the good
        let mut best_option: Option<(String, i64, i64)> = None; // (marketplace, price, available)
        
        for marketplace in &candidate_marketplaces {
            o_debug!("    🏪 Scouting {} for {}...", marketplace.symbol, good);
            
            // Navigate scout ship to marketplace if needed
            if scout_ship.nav.waypoint_symbol != marketplace.symbol {
                o_debug!("      🚀 Navigating {} to {}...", scout_ship.symbol, marketplace.symbol);
                
                // Ensure ship is in orbit before navigation
                if scout_ship.nav.status == "DOCKED" {
                    match self.client.orbit_ship(&scout_ship.symbol).await {
                        Ok(_) => o_debug!("        ✅ Ship in orbit"),
                        Err(e) => {
                            if !e.to_string().contains("already in orbit") {
                                o_error!("        ⚠️ Could not orbit: {}", e);
                                continue;
                            }
                        }
                    }
                }
                
                match self.client.navigate_ship(&scout_ship.symbol, &marketplace.symbol).await {
                    Ok(_) => {
                        o_debug!("        ✅ Navigation to {} started", marketplace.symbol);
                        // Wait briefly for arrival
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    }
                    Err(e) => {
                        o_error!("        ❌ Navigation failed: {}", e);
                        continue;
                    }
                }
            }
            
            // Dock and check market
            match self.client.dock_ship(&scout_ship.symbol).await {
                Ok(_) => o_debug!("        ✅ Docked at {}", marketplace.symbol),
                Err(e) => {
                    if !e.to_string().contains("already docked") {
                        o_error!("        ❌ Docking failed: {}", e);
                        continue;
                    }
                }
            }
            
            // Check market for the good (extract system from marketplace waypoint)
            let system_symbol = self.extract_system_from_waypoint(&marketplace.symbol);
            let system_symbol = system_symbol.as_deref().unwrap_or("X1-N5");
            
            match self.client.get_market(system_symbol, &marketplace.symbol).await {
                Ok(market) => {
                    if let Some(trade_goods) = &market.trade_goods {
                        if let Some(item) = trade_goods.iter().find(|g| g.symbol == good) {
                            o_debug!("        ✅ {} FOUND!", good);
                            o_debug!("          💰 Price: {} credits/unit", item.purchase_price);
                            o_debug!("          📦 Available: {} units", item.trade_volume);
                            
                            // Use product knowledge for pricing and supply logic
                            let price_reasonable = product_db.is_reasonable_price(good, item.purchase_price.into());
                            let transaction_limit = product_db.get_transaction_limit(good);
                            let can_fulfill = if let Some(_limit) = transaction_limit {
                                // Can handle transaction limits with multiple purchases
                                item.trade_volume > 0
                            } else {
                                i64::from(item.trade_volume) >= needed
                            };
                            
                            if price_reasonable && can_fulfill {
                                o_debug!("          🎯 VIABLE OPTION: Within budget and sufficient supply");
                                
                                // Check if this is better than current best option
                                let is_better = match &best_option {
                                    Some((_, best_price, _)) => i64::from(item.purchase_price) < *best_price,
                                    None => true
                                };
                                
                                if is_better {
                                    best_option = Some((marketplace.symbol.clone(), item.purchase_price.into(), item.trade_volume.into()));
                                    o_debug!("          ⭐ NEW BEST OPTION");
                                }
                            } else {
                                if !product_db.is_reasonable_price(good, item.purchase_price.into()) {
                                    o_debug!("          ❌ Price unreasonable: {} credits (max reasonable: {})", item.purchase_price, max_reasonable_price);
                                }
                                if item.trade_volume == 0 {
                                    o_debug!("          ❌ No supply available");
                                } else if transaction_limit.is_none() && i64::from(item.trade_volume) < needed {
                                    o_debug!("          ❌ Insufficient supply: {} < {} needed (no transaction limit support)", item.trade_volume, needed);
                                }
                            }
                        } else {
                            o_debug!("        ❌ {} not available", good);
                        }
                    }
                }
                Err(e) => {
                    o_error!("        ❌ Market access failed: {}", e);
                }
            }
            
            // Rate limiting
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
        
        // Create trading plan if we found a viable option
        if let Some((marketplace, price, _available)) = best_option {
            let total_cost = needed * price;
            
            // Try multi-ship cargo distribution
            let ship_allocations = self.distribute_cargo_across_fleet(needed, &ships);
            
            if !ship_allocations.is_empty() {
                let total_capacity: i32 = ship_allocations.iter().map(|a| a.units_to_purchase).sum();
                
                if total_capacity >= needed as i32 {
                    o_info!("    ✅ TRADING PLAN CREATED:");
                    o_info!("      🏪 Source: {}", marketplace);
                    o_info!("      💰 Price: {} credits/unit", price);
                    o_info!("      📦 Quantity: {} units", needed);
                    o_summary!("      💸 Total cost: {} credits", total_cost);
                    o_info!("      🚢 Multi-ship allocation ({} ships):", ship_allocations.len());
                    for allocation in &ship_allocations {
                        o_info!("         • {}: {} units (space: {})", allocation.ship_symbol, 
                               allocation.units_to_purchase, allocation.available_cargo_space);
                    }
                    
                    // Execute trading plans for each ship
                    o_info!("    🚀 Executing multi-ship trading plans...");
                    let mut all_successful = true;
                    let mut total_purchased = 0i32;
                    
                    for allocation in &ship_allocations {
                        let ship_plan = TradingPlan {
                            good: good.to_string(),
                            quantity: allocation.units_to_purchase as i64,
                            source_marketplace: marketplace.clone(),
                            price_per_unit: price,
                            total_cost: allocation.units_to_purchase as i64 * price,
                            assigned_ship: allocation.ship_symbol.clone(),
                            contract_id: contract_id.map(|s| s.to_string()),
                            delivery_destination: delivery_destination.map(|s| s.to_string()),
                        };
                        
                        o_info!("      🚢 Executing plan for {} ({} units)...", allocation.ship_symbol, allocation.units_to_purchase);
                        
                        match self.execute_trading_plan(&ship_plan).await {
                            Ok(success) => {
                                if success {
                                    total_purchased += allocation.units_to_purchase;
                                    o_info!("      ✅ {} completed purchase", allocation.ship_symbol);
                                } else {
                                    o_error!("      ❌ {} purchase failed", allocation.ship_symbol);
                                    all_successful = false;
                                }
                            }
                            Err(e) => {
                                o_error!("      ❌ {} error: {}", allocation.ship_symbol, e);
                                all_successful = false;
                            }
                        }
                    }
                    
                    if all_successful {
                        o_summary!("    ✅ Multi-ship trading plan executed successfully!");
                        o_summary!("      📦 Total purchased: {} units across {} ships", total_purchased, ship_allocations.len());
                        
                        // Return a representative plan for the successful multi-ship operation
                        let representative_plan = TradingPlan {
                            good: good.to_string(),
                            quantity: total_purchased as i64,
                            source_marketplace: marketplace,
                            price_per_unit: price,
                            total_cost: total_purchased as i64 * price,
                            assigned_ship: format!("MULTI_SHIP_{}_SHIPS", ship_allocations.len()),
                            contract_id: contract_id.map(|s| s.to_string()),
                            delivery_destination: delivery_destination.map(|s| s.to_string()),
                        };
                        return Ok(Some(representative_plan));
                    } else {
                        o_error!("    ⚠️ Multi-ship trading partially failed (purchased: {} units)", total_purchased);
                        if total_purchased > 0 {
                            // Partial success is still progress
                            let partial_plan = TradingPlan {
                                good: good.to_string(),
                                quantity: total_purchased as i64,
                                source_marketplace: marketplace,
                                price_per_unit: price,
                                total_cost: total_purchased as i64 * price,
                                assigned_ship: format!("PARTIAL_MULTI_SHIP_{}_SHIPS", ship_allocations.len()),
                                contract_id: contract_id.map(|s| s.to_string()),
                                delivery_destination: delivery_destination.map(|s| s.to_string()),
                            };
                            return Ok(Some(partial_plan));
                        }
                    }
                } else {
                    o_error!("    ❌ No suitable trading ship found with capacity >= {}", needed);
                }
            }
        } else {
            o_error!("    ❌ No viable trading opportunities found for {}", good);
            o_info!("      💡 Consider: exploring other systems, increasing budget, or waiting for supply");
        }
        
        Ok(None)
    }
    
    /// Execute a trading plan by navigating ship to marketplace and purchasing goods
    async fn execute_trading_plan(&self, plan: &TradingPlan) -> Result<bool, Box<dyn std::error::Error>> {
        o_info!("  🛒 Executing trading plan for {} {} from {}", plan.quantity, plan.good, plan.source_marketplace);
        
        // Get current ship status
        let ship = self.client.get_ship(&plan.assigned_ship).await?;
        
        // Navigate to marketplace if not already there
        if ship.nav.waypoint_symbol != plan.source_marketplace {
            o_info!("    🚀 Navigating {} to {}...", ship.symbol, plan.source_marketplace);
            
            if ship.nav.status == "DOCKED" {
                self.client.orbit_ship(&ship.symbol).await?;
            }
            
            self.client.navigate_ship(&ship.symbol, &plan.source_marketplace).await?;
            
            // Wait for arrival (simplified - in real implementation check arrival time)
            o_info!("    ⏳ Waiting for arrival...");
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        }
        
        // Dock at marketplace
        match self.client.dock_ship(&ship.symbol).await {
            Ok(_) => o_info!("    ✅ Docked at marketplace"),
            Err(e) => {
                if !e.to_string().contains("already docked") {
                    return Err(format!("Failed to dock: {}", e).into());
                }
            }
        }
        
        // Cargo management - ensure sufficient space for purchase
        o_info!("    📦 Checking cargo space before purchase...");
        let ship = self.client.get_ship(&ship.symbol).await?;
        let needed_space = plan.quantity as i32;
        let available_space = ship.cargo.capacity - ship.cargo.units;
        
        if available_space < needed_space {
            let space_to_clear = needed_space - available_space;
            o_info!("    ⚠️ Need to clear {} cargo units (available: {}, needed: {})", 
                   space_to_clear, available_space, needed_space);
            
            // Try to sell non-contract cargo first
            let mut space_cleared = 0;
            for cargo_item in &ship.cargo.inventory {
                if space_cleared >= space_to_clear {
                    break;
                }
                
                // Don't sell contract materials - basic heuristic
                if cargo_item.symbol.contains("ORE") || cargo_item.symbol.contains("CRYSTAL") {
                    continue;
                }
                
                o_info!("    💰 Attempting to sell {} {} to make space...", cargo_item.units, cargo_item.symbol);
                
                match self.client.sell_cargo(&ship.symbol, &cargo_item.symbol, cargo_item.units).await {
                    Ok(sell_data) => {
                        space_cleared += cargo_item.units;
                        o_info!("    ✅ Sold {} {} for {} credits (space cleared: {})", 
                               cargo_item.units, cargo_item.symbol, sell_data.transaction.total_price, space_cleared);
                    }
                    Err(e) => {
                        // If selling fails, try jettisoning as last resort
                        o_info!("    ⚠️ Selling failed ({}), jettisoning instead...", e);
                        match self.client.jettison_cargo(&ship.symbol, &cargo_item.symbol, cargo_item.units).await {
                            Ok(_) => {
                                space_cleared += cargo_item.units;
                                o_info!("    🗑️ Jettisoned {} {} (space cleared: {})", 
                                       cargo_item.units, cargo_item.symbol, space_cleared);
                            }
                            Err(je) => {
                                o_error!("    ❌ Failed to clear cargo: {}", je);
                            }
                        }
                    }
                }
            }
            
            if space_cleared < space_to_clear {
                o_error!("    ❌ Could not clear enough cargo space (cleared: {}, needed: {})", 
                        space_cleared, space_to_clear);
                return Ok(false);
            } else {
                o_info!("    ✅ Cleared {} cargo units successfully", space_cleared);
            }
        } else {
            o_info!("    ✅ Sufficient cargo space available ({} units)", available_space);
        }
        
        // Purchase the goods with transaction limit handling
        let product_db = ProductKnowledge::new();
        let transaction_limit = product_db.get_transaction_limit(&plan.good);
        
        o_info!("    💰 Purchasing {} {} at {} credits/unit...", plan.quantity, plan.good, plan.price_per_unit);
        if let Some(limit) = transaction_limit {
            o_info!("    📋 Transaction limit: {} units per purchase", limit);
        }
        
        let mut remaining_to_purchase = plan.quantity as i32;
        let mut total_purchased = 0i32;
        let mut total_cost = 0i64;
        let mut final_credits = 0i64;
        
        while remaining_to_purchase > 0 {
            let purchase_amount = if let Some(limit) = transaction_limit {
                std::cmp::min(remaining_to_purchase, limit)
            } else {
                remaining_to_purchase
            };
            
            o_info!("    🛒 Attempting to purchase {} units (remaining: {})", purchase_amount, remaining_to_purchase);
            
            match self.client.purchase_cargo(&ship.symbol, &plan.good, purchase_amount).await {
                Ok(purchase_data) => {
                    total_purchased += purchase_data.transaction.units;
                    total_cost += purchase_data.transaction.total_price as i64;
                    final_credits = purchase_data.agent.credits;
                    remaining_to_purchase -= purchase_data.transaction.units;
                    
                    o_info!("    ✅ Purchased {} units (total so far: {})", 
                           purchase_data.transaction.units, total_purchased);
                    
                    if remaining_to_purchase <= 0 {
                        break;
                    }
                    
                    // Small delay between transactions to avoid rate limiting
                    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                }
                Err(e) => {
                    if total_purchased > 0 {
                        o_error!("    ⚠️ Partial purchase completed: {} units bought, but {} failed: {}", 
                                total_purchased, remaining_to_purchase, e);
                        break;
                    } else {
                        o_error!("    ❌ Purchase failed: {}", e);
                        return Ok(false);
                    }
                }
            }
        }
        
        if total_purchased > 0 {
            o_summary!("    ✅ Purchase completed!");
            o_summary!("      📦 Total purchased: {} {}", total_purchased, plan.good);
            o_summary!("      💸 Total cost: {} credits", total_cost);
            o_summary!("      💰 Remaining credits: {}", final_credits);
            
            if remaining_to_purchase > 0 {
                o_info!("      ⚠️ Partial fulfillment: {} units not purchased", remaining_to_purchase);
            }
            
            // Contract delivery integration
            if let (Some(contract_id), Some(delivery_dest)) = (&plan.contract_id, &plan.delivery_destination) {
                o_info!("    🚚 Starting contract delivery to {}...", delivery_dest);
                
                // Navigate to delivery destination
                let ship = self.client.get_ship(&plan.assigned_ship).await?;
                if ship.nav.waypoint_symbol != *delivery_dest {
                    o_info!("    🚀 Navigating {} to delivery destination {}...", ship.symbol, delivery_dest);
                    
                    if ship.nav.status == "DOCKED" {
                        self.client.orbit_ship(&ship.symbol).await?;
                    }
                    
                    self.client.navigate_ship(&ship.symbol, delivery_dest).await?;
                    
                    // Wait for arrival (simplified - could be improved with proper arrival time checking)
                    o_info!("    ⏳ Waiting for arrival at delivery destination...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                }
                
                // Dock at delivery destination
                match self.client.dock_ship(&ship.symbol).await {
                    Ok(_) => o_info!("    ✅ Docked at delivery destination"),
                    Err(e) => {
                        if !e.to_string().contains("already docked") {
                            o_error!("    ⚠️ Failed to dock at delivery destination: {}", e);
                        }
                    }
                }
                
                // Deliver cargo to contract
                let ship = self.client.get_ship(&plan.assigned_ship).await?;
                let mut total_delivered = 0i32;
                
                // Find the purchased goods in cargo and deliver them
                for cargo_item in &ship.cargo.inventory {
                    if cargo_item.symbol == plan.good {
                        let units_to_deliver = std::cmp::min(cargo_item.units, total_purchased - total_delivered);
                        
                        if units_to_deliver > 0 {
                            o_info!("    📦 Delivering {} units of {} to contract...", units_to_deliver, plan.good);
                            
                            match self.deliver_cargo(&ship.symbol, contract_id, &plan.good, units_to_deliver).await {
                                Ok(delivery_data) => {
                                    total_delivered += units_to_deliver;
                                    o_summary!("    ✅ DELIVERED {} units to contract!", units_to_deliver);
                                    
                                    // Show updated contract progress
                                    if let Some(delivery_req) = delivery_data.contract.terms.deliver
                                        .iter().find(|d| d.trade_symbol == plan.good) {
                                        o_summary!("      📊 Contract progress: {}/{} {} delivered", 
                                                 delivery_req.units_fulfilled, 
                                                 delivery_req.units_required, 
                                                 plan.good);
                                    }
                                }
                                Err(e) => {
                                    o_error!("    ❌ Failed to deliver cargo: {}", e);
                                    return Ok(false);
                                }
                            }
                        }
                        
                        if total_delivered >= total_purchased {
                            break;
                        }
                    }
                }
                
                if total_delivered > 0 {
                    o_summary!("    🎉 Contract delivery completed! Delivered {} {} units", total_delivered, plan.good);
                } else {
                    o_error!("    ⚠️ No cargo was delivered to contract");
                }
            }
            
            return Ok(true);
        } else {
            o_error!("    ❌ No units purchased");
            return Ok(false);
        }
    }
    
    /// Extract system symbol from waypoint symbol (e.g., "X1-N5-F44" -> "X1-N5")
    fn extract_system_from_waypoint(&self, waypoint_symbol: &str) -> Option<String> {
        // Waypoint format is usually SYSTEM-WAYPOINT (e.g., X1-N5-F44)
        // Find the last dash and take everything before it
        let parts: Vec<&str> = waypoint_symbol.split('-').collect();
        if parts.len() >= 3 {
            // Take first two parts for system (e.g., "X1" and "N5")
            Some(format!("{}-{}", parts[0], parts[1]))
        } else {
            None
        }
    }
    
    /// Distribute cargo requirements across multiple ships
    fn distribute_cargo_across_fleet(&self, needed_units: i64, ships: &[Ship]) -> Vec<ShipAllocation> {
        let mut allocations = Vec::new();
        let mut remaining_units = needed_units as i32;
        
        // Sort ships by available cargo space (largest first)
        let mut available_ships: Vec<_> = ships.iter()
            .filter(|ship| ship.nav.status != "IN_TRANSIT")
            .map(|ship| {
                let available_space = ship.cargo.capacity - ship.cargo.units;
                (ship, available_space)
            })
            .filter(|(_, space)| *space > 0)
            .collect();
            
        available_ships.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by available space descending
        
        for (ship, available_space) in available_ships {
            if remaining_units <= 0 {
                break;
            }
            
            let allocation_size = std::cmp::min(remaining_units, available_space);
            if allocation_size > 0 {
                allocations.push(ShipAllocation {
                    ship_symbol: ship.symbol.clone(),
                    units_to_purchase: allocation_size,
                    available_cargo_space: available_space,
                });
                remaining_units -= allocation_size;
            }
        }
        
        allocations
    }
}

// Multi-ship trading structures
#[derive(Debug, Clone)]
pub struct ShipAllocation {
    pub ship_symbol: String,
    pub units_to_purchase: i32,
    pub available_cargo_space: i32,
}

#[derive(Debug, Clone)]
pub struct MultiShipTradingPlan {
    pub good: String,
    pub total_needed: i64,
    pub source_marketplace: String,
    pub price_per_unit: i64,
    pub total_cost: i64,
    pub ship_allocations: Vec<ShipAllocation>,
}

// Enhanced trading plan structure with contract delivery integration
#[derive(Debug, Clone)]
pub struct TradingPlan {
    pub good: String,
    pub quantity: i64,
    pub source_marketplace: String,
    pub price_per_unit: i64,
    pub total_cost: i64,
    pub assigned_ship: String,
    pub contract_id: Option<String>,
    pub delivery_destination: Option<String>,
}