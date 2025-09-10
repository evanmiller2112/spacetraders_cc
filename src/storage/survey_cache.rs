// Persistent scan and survey data storage system
use std::collections::HashMap;
use std::fs;
use crate::{o_debug};
use std::path::Path;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::models::{Waypoint, SurveyData};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedWaypointData {
    pub waypoints: Vec<Waypoint>,
    pub last_scanned: DateTime<Utc>,
    pub system_symbol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedSurveyData {
    pub surveys: Vec<crate::models::Survey>,
    pub last_surveyed: DateTime<Utc>,
    pub waypoint_symbol: String,
    pub expires_at: DateTime<Utc>, // Surveys have expiration times
}


pub struct SurveyCache {
    storage_path: String,
    waypoint_cache: HashMap<String, CachedWaypointData>, // system_symbol -> waypoints
    survey_cache: HashMap<String, CachedSurveyData>,     // waypoint_symbol -> surveys
    cache_duration_hours: i64,
}

impl SurveyCache {
    pub fn new(storage_path: &str, cache_duration_hours: i64) -> Self {
        let mut cache = Self {
            storage_path: storage_path.to_string(),
            waypoint_cache: HashMap::new(),
            survey_cache: HashMap::new(),
            cache_duration_hours,
        };
        
        // Load existing cache
        if let Err(e) = cache.load_from_disk() {
            o_debug!("⚠️ Failed to load survey cache: {}", e);
            o_debug!("💾 Starting with empty survey cache");
        }
        
        cache
    }
    
    // Waypoint caching methods
    pub fn cache_system_waypoints(&mut self, system_symbol: &str, waypoints: Vec<Waypoint>) -> Result<(), Box<dyn std::error::Error>> {
        let cached_data = CachedWaypointData {
            waypoints,
            last_scanned: Utc::now(),
            system_symbol: system_symbol.to_string(),
        };
        
        self.waypoint_cache.insert(system_symbol.to_string(), cached_data);
        
        o_debug!("💾 Cached {} waypoints for system {}", 
                self.waypoint_cache[system_symbol].waypoints.len(), system_symbol);
        
        self.save_to_disk()?;
        Ok(())
    }
    
    pub fn get_cached_waypoints(&self, system_symbol: &str) -> Option<&Vec<Waypoint>> {
        if let Some(cached) = self.waypoint_cache.get(system_symbol) {
            let age_hours = Utc::now().signed_duration_since(cached.last_scanned).num_hours();
            
            if age_hours <= self.cache_duration_hours {
                o_debug!("📋 Using cached waypoints for {} (age: {}h)", system_symbol, age_hours);
                return Some(&cached.waypoints);
            } else {
                o_debug!("⏰ Waypoints for {} are stale (age: {}h > {}h)", 
                        system_symbol, age_hours, self.cache_duration_hours);
            }
        }
        None
    }
    
    pub fn should_scan_system(&self, system_symbol: &str) -> bool {
        !self.get_cached_waypoints(system_symbol).is_some()
    }
    
    // Survey caching methods
    pub fn cache_survey_data(&mut self, waypoint_symbol: &str, survey_data: &SurveyData) -> Result<(), Box<dyn std::error::Error>> {
        // Surveys typically expire in 30 minutes
        let expires_at = Utc::now() + chrono::Duration::minutes(30);
        
        let cached_data = CachedSurveyData {
            surveys: survey_data.surveys.clone(),
            last_surveyed: Utc::now(),
            waypoint_symbol: waypoint_symbol.to_string(),
            expires_at,
        };
        
        self.survey_cache.insert(waypoint_symbol.to_string(), cached_data);
        
        o_debug!("💾 Cached {} surveys for waypoint {} (expires: {})", 
                survey_data.surveys.len(), waypoint_symbol, expires_at.format("%H:%M:%S UTC"));
        
        self.save_to_disk()?;
        Ok(())
    }
    
    pub fn get_cached_surveys(&self, waypoint_symbol: &str) -> Option<&Vec<crate::models::Survey>> {
        if let Some(cached) = self.survey_cache.get(waypoint_symbol) {
            let now = Utc::now();
            
            if now < cached.expires_at {
                let remaining_minutes = cached.expires_at.signed_duration_since(now).num_minutes();
                o_debug!("📋 Using cached surveys for {} ({}min remaining)", waypoint_symbol, remaining_minutes);
                return Some(&cached.surveys);
            } else {
                o_debug!("⏰ Surveys for {} have expired", waypoint_symbol);
            }
        }
        None
    }
    
    pub fn should_survey_waypoint(&self, waypoint_symbol: &str) -> bool {
        !self.get_cached_surveys(waypoint_symbol).is_some()
    }
    
    
    // Cache management methods
    pub fn cleanup_expired(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let now = Utc::now();
        let initial_surveys = self.survey_cache.len();
        let initial_waypoints = self.waypoint_cache.len();
        
        // Remove expired surveys
        self.survey_cache.retain(|_, cached| now < cached.expires_at);
        
        // Remove stale waypoint data
        let waypoint_threshold = chrono::Duration::hours(self.cache_duration_hours);
        self.waypoint_cache.retain(|_, cached| {
            now.signed_duration_since(cached.last_scanned) <= waypoint_threshold
        });
        
        let removed_surveys = initial_surveys - self.survey_cache.len();
        let removed_waypoints = initial_waypoints - self.waypoint_cache.len();
        
        if removed_surveys > 0 || removed_waypoints > 0 {
            o_debug!("🧹 Cleaned up cache: {} surveys, {} waypoints expired", 
                    removed_surveys, removed_waypoints);
            self.save_to_disk()?;
        }
        
        Ok(())
    }
    
    pub fn print_cache_status(&self) {
        o_debug!("💾 Survey Cache Status:");
        o_debug!("   📊 Cached systems: {}", self.waypoint_cache.len());
        o_debug!("   🔍 Active surveys: {}", self.survey_cache.len());
        o_debug!("   ⏱️  Cache duration: {}h", self.cache_duration_hours);
        
        // Show system details
        for (system, cached) in &self.waypoint_cache {
            let age_hours = Utc::now().signed_duration_since(cached.last_scanned).num_hours();
            let status = if age_hours <= self.cache_duration_hours { "✅ FRESH" } else { "🔄 STALE" };
            o_debug!("     • System {}: {} waypoints {} ({}h ago)", 
                    system, cached.waypoints.len(), status, age_hours);
        }
        
        // Show survey details
        for (waypoint, cached) in &self.survey_cache {
            let remaining_minutes = cached.expires_at.signed_duration_since(Utc::now()).num_minutes();
            if remaining_minutes > 0 {
                o_debug!("     • Surveys {}: {} deposits ({}min left)", 
                        waypoint, cached.surveys.len(), remaining_minutes);
            }
        }
    }
    
    // Find cached waypoints by type or trait
    pub fn find_waypoints_with_trait(&self, system_symbol: &str, trait_symbol: &str) -> Vec<&Waypoint> {
        if let Some(waypoints) = self.get_cached_waypoints(system_symbol) {
            return waypoints.iter()
                .filter(|waypoint| {
                    waypoint.traits.iter().any(|trait_| trait_.symbol == trait_symbol)
                })
                .collect();
        }
        Vec::new()
    }
    
    pub fn find_nearest_waypoint_with_trait(&self, system_symbol: &str, trait_symbol: &str, 
                                          from_x: i32, from_y: i32) -> Option<&Waypoint> {
        let waypoints = self.find_waypoints_with_trait(system_symbol, trait_symbol);
        
        if waypoints.is_empty() {
            return None;
        }
        
        let mut nearest = waypoints[0];
        let mut min_distance = f64::MAX;
        
        for waypoint in waypoints {
            let dx = (waypoint.x - from_x) as f64;
            let dy = (waypoint.y - from_y) as f64;
            let distance = (dx * dx + dy * dy).sqrt();
            
            if distance < min_distance {
                min_distance = distance;
                nearest = waypoint;
            }
        }
        
        Some(nearest)
    }
    
    fn load_from_disk(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !Path::new(&self.storage_path).exists() {
            return Ok(()); // File doesn't exist yet, start fresh
        }
        
        let content = fs::read_to_string(&self.storage_path)?;
        
        #[derive(Deserialize)]
        struct CacheData {
            waypoint_cache: HashMap<String, CachedWaypointData>,
            survey_cache: HashMap<String, CachedSurveyData>,
        }
        
        let cache_data: CacheData = serde_json::from_str(&content)?;
        
        self.waypoint_cache = cache_data.waypoint_cache;
        self.survey_cache = cache_data.survey_cache;
        
        o_debug!("💾 Loaded cache: {} systems, {} surveys", 
                self.waypoint_cache.len(), self.survey_cache.len());
        
        // Clean up expired entries immediately after loading
        self.cleanup_expired()?;
        
        Ok(())
    }
    
    fn save_to_disk(&self) -> Result<(), Box<dyn std::error::Error>> {
        #[derive(Serialize)]
        struct CacheData<'a> {
            waypoint_cache: &'a HashMap<String, CachedWaypointData>,
            survey_cache: &'a HashMap<String, CachedSurveyData>,
        }
        
        let cache_data = CacheData {
            waypoint_cache: &self.waypoint_cache,
            survey_cache: &self.survey_cache,
        };
        
        let content = serde_json::to_string_pretty(&cache_data)?;
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = Path::new(&self.storage_path).parent() {
            fs::create_dir_all(parent)?;
        }
        
        fs::write(&self.storage_path, content)?;
        Ok(())
    }
}