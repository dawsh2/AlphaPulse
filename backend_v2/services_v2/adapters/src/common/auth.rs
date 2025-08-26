//! Authentication and API key management

use alphapulse_types::VenueId;
use std::collections::HashMap;

/// API credentials for a venue
#[derive(Debug, Clone)]
pub struct ApiCredentials {
    /// Primary authentication token
    pub primary_token: String,
    /// Secondary authentication token (optional for some venues)
    pub secondary_token: Option<String>,
    /// Additional headers required by the venue
    pub headers: HashMap<String, String>,
}

impl ApiCredentials {
    /// Create new credentials with just a primary token
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            primary_token: token.into(),
            secondary_token: None,
            headers: HashMap::new(),
        }
    }

    /// Add a secondary token
    pub fn with_secondary(mut self, token: impl Into<String>) -> Self {
        self.secondary_token = Some(token.into());
        self
    }

    /// Add a custom header
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }
}

/// Manages API credentials for multiple venues
#[derive(Debug, Clone)]
pub struct AuthManager {
    credentials: HashMap<VenueId, ApiCredentials>,
}

impl AuthManager {
    /// Create a new auth manager
    pub fn new() -> Self {
        Self {
            credentials: HashMap::new(),
        }
    }

    /// Add credentials for a venue
    pub fn add_venue(&mut self, venue: VenueId, credentials: ApiCredentials) {
        self.credentials.insert(venue, credentials);
    }

    /// Set credentials for a venue using API key and secret
    pub fn set_credentials(&mut self, venue: VenueId, api_key: String, api_secret: String) {
        let credentials = ApiCredentials::new(api_key).with_secret(api_secret);
        self.add_venue(venue, credentials);
    }

    /// Get credentials for a venue
    pub fn get_credentials(&self, venue: VenueId) -> Option<&ApiCredentials> {
        self.credentials.get(&venue)
    }

    /// Build WebSocket URL with authentication
    pub fn build_websocket_url(&self, venue: VenueId, base_url: &str) -> anyhow::Result<String> {
        let credentials = self
            .get_credentials(venue)
            .ok_or_else(|| crate::AdapterError::AuthenticationFailed { venue })?;

        // Venue-specific URL building
        let url = match venue {
            VenueId::Binance => {
                // Binance uses stream keys
                format!("{}/stream?streams=", base_url)
            }
            VenueId::Polygon => {
                // Polygon includes API key in URL
                format!("{}?apikey={}", base_url, credentials.api_key)
            }
            _ => base_url.to_string(),
        };

        Ok(url)
    }

    /// Get authentication headers for REST requests
    pub fn get_auth_headers(&self, venue: VenueId) -> HashMap<String, String> {
        self.get_credentials(venue)
            .map(|creds| {
                let mut headers = creds.headers.clone();

                // Add venue-specific headers
                match venue {
                    VenueId::Binance => {
                        headers.insert("X-MBX-APIKEY".to_string(), creds.api_key.clone());
                    }
                    VenueId::Coinbase => {
                        headers.insert("CB-ACCESS-KEY".to_string(), creds.api_key.clone());
                        if let Some(secret) = &creds.api_secret {
                            headers.insert("CB-ACCESS-SECRET".to_string(), secret.clone());
                        }
                    }
                    _ => {}
                }

                headers
            })
            .unwrap_or_default()
    }
}

impl Default for AuthManager {
    fn default() -> Self {
        Self::new()
    }
}
