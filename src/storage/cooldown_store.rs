// Persistent cooldown storage system
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use chrono::{DateTime, Utc, TimeZone};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CooldownEntry {
    pub ship_symbol: String,
    pub cooldown_until: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

pub struct CooldownStore {
    storage_path: String,
    cooldowns: HashMap<String, CooldownEntry>,
}

impl CooldownStore {
    pub fn new(storage_path: &str) -> Self {
        let mut store = Self {
            storage_path: storage_path.to_string(),
            cooldowns: HashMap::new(),
        };
        
        // Load existing cooldowns
        if let Err(e) = store.load_from_disk() {
            println!("⚠️ Failed to load cooldown storage: {}", e);
            println!("💾 Starting with empty cooldown storage");
        }
        
        store
    }
    
    pub fn set_cooldown(&mut self, ship_symbol: &str, cooldown_seconds: f64) -> Result<(), Box<dyn std::error::Error>> {
        let now = Utc::now();
        let cooldown_until = now + chrono::Duration::seconds(cooldown_seconds as i64);
        
        let entry = CooldownEntry {
            ship_symbol: ship_symbol.to_string(),
            cooldown_until,
            last_updated: now,
        };
        
        self.cooldowns.insert(ship_symbol.to_string(), entry.clone());
        
        println!("💾 Stored cooldown for {}: until {} ({:.1}s)", 
                ship_symbol, 
                cooldown_until.format("%H:%M:%S UTC"),
                cooldown_seconds);
        
        self.save_to_disk()?;
        Ok(())
    }
    
    pub fn get_remaining_cooldown(&self, ship_symbol: &str) -> Option<f64> {
        if let Some(entry) = self.cooldowns.get(ship_symbol) {
            let now = Utc::now();
            let remaining = entry.cooldown_until.signed_duration_since(now);
            
            if remaining.num_seconds() > 0 {
                Some(remaining.num_seconds() as f64)
            } else {
                None // Cooldown expired
            }
        } else {
            None // No cooldown recorded
        }
    }
    
    pub fn is_on_cooldown(&self, ship_symbol: &str) -> bool {
        self.get_remaining_cooldown(ship_symbol).is_some()
    }
    
    pub fn clear_cooldown(&mut self, ship_symbol: &str) -> Result<(), Box<dyn std::error::Error>> {
        if self.cooldowns.remove(ship_symbol).is_some() {
            println!("🗑️ Cleared cooldown for {}", ship_symbol);
            self.save_to_disk()?;
        }
        Ok(())
    }
    
    pub fn cleanup_expired(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let now = Utc::now();
        let initial_count = self.cooldowns.len();
        
        self.cooldowns.retain(|_, entry| {
            entry.cooldown_until > now
        });
        
        let removed = initial_count - self.cooldowns.len();
        if removed > 0 {
            println!("🧹 Cleaned up {} expired cooldown entries", removed);
            self.save_to_disk()?;
        }
        
        Ok(())
    }
    
    pub fn list_active_cooldowns(&self) -> Vec<(String, f64)> {
        let now = Utc::now();
        let mut active = Vec::new();
        
        for (ship, entry) in &self.cooldowns {
            let remaining = entry.cooldown_until.signed_duration_since(now);
            if remaining.num_seconds() > 0 {
                active.push((ship.clone(), remaining.num_seconds() as f64));
            }
        }
        
        active.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        active
    }
    
    fn load_from_disk(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !Path::new(&self.storage_path).exists() {
            return Ok(()); // File doesn't exist yet, start fresh
        }
        
        let content = fs::read_to_string(&self.storage_path)?;
        let entries: Vec<CooldownEntry> = serde_json::from_str(&content)?;
        
        // Convert to HashMap
        self.cooldowns.clear();
        for entry in entries {
            self.cooldowns.insert(entry.ship_symbol.clone(), entry);
        }
        
        println!("💾 Loaded {} cooldown entries from disk", self.cooldowns.len());
        
        // Clean up expired entries immediately after loading
        self.cleanup_expired()?;
        
        Ok(())
    }
    
    fn save_to_disk(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Convert HashMap to Vec for serialization
        let entries: Vec<CooldownEntry> = self.cooldowns.values().cloned().collect();
        let content = serde_json::to_string_pretty(&entries)?;
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = Path::new(&self.storage_path).parent() {
            fs::create_dir_all(parent)?;
        }
        
        fs::write(&self.storage_path, content)?;
        Ok(())
    }
    
    pub fn print_status(&self) {
        let active = self.list_active_cooldowns();
        
        if active.is_empty() {
            println!("💾 Cooldown Storage: All ships ready");
        } else {
            println!("💾 Cooldown Storage: {} active cooldowns", active.len());
            for (ship, remaining) in active {
                println!("   • {}: {:.1}s remaining", ship, remaining);
            }
        }
    }
}