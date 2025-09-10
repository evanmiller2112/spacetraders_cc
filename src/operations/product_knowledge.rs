// Product knowledge database for marketplace trading optimization
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ProductInfo {
    pub preferred_waypoint_traits: Vec<&'static str>,
    pub typical_price_range: (i64, i64),
    pub transaction_limit: Option<i32>,
    pub cargo_per_unit: i32,
}

pub struct ProductKnowledge {
    products: HashMap<&'static str, ProductInfo>,
}

impl ProductKnowledge {
    pub fn new() -> Self {
        let mut products = HashMap::new();
        
        // Electronics - found at HIGH_TECH locations
        products.insert("ELECTRONICS", ProductInfo {
            preferred_waypoint_traits: vec!["HIGH_TECH", "MARKETPLACE"],
            typical_price_range: (1000, 2000),
            transaction_limit: Some(20),
            cargo_per_unit: 1,
        });
        
        // Machinery - industrial goods
        products.insert("MACHINERY", ProductInfo {
            preferred_waypoint_traits: vec!["INDUSTRIAL", "MARKETPLACE"],
            typical_price_range: (800, 1500),
            transaction_limit: Some(15),
            cargo_per_unit: 1,
        });
        
        // Medicine - research/medical facilities
        products.insert("MEDICINE", ProductInfo {
            preferred_waypoint_traits: vec!["RESEARCH", "MARKETPLACE"],
            typical_price_range: (600, 1200),
            transaction_limit: Some(25),
            cargo_per_unit: 1,
        });
        
        // Food - agricultural or marketplace
        products.insert("FOOD", ProductInfo {
            preferred_waypoint_traits: vec!["AGRICULTURAL", "MARKETPLACE"],
            typical_price_range: (300, 800),
            transaction_limit: Some(30),
            cargo_per_unit: 1,
        });
        
        // Clothing - marketplace or industrial
        products.insert("CLOTHING", ProductInfo {
            preferred_waypoint_traits: vec!["MARKETPLACE", "INDUSTRIAL"],
            typical_price_range: (400, 900),
            transaction_limit: Some(25),
            cargo_per_unit: 1,
        });
        
        // Tools - industrial locations
        products.insert("TOOLS", ProductInfo {
            preferred_waypoint_traits: vec!["INDUSTRIAL", "MARKETPLACE"],
            typical_price_range: (500, 1000),
            transaction_limit: Some(20),
            cargo_per_unit: 1,
        });
        
        // Weapons - military or industrial
        products.insert("WEAPONS", ProductInfo {
            preferred_waypoint_traits: vec!["MILITARY", "INDUSTRIAL", "MARKETPLACE"],
            typical_price_range: (1200, 2500),
            transaction_limit: Some(10),
            cargo_per_unit: 1,
        });
        
        // Drugs - research facilities or black markets
        products.insert("DRUGS", ProductInfo {
            preferred_waypoint_traits: vec!["RESEARCH", "MARKETPLACE"],
            typical_price_range: (800, 1800),
            transaction_limit: Some(15),
            cargo_per_unit: 1,
        });
        
        // Equipment - industrial or marketplace  
        products.insert("EQUIPMENT", ProductInfo {
            preferred_waypoint_traits: vec!["INDUSTRIAL", "MARKETPLACE"],
            typical_price_range: (600, 1400),
            transaction_limit: Some(15),
            cargo_per_unit: 1,
        });
        
        // Jewelry - luxury or marketplace
        products.insert("JEWELRY", ProductInfo {
            preferred_waypoint_traits: vec!["MARKETPLACE"],
            typical_price_range: (1000, 3000),
            transaction_limit: Some(10),
            cargo_per_unit: 1,
        });
        
        Self { products }
    }
    
    pub fn get_product_info(&self, product: &str) -> Option<&ProductInfo> {
        self.products.get(product)
    }
    
    pub fn is_manufactured_good(&self, product: &str) -> bool {
        self.products.contains_key(product)
    }
    
    pub fn get_preferred_traits(&self, product: &str) -> Vec<&'static str> {
        self.get_product_info(product)
            .map(|info| info.preferred_waypoint_traits.clone())
            .unwrap_or_else(|| vec!["MARKETPLACE"])
    }
    
    pub fn get_transaction_limit(&self, product: &str) -> Option<i32> {
        self.get_product_info(product)
            .and_then(|info| info.transaction_limit)
    }
    
    pub fn is_reasonable_price(&self, product: &str, price: i64) -> bool {
        if let Some(info) = self.get_product_info(product) {
            price >= info.typical_price_range.0 && price <= info.typical_price_range.1 * 2 // Allow 2x typical max
        } else {
            // For unknown products, use generic reasonable limits
            price <= 5000 // Max 5000 credits per unit for unknown products
        }
    }
    
    pub fn get_max_reasonable_price(&self, product: &str) -> i64 {
        if let Some(info) = self.get_product_info(product) {
            info.typical_price_range.1 * 2 // Allow 2x typical max price
        } else {
            5000 // Default max for unknown products
        }
    }
}

impl Default for ProductKnowledge {
    fn default() -> Self {
        Self::new()
    }
}