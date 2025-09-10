use serde::{Deserialize, Serialize};
use crate::{o_info};
use std::fs;
use std::path::Path;
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceTradersConfig {
    pub fleet: FleetConfig,
    pub fuel: FuelConfig,
    pub credits: CreditsConfig,
    pub contracts: ContractConfig,
    pub timing: TimingConfig,
    pub navigation: NavigationConfig,
    pub caching: CachingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetConfig {
    /// Minimum credits required to consider buying a new ship
    pub min_credits_for_ship_purchase: i64,
    /// Maximum number of mining ships to maintain
    pub max_mining_ships: usize,
    /// Credits threshold for fleet expansion consideration
    pub fleet_expansion_threshold: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuelConfig {
    /// Fuel percentage below which ships should refuel (0.0 to 1.0)
    pub refuel_threshold: f64,
    /// Fuel percentage for mining ships to maintain before operations (0.0 to 1.0)
    pub mining_fuel_threshold: f64,
    /// Safety margin for fuel calculations (extra fuel units)
    pub fuel_safety_margin: i32,
    /// Fuel buffer multiplier for route planning (1.0 = no buffer, 1.2 = 20% buffer)
    pub fuel_buffer_multiplier: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditsConfig {
    /// Minimum credits to keep in reserve for emergencies
    pub min_reserve_credits: i64,
    /// Credits threshold for large contract consideration
    pub large_contract_threshold: i64,
    /// Minimum contract value to consider profitable
    pub min_profitable_contract: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractConfig {
    /// Minimum contract units to be considered "large"
    pub large_contract_units: i32,
    /// Contract cache duration in seconds
    pub cache_duration_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingConfig {
    /// Main cycle delay in seconds
    pub main_cycle_delay_seconds: u64,
    /// Retry delay after errors in seconds
    pub error_retry_delay_seconds: u64,
    /// Config hot-reload check interval in seconds
    pub config_reload_interval_seconds: u64,
    /// Fleet coordination timeout in seconds
    pub fleet_coordination_timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationConfig {
    /// Maximum distance to consider for route planning
    pub max_route_distance: f64,
    /// Minimum API retry delay in seconds
    pub min_retry_delay_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachingConfig {
    /// Ship state cache staleness threshold in minutes
    pub ship_state_staleness_minutes: i64,
    /// Survey cache duration in hours
    pub survey_cache_hours: i64,
    /// Survey expiration time in minutes
    pub survey_expiration_minutes: i64,
}

impl Default for SpaceTradersConfig {
    fn default() -> Self {
        Self {
            fleet: FleetConfig {
                min_credits_for_ship_purchase: 150000,
                max_mining_ships: 5,
                fleet_expansion_threshold: 200000,
            },
            fuel: FuelConfig {
                refuel_threshold: 0.2,        // 20%
                mining_fuel_threshold: 0.5,   // 50%
                fuel_safety_margin: 10,       // 10 fuel units
                fuel_buffer_multiplier: 1.2,  // 20% buffer
            },
            credits: CreditsConfig {
                min_reserve_credits: 20000,
                large_contract_threshold: 10000,
                min_profitable_contract: 10000,
            },
            contracts: ContractConfig {
                large_contract_units: 30,
                cache_duration_seconds: 30,
            },
            timing: TimingConfig {
                main_cycle_delay_seconds: 30,
                error_retry_delay_seconds: 60,
                config_reload_interval_seconds: 30,
                fleet_coordination_timeout_seconds: 300,
            },
            navigation: NavigationConfig {
                max_route_distance: 100.0,
                min_retry_delay_seconds: 0.1,
            },
            caching: CachingConfig {
                ship_state_staleness_minutes: 5,
                survey_cache_hours: 12,
                survey_expiration_minutes: 30,
            },
        }
    }
}

impl SpaceTradersConfig {
    /// Load configuration from file, creating default if it doesn't exist
    pub fn load_or_create(config_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        if Path::new(config_path).exists() {
            o_info!("📋 Loading configuration from {}", config_path);
            let config_str = fs::read_to_string(config_path)?;
            let config: SpaceTradersConfig = toml::from_str(&config_str)?;
            Ok(config)
        } else {
            o_info!("📋 Creating default configuration at {}", config_path);
            let config = SpaceTradersConfig::default();
            config.save(config_path)?;
            o_info!("💡 Edit {} to customize bot behavior", config_path);
            Ok(config)
        }
    }

    /// Save configuration to file
    pub fn save(&self, config_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Create directory if it doesn't exist
        if let Some(parent) = Path::new(config_path).parent() {
            fs::create_dir_all(parent)?;
        }
        
        let config_str = toml::to_string_pretty(self)?;
        fs::write(config_path, config_str)?;
        Ok(())
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<(), String> {
        // Validate fuel thresholds are percentages
        if self.fuel.refuel_threshold < 0.0 || self.fuel.refuel_threshold > 1.0 {
            return Err("refuel_threshold must be between 0.0 and 1.0".to_string());
        }
        if self.fuel.mining_fuel_threshold < 0.0 || self.fuel.mining_fuel_threshold > 1.0 {
            return Err("mining_fuel_threshold must be between 0.0 and 1.0".to_string());
        }

        // Validate positive values
        if self.fleet.min_credits_for_ship_purchase < 0 {
            return Err("min_credits_for_ship_purchase must be positive".to_string());
        }
        if self.credits.min_reserve_credits < 0 {
            return Err("min_reserve_credits must be positive".to_string());
        }

        // Validate timing values
        if self.timing.main_cycle_delay_seconds == 0 {
            return Err("main_cycle_delay_seconds must be greater than 0".to_string());
        }

        o_info!("✅ Configuration validation passed");
        Ok(())
    }

    /// Print configuration summary
    pub fn print_summary(&self) {
        o_info!("📋 Configuration Summary:");
        o_info!("   💰 Ship purchase: {} credits minimum", self.fleet.min_credits_for_ship_purchase);
        o_info!("   ⛽ Refuel threshold: {:.1}%", self.fuel.refuel_threshold * 100.0);
        o_info!("   ⏰ Cycle delay: {}s", self.timing.main_cycle_delay_seconds);
        o_info!("   📦 Contract cache: {}s", self.contracts.cache_duration_seconds);
        o_info!("   🔄 Config reload: {}s", self.timing.config_reload_interval_seconds);
    }
}

/// Hot-reloadable configuration manager
#[derive(Debug)]
pub struct ConfigManager {
    config: SpaceTradersConfig,
    config_path: String,
    last_modified: Option<SystemTime>,
    last_reload_check: SystemTime,
}

impl ConfigManager {
    pub fn new(config_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config = SpaceTradersConfig::load_or_create(config_path)?;
        config.validate()?;
        config.print_summary();
        
        let last_modified = fs::metadata(config_path)
            .and_then(|m| m.modified())
            .ok();
        
        Ok(Self {
            config,
            config_path: config_path.to_string(),
            last_modified,
            last_reload_check: SystemTime::now(),
        })
    }
    
    /// Get current configuration
    pub fn config(&self) -> &SpaceTradersConfig {
        &self.config
    }
    
    /// Check if config should be reloaded and do so if needed
    pub fn check_and_reload(&mut self) -> bool {
        let now = SystemTime::now();
        let reload_interval = std::time::Duration::from_secs(self.config.timing.config_reload_interval_seconds);
        
        // Only check file system at the configured interval
        if now.duration_since(self.last_reload_check).unwrap_or_default() < reload_interval {
            return false;
        }
        
        self.last_reload_check = now;
        
        // Check if file was modified
        if let Ok(metadata) = fs::metadata(&self.config_path) {
            if let Ok(modified) = metadata.modified() {
                if Some(modified) != self.last_modified {
                    return self.reload_config(modified);
                }
            }
        }
        
        false
    }
    
    fn reload_config(&mut self, new_modified_time: SystemTime) -> bool {
        match SpaceTradersConfig::load_or_create(&self.config_path) {
            Ok(new_config) => {
                match new_config.validate() {
                    Ok(_) => {
                        let old_values = format!("cycle: {}s, reload: {}s, ship purchase: {}", 
                                                self.config.timing.main_cycle_delay_seconds,
                                                self.config.timing.config_reload_interval_seconds, 
                                                self.config.fleet.min_credits_for_ship_purchase);
                        
                        self.config = new_config;
                        self.last_modified = Some(new_modified_time);
                        
                        let new_values = format!("cycle: {}s, reload: {}s, ship purchase: {}", 
                                                self.config.timing.main_cycle_delay_seconds,
                                                self.config.timing.config_reload_interval_seconds, 
                                                self.config.fleet.min_credits_for_ship_purchase);
                        
                        o_info!("🔄 Configuration reloaded successfully!");
                        if old_values != new_values {
                            o_info!("   📝 Changes: {} → {}", old_values, new_values);
                        }
                        true
                    }
                    Err(e) => {
                        o_info!("⚠️ Invalid configuration detected, keeping current config: {}", e);
                        false
                    }
                }
            }
            Err(e) => {
                o_info!("⚠️ Failed to reload configuration, keeping current config: {}", e);
                false
            }
        }
    }
}