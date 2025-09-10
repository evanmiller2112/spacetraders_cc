use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::sync::OnceLock;
use tokio::sync::{mpsc, oneshot};
use tokio::time::sleep;
use crate::{o_info, o_debug, o_trace};

/// Global singleton broker instance
static GLOBAL_BROKER: OnceLock<ApiRequestBroker> = OnceLock::new();

/// Central API broker that manages ALL SpaceTraders API requests
/// Ensures global rate limiting and prevents 429 errors
#[derive(Clone)]
pub struct ApiRequestBroker {
    request_sender: mpsc::UnboundedSender<ApiRequest>,
}

/// API request that gets queued through the broker
pub struct ApiRequest {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub response_sender: oneshot::Sender<Result<ApiResponse, String>>,
}

/// API response from the broker
#[derive(Debug)]
pub struct ApiResponse {
    pub status: u16,
    pub body: String,
    pub headers: HashMap<String, String>,
}

/// Internal broker state
struct BrokerState {
    client: reqwest::Client,
    last_request_time: Option<Instant>,
    backoff_until: Option<Instant>,
    current_backoff_duration: Duration,
    request_count: u64,
}

impl ApiRequestBroker {
    /// Create a new API broker and start the background processing loop
    pub fn new() -> Self {
        let (request_sender, request_receiver) = mpsc::unbounded_channel();
        
        // Spawn the broker worker task
        tokio::spawn(Self::broker_worker(request_receiver));
        
        Self { request_sender }
    }
    
    /// Get or create the global singleton broker instance
    pub fn global() -> &'static ApiRequestBroker {
        GLOBAL_BROKER.get_or_init(|| {
            // Using stderr to ensure this message is visible immediately
            eprintln!("🌐🌐🌐 INITIALIZING GLOBAL SINGLETON API BROKER 🌐🌐🌐");
            let broker = Self::new();
            eprintln!("🌐🌐🌐 GLOBAL SINGLETON API BROKER CREATED SUCCESSFULLY 🌐🌐🌐");
            broker
        })
    }
    
    /// Submit an API request through the broker
    pub async fn request(
        &self,
        method: &str,
        url: &str,
        headers: HashMap<String, String>,
        body: Option<String>,
    ) -> Result<ApiResponse, String> {
        let (response_sender, response_receiver) = oneshot::channel();
        
        let request = ApiRequest {
            method: method.to_string(),
            url: url.to_string(),
            headers,
            body,
            response_sender,
        };
        
        // Send request to broker queue
        self.request_sender
            .send(request)
            .map_err(|_| "API broker is not running".to_string())?;
        
        // Wait for response
        response_receiver
            .await
            .map_err(|_| "API broker response channel closed".to_string())?
    }
    
    /// Background worker that processes all API requests with rate limiting
    async fn broker_worker(mut request_receiver: mpsc::UnboundedReceiver<ApiRequest>) {
        let mut state = BrokerState {
            client: reqwest::Client::new(),
            last_request_time: None,
            backoff_until: None,
            current_backoff_duration: Duration::from_millis(1000), // Start with 1s
            request_count: 0,
        };
        
        o_info!("🌐 API Request Broker started - centralizing ALL API calls");
        
        while let Some(request) = request_receiver.recv().await {
            Self::handle_request(&mut state, request).await;
        }
        
        o_info!("⚠️ API Request Broker stopped");
    }
    
    /// Handle a single API request with proper rate limiting
    async fn handle_request(state: &mut BrokerState, request: ApiRequest) {
        // Apply global backoff if needed
        if let Some(backoff_until) = state.backoff_until {
            let now = Instant::now();
            if now < backoff_until {
                let wait_duration = backoff_until - now;
                o_debug!("🌐 GLOBAL BACKOFF: Waiting {:.1}s before next request", wait_duration.as_secs_f64());
                sleep(wait_duration).await;
                state.backoff_until = None;
            }
        }
        
        // Rate limiting: Ensure minimum 600ms between requests (1.67 requests/second - more conservative)
        if let Some(last_time) = state.last_request_time {
            let elapsed = last_time.elapsed();
            let min_interval = Duration::from_millis(600);
            if elapsed < min_interval {
                let wait_time = min_interval - elapsed;
                sleep(wait_time).await;
            }
        }
        
        state.last_request_time = Some(Instant::now());
        state.request_count += 1;
        
        // Execute the HTTP request
        let result = Self::execute_http_request(state, &request).await;
        
        // Handle rate limiting responses
        if let Ok(ref response) = result {
            if response.status == 429 {
                // Exponential backoff on 429
                o_debug!("🌐 429 Rate Limited - applying exponential backoff: {:.1}s", 
                        state.current_backoff_duration.as_secs_f64());
                        
                state.backoff_until = Some(Instant::now() + state.current_backoff_duration);
                state.current_backoff_duration = std::cmp::min(
                    state.current_backoff_duration * 2,
                    Duration::from_secs(60) // Max 60s backoff
                );
            } else {
                // Success - reset backoff
                state.current_backoff_duration = Duration::from_millis(1000);
            }
        }
        
        // Send response back to caller
        if let Err(_) = request.response_sender.send(result) {
            o_info!("⚠️ Failed to send API response - caller dropped receiver");
        }
    }
    
    /// Execute the actual HTTP request
    async fn execute_http_request(
        state: &BrokerState, 
        request: &ApiRequest
    ) -> Result<ApiResponse, String> {
        
        o_trace!("🌐 API[{}] {} {}", 
                state.request_count, 
                request.method, 
                Self::sanitize_url(&request.url));
        
        let mut req_builder = match request.method.as_str() {
            "GET" => state.client.get(&request.url),
            "POST" => state.client.post(&request.url),
            "PUT" => state.client.put(&request.url),
            "DELETE" => state.client.delete(&request.url),
            "PATCH" => state.client.patch(&request.url),
            method => return Err(format!("Unsupported HTTP method: {}", method)),
        };
        
        // Add headers
        for (key, value) in &request.headers {
            req_builder = req_builder.header(key, value);
        }
        
        // Add body if present
        if let Some(ref body) = request.body {
            req_builder = req_builder.body(body.clone());
        }
        
        // Execute request with timeout
        let response = req_builder
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;
        
        let status = response.status().as_u16();
        let headers: HashMap<String, String> = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        
        let body = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response body: {}", e))?;
        
        if status != 200 && status != 201 {
            o_debug!("🌐 API[{}] Response: {} ({})", state.request_count, status, 
                    if body.len() > 100 { &body[..100] } else { &body });
        }
        
        Ok(ApiResponse {
            status,
            body,
            headers,
        })
    }
    
    /// Remove sensitive information from URLs for logging
    fn sanitize_url(url: &str) -> String {
        // Remove token from Authorization header by just showing the endpoint
        if let Some(api_pos) = url.find("/v2/") {
            url[api_pos..].to_string()
        } else {
            url.to_string()
        }
    }
}