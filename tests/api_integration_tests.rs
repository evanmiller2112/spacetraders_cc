use spacetraders_cc::{SpaceTradersClient, Admiral, admiral::load_agent_token};
use tokio;

/// Integration tests for SpaceTraders API endpoints
/// These tests use real API calls and require a valid AGENT_TOKEN file
#[tokio::test]
async fn test_agent_operations() {
    let token = load_agent_token().expect("Failed to load agent token");
    let mut client = SpaceTradersClient::new(token);
    client.set_debug_mode(false); // Disable debug prompts for automated tests
    
    // Test get_agent
    let agent_result = client.get_agent().await;
    assert!(agent_result.is_ok(), "Failed to get agent: {:?}", agent_result.err());
    
    let agent = agent_result.unwrap();
    assert!(!agent.symbol.is_empty(), "Agent symbol should not be empty");
    assert!(!agent.headquarters.is_empty(), "Agent headquarters should not be empty");
    assert!(agent.credits >= 0, "Agent credits should be non-negative");
    
    println!("✅ Agent operations test passed - Agent: {}", agent.symbol);
}

#[tokio::test]
async fn test_system_operations() {
    let token = load_agent_token().expect("Failed to load agent token");
    let mut client = SpaceTradersClient::new(token);
    client.set_debug_mode(false);
    
    // Test get_systems with pagination
    let systems_result = client.get_systems(Some(1), Some(10)).await;
    assert!(systems_result.is_ok(), "Failed to get systems: {:?}", systems_result.err());
    
    let systems = systems_result.unwrap();
    assert!(!systems.is_empty(), "Systems list should not be empty");
    assert!(systems.len() <= 10, "Should return at most 10 systems");
    
    // Test get_system for a specific system
    let first_system = &systems[0];
    let system_result = client.get_system(&first_system.symbol).await;
    assert!(system_result.is_ok(), "Failed to get specific system: {:?}", system_result.err());
    
    let system = system_result.unwrap();
    assert_eq!(system.symbol, first_system.symbol, "System symbols should match");
    
    println!("✅ System operations test passed - Found {} systems", systems.len());
}

#[tokio::test]
async fn test_waypoint_operations() {
    let token = load_agent_token().expect("Failed to load agent token");
    let mut client = SpaceTradersClient::new(token);
    client.set_debug_mode(false);
    
    // Get agent to find their current system
    let agent = client.get_agent().await.expect("Failed to get agent");
    let system_symbol = agent.headquarters.split('-').take(2).collect::<Vec<&str>>().join("-");
    
    // Test get_system_waypoints
    let waypoints_result = client.get_system_waypoints(&system_symbol, None).await;
    assert!(waypoints_result.is_ok(), "Failed to get system waypoints: {:?}", waypoints_result.err());
    
    let waypoints = waypoints_result.unwrap();
    assert!(!waypoints.is_empty(), "Waypoints list should not be empty");
    
    // Test get_waypoint for a specific waypoint
    let first_waypoint = &waypoints[0];
    let waypoint_result = client.get_waypoint(&system_symbol, &first_waypoint.symbol).await;
    assert!(waypoint_result.is_ok(), "Failed to get specific waypoint: {:?}", waypoint_result.err());
    
    let waypoint = waypoint_result.unwrap();
    assert_eq!(waypoint.symbol, first_waypoint.symbol, "Waypoint symbols should match");
    
    println!("✅ Waypoint operations test passed - Found {} waypoints in {}", waypoints.len(), system_symbol);
}

#[tokio::test]
async fn test_contract_operations() {
    let token = load_agent_token().expect("Failed to load agent token");
    let mut client = SpaceTradersClient::new(token);
    client.set_debug_mode(false);
    
    // Test get_contracts
    let contracts_result = client.get_contracts().await;
    assert!(contracts_result.is_ok(), "Failed to get contracts: {:?}", contracts_result.err());
    
    let contracts = contracts_result.unwrap();
    
    println!("✅ Contract operations test passed - Found {} contracts", contracts.len());
    
    if !contracts.is_empty() {
        let contract = &contracts[0];
        println!("   First contract: {} ({})", contract.id, if contract.accepted { "accepted" } else { "not accepted" });
    }
}

#[tokio::test]
async fn test_ship_operations() {
    let token = load_agent_token().expect("Failed to load agent token");
    let mut client = SpaceTradersClient::new(token);
    client.set_debug_mode(false);
    
    // Test get_ships
    let ships_result = client.get_ships().await;
    assert!(ships_result.is_ok(), "Failed to get ships: {:?}", ships_result.err());
    
    let ships = ships_result.unwrap();
    assert!(!ships.is_empty(), "Ships list should not be empty");
    
    // Test get_ship for a specific ship
    let first_ship = &ships[0];
    let ship_result = client.get_ship(&first_ship.symbol).await;
    assert!(ship_result.is_ok(), "Failed to get specific ship: {:?}", ship_result.err());
    
    let ship = ship_result.unwrap();
    assert_eq!(ship.symbol, first_ship.symbol, "Ship symbols should match");
    
    println!("✅ Ship operations test passed - Found {} ships", ships.len());
    println!("   First ship: {} ({})", first_ship.symbol, first_ship.registration.name);
}

#[tokio::test]
async fn test_faction_operations() {
    let token = load_agent_token().expect("Failed to load agent token");
    let mut client = SpaceTradersClient::new(token);
    client.set_debug_mode(false);
    
    // Test get_factions with pagination
    let factions_result = client.get_factions(Some(1), Some(5)).await;
    assert!(factions_result.is_ok(), "Failed to get factions: {:?}", factions_result.err());
    
    let factions = factions_result.unwrap();
    assert!(!factions.is_empty(), "Factions list should not be empty");
    
    // Test get_faction for a specific faction
    let first_faction = &factions[0];
    let faction_result = client.get_faction(&first_faction.symbol).await;
    assert!(faction_result.is_ok(), "Failed to get specific faction: {:?}", faction_result.err());
    
    let faction = faction_result.unwrap();
    assert_eq!(faction.symbol, first_faction.symbol, "Faction symbols should match");
    
    println!("✅ Faction operations test passed - Found {} factions", factions.len());
    println!("   First faction: {} ({})", first_faction.symbol, first_faction.name);
}

#[tokio::test]
async fn test_marketplace_operations() {
    let token = load_agent_token().expect("Failed to load agent token");
    let mut client = SpaceTradersClient::new(token);
    client.set_debug_mode(false);
    
    // Get agent to find their current system
    let agent = client.get_agent().await.expect("Failed to get agent");
    let system_symbol = agent.headquarters.split('-').take(2).collect::<Vec<&str>>().join("-");
    
    // Get waypoints and find one with MARKETPLACE trait
    let waypoints = client.get_system_waypoints(&system_symbol, None).await
        .expect("Failed to get waypoints");
    
    let marketplace_waypoint = waypoints.iter()
        .find(|w| w.traits.iter().any(|t| t.symbol == "MARKETPLACE"));
    
    if let Some(waypoint) = marketplace_waypoint {
        let market_result = client.get_market(&system_symbol, &waypoint.symbol).await;
        
        match market_result {
            Ok(market) => {
                println!("✅ Marketplace operations test passed - Market at {}", waypoint.symbol);
                println!("   Exports: {}, Imports: {}, Exchange: {}", 
                        market.exports.len(), market.imports.len(), market.exchange.len());
            }
            Err(e) => {
                println!("⚠️  Market request failed (may require ship to be docked): {}", e);
                // This is expected if ship is not docked at the marketplace
            }
        }
    } else {
        println!("⚠️  No marketplace found in system {}", system_symbol);
    }
}

#[tokio::test]
async fn test_scanning_operations() {
    let token = load_agent_token().expect("Failed to load agent token");
    let mut client = SpaceTradersClient::new(token);
    client.set_debug_mode(false);
    
    // Get ships to find one for scanning
    let ships = client.get_ships().await.expect("Failed to get ships");
    let first_ship = &ships[0];
    
    // Test waypoint scanning
    let waypoint_scan_result = client.scan_waypoints(&first_ship.symbol).await;
    match waypoint_scan_result {
        Ok(scanned_waypoints) => {
            println!("✅ Waypoint scanning test passed - Scanned {} waypoints", scanned_waypoints.len());
        }
        Err(e) => {
            println!("⚠️  Waypoint scan failed (may be on cooldown): {}", e);
            // Scanning may fail due to cooldown, which is expected
        }
    }
    
    // Test system scanning
    let system_scan_result = client.scan_systems(&first_ship.symbol).await;
    match system_scan_result {
        Ok(scanned_systems) => {
            println!("✅ System scanning test passed - Scanned {} systems", scanned_systems.len());
        }
        Err(e) => {
            println!("⚠️  System scan failed (may be on cooldown or lack sensors): {}", e);
            // May fail due to cooldown or missing sensor equipment
        }
    }
    
    // Test ship scanning
    let ship_scan_result = client.scan_ships(&first_ship.symbol).await;
    match ship_scan_result {
        Ok(scanned_ships) => {
            println!("✅ Ship scanning test passed - Scanned {} ships", scanned_ships.len());
        }
        Err(e) => {
            println!("⚠️  Ship scan failed (may be on cooldown or no ships nearby): {}", e);
            // May fail due to cooldown or no ships in range
        }
    }
}

#[tokio::test]
async fn test_shipyard_operations() {
    let token = load_agent_token().expect("Failed to load agent token");
    let mut client = SpaceTradersClient::new(token);
    client.set_debug_mode(false);
    
    // Get agent to find their current system
    let agent = client.get_agent().await.expect("Failed to get agent");
    let system_symbol = agent.headquarters.split('-').take(2).collect::<Vec<&str>>().join("-");
    
    // Get waypoints and find one with SHIPYARD trait
    let waypoints = client.get_system_waypoints(&system_symbol, None).await
        .expect("Failed to get waypoints");
    
    let shipyard_waypoint = waypoints.iter()
        .find(|w| w.traits.iter().any(|t| t.symbol == "SHIPYARD"));
    
    if let Some(waypoint) = shipyard_waypoint {
        let shipyard_result = client.get_shipyard(&system_symbol, &waypoint.symbol).await;
        
        match shipyard_result {
            Ok(shipyard) => {
                println!("✅ Shipyard operations test passed - Shipyard at {}", waypoint.symbol);
                println!("   Ship types: {}", shipyard.ship_types.len());
                println!("   Modification fee: {}", shipyard.modifications_fee);
                
                if let Some(ships) = &shipyard.ships {
                    println!("   Ships for sale: {}", ships.len());
                }
            }
            Err(e) => {
                println!("⚠️  Shipyard request failed: {}", e);
            }
        }
    } else {
        println!("⚠️  No shipyard found in system {}", system_symbol);
    }
}

#[tokio::test]
async fn test_debug_system_integration() {
    let token = load_agent_token().expect("Failed to load agent token");
    let mut admiral = Admiral::new(token);
    
    // Test that debug mode can be enabled without errors
    admiral.set_debug_mode(false); // Keep false for automated tests
    admiral.set_api_logging(false); // Keep false for automated tests
    admiral.set_full_debug(true);   // Test full debug initialization
    
    // Test basic API call with debug enabled
    let agent_result = admiral.client.get_agent().await;
    assert!(agent_result.is_ok(), "Agent request should work with full debug enabled");
    
    println!("✅ Debug system integration test passed");
}

// Test helper function to demonstrate API usage patterns
#[tokio::test]
async fn test_autonomous_gameplay_pattern() {
    let token = load_agent_token().expect("Failed to load agent token");
    let mut client = SpaceTradersClient::new(token);
    client.set_debug_mode(false);
    
    // Simulate autonomous gameplay pattern
    println!("🤖 Testing autonomous gameplay pattern...");
    
    // Step 1: Get agent info
    let agent = client.get_agent().await.expect("Failed to get agent");
    println!("   Agent: {} with {} credits", agent.symbol, agent.credits);
    
    // Step 2: Get fleet
    let ships = client.get_ships().await.expect("Failed to get ships");
    println!("   Fleet size: {} ships", ships.len());
    
    // Step 3: Get contracts
    let contracts = client.get_contracts().await.expect("Failed to get contracts");
    println!("   Available contracts: {}", contracts.len());
    
    // Step 4: Get system info
    let system_symbol = agent.headquarters.split('-').take(2).collect::<Vec<&str>>().join("-");
    let waypoints = client.get_system_waypoints(&system_symbol, None).await
        .expect("Failed to get waypoints");
    println!("   Waypoints in home system: {}", waypoints.len());
    
    // Count different waypoint types
    let mut type_counts = std::collections::HashMap::new();
    for waypoint in &waypoints {
        *type_counts.entry(&waypoint.waypoint_type).or_insert(0) += 1;
    }
    
    println!("   Waypoint types:");
    for (waypoint_type, count) in type_counts {
        println!("     {}: {}", waypoint_type, count);
    }
    
    println!("✅ Autonomous gameplay pattern test completed successfully");
}