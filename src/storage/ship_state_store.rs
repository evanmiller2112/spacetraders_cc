// Persistent ship state storage system
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::models::Ship;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedShipState {
    pub ship: Ship,
    pub last_updated: DateTime<Utc>,
    pub last_api_refresh: DateTime<Utc>,
    pub is_stale: bool,
    pub pending_actions: Vec<String>, // Track what we've told the ship to do
}

impl CachedShipState {
    pub fn new(ship: Ship) -> Self {
        let now = Utc::now();
        Self {
            ship,
            last_updated: now,
            last_api_refresh: now,
            is_stale: false,
            pending_actions: Vec::new(),
        }
    }
    
    pub fn mark_stale(&mut self) {
        self.is_stale = true;
    }
    
    pub fn add_pending_action(&mut self, action: &str) {
        self.pending_actions.push(action.to_string());
        self.mark_stale();
    }
    
    pub fn clear_pending_actions(&mut self) {
        self.pending_actions.clear();
    }
    
    pub fn should_refresh(&self, staleness_threshold_minutes: i64) -> bool {
        let threshold = chrono::Duration::minutes(staleness_threshold_minutes);
        let now = Utc::now();
        
        // Force refresh if marked stale or if it's been too long since last API call
        self.is_stale || 
        (now.signed_duration_since(self.last_api_refresh) > threshold) ||
        !self.pending_actions.is_empty()
    }
}

pub struct ShipStateStore {
    storage_path: String,
    ships: HashMap<String, CachedShipState>,
    staleness_threshold_minutes: i64,
}

impl ShipStateStore {
    pub fn new(storage_path: &str, staleness_threshold_minutes: i64) -> Self {
        let mut store = Self {
            storage_path: storage_path.to_string(),
            ships: HashMap::new(),
            staleness_threshold_minutes,
        };
        
        // Load existing ship states
        if let Err(e) = store.load_from_disk() {
            println!("⚠️ Failed to load ship state storage: {}", e);
            println!("💾 Starting with empty ship state cache");
        }
        
        store
    }
    
    pub fn cache_ship(&mut self, ship: Ship) -> Result<(), Box<dyn std::error::Error>> {
        let ship_symbol = ship.symbol.clone();
        let cached_state = CachedShipState::new(ship);
        
        self.ships.insert(ship_symbol.clone(), cached_state);
        
        println!("💾 Cached state for {}: Location: {}, Cargo: {}/{}, Fuel: {}/{}", 
                ship_symbol,
                self.ships[&ship_symbol].ship.nav.waypoint_symbol,
                self.ships[&ship_symbol].ship.cargo.units,
                self.ships[&ship_symbol].ship.cargo.capacity,
                self.ships[&ship_symbol].ship.fuel.current,
                self.ships[&ship_symbol].ship.fuel.capacity);
        
        self.save_to_disk()?;
        Ok(())
    }
    
    pub fn get_ship_state(&self, ship_symbol: &str) -> Option<&CachedShipState> {
        self.ships.get(ship_symbol)
    }
    
    pub fn get_ship_state_mut(&mut self, ship_symbol: &str) -> Option<&mut CachedShipState> {
        self.ships.get_mut(ship_symbol)
    }
    
    pub fn should_refresh_ship(&self, ship_symbol: &str) -> bool {
        if let Some(cached) = self.ships.get(ship_symbol) {
            cached.should_refresh(self.staleness_threshold_minutes)
        } else {
            true // No cache = definitely need to refresh
        }
    }
    
    pub fn mark_ship_action(&mut self, ship_symbol: &str, action: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(cached) = self.ships.get_mut(ship_symbol) {
            cached.add_pending_action(action);
            println!("📝 Marked {} as stale due to action: {}", ship_symbol, action);
            self.save_to_disk()?;
        }
        Ok(())
    }
    
    pub fn update_ship_from_api(&mut self, ship: Ship) -> Result<(), Box<dyn std::error::Error>> {
        let ship_symbol = ship.symbol.clone();
        let now = Utc::now();
        
        if let Some(cached) = self.ships.get_mut(&ship_symbol) {
            cached.ship = ship;
            cached.last_api_refresh = now;
            cached.last_updated = now;
            cached.is_stale = false;
            cached.clear_pending_actions();
        } else {
            // First time seeing this ship
            self.ships.insert(ship_symbol.clone(), CachedShipState::new(ship));
        }
        
        println!("🔄 Refreshed {} from API", ship_symbol);
        self.save_to_disk()?;
        Ok(())
    }
    
    pub fn get_stale_ships(&self) -> Vec<String> {
        let mut stale = Vec::new();
        
        for (ship_symbol, cached) in &self.ships {
            if cached.should_refresh(self.staleness_threshold_minutes) {
                stale.push(ship_symbol.clone());
            }
        }
        
        stale
    }
    
    pub fn list_cached_ships(&self) -> Vec<String> {
        self.ships.keys().cloned().collect()
    }
    
    pub fn print_cache_status(&self) {
        println!("💾 Ship State Cache Status:");
        println!("   📊 Cached ships: {}", self.ships.len());
        println!("   ⏱️  Staleness threshold: {} minutes", self.staleness_threshold_minutes);
        
        let stale_count = self.get_stale_ships().len();
        if stale_count > 0 {
            println!("   🔄 Stale ships needing refresh: {}", stale_count);
        } else {
            println!("   ✅ All ships have fresh cache");
        }
        
        for (ship_symbol, cached) in &self.ships {
            let age_minutes = Utc::now().signed_duration_since(cached.last_api_refresh).num_minutes();
            let status = if cached.should_refresh(self.staleness_threshold_minutes) {
                "🔄 STALE"
            } else {
                "✅ FRESH"
            };
            
            println!("     • {}: {} ({}min ago, {} pending actions)", 
                    ship_symbol, status, age_minutes, cached.pending_actions.len());
            
            if !cached.pending_actions.is_empty() {
                for action in &cached.pending_actions {
                    println!("       - Pending: {}", action);
                }
            }
        }
    }
    
    pub fn cleanup_old_cache(&mut self, max_age_hours: i64) -> Result<(), Box<dyn std::error::Error>> {
        let threshold = chrono::Duration::hours(max_age_hours);
        let now = Utc::now();
        let initial_count = self.ships.len();
        
        self.ships.retain(|_, cached| {
            now.signed_duration_since(cached.last_updated) <= threshold
        });
        
        let removed = initial_count - self.ships.len();
        if removed > 0 {
            println!("🧹 Cleaned up {} old ship cache entries", removed);
            self.save_to_disk()?;
        }
        
        Ok(())
    }
    
    fn load_from_disk(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !Path::new(&self.storage_path).exists() {
            return Ok(()); // File doesn't exist yet, start fresh
        }
        
        let content = fs::read_to_string(&self.storage_path)?;
        let cached_states: Vec<CachedShipState> = serde_json::from_str(&content)?;
        
        // Convert to HashMap
        self.ships.clear();
        for cached in cached_states {
            self.ships.insert(cached.ship.symbol.clone(), cached);
        }
        
        println!("💾 Loaded {} ship states from cache", self.ships.len());
        
        // Clean up old entries immediately after loading
        self.cleanup_old_cache(24)?; // Remove entries older than 24 hours
        
        Ok(())
    }
    
    fn save_to_disk(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Convert HashMap to Vec for serialization
        let cached_states: Vec<CachedShipState> = self.ships.values().cloned().collect();
        let content = serde_json::to_string_pretty(&cached_states)?;
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = Path::new(&self.storage_path).parent() {
            fs::create_dir_all(parent)?;
        }
        
        fs::write(&self.storage_path, content)?;
        Ok(())
    }
    
    // Helper method to get fresh ship data (either from cache or force API refresh)
    pub async fn get_fresh_ship_state(&mut self, ship_symbol: &str, client: &crate::client::SpaceTradersClient) -> Result<&CachedShipState, Box<dyn std::error::Error>> {
        if self.should_refresh_ship(ship_symbol) {
            // Need to refresh from API
            let fresh_ship = client.get_ship(ship_symbol).await?;
            self.update_ship_from_api(fresh_ship)?;
        }
        
        // Return cached state (now fresh)
        self.ships.get(ship_symbol)
            .ok_or_else(|| format!("Ship {} not found after refresh", ship_symbol).into())
    }
}