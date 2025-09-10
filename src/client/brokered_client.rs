use crate::client::api_broker::ApiRequestBroker;
use std::collections::HashMap;

/// A wrapper around reqwest::Client that routes through the API broker
#[derive(Clone)]
pub struct BrokeredClient {
    broker: ApiRequestBroker,
    token: String,
}

impl BrokeredClient {
    pub fn new(token: String) -> Self {
        Self {
            broker: ApiRequestBroker::global().clone(),
            token,
        }
    }
    
    /// GET request through broker
    pub fn get(&self, url: &str) -> BrokeredRequestBuilder {
        BrokeredRequestBuilder::new(self.broker.clone(), "GET".to_string(), url.to_string(), self.token.clone())
    }
    
    /// POST request through broker
    pub fn post(&self, url: &str) -> BrokeredRequestBuilder {
        BrokeredRequestBuilder::new(self.broker.clone(), "POST".to_string(), url.to_string(), self.token.clone())
    }
    
    /// PUT request through broker
    pub fn put(&self, url: &str) -> BrokeredRequestBuilder {
        BrokeredRequestBuilder::new(self.broker.clone(), "PUT".to_string(), url.to_string(), self.token.clone())
    }
    
    /// DELETE request through broker
    pub fn delete(&self, url: &str) -> BrokeredRequestBuilder {
        BrokeredRequestBuilder::new(self.broker.clone(), "DELETE".to_string(), url.to_string(), self.token.clone())
    }
}

/// Request builder that mimics reqwest::RequestBuilder API but uses the broker
pub struct BrokeredRequestBuilder {
    broker: ApiRequestBroker,
    method: String,
    url: String,
    token: String,
    body: Option<String>,
}

impl BrokeredRequestBuilder {
    fn new(broker: ApiRequestBroker, method: String, url: String, token: String) -> Self {
        Self {
            broker,
            method,
            url,
            token,
            body: None,
        }
    }
    
    /// Add JSON body (similar to reqwest's json method)
    pub fn json<T: serde::Serialize>(mut self, json: &T) -> Self {
        self.body = Some(serde_json::to_string(json).unwrap_or_default());
        self
    }
    
    /// Execute the request
    pub async fn send(self) -> Result<BrokeredResponse, reqwest::Error> {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("Authorization".to_string(), format!("Bearer {}", self.token));
        
        match self.broker.request(&self.method, &self.url, headers, self.body).await {
            Ok(response) => Ok(BrokeredResponse {
                status: response.status,
                body: response.body,
            }),
            Err(_e) => {
                // Create a synthetic reqwest error by making a dummy request to an invalid URL
                // This is a workaround since reqwest::Error cannot be constructed directly
                let dummy_err = reqwest::Client::new()
                    .get("http://invalid.localhost")
                    .send()
                    .await
                    .unwrap_err();
                Err(dummy_err)
            }
        }
    }
}

/// Response wrapper that mimics reqwest::Response
pub struct BrokeredResponse {
    status: u16,
    body: String,
}

impl BrokeredResponse {
    pub fn status(&self) -> reqwest::StatusCode {
        reqwest::StatusCode::from_u16(self.status).unwrap_or(reqwest::StatusCode::INTERNAL_SERVER_ERROR)
    }
    
    pub async fn text(self) -> Result<String, reqwest::Error> {
        Ok(self.body)
    }
    
    pub async fn json<T: serde::de::DeserializeOwned>(self) -> Result<T, reqwest::Error> {
        match serde_json::from_str(&self.body) {
            Ok(data) => Ok(data),
            Err(_e) => {
                // Create a synthetic reqwest error for JSON parsing failures
                let dummy_err = reqwest::Client::new()
                    .get("http://invalid.localhost")
                    .send()
                    .await
                    .unwrap_err();
                Err(dummy_err)
            }
        }
    }
}