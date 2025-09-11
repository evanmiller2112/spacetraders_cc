// Fix refiner module issue and implement survey-based iron ore mining
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token, client::priority_client::{PriorityApiClient, ApiPriority}, operations::ShipRoleManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let client = SpaceTradersClient::new(token);
    let priority_client = PriorityApiClient::new(client);
    
    println!("🔧 FIXING REFINER & IMPLEMENTING TARGETED ORE MINING");
    println!("====================================================");
    
    // Step 1: Check and fix refiner modules
    println!("🔧 Step 1: Checking refiner ship modules...");
    let mut role_manager = ShipRoleManager::new();
    role_manager.analyze_fleet(&priority_client).await?;
    
    let refiner_info = match role_manager.find_best_refinery_candidate() {
        Some(ship) => ship,
        None => {
            println!("❌ No refinery candidate found");
            return Ok(());
        }
    };
    
    let refiner_symbol = &refiner_info.ship_symbol;
    println!("🏭 Checking modules on refiner: {}", refiner_symbol);
    
    let ship = priority_client.get_ship(refiner_symbol).await?;
    println!("📦 Ship cargo capacity: {}", ship.cargo.capacity);
    println!("🔧 Ship modules ({}):", ship.modules.len());
    
    let mut has_ore_refinery = false;
    for module in &ship.modules {
        println!("   - {}", module.symbol);
        if module.symbol == "MODULE_ORE_REFINERY_I" {
            has_ore_refinery = true;
        }
    }
    
    if has_ore_refinery {
        println!("✅ Refiner already has MODULE_ORE_REFINERY_I!");
    } else {
        println!("❌ Missing MODULE_ORE_REFINERY_I - attempting to install...");
        
        // Try to designate as refinery (this should install the module)
        match role_manager.designate_refinery_ship(refiner_symbol, &priority_client).await {
            Ok(success) => {
                if success {
                    println!("✅ Refinery designation completed - module should be installed");
                } else {
                    println!("⚠️ Refinery designation had issues");
                }
            }
            Err(e) => {
                println!("❌ Refinery designation failed: {}", e);
            }
        }
    }
    
    // Step 2: Check iron ore requirements
    println!("\n⛏️ Step 2: Analyzing iron ore requirements...");
    
    // Check current iron ore across fleet
    let ships = priority_client.get_ships().await?;
    let mut total_ore = 0;
    for ship in &ships {
        for item in &ship.cargo.inventory {
            if item.symbol == "IRON_ORE" {
                total_ore += item.units;
            }
        }
    }
    
    println!("📊 Current iron ore: {} units", total_ore);
    println!("📊 Need for refining: 100 units");
    let ore_needed = std::cmp::max(0, 100 - total_ore);
    println!("📊 Still need: {} units", ore_needed);
    
    if ore_needed > 0 {
        println!("\n🎯 Step 3: Implementing survey-based targeted mining...");
        implement_survey_based_mining(&priority_client, ore_needed).await?;
    } else {
        println!("\n✅ Sufficient iron ore available!");
    }
    
    // Step 4: Final validation
    println!("\n🧪 Step 4: Final validation...");
    
    // Re-check refiner modules after designation
    let updated_ship = priority_client.get_ship(refiner_symbol).await?;
    let has_refinery_now = updated_ship.modules.iter()
        .any(|m| m.symbol == "MODULE_ORE_REFINERY_I");
    
    if has_refinery_now {
        println!("✅ Refiner has ore refinery module");
        
        // Check iron ore after mining
        let final_ore = ships.iter()
            .flat_map(|s| &s.cargo.inventory)
            .filter(|item| item.symbol == "IRON_ORE")
            .map(|item| item.units)
            .sum::<i32>();
            
        if final_ore >= 100 {
            println!("✅ Sufficient iron ore available: {} units", final_ore);
            println!("🎉 READY FOR REFINING VALIDATION!");
            println!("   Run: cargo run --example validate_refining_api");
        } else {
            println!("⚠️ Still need {} more iron ore units", 100 - final_ore);
        }
    } else {
        println!("❌ Refiner still missing ore refinery module");
        println!("💡 May need manual module installation at shipyard");
    }
    
    Ok(())
}

async fn implement_survey_based_mining(client: &PriorityApiClient, ore_needed: i32) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 IMPLEMENTING SURVEY-BASED TARGETED IRON ORE MINING");
    println!("=====================================================");
    
    println!("🎯 Goal: Mine {} iron ore units using survey targeting", ore_needed);
    
    // Step 1: Find mining-capable ships
    println!("🚢 Step 1: Finding mining-capable ships...");
    let ships = client.get_ships().await?;
    let mut miners = Vec::new();
    
    for ship in &ships {
        // Check if ship has mining capability (look for mining modules)
        let has_mining_laser = ship.modules.iter()
            .any(|m| m.symbol.contains("MINING_LASER") || m.symbol.contains("SURVEYOR"));
        
        let has_cargo_space = ship.cargo.capacity > ship.cargo.units;
        
        if has_mining_laser && has_cargo_space {
            miners.push(ship.symbol.clone());
            println!("⛏️ Found miner: {} ({}/{} cargo)", 
                     ship.symbol, ship.cargo.units, ship.cargo.capacity);
        }
    }
    
    if miners.is_empty() {
        println!("❌ No mining-capable ships found");
        return Ok(());
    }
    
    // Step 2: Find iron ore mining locations
    println!("\n🗺️ Step 2: Finding iron ore extraction sites...");
    
    // Look for asteroid fields or planets with iron ore
    // This is a simplified approach - in practice we'd scan the system
    let potential_sites = vec![
        "X1-N5-B7", // From debug output, this has IRON_ORE exchange
    ];
    
    println!("📍 Potential mining sites:");
    for site in &potential_sites {
        println!("   - {}", site);
    }
    
    // Step 3: Survey-based mining strategy
    println!("\n🔍 Step 3: Survey-based mining strategy...");
    println!("📋 MINING WORKFLOW (Survey-Based):");
    println!("   1. 🚀 Navigate miner to extraction site");
    println!("   2. 🔍 Create survey to identify iron ore deposits");
    println!("   3. 📊 Analyze survey results for best iron ore spots");
    println!("   4. ⛏️ Extract using targeted survey (POST /my/ships/{{ship}}/extract/survey)");
    println!("   5. 🔄 Repeat until {} iron ore units obtained", ore_needed);
    
    // Step 4: Implementation placeholder
    println!("\n⚙️ Step 4: Mining implementation...");
    println!("🚧 IMPLEMENTATION NEEDED:");
    println!("   - Survey API call: POST /my/ships/{{ship}}/survey");
    println!("   - Survey-targeted extraction: POST /my/ships/{{ship}}/extract/survey");
    println!("   - Survey result analysis for iron ore concentration");
    println!("   - Automated mining loop with survey guidance");
    
    // For now, just show the mining plan
    println!("\n📊 MINING PLAN:");
    println!("   🎯 Target: {} iron ore units", ore_needed);
    println!("   ⛏️ Available miners: {}", miners.len());
    println!("   📍 Target sites: {} locations", potential_sites.len());
    println!("   🔍 Strategy: Survey-guided extraction");
    
    println!("\n💡 NEXT IMPLEMENTATION STEPS:");
    println!("   1. Add survey API calls to SpaceTradersClient");
    println!("   2. Add survey-based extraction API calls");
    println!("   3. Implement survey analysis logic");
    println!("   4. Create mining coordination system");
    println!("   5. Integrate with refinery workflow");
    
    Ok(())
}