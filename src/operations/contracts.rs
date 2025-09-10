// Contract operations module
use crate::client::SpaceTradersClient;
use crate::models::*;
use crate::operations::ShipOperations;
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
                match self.find_trading_opportunities(&delivery.trade_symbol, needed as i64).await {
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
    
    /// Find trading opportunities for a specific good
    async fn find_trading_opportunities(&self, good: &str, needed: i64) -> Result<Option<TradingPlan>, Box<dyn std::error::Error>> {
        o_info!("  🔍 Searching for {} trading opportunities...", good);
        
        // Get agent budget
        let agent = self.client.get_agent().await?;
        let budget = agent.credits;
        let max_price_per_unit = budget / needed;
        
        o_info!("    💰 Budget: {} credits", budget);
        o_info!("    📈 Max price per unit: {} credits", max_price_per_unit);
        
        if max_price_per_unit < 100 {
            o_info!("    ⚠️ Budget too low for marketplace trading - need to continue mining");
            return Ok(None);
        }
        
        // Get all waypoints in current system
        let waypoints = self.client.get_system_waypoints("X1-N5", None).await?;
        let marketplaces: Vec<_> = waypoints.iter()
            .filter(|w| w.traits.iter().any(|t| t.symbol == "MARKETPLACE"))
            .collect();
        
        o_info!("    🏪 Found {} marketplaces to check", marketplaces.len());
        
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
        
        for marketplace in &marketplaces {
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
            
            // Check market for the good
            match self.client.get_market("X1-N5", &marketplace.symbol).await {
                Ok(market) => {
                    if let Some(trade_goods) = &market.trade_goods {
                        if let Some(item) = trade_goods.iter().find(|g| g.symbol == good) {
                            o_debug!("        ✅ {} FOUND!", good);
                            o_debug!("          💰 Price: {} credits/unit", item.purchase_price);
                            o_debug!("          📦 Available: {} units", item.trade_volume);
                            
                            if i64::from(item.purchase_price) <= max_price_per_unit && i64::from(item.trade_volume) >= needed {
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
                                if i64::from(item.purchase_price) > max_price_per_unit {
                                    o_debug!("          ❌ Too expensive: {} > {} max", item.purchase_price, max_price_per_unit);
                                }
                                if i64::from(item.trade_volume) < needed {
                                    o_debug!("          ❌ Insufficient supply: {} < {} needed", item.trade_volume, needed);
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
            
            // Find a suitable trading ship (prefer COMMAND with high cargo capacity)
            let trading_ship = ships.iter()
                .filter(|ship| ship.nav.status != "IN_TRANSIT")
                .filter(|ship| ship.cargo.capacity >= needed as i32)
                .max_by_key(|ship| ship.cargo.capacity);
            
            if let Some(ship) = trading_ship {
                o_info!("    ✅ TRADING PLAN CREATED:");
                o_info!("      🏪 Source: {}", marketplace);
                o_info!("      💰 Price: {} credits/unit", price);
                o_info!("      📦 Quantity: {} units", needed);
                o_summary!("      💸 Total cost: {} credits", total_cost);
                o_info!("      🚢 Trading ship: {} (capacity: {})", ship.symbol, ship.cargo.capacity);
                
                let trading_plan = TradingPlan {
                    good: good.to_string(),
                    quantity: needed,
                    source_marketplace: marketplace,
                    price_per_unit: price,
                    total_cost,
                    assigned_ship: ship.symbol.clone(),
                };
                
                // Execute the trading plan immediately
                o_info!("    🚀 Executing trading plan...");
                match self.execute_trading_plan(&trading_plan).await {
                    Ok(success) => {
                        if success {
                            o_summary!("    ✅ Trading plan executed successfully!");
                            return Ok(Some(trading_plan));
                        } else {
                            o_error!("    ❌ Trading plan execution failed");
                        }
                    }
                    Err(e) => {
                        o_error!("    ❌ Error executing trading plan: {}", e);
                    }
                }
            } else {
                o_error!("    ❌ No suitable trading ship found with capacity >= {}", needed);
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
        
        // Purchase the goods
        o_info!("    💰 Purchasing {} {} at {} credits/unit...", plan.quantity, plan.good, plan.price_per_unit);
        
        match self.client.purchase_cargo(&ship.symbol, &plan.good, plan.quantity as i32).await {
            Ok(purchase_data) => {
                o_summary!("    ✅ Purchase successful!");
                o_summary!("      📦 Purchased: {} {}", purchase_data.transaction.units, purchase_data.transaction.trade_symbol);
                o_summary!("      💸 Total cost: {} credits", purchase_data.transaction.total_price);
                o_summary!("      💰 Remaining credits: {}", purchase_data.agent.credits);
                
                return Ok(true);
            }
            Err(e) => {
                o_error!("    ❌ Purchase failed: {}", e);
                return Ok(false);
            }
        }
    }
}

// Trading plan structure for future implementation
#[derive(Debug, Clone)]
pub struct TradingPlan {
    pub good: String,
    pub quantity: i64,
    pub source_marketplace: String,
    pub price_per_unit: i64,
    pub total_cost: i64,
    pub assigned_ship: String,
}