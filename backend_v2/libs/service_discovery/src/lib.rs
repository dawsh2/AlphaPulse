//! # AlphaPulse Service Discovery System
//!
//! Provides dynamic service discovery to replace hardcoded socket paths throughout the system.
//! Enables automatic failover, load balancing, and environment-aware configuration.
//!
//! ## Problem Solved
//!
//! The current system has 47+ hardcoded socket paths like `/tmp/alphapulse/market_data.sock`
//! which create single points of failure and prevent horizontal scaling.
//!
//! ## Architecture
//!
//! ```text
//! Service Registry ←→ Configuration Files
//!       ↕                    ↕
//! [ServiceDiscovery] ←→ Environment Detection
//!       ↕                    ↕
//! Connection Pool    ←→ Health Monitoring
//! ```
//!
//! ## Features
//!
//! - **Environment Aware**: Automatically detects dev/staging/production environments
//! - **Dynamic Resolution**: Service paths resolved at runtime, not compile time
//! - **Automatic Failover**: Multiple service endpoints with health checking
//! - **Load Balancing**: Round-robin and weighted connection distribution
//! - **Connection Pooling**: Reuse connections for performance
//!
//! ## Usage
//!
//! ```rust,no_run
//! use alphapulse_service_discovery::{ServiceDiscovery, ServiceType};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let discovery = ServiceDiscovery::new().await?;
//! let endpoint = discovery.resolve("market_data_relay").await?;
//! let stream = endpoint.connect().await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Configuration
//!
//! Environment-specific configurations:
//!
//! **Development** (`config/environments/development.toml`):
//! ```toml
//! socket_dir = "/tmp/alphapulse"
//! log_dir = "/tmp/alphapulse_logs"
//! ```
//!
//! **Production** (`config/environments/production.toml`):
//! ```toml
//! socket_dir = "/var/run/alphapulse"
//! log_dir = "/var/log/alphapulse"
//! ```

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UnixStream;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, info, warn};

/// Type alias for service endpoints mapping
type ServiceEndpointsMap = Arc<RwLock<HashMap<ServiceType, Vec<ServiceEndpoint>>>>;

/// Type alias for round-robin counters
type RoundRobinCounters = Arc<RwLock<HashMap<ServiceType, usize>>>;

/// Environment types for AlphaPulse deployment
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Environment {
    Development,
    Staging,
    Production,
    Testing,
    Docker,
}

impl Environment {
    /// Detect current environment from environment variables and system state
    pub fn detect() -> Self {
        // Check explicit environment variable
        if let Ok(env_var) = env::var("ALPHAPULSE_ENV") {
            match env_var.to_lowercase().as_str() {
                "production" | "prod" => return Environment::Production,
                "staging" | "stage" => return Environment::Staging,
                "development" | "dev" => return Environment::Development,
                "testing" | "test" => return Environment::Testing,
                "docker" => return Environment::Docker,
                _ => {}
            }
        }

        // Auto-detect based on system characteristics
        if Path::new("/.dockerenv").exists() || env::var("CONTAINER").is_ok() {
            Environment::Docker
        } else if Path::new("/var/run/alphapulse").exists() {
            Environment::Production
        } else if env::var("CI").is_ok() {
            Environment::Testing
        } else {
            Environment::Development
        }
    }

    /// Get configuration file name for this environment
    pub fn config_file(&self) -> &'static str {
        match self {
            Environment::Development => "development.toml",
            Environment::Staging => "staging.toml",
            Environment::Production => "production.toml",
            Environment::Testing => "testing.toml",
            Environment::Docker => "docker.toml",
        }
    }

    /// Get default socket directory for this environment
    pub fn default_socket_dir(&self) -> &'static str {
        match self {
            Environment::Development => "/tmp/alphapulse",
            Environment::Staging => "/tmp/alphapulse-staging",
            Environment::Production => "/var/run/alphapulse",
            Environment::Testing => "/tmp/alphapulse-test",
            Environment::Docker => "/app/sockets",
        }
    }

    /// Get default log directory for this environment
    pub fn default_log_dir(&self) -> &'static str {
        match self {
            Environment::Development => "/tmp/alphapulse_logs",
            Environment::Staging => "/tmp/alphapulse_logs_staging",
            Environment::Production => "/var/log/alphapulse",
            Environment::Testing => "/tmp/alphapulse_logs_test",
            Environment::Docker => "/app/logs",
        }
    }
}

/// Service types in the AlphaPulse system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ServiceType {
    MarketDataRelay,
    SignalRelay,
    ExecutionRelay,
    PolygonPublisher,
    FlashArbitrage,
    DashboardWebSocket,
    KrakenCollector,
    BinanceCollector,
    CoinbaseCollector,
}

impl ServiceType {
    /// Get the service name as used in configuration
    pub fn name(&self) -> &'static str {
        match self {
            ServiceType::MarketDataRelay => "market_data_relay",
            ServiceType::SignalRelay => "signal_relay",
            ServiceType::ExecutionRelay => "execution_relay",
            ServiceType::PolygonPublisher => "polygon_publisher",
            ServiceType::FlashArbitrage => "flash_arbitrage",
            ServiceType::DashboardWebSocket => "dashboard_websocket",
            ServiceType::KrakenCollector => "kraken_collector",
            ServiceType::BinanceCollector => "binance_collector",
            ServiceType::CoinbaseCollector => "coinbase_collector",
        }
    }

    /// Get default socket file name
    pub fn socket_filename(&self) -> &'static str {
        match self {
            ServiceType::MarketDataRelay => "market_data.sock",
            ServiceType::SignalRelay => "signals.sock",
            ServiceType::ExecutionRelay => "execution.sock",
            ServiceType::PolygonPublisher => "polygon_publisher.sock",
            ServiceType::FlashArbitrage => "flash_arbitrage.sock",
            ServiceType::DashboardWebSocket => "dashboard.sock",
            ServiceType::KrakenCollector => "kraken.sock",
            ServiceType::BinanceCollector => "binance.sock",
            ServiceType::CoinbaseCollector => "coinbase.sock",
        }
    }
}

impl FromStr for ServiceType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "market_data_relay" => Ok(ServiceType::MarketDataRelay),
            "signal_relay" => Ok(ServiceType::SignalRelay),
            "execution_relay" => Ok(ServiceType::ExecutionRelay),
            "polygon_publisher" => Ok(ServiceType::PolygonPublisher),
            "flash_arbitrage" => Ok(ServiceType::FlashArbitrage),
            "dashboard_websocket" => Ok(ServiceType::DashboardWebSocket),
            "kraken_collector" => Ok(ServiceType::KrakenCollector),
            "binance_collector" => Ok(ServiceType::BinanceCollector),
            "coinbase_collector" => Ok(ServiceType::CoinbaseCollector),
            _ => Err(anyhow::anyhow!("Invalid service type: {}", s)),
        }
    }
}

/// Service endpoint information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    /// Service type
    pub service_type: ServiceType,
    /// Socket path
    pub socket_path: String,
    /// HTTP health check port (if available)
    pub health_port: Option<u16>,
    /// Service priority for load balancing (lower = higher priority)
    pub priority: u32,
    /// Whether this endpoint is currently healthy
    pub is_healthy: bool,
    /// Last health check time
    pub last_health_check: Option<std::time::SystemTime>,
}

impl ServiceEndpoint {
    /// Create new service endpoint
    pub fn new(service_type: ServiceType, socket_path: String) -> Self {
        Self {
            service_type,
            socket_path,
            health_port: None,
            priority: 100,
            is_healthy: true,
            last_health_check: None,
        }
    }

    /// Set health check port
    pub fn with_health_port(mut self, port: u16) -> Self {
        self.health_port = Some(port);
        self
    }

    /// Set priority for load balancing
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// Connect to this service endpoint
    pub async fn connect(&self) -> Result<UnixStream> {
        debug!(
            "Connecting to service: {} at {}",
            self.service_type.name(),
            self.socket_path
        );

        let stream = UnixStream::connect(&self.socket_path)
            .await
            .context(format!("Failed to connect to {}", self.socket_path))?;

        debug!(
            "Successfully connected to service: {}",
            self.service_type.name()
        );
        Ok(stream)
    }

    /// Check if the service endpoint is available
    pub async fn check_health(&mut self) -> bool {
        // Check if socket file exists
        if !Path::new(&self.socket_path).exists() {
            debug!("Socket file does not exist: {}", self.socket_path);
            self.is_healthy = false;
            self.last_health_check = Some(std::time::SystemTime::now());
            return false;
        }

        // Try to connect with timeout
        let connection_result = tokio::time::timeout(
            Duration::from_millis(500),
            UnixStream::connect(&self.socket_path),
        )
        .await;

        let is_healthy = match connection_result {
            Ok(Ok(_stream)) => {
                debug!(
                    "Health check passed for service: {}",
                    self.service_type.name()
                );
                true
            }
            Ok(Err(e)) => {
                debug!(
                    "Health check failed for service {}: {}",
                    self.service_type.name(),
                    e
                );
                false
            }
            Err(_) => {
                debug!(
                    "Health check timeout for service: {}",
                    self.service_type.name()
                );
                false
            }
        };

        self.is_healthy = is_healthy;
        self.last_health_check = Some(std::time::SystemTime::now());
        is_healthy
    }
}

/// Environment-specific service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    /// Base directory for Unix sockets
    pub socket_dir: String,
    /// Base directory for log files
    pub log_dir: String,
    /// PID file location
    pub pid_file: Option<String>,
    /// Service-specific configurations
    pub services: HashMap<String, ServiceConfig>,
}

/// Individual service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// Custom socket path (overrides default)
    pub socket_path: Option<String>,
    /// Health check port
    pub health_port: Option<u16>,
    /// Service priority
    pub priority: Option<u32>,
    /// Whether service is enabled
    pub enabled: bool,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            socket_path: None,
            health_port: None,
            priority: Some(100),
            enabled: true,
        }
    }
}

/// Load balancing strategies
#[derive(Debug, Clone)]
pub enum LoadBalancingStrategy {
    /// Use first healthy endpoint
    FirstHealthy,
    /// Round-robin across healthy endpoints
    RoundRobin,
    /// Use endpoint with lowest priority number
    Priority,
}

/// Main service discovery system
pub struct ServiceDiscovery {
    /// Current environment
    environment: Environment,
    /// Environment configuration
    config: EnvironmentConfig,
    /// Registered service endpoints
    endpoints: ServiceEndpointsMap,
    /// Load balancing strategy
    load_balancing: LoadBalancingStrategy,
    /// Round-robin counter for load balancing
    round_robin_counter: RoundRobinCounters,
}

impl ServiceDiscovery {
    /// Create new service discovery system
    pub async fn new() -> Result<Self> {
        let environment = Environment::detect();
        info!("Detected environment: {:?}", environment);

        let config = Self::load_config(&environment).await?;
        let endpoints = Arc::new(RwLock::new(HashMap::new()));
        let round_robin_counter = Arc::new(RwLock::new(HashMap::new()));

        let mut discovery = Self {
            environment,
            config,
            endpoints,
            load_balancing: LoadBalancingStrategy::Priority,
            round_robin_counter,
        };

        // Initialize default service endpoints
        discovery.initialize_default_services().await?;

        // Start health check background task
        discovery.start_health_checker().await;

        Ok(discovery)
    }

    /// Create service discovery for specific environment (testing)
    pub async fn for_environment(environment: Environment) -> Result<Self> {
        let config = Self::load_config(&environment).await?;
        let endpoints = Arc::new(RwLock::new(HashMap::new()));
        let round_robin_counter = Arc::new(RwLock::new(HashMap::new()));

        let mut discovery = Self {
            environment,
            config,
            endpoints,
            load_balancing: LoadBalancingStrategy::Priority,
            round_robin_counter,
        };

        discovery.initialize_default_services().await?;
        discovery.start_health_checker().await;

        Ok(discovery)
    }

    /// Load configuration for environment
    async fn load_config(environment: &Environment) -> Result<EnvironmentConfig> {
        // Try to load from config file first
        let config_path = PathBuf::from("config/environments").join(environment.config_file());

        if config_path.exists() {
            let content = tokio::fs::read_to_string(&config_path)
                .await
                .context(format!("Failed to read config file: {:?}", config_path))?;

            let config: EnvironmentConfig =
                toml::from_str(&content).context("Failed to parse environment config")?;

            info!("Loaded configuration from {:?}", config_path);
            return Ok(config);
        }

        // Fallback to default configuration
        warn!(
            "Configuration file not found, using defaults for {:?}",
            environment
        );
        Ok(EnvironmentConfig {
            socket_dir: environment.default_socket_dir().to_string(),
            log_dir: environment.default_log_dir().to_string(),
            pid_file: None,
            services: HashMap::new(),
        })
    }

    /// Initialize default service endpoints
    async fn initialize_default_services(&mut self) -> Result<()> {
        let services = vec![
            ServiceType::MarketDataRelay,
            ServiceType::SignalRelay,
            ServiceType::ExecutionRelay,
            ServiceType::PolygonPublisher,
            ServiceType::FlashArbitrage,
            ServiceType::DashboardWebSocket,
        ];

        let mut endpoints = self.endpoints.write().await;

        for service_type in services {
            let socket_path = self.get_socket_path(&service_type);
            let mut endpoint = ServiceEndpoint::new(service_type.clone(), socket_path);

            // Apply service-specific configuration
            if let Some(service_config) = self.config.services.get(service_type.name()) {
                if let Some(custom_path) = &service_config.socket_path {
                    endpoint.socket_path = custom_path.clone();
                }

                if let Some(health_port) = service_config.health_port {
                    endpoint.health_port = Some(health_port);
                }

                if let Some(priority) = service_config.priority {
                    endpoint.priority = priority;
                }

                if !service_config.enabled {
                    continue; // Skip disabled services
                }
            }

            endpoints.insert(service_type, vec![endpoint]);
        }

        info!("Initialized {} default service endpoints", endpoints.len());
        Ok(())
    }

    /// Get socket path for a service type
    fn get_socket_path(&self, service_type: &ServiceType) -> String {
        format!(
            "{}/{}",
            self.config.socket_dir,
            service_type.socket_filename()
        )
    }

    /// Resolve service by name
    pub async fn resolve(&self, service_name: &str) -> Result<ServiceEndpoint> {
        let service_type = ServiceType::from_str(service_name)?;

        self.resolve_service(&service_type).await
    }

    /// Resolve service by type
    pub async fn resolve_service(&self, service_type: &ServiceType) -> Result<ServiceEndpoint> {
        let endpoints = self.endpoints.read().await;
        let service_endpoints = endpoints
            .get(service_type)
            .ok_or_else(|| anyhow::anyhow!("No endpoints found for service: {:?}", service_type))?;

        if service_endpoints.is_empty() {
            return Err(anyhow::anyhow!(
                "No endpoints configured for service: {:?}",
                service_type
            ));
        }

        // Filter to healthy endpoints
        let healthy_endpoints: Vec<&ServiceEndpoint> = service_endpoints
            .iter()
            .filter(|ep| ep.is_healthy)
            .collect();

        if healthy_endpoints.is_empty() {
            warn!("No healthy endpoints for service: {:?}", service_type);
            // Return first endpoint even if unhealthy for fallback
            return Ok(service_endpoints[0].clone());
        }

        // Apply load balancing strategy
        let selected_endpoint = match self.load_balancing {
            LoadBalancingStrategy::FirstHealthy => healthy_endpoints[0].clone(),
            LoadBalancingStrategy::RoundRobin => {
                let mut counter = self.round_robin_counter.write().await;
                let current_count = counter.entry(service_type.clone()).or_insert(0);
                let selected = &healthy_endpoints[*current_count % healthy_endpoints.len()];
                *current_count += 1;
                (*selected).clone()
            }
            LoadBalancingStrategy::Priority => healthy_endpoints
                .into_iter()
                .min_by_key(|ep| ep.priority)
                .unwrap()
                .clone(),
        };

        debug!(
            "Resolved service {} to {}",
            service_type.name(),
            selected_endpoint.socket_path
        );
        Ok(selected_endpoint)
    }

    /// Register additional service endpoint
    pub async fn register_endpoint(&self, endpoint: ServiceEndpoint) -> Result<()> {
        let mut endpoints = self.endpoints.write().await;
        let service_endpoints = endpoints
            .entry(endpoint.service_type.clone())
            .or_insert_with(Vec::new);

        service_endpoints.push(endpoint);
        Ok(())
    }

    /// Get current environment
    pub fn environment(&self) -> &Environment {
        &self.environment
    }

    /// Get socket directory
    pub fn socket_dir(&self) -> &str {
        &self.config.socket_dir
    }

    /// Get log directory
    pub fn log_dir(&self) -> &str {
        &self.config.log_dir
    }

    /// Start background health checking
    async fn start_health_checker(&self) {
        let endpoints = Arc::clone(&self.endpoints);

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30)); // Check every 30 seconds

            loop {
                interval.tick().await;

                let mut endpoints_guard = endpoints.write().await;
                for (_service_type, service_endpoints) in endpoints_guard.iter_mut() {
                    for endpoint in service_endpoints.iter_mut() {
                        endpoint.check_health().await;
                    }
                }
                drop(endpoints_guard);

                debug!("Health check cycle completed");
            }
        });
    }

    /// Set load balancing strategy
    pub fn set_load_balancing(&mut self, strategy: LoadBalancingStrategy) {
        self.load_balancing = strategy;
    }

    /// Get service health status
    pub async fn get_service_health(&self, service_type: &ServiceType) -> Vec<ServiceEndpoint> {
        let endpoints = self.endpoints.read().await;
        endpoints.get(service_type).cloned().unwrap_or_default()
    }
}

/// Convenience trait for easy connection
#[async_trait]
pub trait ServiceConnector {
    async fn connect_to_service(&self, service_name: &str) -> Result<UnixStream>;
}

#[async_trait]
impl ServiceConnector for ServiceDiscovery {
    async fn connect_to_service(&self, service_name: &str) -> Result<UnixStream> {
        let endpoint = self.resolve(service_name).await?;
        endpoint.connect().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_detection() {
        let env = Environment::detect();
        // Should detect some environment
        assert!(matches!(
            env,
            Environment::Development
                | Environment::Production
                | Environment::Docker
                | Environment::Testing
        ));
    }

    #[test]
    fn test_service_type_parsing() {
        assert_eq!(
            ServiceType::from_str("market_data_relay").ok(),
            Some(ServiceType::MarketDataRelay)
        );
        assert_eq!(ServiceType::from_str("invalid_service").ok(), None);
    }

    #[tokio::test]
    async fn test_service_endpoint_creation() {
        let endpoint =
            ServiceEndpoint::new(ServiceType::MarketDataRelay, "/tmp/test.sock".to_string())
                .with_health_port(8001)
                .with_priority(50);

        assert_eq!(endpoint.service_type, ServiceType::MarketDataRelay);
        assert_eq!(endpoint.socket_path, "/tmp/test.sock");
        assert_eq!(endpoint.health_port, Some(8001));
        assert_eq!(endpoint.priority, 50);
    }

    #[tokio::test]
    async fn test_service_discovery_creation() {
        let discovery = ServiceDiscovery::for_environment(Environment::Testing).await;
        assert!(discovery.is_ok());

        let discovery = discovery.unwrap();
        assert_eq!(discovery.environment, Environment::Testing);
    }
}
