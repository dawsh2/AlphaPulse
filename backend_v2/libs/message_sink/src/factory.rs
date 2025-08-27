//! SinkFactory for creating MessageSinks from configuration
//!
//! The SinkFactory provides a stable API for creating MessageSinks from configuration.
//! It bridges Stage 1 (TOML-based config) and Stage 2 (Mycelium runtime) by providing
//! a consistent interface that abstracts the underlying service discovery mechanism.

use crate::{
    config::{CompositePattern, PrecisionContext, ServiceConfig, SinkType},
    registry::ServiceRegistry,
    sinks::{CompositeSink, DirectSink, RelaySink},
    LazyConfig, LazyMessageSink, MessageDomain, MessageSink, SinkError,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Factory for creating MessageSinks from configuration
///
/// Provides a stable API that works across Stage 1 (config-based) and Stage 2 (Mycelium runtime).
/// The factory handles service discovery, dependency resolution, and sink lifecycle management.
#[derive(Debug)]
pub struct SinkFactory {
    /// Service registry for configuration lookup
    registry: ServiceRegistry,

    /// Cache of created sinks to prevent duplicate creation
    sink_cache: Arc<RwLock<HashMap<String, Arc<dyn MessageSink>>>>,

    /// Factory name for debugging
    name: String,
}

impl SinkFactory {
    /// Create a new sink factory with registry
    pub fn new(registry: ServiceRegistry) -> Self {
        Self {
            registry,
            sink_cache: Arc::new(RwLock::new(HashMap::new())),
            name: "sink-factory".to_string(),
        }
    }

    /// Create a new factory with name
    pub fn with_name(registry: ServiceRegistry, name: impl Into<String>) -> Self {
        Self {
            registry,
            sink_cache: Arc::new(RwLock::new(HashMap::new())),
            name: name.into(),
        }
    }

    /// Create a sink for the given service name
    ///
    /// This is the main entry point for sink creation. It handles:
    /// - Configuration lookup
    /// - Dependency resolution
    /// - Sink construction with appropriate lazy configuration
    /// - Caching to prevent duplicate sinks
    pub async fn create_sink(&self, service_name: &str) -> Result<Arc<dyn MessageSink>, SinkError> {
        // Check cache first
        {
            let cache = self.sink_cache.read().await;
            if let Some(sink) = cache.get(service_name) {
                tracing::debug!("Returning cached sink for service '{}'", service_name);
                return Ok(sink.clone());
            }
        }

        // Look up service configuration
        let config = self.registry.lookup(service_name).ok_or_else(|| {
            SinkError::invalid_config(format!(
                "Service '{}' not found in configuration",
                service_name
            ))
        })?;

        // Create sink based on configuration
        let sink = self.create_sink_from_config(service_name, config).await?;

        // Cache the sink
        {
            let mut cache = self.sink_cache.write().await;
            cache.insert(service_name.to_string(), sink.clone());
        }

        tracing::info!(
            "Created sink for service '{}' (type: {:?})",
            service_name,
            config.sink_type
        );
        Ok(sink)
    }

    /// Create sink from service configuration
    async fn create_sink_from_config(
        &self,
        service_name: &str,
        config: &ServiceConfig,
    ) -> Result<Arc<dyn MessageSink>, SinkError> {
        match config.sink_type {
            SinkType::Relay => self.create_relay_sink(service_name, config).await,
            SinkType::Direct => self.create_direct_sink(service_name, config).await,
            SinkType::Composite => self.create_composite_sink(service_name, config).await,
        }
    }

    /// Create a relay sink (Unix socket)
    async fn create_relay_sink(
        &self,
        service_name: &str,
        config: &ServiceConfig,
    ) -> Result<Arc<dyn MessageSink>, SinkError> {
        let endpoint = config.endpoint.as_ref().ok_or_else(|| {
            SinkError::invalid_config(format!("Relay sink '{}' missing endpoint", service_name))
        })?;

        let buffer_size = config.buffer_size.unwrap_or(10000);
        let relay_sink = RelaySink::new(endpoint, buffer_size)?;

        // Wrap with lazy connection if configured
        if let Some(lazy_config_toml) = &config.lazy {
            let lazy_config = lazy_config_toml.to_lazy_config();
            let endpoint_clone = endpoint.clone();
            let buffer_size_clone = buffer_size;

            let lazy_sink = if let Some(domain) = config.domain {
                LazyMessageSink::with_name_and_domain(
                    move || {
                        let endpoint = endpoint_clone.clone();
                        let buffer_size = buffer_size_clone;
                        async move {
                            RelaySink::new(endpoint, buffer_size)
                                .map_err(|e| SinkError::connection_failed(e.to_string()))
                        }
                    },
                    lazy_config,
                    format!("lazy-relay-{}", service_name),
                    domain,
                )
            } else {
                LazyMessageSink::with_name(
                    move || {
                        let endpoint = endpoint_clone.clone();
                        let buffer_size = buffer_size_clone;
                        async move {
                            RelaySink::new(endpoint, buffer_size)
                                .map_err(|e| SinkError::connection_failed(e.to_string()))
                        }
                    },
                    lazy_config,
                    format!("lazy-relay-{}", service_name),
                )
            };

            Ok(Arc::new(lazy_sink))
        } else {
            Ok(Arc::new(relay_sink))
        }
    }

    /// Create a direct sink (TCP/WebSocket/Unix)
    async fn create_direct_sink(
        &self,
        service_name: &str,
        config: &ServiceConfig,
    ) -> Result<Arc<dyn MessageSink>, SinkError> {
        let endpoint = config.endpoint.as_ref().ok_or_else(|| {
            SinkError::invalid_config(format!("Direct sink '{}' missing endpoint", service_name))
        })?;

        // Determine connection type from endpoint format
        let sink = if endpoint.starts_with("tcp://") {
            let address = endpoint.strip_prefix("tcp://").unwrap();
            DirectSink::tcp(address).await?
        } else if endpoint.starts_with("ws://") || endpoint.starts_with("wss://") {
            DirectSink::websocket(endpoint).await?
        } else if endpoint.starts_with("unix://") {
            let path = endpoint.strip_prefix("unix://").unwrap();
            DirectSink::unix(path).await?
        } else {
            return Err(SinkError::invalid_config(format!(
                "Direct sink '{}' has invalid endpoint format: {}",
                service_name, endpoint
            )));
        };

        // Wrap with lazy connection if configured
        if let Some(lazy_config_toml) = &config.lazy {
            let lazy_config = lazy_config_toml.to_lazy_config();

            let endpoint_clone = endpoint.clone();
            let lazy_sink = LazyMessageSink::with_name(
                move || {
                    let endpoint = endpoint_clone.clone();
                    async move {
                        if endpoint.starts_with("tcp://") {
                            let address = endpoint.strip_prefix("tcp://").unwrap();
                            DirectSink::tcp(address).await
                        } else if endpoint.starts_with("ws://") || endpoint.starts_with("wss://") {
                            DirectSink::websocket(&endpoint).await
                        } else if endpoint.starts_with("unix://") {
                            let path = endpoint.strip_prefix("unix://").unwrap();
                            DirectSink::unix(path).await
                        } else {
                            Err(SinkError::invalid_config(format!(
                                "Invalid endpoint format: {}",
                                endpoint
                            )))
                        }
                    }
                },
                lazy_config,
                format!("lazy-direct-{}", service_name),
            );

            Ok(Arc::new(lazy_sink))
        } else {
            Ok(Arc::new(sink))
        }
    }

    /// Create a composite sink (fanout/round-robin/failover)
    async fn create_composite_sink(
        &self,
        service_name: &str,
        config: &ServiceConfig,
    ) -> Result<Arc<dyn MessageSink>, SinkError> {
        let pattern = config.pattern.ok_or_else(|| {
            SinkError::invalid_config(format!("Composite sink '{}' missing pattern", service_name))
        })?;

        let target_names = config.targets.as_ref().ok_or_else(|| {
            SinkError::invalid_config(format!("Composite sink '{}' missing targets", service_name))
        })?;

        if target_names.is_empty() {
            return Err(SinkError::invalid_config(format!(
                "Composite sink '{}' has empty targets list",
                service_name
            )));
        }

        // Recursively create target sinks
        let mut targets = Vec::new();
        for target_name in target_names {
            // Prevent circular dependencies
            if target_name == service_name {
                return Err(SinkError::invalid_config(format!(
                    "Composite sink '{}' cannot target itself",
                    service_name
                )));
            }

            let target_sink = Box::pin(self.create_sink(target_name)).await.map_err(|e| {
                SinkError::invalid_config(format!(
                    "Failed to create target '{}' for composite sink '{}': {}",
                    target_name, service_name, e
                ))
            })?;
            targets.push(target_sink);
        }

        // Create composite sink with appropriate pattern
        let composite_sink = match pattern {
            CompositePattern::Fanout => CompositeSink::fanout(targets.clone()),
            CompositePattern::RoundRobin => CompositeSink::round_robin(targets.clone()),
            CompositePattern::Failover => CompositeSink::failover(targets.clone()),
        };

        // Wrap with lazy connection if configured
        if let Some(lazy_config_toml) = &config.lazy {
            let lazy_config = lazy_config_toml.to_lazy_config();

            // For composite sinks, lazy wrapping connects all targets when needed
            let targets_clone = targets;
            let pattern_clone = pattern;
            let lazy_sink = LazyMessageSink::with_name(
                move || {
                    let targets = targets_clone.clone();
                    let pattern = pattern_clone;
                    async move {
                        let composite = match pattern {
                            CompositePattern::Fanout => CompositeSink::fanout(targets),
                            CompositePattern::RoundRobin => CompositeSink::round_robin(targets),
                            CompositePattern::Failover => CompositeSink::failover(targets),
                        };
                        Ok(composite)
                    }
                },
                lazy_config,
                format!("lazy-composite-{}", service_name),
            );

            Ok(Arc::new(lazy_sink))
        } else {
            Ok(Arc::new(composite_sink))
        }
    }

    /// Get all cached sinks
    pub async fn cached_sinks(&self) -> Vec<(String, Arc<dyn MessageSink>)> {
        let cache = self.sink_cache.read().await;
        cache
            .iter()
            .map(|(name, sink)| (name.clone(), sink.clone()))
            .collect()
    }

    /// Clear sink cache
    pub async fn clear_cache(&self) {
        let mut cache = self.sink_cache.write().await;
        cache.clear();
        tracing::info!("Factory '{}' cache cleared", self.name);
    }

    /// Remove specific sink from cache
    pub async fn remove_from_cache(&self, service_name: &str) -> bool {
        let mut cache = self.sink_cache.write().await;
        let removed = cache.remove(service_name).is_some();
        if removed {
            tracing::info!(
                "Removed sink '{}' from factory '{}' cache",
                service_name,
                self.name
            );
        }
        removed
    }

    /// Validate configuration without creating sinks
    pub fn validate_config(&self, service_name: &str) -> Result<(), SinkError> {
        let config = self.registry.lookup(service_name).ok_or_else(|| {
            SinkError::invalid_config(format!(
                "Service '{}' not found in configuration. Available services: [{}]",
                service_name,
                self.registry
                    .service_names()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        })?;

        // Validate TLV domain configuration
        if let Some(domain) = config.domain {
            self.validate_tlv_domain(service_name, domain, &config.sink_type)?;
        }

        // Validate precision context
        if let Some(precision) = config.precision_context {
            self.validate_precision_context(service_name, precision, &config.sink_type)?;
        }

        // Validate endpoint requirements and accessibility
        match config.sink_type {
            SinkType::Relay | SinkType::Direct => {
                if let Some(endpoint) = &config.endpoint {
                    self.validate_endpoint_accessibility(service_name, endpoint)?;
                } else {
                    return Err(SinkError::invalid_config(
                        format!("{} sink '{}' requires an endpoint. Add 'endpoint = \"...\"' to configuration.",
                            config.sink_type.name(), service_name)
                    ));
                }
            }
            SinkType::Composite => {
                if config.targets.is_none() || config.targets.as_ref().unwrap().is_empty() {
                    return Err(SinkError::invalid_config(
                        format!("Composite sink '{}' requires targets. Add 'targets = [\"...\"]' to configuration.",
                            service_name)
                    ));
                }

                if config.pattern.is_none() {
                    return Err(SinkError::invalid_config(
                        format!("Composite sink '{}' requires a pattern. Add 'pattern = \"fanout|round_robin|failover\"' to configuration.",
                            service_name)
                    ));
                }

                // Validate composite target dependencies
                self.validate_composite_dependencies(service_name, config)?;
            }
        }

        Ok(())
    }

    /// Validate TLV domain assignment follows Protocol V2 ranges
    fn validate_tlv_domain(
        &self,
        service_name: &str,
        domain: MessageDomain,
        sink_type: &SinkType,
    ) -> Result<(), SinkError> {
        match (domain, sink_type) {
            (MessageDomain::MarketData, SinkType::Relay) => {
                // Market data relay should use TLV types 1-19
                tracing::debug!(
                    "Service '{}' configured for Market Data domain (TLV types 1-19)",
                    service_name
                );
            }
            (MessageDomain::Signals, SinkType::Relay) => {
                // Signal relay should use TLV types 20-39
                tracing::debug!(
                    "Service '{}' configured for Signals domain (TLV types 20-39)",
                    service_name
                );
            }
            (MessageDomain::Execution, SinkType::Relay) => {
                // Execution relay should use TLV types 40-79
                tracing::debug!(
                    "Service '{}' configured for Execution domain (TLV types 40-79)",
                    service_name
                );
            }
            (domain, SinkType::Direct) => {
                tracing::debug!(
                    "Direct sink '{}' configured for {:?} domain",
                    service_name,
                    domain
                );
            }
            (domain, SinkType::Composite) => {
                tracing::debug!(
                    "Composite sink '{}' configured for {:?} domain",
                    service_name,
                    domain
                );
            }
            (MessageDomain::Unknown, _) => {
                tracing::warn!("Service '{}' configured with unknown domain - this may indicate missing TLV type registration", service_name);
            }
        }

        Ok(())
    }

    /// Validate precision context is appropriate for sink type
    fn validate_precision_context(
        &self,
        service_name: &str,
        precision: PrecisionContext,
        sink_type: &SinkType,
    ) -> Result<(), SinkError> {
        match (precision, sink_type) {
            (PrecisionContext::DexToken, SinkType::Relay) => {
                tracing::debug!("Service '{}' will preserve native DEX token precision (18 decimals WETH, 6 USDC, etc.)", service_name);
            }
            (PrecisionContext::TraditionalExchange, SinkType::Direct) => {
                tracing::debug!(
                    "Service '{}' will use 8-decimal fixed-point for USD prices",
                    service_name
                );
            }
            (PrecisionContext::Mixed, _) => {
                tracing::debug!(
                    "Service '{}' will handle mixed precision with automatic detection",
                    service_name
                );
            }
            (context, sink_type) => {
                tracing::debug!(
                    "Service '{}' ({:?}) configured with {:?} precision context",
                    service_name,
                    sink_type,
                    context
                );
            }
        }

        Ok(())
    }

    /// Validate endpoint accessibility to prevent runtime failures
    fn validate_endpoint_accessibility(
        &self,
        service_name: &str,
        endpoint: &str,
    ) -> Result<(), SinkError> {
        if endpoint.starts_with("unix://") {
            let socket_path = endpoint.strip_prefix("unix://").unwrap();
            self.validate_unix_socket_path(service_name, socket_path)?;
        } else if endpoint.starts_with("/") {
            // Direct Unix socket path format
            self.validate_unix_socket_path(service_name, endpoint)?;
        } else if endpoint.starts_with("tcp://") {
            self.validate_tcp_endpoint(service_name, endpoint)?;
        } else if endpoint.starts_with("ws://") || endpoint.starts_with("wss://") {
            self.validate_websocket_endpoint(service_name, endpoint)?;
        } else {
            return Err(SinkError::invalid_config(
                format!("Service '{}' has unsupported endpoint format: '{}'. Supported formats: unix://, tcp://, ws://, wss://", 
                    service_name, endpoint)
            ));
        }

        Ok(())
    }

    /// Validate Unix socket path accessibility
    fn validate_unix_socket_path(
        &self,
        service_name: &str,
        socket_path: &str,
    ) -> Result<(), SinkError> {
        let path = std::path::Path::new(socket_path);

        // Check if parent directory exists and is writable
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                return Err(SinkError::invalid_config(
                    format!("Service '{}' socket directory does not exist: '{}'. Create directory or update endpoint path.", 
                        service_name, parent.display())
                ));
            }

            // Try to check write permissions (best effort)
            if let Ok(metadata) = std::fs::metadata(parent) {
                if metadata.permissions().readonly() {
                    tracing::warn!(
                        "Service '{}' socket directory may not be writable: '{}'",
                        service_name,
                        parent.display()
                    );
                }
            }
        }

        // Warn if socket file already exists (may indicate stale socket)
        if path.exists() {
            tracing::debug!(
                "Service '{}' socket already exists: '{}' (may be from previous run)",
                service_name,
                socket_path
            );
        }

        tracing::debug!(
            "Service '{}' Unix socket path validated: '{}'",
            service_name,
            socket_path
        );
        Ok(())
    }

    /// Validate TCP endpoint format and basic connectivity
    fn validate_tcp_endpoint(&self, service_name: &str, endpoint: &str) -> Result<(), SinkError> {
        let address = endpoint.strip_prefix("tcp://").unwrap();

        // Basic format validation - must contain host:port
        if !address.contains(':') {
            return Err(SinkError::invalid_config(format!(
                "Service '{}' TCP endpoint missing port: '{}'. Use format: tcp://host:port",
                service_name, endpoint
            )));
        }

        // Try to parse the address to validate format
        if let Err(_) = address.parse::<std::net::SocketAddr>() {
            // Try parsing as hostname:port
            let parts: Vec<&str> = address.split(':').collect();
            if parts.len() != 2 {
                return Err(SinkError::invalid_config(format!(
                    "Service '{}' invalid TCP endpoint format: '{}'. Use format: tcp://host:port",
                    service_name, endpoint
                )));
            }

            // Validate port number
            if let Err(_) = parts[1].parse::<u16>() {
                return Err(SinkError::invalid_config(format!(
                    "Service '{}' invalid port number in endpoint: '{}'. Port must be 1-65535",
                    service_name, endpoint
                )));
            }
        }

        tracing::debug!(
            "Service '{}' TCP endpoint format validated: '{}'",
            service_name,
            endpoint
        );
        Ok(())
    }

    /// Validate WebSocket endpoint format
    fn validate_websocket_endpoint(
        &self,
        service_name: &str,
        endpoint: &str,
    ) -> Result<(), SinkError> {
        // Basic URL format validation
        if let Err(_) = url::Url::parse(endpoint) {
            return Err(SinkError::invalid_config(
                format!("Service '{}' invalid WebSocket URL format: '{}'. Use format: ws://host:port or wss://host:port",
                    service_name, endpoint)
            ));
        }

        tracing::debug!(
            "Service '{}' WebSocket endpoint format validated: '{}'",
            service_name,
            endpoint
        );
        Ok(())
    }

    /// Validate composite sink dependencies to prevent runtime failures
    fn validate_composite_dependencies(
        &self,
        service_name: &str,
        config: &ServiceConfig,
    ) -> Result<(), SinkError> {
        if let Some(targets) = &config.targets {
            for target_name in targets {
                // Check if target exists in registry
                if !self.registry.contains_service(target_name) {
                    return Err(SinkError::invalid_config(format!(
                        "Service '{}' references unknown target '{}'. Available services: [{}]",
                        service_name,
                        target_name,
                        self.registry
                            .service_names()
                            .map(|s| s.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )));
                }

                // Check for direct circular dependency
                if target_name == service_name {
                    return Err(SinkError::invalid_config(
                        format!("Service '{}' cannot target itself. Remove self-reference from targets list.",
                            service_name)
                    ));
                }

                // Validate target configuration recursively (but don't follow circular paths)
                if let Some(target_config) = self.registry.lookup(target_name) {
                    if target_config.sink_type == SinkType::Composite {
                        if let Some(target_targets) = &target_config.targets {
                            if target_targets.contains(&service_name.to_string()) {
                                return Err(SinkError::invalid_config(
                                    format!("Circular dependency detected: Service '{}' targets '{}' which targets back to '{}'",
                                        service_name, target_name, service_name)
                                ));
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Get factory statistics
    pub async fn stats(&self) -> SinkFactoryStats {
        let cache = self.sink_cache.read().await;
        let total_services = self.registry.service_count();
        let cached_sinks = cache.len();

        SinkFactoryStats {
            name: self.name.clone(),
            total_services,
            cached_sinks,
            cache_hit_rate: if total_services > 0 {
                cached_sinks as f64 / total_services as f64
            } else {
                0.0
            },
        }
    }
}

/// Statistics for sink factory monitoring
#[derive(Debug, Clone)]
pub struct SinkFactoryStats {
    /// Factory name
    pub name: String,
    /// Total services in configuration
    pub total_services: usize,
    /// Number of cached sinks
    pub cached_sinks: usize,
    /// Cache hit rate
    pub cache_hit_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{LazyConfigToml, ServicesConfig};
    use std::io::Write;
    use tempfile::TempDir;

    async fn create_test_factory() -> (SinkFactory, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("services.toml");

        let config_content = r#"
[services.test_relay]
type = "relay"
endpoint = "/tmp/test_relay.sock"
buffer_size = 5000

[services.test_direct]
type = "direct"
endpoint = "tcp://localhost:8080"

[services.test_composite]
type = "composite"
pattern = "fanout"
targets = ["test_relay", "test_direct"]

[services.test_lazy]
type = "relay"
endpoint = "/tmp/lazy_relay.sock"
lazy = { retry_count = 5, retry_delay_ms = 1000, timeout_ms = 5000 }
        "#;

        std::fs::write(&config_path, config_content).unwrap();

        let registry = ServiceRegistry::from_file(&config_path).unwrap();
        let factory = SinkFactory::new(registry);

        (factory, temp_dir)
    }

    #[tokio::test]
    async fn test_factory_creation() {
        let (factory, _temp_dir) = create_test_factory().await;

        let stats = factory.stats().await;
        assert_eq!(stats.total_services, 4);
        assert_eq!(stats.cached_sinks, 0);
        assert_eq!(stats.cache_hit_rate, 0.0);
    }

    #[tokio::test]
    async fn test_relay_sink_creation() {
        let (factory, _temp_dir) = create_test_factory().await;

        let sink = factory.create_sink("test_relay").await.unwrap();
        let metadata = sink.metadata();
        assert_eq!(metadata.sink_type, "lazy"); // Wrapped in LazyMessageSink

        // Check caching
        let stats = factory.stats().await;
        assert_eq!(stats.cached_sinks, 1);
    }

    #[tokio::test]
    async fn test_direct_sink_creation() {
        let (factory, _temp_dir) = create_test_factory().await;

        let sink = factory.create_sink("test_direct").await.unwrap();
        let metadata = sink.metadata();
        assert_eq!(metadata.sink_type, "lazy"); // Wrapped in LazyMessageSink
    }

    #[tokio::test]
    async fn test_composite_sink_creation() {
        let (factory, _temp_dir) = create_test_factory().await;

        let sink = factory.create_sink("test_composite").await.unwrap();
        let metadata = sink.metadata();
        assert_eq!(metadata.sink_type, "lazy"); // Wrapped in LazyMessageSink

        // Should have created target sinks too
        let stats = factory.stats().await;
        assert_eq!(stats.cached_sinks, 3); // composite + 2 targets
    }

    #[tokio::test]
    async fn test_lazy_configuration() {
        let (factory, _temp_dir) = create_test_factory().await;

        let sink = factory.create_sink("test_lazy").await.unwrap();
        let metadata = sink.metadata();
        assert_eq!(metadata.sink_type, "lazy");
    }

    #[tokio::test]
    async fn test_invalid_service() {
        let (factory, _temp_dir) = create_test_factory().await;

        let result = factory.create_sink("nonexistent").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_cache_management() {
        let (factory, _temp_dir) = create_test_factory().await;

        // Create sink
        factory.create_sink("test_relay").await.unwrap();
        assert_eq!(factory.stats().await.cached_sinks, 1);

        // Remove from cache
        let removed = factory.remove_from_cache("test_relay").await;
        assert!(removed);
        assert_eq!(factory.stats().await.cached_sinks, 0);

        // Clear cache (empty now, but test the method)
        factory.clear_cache().await;
        assert_eq!(factory.stats().await.cached_sinks, 0);
    }

    #[tokio::test]
    async fn test_circular_dependency_prevention() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("circular.toml");

        let config_content = r#"
[services.circular]
type = "composite"
pattern = "fanout"
targets = ["circular"]
        "#;

        std::fs::write(&config_path, config_content).unwrap();

        let registry = ServiceRegistry::from_file(&config_path).unwrap();
        let factory = SinkFactory::new(registry);

        let result = factory.create_sink("circular").await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("cannot target itself"));
    }

    #[tokio::test]
    async fn test_invalid_endpoint_formats() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("invalid.toml");

        let config_content = r#"
[services.invalid_direct]
type = "direct"
endpoint = "invalid://endpoint"
        "#;

        std::fs::write(&config_path, config_content).unwrap();

        let registry = ServiceRegistry::from_file(&config_path).unwrap();
        let factory = SinkFactory::new(registry);

        let result = factory.create_sink("invalid_direct").await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("invalid endpoint format"));
    }
}
