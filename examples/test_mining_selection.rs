// Test the new mining target selection logic
use spacetraders_cc::{SpaceTradersClient, admiral::load_agent_token};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = load_agent_token()?;
    let mut client = SpaceTradersClient::new(token);
    client.set_debug_mode(false);
    
    println!("🎯 Testing mining target selection for COPPER_ORE...");
    
    // Get waypoints in the system
    let waypoints = client.get_system_waypoints("X1-N5", None).await?;
    
    // Test the logic
    let needed_materials = vec!["COPPER_ORE".to_string()];
    
    // Simulate the deposit type determination
    let needed_deposit_trait = determine_needed_deposit_type(&needed_materials);
    println!("🔍 For {:?}, we need deposit type: {}", needed_materials, needed_deposit_trait);
    
    // Find asteroids with the right deposit type
    let suitable_asteroids: Vec<_> = waypoints.iter()
        .filter(|w| {
            (w.waypoint_type == "ASTEROID" || w.waypoint_type == "ENGINEERED_ASTEROID") &&
            w.traits.iter().any(|t| t.symbol == needed_deposit_trait)
        })
        .collect();
    
    println!("\n✅ Found {} suitable asteroids with {}:", suitable_asteroids.len(), needed_deposit_trait);
    
    for asteroid in &suitable_asteroids {
        let deposit_types: Vec<String> = asteroid.traits.iter()
            .filter(|t| t.symbol.contains("DEPOSIT"))
            .map(|t| t.symbol.clone())
            .collect();
        
        let score = calculate_mining_preference_score(asteroid);
        let has_marketplace = asteroid.traits.iter().any(|t| t.symbol == "MARKETPLACE");
        let has_fuel = asteroid.traits.iter().any(|t| t.symbol == "FUEL_STATION");
        
        println!("  🪨 {} ({}) - Score: {} {}{}", 
                asteroid.symbol, 
                asteroid.waypoint_type,
                score,
                if has_marketplace { "🏪" } else { "" },
                if has_fuel { "⛽" } else { "" }
        );
        println!("     💎 Deposits: {:?}", deposit_types);
    }
    
    // Show the best choice
    let mut sorted_asteroids = suitable_asteroids.clone();
    sorted_asteroids.sort_by(|a, b| {
        let a_score = calculate_mining_preference_score(a);
        let b_score = calculate_mining_preference_score(b);
        b_score.cmp(&a_score)
    });
    
    if let Some(best) = sorted_asteroids.first() {
        println!("\n🎯 BEST TARGET: {} (score: {})", best.symbol, calculate_mining_preference_score(best));
        println!("   This should be selected for COPPER_ORE mining!");
    }
    
    // Compare with what we USED to target (first asteroid regardless of deposits)
    let old_logic_target = waypoints.iter()
        .find(|w| w.waypoint_type == "ASTEROID" || w.waypoint_type == "ENGINEERED_ASTEROID");
        
    if let Some(old_target) = old_logic_target {
        let old_deposits: Vec<String> = old_target.traits.iter()
            .filter(|t| t.symbol.contains("DEPOSIT"))
            .map(|t| t.symbol.clone())
            .collect();
        println!("\n❌ OLD LOGIC would have chosen: {} - {:?}", old_target.symbol, old_deposits);
        println!("   (This is likely why we were getting minerals instead of copper!)");
    }
    
    Ok(())
}

/// Determine what deposit type is needed based on the materials we're looking for
fn determine_needed_deposit_type(needed_materials: &[String]) -> &'static str {
    // Check if we need metal ores (iron, copper, aluminum, etc.)
    let metal_ores = [
        "IRON_ORE", "COPPER_ORE", "ALUMINUM_ORE", "GOLD_ORE", "PLATINUM_ORE", 
        "SILVER_ORE", "URANIUM_ORE", "TITANIUM_ORE", "ZINC_ORE"
    ];
    
    // Check if we need precious materials  
    let precious_materials = [
        "PRECIOUS_STONES", "DIAMONDS", "RARE_EARTH_ELEMENTS"
    ];
    
    // Check if we need industrial minerals
    let industrial_minerals = [
        "QUARTZ_SAND", "SILICON_CRYSTALS", "CRYSTALLIZED_SULFUR", "SALT",
        "GRAPHITE", "LIMESTONE", "CLAY"
    ];
    
    for material in needed_materials {
        if metal_ores.iter().any(|&ore| material.contains(ore)) {
            return "COMMON_METAL_DEPOSITS";
        }
        if precious_materials.iter().any(|&precious| material.contains(precious)) {
            return "PRECIOUS_METAL_DEPOSITS"; // If this exists
        }
        if industrial_minerals.iter().any(|&mineral| material.contains(mineral)) {
            return "MINERAL_DEPOSITS";
        }
    }
    
    // Default to common metal deposits for unknown materials that might be ores
    "COMMON_METAL_DEPOSITS"
}

/// Calculate mining preference score (higher = better)
fn calculate_mining_preference_score(waypoint: &spacetraders_cc::Waypoint) -> i32 {
    let mut score = 0;
    
    // Prefer engineered asteroids (usually better yields)
    if waypoint.waypoint_type == "ENGINEERED_ASTEROID" {
        score += 100;
    }
    
    // Huge bonus for having a marketplace (can sell immediately)
    if waypoint.traits.iter().any(|t| t.symbol == "MARKETPLACE") {
        score += 1000;
    }
    
    // Bonus for fuel stations (can refuel on-site)  
    if waypoint.traits.iter().any(|t| t.symbol == "FUEL_STATION") {
        score += 200;
    }
    
    // Small penalty for dangerous traits
    if waypoint.traits.iter().any(|t| t.symbol == "EXPLOSIVE_GASES") {
        score -= 10;
    }
    
    score
}