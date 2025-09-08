use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use crate::models::*;
use crate::API_BASE_URL;
use std::fs::OpenOptions;
use std::io::Write;

#[derive(Clone)]
pub struct SpaceTradersClient {
    client: reqwest::Client,
    pub token: String,
    debug_mode: bool,
    api_logging: bool,
}

impl SpaceTradersClient {
    pub fn new(token: String) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", token)).unwrap(),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();

        SpaceTradersClient { 
            client, 
            token,
            debug_mode: false,
            api_logging: false,
        }
    }
    
    pub fn set_debug_mode(&mut self, debug: bool) {
        self.debug_mode = debug;
    }
    
    pub fn set_api_logging(&mut self, logging: bool) {
        self.api_logging = logging;
    }
    
    async fn request_approval(&self, method: &str, url: &str, body: Option<&str>) -> bool {
        if !self.debug_mode {
            return true; // Always approve if not in debug mode
        }
        
        println!("\n🐛 DEBUG API CALL:");
        println!("   Method: {}", method);
        println!("   URL: {}", url);
        if let Some(body) = body {
            println!("   Body: {}", body);
        }
        print!("   Approve? (y/n): ");
        
        use std::io::{self, Write};
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        
        matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
    }
    
    fn log_api_call(&self, method: &str, url: &str, body: Option<&str>, response_status: u16, response_body: Option<&str>) {
        if !self.api_logging {
            return;
        }
        
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
        let log_entry = format!(
            "\n=== API CALL [{timestamp}] ===\n\
             Method: {method}\n\
             URL: {url}\n\
             Request Body: {request_body}\n\
             Response Status: {response_status}\n\
             Response Body: {response_body}\n\
             ========================================\n",
            timestamp = timestamp,
            method = method,
            url = url,
            request_body = body.unwrap_or("None"),
            response_status = response_status,
            response_body = response_body.unwrap_or("Not captured")
        );
        
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("api_debug.log")
        {
            let _ = file.write_all(log_entry.as_bytes());
        }
    }

    // Scanning operations
    pub async fn scan_waypoints(&self, ship_symbol: &str) -> Result<Vec<ScannedWaypoint>, Box<dyn std::error::Error>> {
        let url = format!("{}/my/ships/{}/scan/waypoints", API_BASE_URL, ship_symbol);
        
        if !self.request_approval("POST", &url, Some("{}")).await {
            return Err("API call not approved".into());
        }
        
        let response = self.client.post(&url).json(&serde_json::json!({})).send().await?;
        let status = response.status().as_u16();
        
        if !response.status().is_success() {
            let error_body = response.text().await.unwrap_or_else(|_| "Could not read response".to_string());
            self.log_api_call("POST", &url, Some("{}"), status, Some(&error_body));
            return Err(format!("Waypoint scan failed with status: {}", status).into());
        }

        let response_text = response.text().await?;
        self.log_api_call("POST", &url, Some("{}"), status, Some(&response_text));
        
        let scan_response: WaypointScanResponse = serde_json::from_str(&response_text)?;
        Ok(scan_response.data.waypoints)
    }

    // Agent operations
    pub async fn get_agent(&self) -> Result<Agent, Box<dyn std::error::Error>> {
        let url = format!("{}/my/agent", API_BASE_URL);
        
        if !self.request_approval("GET", &url, None).await {
            return Err("API call not approved".into());
        }
        
        let response = self.client.get(&url).send().await?;
        let status = response.status().as_u16();
        
        if !response.status().is_success() {
            let error_body = response.text().await.unwrap_or_else(|_| "Could not read response".to_string());
            self.log_api_call("GET", &url, None, status, Some(&error_body));
            return Err(format!("API request failed with status: {}", status).into());
        }

        let response_text = response.text().await?;
        self.log_api_call("GET", &url, None, status, Some(&response_text));
        
        let agent_response: AgentResponse = serde_json::from_str(&response_text)?;
        Ok(agent_response.data)
    }

    // Waypoint operations
    pub async fn get_waypoint(&self, system_symbol: &str, waypoint_symbol: &str) -> Result<Waypoint, Box<dyn std::error::Error>> {
        let url = format!("{}/systems/{}/waypoints/{}", API_BASE_URL, system_symbol, waypoint_symbol);
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(format!("API request failed with status: {}", response.status()).into());
        }

        let waypoint_response: WaypointResponse = response.json().await?;
        Ok(waypoint_response.data)
    }

    pub async fn get_system_waypoints(&self, system_symbol: &str, waypoint_type: Option<&str>) -> Result<Vec<Waypoint>, Box<dyn std::error::Error>> {
        let mut url = format!("{}/systems/{}/waypoints", API_BASE_URL, system_symbol);
        if let Some(wp_type) = waypoint_type {
            url.push_str(&format!("?type={}", wp_type));
        }
        
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(format!("API request failed with status: {}", response.status()).into());
        }

        let waypoints_response: WaypointsResponse = response.json().await?;
        Ok(waypoints_response.data)
    }

    // Contract operations
    pub async fn get_contracts(&self) -> Result<Vec<Contract>, Box<dyn std::error::Error>> {
        let url = format!("{}/my/contracts", API_BASE_URL);
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(format!("API request failed with status: {}", response.status()).into());
        }

        let contracts_response: ContractsResponse = response.json().await?;
        Ok(contracts_response.data)
    }

    pub async fn accept_contract(&self, contract_id: &str) -> Result<ContractAcceptData, Box<dyn std::error::Error>> {
        let url = format!("{}/my/contracts/{}/accept", API_BASE_URL, contract_id);
        let response = self.client.post(&url).json(&serde_json::json!({})).send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_else(|_| "Could not read error response".to_string());
            return Err(format!("Accept contract failed with status {}: {}", status, error_body).into());
        }

        let contract_accept_response: ContractAcceptResponse = response.json().await?;
        Ok(contract_accept_response.data)
    }

    pub async fn deliver_cargo(&self, ship_symbol: &str, contract_id: &str, trade_symbol: &str, units: i32) -> Result<DeliverCargoData, Box<dyn std::error::Error>> {
        let url = format!("{}/my/contracts/{}/deliver", API_BASE_URL, contract_id);
        let payload = serde_json::json!({
            "shipSymbol": ship_symbol,
            "tradeSymbol": trade_symbol,
            "units": units
        });
        let response = self.client.post(&url).json(&payload).send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_else(|_| "Could not read error response".to_string());
            return Err(format!("Contract delivery failed with status {}: {}", status, error_body).into());
        }

        let delivery_response: DeliverCargoResponse = response.json().await?;
        Ok(delivery_response.data)
    }

    pub async fn fulfill_contract(&self, contract_id: &str) -> Result<FulfillContractData, Box<dyn std::error::Error>> {
        let url = format!("{}/my/contracts/{}/fulfill", API_BASE_URL, contract_id);
        let response = self.client.post(&url).json(&serde_json::json!({})).send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_else(|_| "Could not read error response".to_string());
            return Err(format!("Contract fulfillment failed with status {}: {}", status, error_body).into());
        }

        let fulfill_response: FulfillContractResponse = response.json().await?;
        Ok(fulfill_response.data)
    }

    // Ship operations
    pub async fn get_ships(&self) -> Result<Vec<Ship>, Box<dyn std::error::Error>> {
        let url = format!("{}/my/ships", API_BASE_URL);
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(format!("API request failed with status: {}", response.status()).into());
        }

        let ships_response: ShipsResponse = response.json().await?;
        Ok(ships_response.data)
    }

    pub async fn get_ship(&self, ship_symbol: &str) -> Result<Ship, Box<dyn std::error::Error>> {
        let url = format!("{}/my/ships/{}", API_BASE_URL, ship_symbol);
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_else(|_| "Could not read error response".to_string());
            return Err(format!("Get ship failed with status {}: {}", status, error_body).into());
        }

        let ship_response: ShipResponse = response.json().await?;
        Ok(ship_response.data)
    }

    pub async fn orbit_ship(&self, ship_symbol: &str) -> Result<ShipNav, Box<dyn std::error::Error>> {
        let url = format!("{}/my/ships/{}/orbit", API_BASE_URL, ship_symbol);
        let response = self.client.post(&url).json(&serde_json::json!({})).send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_else(|_| "Could not read error response".to_string());
            return Err(format!("Orbit failed with status {}: {}", status, error_body).into());
        }

        let orbit_response: OrbitResponse = response.json().await?;
        Ok(orbit_response.data.nav)
    }

    pub async fn dock_ship(&self, ship_symbol: &str) -> Result<ShipNav, Box<dyn std::error::Error>> {
        let url = format!("{}/my/ships/{}/dock", API_BASE_URL, ship_symbol);
        let response = self.client.post(&url).json(&serde_json::json!({})).send().await?;
        
        if !response.status().is_success() {
            return Err(format!("API request failed with status: {}", response.status()).into());
        }

        let dock_response: DockResponse = response.json().await?;
        Ok(dock_response.data.nav)
    }

    pub async fn navigate_ship(&self, ship_symbol: &str, waypoint_symbol: &str) -> Result<NavigationData, Box<dyn std::error::Error>> {
        let url = format!("{}/my/ships/{}/navigate", API_BASE_URL, ship_symbol);
        let payload = serde_json::json!({
            "waypointSymbol": waypoint_symbol
        });
        
        if !self.request_approval("POST", &url, Some(&payload.to_string())).await {
            return Err("API call not approved".into());
        }
        
        let response = self.client.post(&url).json(&payload).send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_else(|_| "Could not read error response".to_string());
            return Err(format!("Navigation failed with status {}: {}", status, error_body).into());
        }

        let nav_response: NavigationResponse = response.json().await?;
        Ok(nav_response.data)
    }

    // Mining operations
    pub async fn create_survey(&self, ship_symbol: &str) -> Result<SurveyData, Box<dyn std::error::Error>> {
        let url = format!("{}/my/ships/{}/survey", API_BASE_URL, ship_symbol);
        let response = self.client.post(&url).json(&serde_json::json!({})).send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_else(|_| "Could not read error response".to_string());
            return Err(format!("Survey creation failed with status {}: {}", status, error_body).into());
        }

        let survey_response: SurveyResponse = response.json().await?;
        Ok(survey_response.data)
    }

    pub async fn extract_resources(&self, ship_symbol: &str) -> Result<ExtractionData, Box<dyn std::error::Error>> {
        let url = format!("{}/my/ships/{}/extract", API_BASE_URL, ship_symbol);
        let response = self.client.post(&url).json(&serde_json::json!({})).send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_else(|_| "Could not read error response".to_string());
            return Err(format!("Extraction failed with status {}: {}", status, error_body).into());
        }

        let extraction_response: ExtractionResponse = response.json().await?;
        Ok(extraction_response.data)
    }

    pub async fn extract_resources_with_survey(&self, ship_symbol: &str, survey: &Survey) -> Result<ExtractionData, Box<dyn std::error::Error>> {
        let url = format!("{}/my/ships/{}/extract", API_BASE_URL, ship_symbol);
        let payload = serde_json::json!({
            "survey": survey
        });
        let response = self.client.post(&url).json(&payload).send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_else(|_| "Could not read error response".to_string());
            return Err(format!("Targeted extraction failed with status {}: {}", status, error_body).into());
        }

        let extraction_response: ExtractionResponse = response.json().await?;
        Ok(extraction_response.data)
    }

    // Trading operations
    pub async fn sell_cargo(&self, ship_symbol: &str, trade_symbol: &str, units: i32) -> Result<SellCargoData, Box<dyn std::error::Error>> {
        let url = format!("{}/my/ships/{}/sell", API_BASE_URL, ship_symbol);
        let payload = serde_json::json!({
            "symbol": trade_symbol,
            "units": units
        });
        let response = self.client.post(&url).json(&payload).send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_else(|_| "Could not read error response".to_string());
            return Err(format!("Cargo sell failed with status {}: {}", status, error_body).into());
        }

        let sell_response: SellCargoResponse = response.json().await?;
        Ok(sell_response.data)
    }

    // Refueling operations
    pub async fn refuel_ship(&self, ship_symbol: &str) -> Result<RefuelData, Box<dyn std::error::Error>> {
        let url = format!("{}/my/ships/{}/refuel", API_BASE_URL, ship_symbol);
        
        if !self.request_approval("POST", &url, Some("{}")).await {
            return Err("API call not approved".into());
        }
        
        let response = self.client.post(&url).json(&serde_json::json!({})).send().await?;
        
        if !response.status().is_success() {
            return Err(format!("API request failed with status: {}", response.status()).into());
        }

        let refuel_response: RefuelResponse = response.json().await?;
        Ok(refuel_response.data)
    }

    // Shipyard operations
    pub async fn get_shipyard(&self, system_symbol: &str, waypoint_symbol: &str) -> Result<Shipyard, Box<dyn std::error::Error>> {
        let url = format!("{}/systems/{}/waypoints/{}/shipyard", API_BASE_URL, system_symbol, waypoint_symbol);
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(format!("API request failed with status: {}", response.status()).into());
        }

        let shipyard_response: ShipyardResponse = response.json().await?;
        Ok(shipyard_response.data)
    }

    pub async fn purchase_ship(&self, ship_type: &str, waypoint_symbol: &str) -> Result<ShipPurchaseData, Box<dyn std::error::Error>> {
        let url = format!("{}/my/ships", API_BASE_URL);
        let payload = serde_json::json!({
            "shipType": ship_type,
            "waypointSymbol": waypoint_symbol
        });
        
        let response = self.client.post(&url).json(&payload).send().await?;
        
        if !response.status().is_success() {
            return Err(format!("API request failed with status: {}", response.status()).into());
        }

        let purchase_response: ShipPurchaseResponse = response.json().await?;
        Ok(purchase_response.data)
    }
}