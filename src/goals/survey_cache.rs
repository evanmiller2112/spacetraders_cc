// Survey Cache - Manages survey data for efficient mining operations
use crate::models::transaction::Survey;
use crate::client::PriorityApiClient;
use crate::{o_debug, o_info};
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct SurveyCache {
    // waypoint_symbol -> surveys
    cached_surveys: HashMap<String, Vec<CachedSurvey>>,
    cache_ttl: Duration,
    last_cleanup: Instant,
}

#[derive(Debug, Clone)]
struct CachedSurvey {
    survey: Survey,
    cached_at: Instant,
    quality_score: f64, // Calculated based on deposit value and quantity
}

impl SurveyCache {
    pub fn new() -> Self {
        Self {
            cached_surveys: HashMap::new(),
            cache_ttl: Duration::from_secs(1800), // 30 minutes cache
            last_cleanup: Instant::now(),
        }
    }

    /// Get the best survey for a specific resource at a location
    pub async fn get_best_survey(&mut self, client: &PriorityApiClient, ship_symbol: &str, waypoint: &str, target_resource: &str) -> Result<Option<Survey>, Box<dyn std::error::Error>> {
        // Clean up expired surveys
        self.cleanup_expired_surveys();
        
        // Check if we have cached surveys for this waypoint
        if let Some(cached_surveys) = self.cached_surveys.get(waypoint) {
            // Find surveys that contain the target resource
            let matching_surveys: Vec<_> = cached_surveys.iter()
                .filter(|cached| !self.is_survey_expired(&cached.survey))
                .filter(|cached| cached.survey.deposits.iter().any(|d| d.symbol == target_resource))
                .collect();
            
            if !matching_surveys.is_empty() {
                // Sort by quality score and return the best one
                let best_survey = matching_surveys.iter()
                    .max_by(|a, b| a.quality_score.partial_cmp(&b.quality_score).unwrap())
                    .unwrap();
                
                o_debug!("📋 Found cached survey for {} at {}: {} (score: {:.2})", 
                        target_resource, waypoint, best_survey.survey.signature, best_survey.quality_score);
                return Ok(Some(best_survey.survey.clone()));
            }
        }
        
        // No suitable cached survey found, create new ones
        o_info!("🔍 Creating new surveys at {} for {}", waypoint, target_resource);
        match client.create_survey_with_priority(ship_symbol, crate::client::ApiPriority::ActiveGoal).await {
            Ok(survey_data) => {
                // Cache the new surveys
                self.cache_surveys(waypoint, survey_data.surveys.clone());
                
                // Find the best survey for our target resource
                let best_survey = self.find_best_survey_for_resource(&survey_data.surveys, target_resource);
                if let Some(survey) = &best_survey {
                    o_info!("✅ Created and selected survey for {}: {} ({} deposits)", 
                           target_resource, survey.signature, survey.deposits.len());
                }
                Ok(best_survey)
            }
            Err(e) => {
                o_debug!("⚠️ Survey creation failed: {} - falling back to regular mining", e);
                Ok(None) // Fall back to regular mining
            }
        }
    }

    /// Cache surveys for a waypoint
    fn cache_surveys(&mut self, waypoint: &str, surveys: Vec<Survey>) {
        let cached_surveys: Vec<CachedSurvey> = surveys.into_iter()
            .map(|survey| {
                let quality_score = self.calculate_survey_quality(&survey);
                CachedSurvey {
                    survey,
                    cached_at: Instant::now(),
                    quality_score,
                }
            })
            .collect();
        
        o_debug!("💾 Cached {} surveys for waypoint {}", cached_surveys.len(), waypoint);
        self.cached_surveys.insert(waypoint.to_string(), cached_surveys);
    }

    /// Calculate quality score for a survey based on deposit types and survey size
    fn calculate_survey_quality(&self, survey: &Survey) -> f64 {
        let mut score = 0.0;
        
        // Base score from survey size
        score += match survey.size.as_str() {
            "SMALL" => 1.0,
            "MODERATE" => 2.0,
            "LARGE" => 3.0,
            _ => 1.0,
        };
        
        // Bonus for valuable resources
        for deposit in &survey.deposits {
            score += match deposit.symbol.as_str() {
                // High-value refined materials
                "PRECIOUS_STONES" | "RARE_METALS" => 5.0,
                "GOLD_ORE" | "PLATINUM_ORE" => 4.0,
                // Common but useful ores
                "IRON_ORE" | "COPPER_ORE" | "ALUMINUM_ORE" => 2.0,
                "SILICON_CRYSTALS" => 2.5,
                // Basic materials
                "ICE_WATER" | "AMMONIA_ICE" => 1.0,
                _ => 1.0,
            };
        }
        
        // Bonus for multiple deposit types (diversified mining)
        score += (survey.deposits.len() as f64 - 1.0) * 0.5;
        
        score
    }

    /// Find the best survey for a specific resource from a list
    fn find_best_survey_for_resource(&self, surveys: &[Survey], target_resource: &str) -> Option<Survey> {
        surveys.iter()
            .filter(|survey| survey.deposits.iter().any(|d| d.symbol == target_resource))
            .max_by_key(|survey| {
                // Count how many of the target resource deposits this survey has
                survey.deposits.iter().filter(|d| d.symbol == target_resource).count()
            })
            .cloned()
    }

    /// Check if a survey has expired
    fn is_survey_expired(&self, survey: &Survey) -> bool {
        // Parse the expiration time and check if it's passed
        match chrono::DateTime::parse_from_rfc3339(&survey.expiration) {
            Ok(expiration_time) => {
                let now = chrono::Utc::now();
                expiration_time.timestamp() <= now.timestamp()
            }
            Err(_) => {
                // If we can't parse the expiration, assume it's expired
                true
            }
        }
    }

    /// Clean up expired surveys from cache
    fn cleanup_expired_surveys(&mut self) {
        if self.last_cleanup.elapsed() < Duration::from_secs(300) {
            return; // Only cleanup every 5 minutes
        }
        
        let mut total_removed = 0;
        let cache_ttl = self.cache_ttl; // Capture TTL to avoid borrowing issues
        let _waypoints_to_remove: Vec<String> = Vec::new(); // Remove unused variable
        
        for (waypoint, surveys) in self.cached_surveys.iter_mut() {
            let original_count = surveys.len();
            surveys.retain(|cached| {
                let survey_expired = match chrono::DateTime::parse_from_rfc3339(&cached.survey.expiration) {
                    Ok(expiration_time) => {
                        let now = chrono::Utc::now();
                        expiration_time.timestamp() > now.timestamp()
                    }
                    Err(_) => false, // If we can't parse, assume expired
                };
                survey_expired && cached.cached_at.elapsed() < cache_ttl
            });
            
            let removed = original_count - surveys.len();
            total_removed += removed;
            
            if removed > 0 {
                o_debug!("🧹 Cleaned up {} expired surveys from {}", removed, waypoint);
            }
        }
        
        // Remove empty waypoint entries
        self.cached_surveys.retain(|_, surveys| !surveys.is_empty());
        
        if total_removed > 0 {
            o_debug!("🧹 Survey cache cleanup: removed {} expired surveys", total_removed);
        }
        
        self.last_cleanup = Instant::now();
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> SurveyCacheStats {
        let total_surveys: usize = self.cached_surveys.values().map(|v| v.len()).sum();
        let waypoints_cached = self.cached_surveys.len();
        
        SurveyCacheStats {
            total_surveys,
            waypoints_cached,
            cache_hit_potential: if total_surveys > 0 { 0.8 } else { 0.0 }, // Rough estimate
        }
    }
}

#[derive(Debug)]
pub struct SurveyCacheStats {
    pub total_surveys: usize,
    pub waypoints_cached: usize,
    pub cache_hit_potential: f64,
}