use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use crate::models::*;
use crate::API_BASE_URL;
use std::fs::OpenOptions;
use std::io::Write;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tokio::sync::Mutex as TokioMutex;

#[derive(Clone)]
pub struct SpaceTradersClient {
    client: reqwest::Client,
    pub token: String,
    debug_mode: bool,
    api_logging: bool,
    last_request_time: std::sync::Arc<TokioMutex<Option<Instant>>>,
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
            last_request_time: std::sync::Arc::new(TokioMutex::new(None)),
        }
    }
    
    pub fn set_debug_mode(&mut self, debug: bool) {
        self.debug_mode = debug;
    }
    
    pub fn set_api_logging(&mut self, logging: bool) {
        self.api_logging = logging;
    }
    
    /// Enforce rate limiting: SpaceTraders allows 2 requests per second with burst of 30
    async fn enforce_rate_limit(&self) {
        const MIN_REQUEST_INTERVAL: Duration = Duration::from_millis(500); // 2 req/sec = 500ms between requests
        
        let mut last_time = self.last_request_time.lock().await;
        if let Some(last) = *last_time {
            let elapsed = last.elapsed();
            if elapsed < MIN_REQUEST_INTERVAL {
                let sleep_duration = MIN_REQUEST_INTERVAL - elapsed;
                sleep(sleep_duration).await;
            }
        }
        *last_time = Some(Instant::now());
    }
    
    /// Handle 429 rate limit errors with exponential backoff
    async fn handle_rate_limit_error(&self, retry_after: Option<f64>) {
        let sleep_duration = if let Some(retry_after) = retry_after {
            // Use the server-provided retry time
            Duration::from_secs_f64(retry_after.max(0.1)) // Minimum 100ms
        } else {
            // Default backoff if no retry-after header
            Duration::from_secs(1)
        };
        
        println!("⚠️ Rate limit hit, waiting {:.1}s before retry", sleep_duration.as_secs_f64());
        sleep(sleep_duration).await;
    }
    
    /// Parse retry-after from 429 response
    fn parse_retry_after(error_text: &str) -> Option<f64> {
        // Try to extract retryAfter from the JSON error response
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(error_text) {
            if let Some(retry_after) = json.get("error")?.get("data")?.get("retryAfter") {
                return retry_after.as_f64();
            }
        }
        None
    }
    
    /// Wrapper for all HTTP requests with rate limiting and retry logic
    async fn make_request_with_retry<T, F, Fut>(
        &self, 
        _method: &str, 
        _url: &str, 
        request_fn: F,
        max_retries: u32
    ) -> Result<T, String>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, String>>,
    {
        for attempt in 0..=max_retries {
            // Enforce rate limiting before each request
            self.enforce_rate_limit().await;
            
            match request_fn().await {
                Ok(result) => return Ok(result),
                Err(error_msg) => {
                    // Check if it's a 429 rate limit error
                    if error_msg.contains("429") && error_msg.contains("Too Many Requests") {
                        if attempt < max_retries {
                            let retry_after = Self::parse_retry_after(&error_msg);
                            self.handle_rate_limit_error(retry_after).await;
                            continue; // Retry
                        }
                    }
                    
                    // For non-rate-limit errors or final attempt, return the error
                    return Err(error_msg);
                }
            }
        }
        
        unreachable!("Loop should always return")
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
        crate::debug_fn_enter!("SpaceTradersClient::scan_waypoints", "ship_symbol={}", ship_symbol);
        
        let url = format!("{}/my/ships/{}/scan/waypoints", API_BASE_URL, ship_symbol);
        crate::debug_api_call!("POST", &url, "{}");
        
        if !self.request_approval("POST", &url, Some("{}")).await {
            let error = Err("API call not approved".into());
            crate::debug_fn_exit!("SpaceTradersClient::scan_waypoints", &error);
            return error;
        }
        
        let response = self.client.post(&url).json(&serde_json::json!({})).send().await?;
        let status = response.status().as_u16();
        
        if !response.status().is_success() {
            let error_body = response.text().await.unwrap_or_else(|_| "Could not read response".to_string());
            self.log_api_call("POST", &url, Some("{}"), status, Some(&error_body));
            let error = Err(format!("Waypoint scan failed with status: {}", status).into());
            crate::debug_fn_exit!("SpaceTradersClient::scan_waypoints", &error);
            return error;
        }

        let response_text = response.text().await?;
        self.log_api_call("POST", &url, Some("{}"), status, Some(&response_text));
        
        let scan_response: WaypointScanResponse = serde_json::from_str(&response_text)?;
        let result = Ok(scan_response.data.waypoints);
        crate::debug_fn_exit!("SpaceTradersClient::scan_waypoints", &result);
        result
    }

    // Agent operations
    pub async fn get_agent(&self) -> Result<Agent, Box<dyn std::error::Error>> {
        crate::debug_fn_enter!("SpaceTradersClient::get_agent");
        
        let url = format!("{}/my/agent", API_BASE_URL);
        crate::debug_api_call!("GET", &url);
        
        if !self.request_approval("GET", &url, None).await {
            let error = Err("API call not approved".into());
            crate::debug_fn_exit!("SpaceTradersClient::get_agent", &error);
            return error;
        }
        
        let result = self.make_request_with_retry("GET", &url, || async {
            match self.client.get(&url).send().await {
                Ok(response) => {
                    let status = response.status().as_u16();
                    
                    if !response.status().is_success() {
                        let error_body = response.text().await.unwrap_or_else(|_| "Could not read response".to_string());
                        self.log_api_call("GET", &url, None, status, Some(&error_body));
                        return Err(format!("Get agent failed with status {}: {}", status, error_body));
                    }

                    match response.text().await {
                        Ok(response_text) => {
                            self.log_api_call("GET", &url, None, status, Some(&response_text));
                            
                            match serde_json::from_str::<AgentResponse>(&response_text) {
                                Ok(agent_response) => Ok(agent_response.data),
                                Err(e) => Err(format!("JSON parse error: {}", e))
                            }
                        },
                        Err(e) => Err(format!("Failed to read response: {}", e))
                    }
                },
                Err(e) => Err(format!("Request failed: {}", e))
            }
        }, 3).await;
        
        let result = result.map_err(|e| e.into());
        crate::debug_fn_exit!("SpaceTradersClient::get_agent", &result);
        result
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

    pub async fn get_system_waypoints_with_traits(&self, system_symbol: &str, traits: &str) -> Result<Vec<Waypoint>, Box<dyn std::error::Error>> {
        let url = format!("{}/systems/{}/waypoints?traits={}", API_BASE_URL, system_symbol, traits);
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
        let result = self.make_request_with_retry("GET", &url, || async {
            match self.client.get(&url).send().await {
                Ok(response) => {
                    if !response.status().is_success() {
                        return Err(format!("Get ships failed with status: {}", response.status()));
                    }

                    match response.json::<ShipsResponse>().await {
                        Ok(ships_response) => Ok(ships_response.data),
                        Err(e) => Err(format!("JSON parse error: {}", e))
                    }
                },
                Err(e) => Err(format!("Request failed: {}", e))
            }
        }, 3).await;
        result.map_err(|e| e.into())
    }

    pub async fn get_ship(&self, ship_symbol: &str) -> Result<Ship, Box<dyn std::error::Error>> {
        let url = format!("{}/my/ships/{}", API_BASE_URL, ship_symbol);
        
        let result = self.make_request_with_retry("GET", &url, || async {
            match self.client.get(&url).send().await {
                Ok(response) => {
                    if !response.status().is_success() {
                        let status = response.status();
                        let error_body = response.text().await.unwrap_or_else(|_| "Could not read error response".to_string());
                        return Err(format!("Get ship failed with status {}: {}", status, error_body));
                    }

                    match response.json::<ShipResponse>().await {
                        Ok(ship_response) => Ok(ship_response.data),
                        Err(e) => Err(format!("JSON parse error: {}", e))
                    }
                },
                Err(e) => Err(format!("Request failed: {}", e))
            }
        }, 3).await;
        result.map_err(|e| e.into())
    }

    pub async fn orbit_ship(&self, ship_symbol: &str) -> Result<ShipNav, Box<dyn std::error::Error>> {
        let url = format!("{}/my/ships/{}/orbit", API_BASE_URL, ship_symbol);
        let result = self.make_request_with_retry("POST", &url, || async {
            match self.client.post(&url).json(&serde_json::json!({})).send().await {
                Ok(response) => {
                    if !response.status().is_success() {
                        let status = response.status();
                        let error_body = response.text().await.unwrap_or_else(|_| "Could not read error response".to_string());
                        return Err(format!("Orbit failed with status {}: {}", status, error_body));
                    }

                    match response.json::<OrbitResponse>().await {
                        Ok(orbit_response) => Ok(orbit_response.data.nav),
                        Err(e) => Err(format!("JSON parse error: {}", e))
                    }
                },
                Err(e) => Err(format!("Request failed: {}", e))
            }
        }, 3).await;
        result.map_err(|e| e.into())
    }

    pub async fn dock_ship(&self, ship_symbol: &str) -> Result<ShipNav, Box<dyn std::error::Error>> {
        let url = format!("{}/my/ships/{}/dock", API_BASE_URL, ship_symbol);
        let result = self.make_request_with_retry("POST", &url, || async {
            match self.client.post(&url).json(&serde_json::json!({})).send().await {
                Ok(response) => {
                    if !response.status().is_success() {
                        let status = response.status();
                        let error_body = response.text().await.unwrap_or_else(|_| "Could not read error response".to_string());
                        return Err(format!("Dock failed with status {}: {}", status, error_body));
                    }

                    match response.json::<DockResponse>().await {
                        Ok(dock_response) => Ok(dock_response.data.nav),
                        Err(e) => Err(format!("JSON parse error: {}", e))
                    }
                },
                Err(e) => Err(format!("Request failed: {}", e))
            }
        }, 3).await;
        result.map_err(|e| e.into())
    }

    pub async fn navigate_ship(&self, ship_symbol: &str, waypoint_symbol: &str) -> Result<NavigationData, Box<dyn std::error::Error>> {
        let url = format!("{}/my/ships/{}/navigate", API_BASE_URL, ship_symbol);
        let payload = serde_json::json!({
            "waypointSymbol": waypoint_symbol
        });
        
        if !self.request_approval("POST", &url, Some(&payload.to_string())).await {
            return Err("API call not approved".into());
        }
        
        let result = self.make_request_with_retry("POST", &url, || async {
            match self.client.post(&url).json(&payload).send().await {
                Ok(response) => {
                    if !response.status().is_success() {
                        let status = response.status();
                        let error_body = response.text().await.unwrap_or_else(|_| "Could not read error response".to_string());
                        return Err(format!("Navigation failed with status {}: {}", status, error_body));
                    }

                    match response.json::<NavigationResponse>().await {
                        Ok(nav_response) => Ok(nav_response.data),
                        Err(e) => Err(format!("JSON parse error: {}", e))
                    }
                },
                Err(e) => Err(format!("Request failed: {}", e))
            }
        }, 3).await;
        result.map_err(|e| e.into())
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
        let result = self.make_request_with_retry("POST", &url, || async {
            match self.client.post(&url).json(&serde_json::json!({})).send().await {
                Ok(response) => {
                    if !response.status().is_success() {
                        let status = response.status();
                        let error_body = response.text().await.unwrap_or_else(|_| "Could not read error response".to_string());
                        return Err(format!("Extraction failed with status {}: {}", status, error_body));
                    }

                    match response.json::<ExtractionResponse>().await {
                        Ok(extraction_response) => Ok(extraction_response.data),
                        Err(e) => Err(format!("JSON parse error: {}", e))
                    }
                },
                Err(e) => Err(format!("Request failed: {}", e))
            }
        }, 3).await;
        result.map_err(|e| e.into())
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
            let error_text = response.text().await?;
            return Err(format!("Refuel failed: {}", error_text).into());
        }

        let response_text = response.text().await?;
        if self.debug_mode {
            println!("🔍 Refuel API response: {}", response_text);
        }
        
        let refuel_response: RefuelResponse = serde_json::from_str(&response_text)?;
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

    pub async fn jettison_cargo(&self, ship_symbol: &str, item_symbol: &str, units: i32) -> Result<JettisonCargoData, Box<dyn std::error::Error>> {
        let url = format!("{}/my/ships/{}/jettison", API_BASE_URL, ship_symbol);
        let payload = serde_json::json!({
            "symbol": item_symbol,
            "units": units
        });
        
        if !self.request_approval("POST", &url, Some(&payload.to_string())).await {
            return Err("API call not approved".into());
        }
        
        let response = self.client.post(&url).json(&payload).send().await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Jettison failed: {}", error_text).into());
        }

        let response_text = response.text().await?;
        if self.debug_mode {
            println!("🔍 Jettison API response: {}", response_text);
        }
        
        let jettison_response: JettisonCargoResponse = serde_json::from_str(&response_text)?;
        Ok(jettison_response.data)
    }

    // Systems operations
    pub async fn get_systems(&self, page: Option<i32>, limit: Option<i32>) -> Result<Vec<System>, Box<dyn std::error::Error>> {
        crate::debug_fn_enter!("SpaceTradersClient::get_systems");
        
        let mut url = format!("{}/systems", API_BASE_URL);
        let mut query_params = Vec::new();
        
        if let Some(p) = page {
            query_params.push(format!("page={}", p));
        }
        if let Some(l) = limit {
            query_params.push(format!("limit={}", l));
        }
        
        if !query_params.is_empty() {
            url.push_str(&format!("?{}", query_params.join("&")));
        }
        
        crate::debug_api_call!("GET", &url);
        
        if !self.request_approval("GET", &url, None).await {
            let error = Err("API call not approved".into());
            crate::debug_fn_exit!("SpaceTradersClient::get_systems", &error);
            return error;
        }
        
        let response = self.client.get(&url).send().await?;
        let status = response.status().as_u16();
        
        if !response.status().is_success() {
            let error_body = response.text().await.unwrap_or_else(|_| "Could not read response".to_string());
            self.log_api_call("GET", &url, None, status, Some(&error_body));
            let error = Err(format!("Get systems failed with status: {}", status).into());
            crate::debug_fn_exit!("SpaceTradersClient::get_systems", &error);
            return error;
        }

        let response_text = response.text().await?;
        self.log_api_call("GET", &url, None, status, Some(&response_text));
        
        let systems_response: SystemsResponse = serde_json::from_str(&response_text)?;
        let result = Ok(systems_response.data);
        crate::debug_fn_exit!("SpaceTradersClient::get_systems", &result);
        result
    }

    pub async fn get_system(&self, system_symbol: &str) -> Result<System, Box<dyn std::error::Error>> {
        crate::debug_fn_enter!("SpaceTradersClient::get_system", "system_symbol={}", system_symbol);
        
        let url = format!("{}/systems/{}", API_BASE_URL, system_symbol);
        crate::debug_api_call!("GET", &url);
        
        if !self.request_approval("GET", &url, None).await {
            let error = Err("API call not approved".into());
            crate::debug_fn_exit!("SpaceTradersClient::get_system", &error);
            return error;
        }
        
        let response = self.client.get(&url).send().await?;
        let status = response.status().as_u16();
        
        if !response.status().is_success() {
            let error_body = response.text().await.unwrap_or_else(|_| "Could not read response".to_string());
            self.log_api_call("GET", &url, None, status, Some(&error_body));
            let error = Err(format!("Get system failed with status: {}", status).into());
            crate::debug_fn_exit!("SpaceTradersClient::get_system", &error);
            return error;
        }

        let response_text = response.text().await?;
        self.log_api_call("GET", &url, None, status, Some(&response_text));
        
        let system_response: SystemResponse = serde_json::from_str(&response_text)?;
        let result = Ok(system_response.data);
        crate::debug_fn_exit!("SpaceTradersClient::get_system", &result);
        result
    }

    // Marketplace operations
    pub async fn get_market(&self, system_symbol: &str, waypoint_symbol: &str) -> Result<Market, Box<dyn std::error::Error>> {
        crate::debug_fn_enter!("SpaceTradersClient::get_market", "system={}, waypoint={}", system_symbol, waypoint_symbol);
        
        let url = format!("{}/systems/{}/waypoints/{}/market", API_BASE_URL, system_symbol, waypoint_symbol);
        crate::debug_api_call!("GET", &url);
        
        if !self.request_approval("GET", &url, None).await {
            let error = Err("API call not approved".into());
            crate::debug_fn_exit!("SpaceTradersClient::get_market", &error);
            return error;
        }
        
        let response = self.client.get(&url).send().await?;
        let status = response.status().as_u16();
        
        if !response.status().is_success() {
            let error_body = response.text().await.unwrap_or_else(|_| "Could not read response".to_string());
            self.log_api_call("GET", &url, None, status, Some(&error_body));
            let error = Err(format!("Get market failed with status: {}", status).into());
            crate::debug_fn_exit!("SpaceTradersClient::get_market", &error);
            return error;
        }

        let response_text = response.text().await?;
        self.log_api_call("GET", &url, None, status, Some(&response_text));
        
        let market_response: MarketResponse = serde_json::from_str(&response_text)?;
        let result = Ok(market_response.data);
        crate::debug_fn_exit!("SpaceTradersClient::get_market", &result);
        result
    }

    // Additional trading operations
    pub async fn purchase_cargo(&self, ship_symbol: &str, trade_symbol: &str, units: i32) -> Result<PurchaseCargoData, Box<dyn std::error::Error>> {
        crate::debug_fn_enter!("SpaceTradersClient::purchase_cargo", "ship={}, trade_symbol={}, units={}", ship_symbol, trade_symbol, units);
        
        let url = format!("{}/my/ships/{}/purchase", API_BASE_URL, ship_symbol);
        let payload = serde_json::json!({
            "symbol": trade_symbol,
            "units": units
        });
        
        crate::debug_api_call!("POST", &url, &payload.to_string());
        
        if !self.request_approval("POST", &url, Some(&payload.to_string())).await {
            let error = Err("API call not approved".into());
            crate::debug_fn_exit!("SpaceTradersClient::purchase_cargo", &error);
            return error;
        }
        
        let response = self.client.post(&url).json(&payload).send().await?;
        let status = response.status().as_u16();
        
        if !response.status().is_success() {
            let error_body = response.text().await.unwrap_or_else(|_| "Could not read response".to_string());
            self.log_api_call("POST", &url, Some(&payload.to_string()), status, Some(&error_body));
            let error = Err(format!("Purchase cargo failed with status: {}", status).into());
            crate::debug_fn_exit!("SpaceTradersClient::purchase_cargo", &error);
            return error;
        }

        let response_text = response.text().await?;
        self.log_api_call("POST", &url, Some(&payload.to_string()), status, Some(&response_text));
        
        let purchase_response: PurchaseCargoResponse = serde_json::from_str(&response_text)?;
        let result = Ok(purchase_response.data);
        crate::debug_fn_exit!("SpaceTradersClient::purchase_cargo", &result);
        result
    }

    // Additional scanning operations
    pub async fn scan_systems(&self, ship_symbol: &str) -> Result<Vec<ScannedSystem>, Box<dyn std::error::Error>> {
        crate::debug_fn_enter!("SpaceTradersClient::scan_systems", "ship_symbol={}", ship_symbol);
        
        let url = format!("{}/my/ships/{}/scan/systems", API_BASE_URL, ship_symbol);
        crate::debug_api_call!("POST", &url, "{}");
        
        if !self.request_approval("POST", &url, Some("{}")).await {
            let error = Err("API call not approved".into());
            crate::debug_fn_exit!("SpaceTradersClient::scan_systems", &error);
            return error;
        }
        
        let response = self.client.post(&url).json(&serde_json::json!({})).send().await?;
        let status = response.status().as_u16();
        
        if !response.status().is_success() {
            let error_body = response.text().await.unwrap_or_else(|_| "Could not read response".to_string());
            self.log_api_call("POST", &url, Some("{}"), status, Some(&error_body));
            let error = Err(format!("System scan failed with status: {}", status).into());
            crate::debug_fn_exit!("SpaceTradersClient::scan_systems", &error);
            return error;
        }

        let response_text = response.text().await?;
        self.log_api_call("POST", &url, Some("{}"), status, Some(&response_text));
        
        let scan_response: SystemScanResponse = serde_json::from_str(&response_text)?;
        let result = Ok(scan_response.data.systems);
        crate::debug_fn_exit!("SpaceTradersClient::scan_systems", &result);
        result
    }

    pub async fn scan_ships(&self, ship_symbol: &str) -> Result<Vec<ScannedShip>, Box<dyn std::error::Error>> {
        crate::debug_fn_enter!("SpaceTradersClient::scan_ships", "ship_symbol={}", ship_symbol);
        
        let url = format!("{}/my/ships/{}/scan/ships", API_BASE_URL, ship_symbol);
        crate::debug_api_call!("POST", &url, "{}");
        
        if !self.request_approval("POST", &url, Some("{}")).await {
            let error = Err("API call not approved".into());
            crate::debug_fn_exit!("SpaceTradersClient::scan_ships", &error);
            return error;
        }
        
        let response = self.client.post(&url).json(&serde_json::json!({})).send().await?;
        let status = response.status().as_u16();
        
        if !response.status().is_success() {
            let error_body = response.text().await.unwrap_or_else(|_| "Could not read response".to_string());
            self.log_api_call("POST", &url, Some("{}"), status, Some(&error_body));
            let error = Err(format!("Ship scan failed with status: {}", status).into());
            crate::debug_fn_exit!("SpaceTradersClient::scan_ships", &error);
            return error;
        }

        let response_text = response.text().await?;
        self.log_api_call("POST", &url, Some("{}"), status, Some(&response_text));
        
        let scan_response: ShipScanResponse = serde_json::from_str(&response_text)?;
        let result = Ok(scan_response.data.ships);
        crate::debug_fn_exit!("SpaceTradersClient::scan_ships", &result);
        result
    }

    // Faction operations
    pub async fn get_factions(&self, page: Option<i32>, limit: Option<i32>) -> Result<Vec<Faction>, Box<dyn std::error::Error>> {
        crate::debug_fn_enter!("SpaceTradersClient::get_factions");
        
        let mut url = format!("{}/factions", API_BASE_URL);
        let mut query_params = Vec::new();
        
        if let Some(p) = page {
            query_params.push(format!("page={}", p));
        }
        if let Some(l) = limit {
            query_params.push(format!("limit={}", l));
        }
        
        if !query_params.is_empty() {
            url.push_str(&format!("?{}", query_params.join("&")));
        }
        
        crate::debug_api_call!("GET", &url);
        
        if !self.request_approval("GET", &url, None).await {
            let error = Err("API call not approved".into());
            crate::debug_fn_exit!("SpaceTradersClient::get_factions", &error);
            return error;
        }
        
        let response = self.client.get(&url).send().await?;
        let status = response.status().as_u16();
        
        if !response.status().is_success() {
            let error_body = response.text().await.unwrap_or_else(|_| "Could not read response".to_string());
            self.log_api_call("GET", &url, None, status, Some(&error_body));
            let error = Err(format!("Get factions failed with status: {}", status).into());
            crate::debug_fn_exit!("SpaceTradersClient::get_factions", &error);
            return error;
        }

        let response_text = response.text().await?;
        self.log_api_call("GET", &url, None, status, Some(&response_text));
        
        let factions_response: FactionsResponse = serde_json::from_str(&response_text)?;
        let result = Ok(factions_response.data);
        crate::debug_fn_exit!("SpaceTradersClient::get_factions", &result);
        result
    }

    pub async fn get_faction(&self, faction_symbol: &str) -> Result<Faction, Box<dyn std::error::Error>> {
        crate::debug_fn_enter!("SpaceTradersClient::get_faction", "faction_symbol={}", faction_symbol);
        
        let url = format!("{}/factions/{}", API_BASE_URL, faction_symbol);
        crate::debug_api_call!("GET", &url);
        
        if !self.request_approval("GET", &url, None).await {
            let error = Err("API call not approved".into());
            crate::debug_fn_exit!("SpaceTradersClient::get_faction", &error);
            return error;
        }
        
        let response = self.client.get(&url).send().await?;
        let status = response.status().as_u16();
        
        if !response.status().is_success() {
            let error_body = response.text().await.unwrap_or_else(|_| "Could not read response".to_string());
            self.log_api_call("GET", &url, None, status, Some(&error_body));
            let error = Err(format!("Get faction failed with status: {}", status).into());
            crate::debug_fn_exit!("SpaceTradersClient::get_faction", &error);
            return error;
        }

        let response_text = response.text().await?;
        self.log_api_call("GET", &url, None, status, Some(&response_text));
        
        let faction_response: FactionResponse = serde_json::from_str(&response_text)?;
        let result = Ok(faction_response.data);
        crate::debug_fn_exit!("SpaceTradersClient::get_faction", &result);
        result
    }

    // Jump gate operations
    pub async fn get_jump_gate(&self, system_symbol: &str, waypoint_symbol: &str) -> Result<JumpGate, Box<dyn std::error::Error>> {
        crate::debug_fn_enter!("SpaceTradersClient::get_jump_gate", "system={}, waypoint={}", system_symbol, waypoint_symbol);
        
        let url = format!("{}/systems/{}/waypoints/{}/jump-gate", API_BASE_URL, system_symbol, waypoint_symbol);
        crate::debug_api_call!("GET", &url);
        
        if !self.request_approval("GET", &url, None).await {
            let error = Err("API call not approved".into());
            crate::debug_fn_exit!("SpaceTradersClient::get_jump_gate", &error);
            return error;
        }
        
        let response = self.client.get(&url).send().await?;
        let status = response.status().as_u16();
        
        if !response.status().is_success() {
            let error_body = response.text().await.unwrap_or_else(|_| "Could not read response".to_string());
            self.log_api_call("GET", &url, None, status, Some(&error_body));
            let error = Err(format!("Get jump gate failed with status: {}", status).into());
            crate::debug_fn_exit!("SpaceTradersClient::get_jump_gate", &error);
            return error;
        }

        let response_text = response.text().await?;
        self.log_api_call("GET", &url, None, status, Some(&response_text));
        
        let jump_gate_response: JumpGateResponse = serde_json::from_str(&response_text)?;
        let result = Ok(jump_gate_response.data);
        crate::debug_fn_exit!("SpaceTradersClient::get_jump_gate", &result);
        result
    }

    // Jump navigation
    pub async fn jump_ship(&self, ship_symbol: &str, system_symbol: &str) -> Result<JumpData, Box<dyn std::error::Error>> {
        crate::debug_fn_enter!("SpaceTradersClient::jump_ship", "ship_symbol={}, system_symbol={}", ship_symbol, system_symbol);
        
        let url = format!("{}/my/ships/{}/jump", API_BASE_URL, ship_symbol);
        let payload = serde_json::json!({
            "systemSymbol": system_symbol
        });
        
        crate::debug_api_call!("POST", &url, &payload.to_string());
        
        if !self.request_approval("POST", &url, Some(&payload.to_string())).await {
            let error = Err("API call not approved".into());
            crate::debug_fn_exit!("SpaceTradersClient::jump_ship", &error);
            return error;
        }
        
        let response = self.client.post(&url).json(&payload).send().await?;
        let status = response.status().as_u16();
        
        if !response.status().is_success() {
            let error_body = response.text().await.unwrap_or_else(|_| "Could not read response".to_string());
            self.log_api_call("POST", &url, Some(&payload.to_string()), status, Some(&error_body));
            let error = Err(format!("Jump failed with status: {}", status).into());
            crate::debug_fn_exit!("SpaceTradersClient::jump_ship", &error);
            return error;
        }

        let response_text = response.text().await?;
        self.log_api_call("POST", &url, Some(&payload.to_string()), status, Some(&response_text));
        
        let jump_response: JumpResponse = serde_json::from_str(&response_text)?;
        let result = Ok(jump_response.data);
        crate::debug_fn_exit!("SpaceTradersClient::jump_ship", &result);
        result
    }
}