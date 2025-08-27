//! Service registry for configuration-based sink lookup
//!
//! Provides fast lookup of service configurations during Stage 1 (config-based)
//! sink creation. The registry loads service definitions from TOML files and
//! enables efficient service discovery during factory operations.

use crate::config::{ServicesConfig, ServiceConfig};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// Registry for service configuration lookup
#[derive(Debug, Clone)]
pub struct ServiceRegistry {
    /// Map of service name to configuration
    services: HashMap<String, ServiceConfig>,
    
    /// Configuration file path (for reloading)
    config_path: Option<std::path::PathBuf>,
    
    /// Registry metadata
    metadata: RegistryMetadata,
}

/// Metadata about the service registry
#[derive(Debug, Clone)]
pub struct RegistryMetadata {
    /// When the registry was loaded
    pub loaded_at: std::time::SystemTime,
    
    /// Number of services registered
    pub service_count: usize,
    
    /// Registry version (for cache invalidation)
    pub version: u64,
    
    /// Source of the registry (file path or inline)
    pub source: String,
}

impl ServiceRegistry {
    /// Create registry from configuration file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let path = path.as_ref();
        let config = ServicesConfig::from_file(path)?;
        config.validate()?;
        
        let metadata = RegistryMetadata {
            loaded_at: std::time::SystemTime::now(),
            service_count: config.services.len(),
            version: 1,
            source: path.to_string_lossy().to_string(),
        };
        
        Ok(Self {
            services: config.services,
            config_path: Some(path.to_owned()),
            metadata,
        })
    }
    
    /// Create registry from TOML string
    pub fn from_toml_str(toml_str: &str) -> Result<Self, String> {
        let config = ServicesConfig::from_toml(toml_str)?;
        config.validate()?;
        
        let metadata = RegistryMetadata {
            loaded_at: std::time::SystemTime::now(),
            service_count: config.services.len(),
            version: 1,
            source: "<inline>".to_string(),
        };
        
        Ok(Self {
            services: config.services,
            config_path: None,
            metadata,
        })
    }
    
    /// Create registry from ServicesConfig
    pub fn from_config(config: ServicesConfig) -> Result<Self, String> {
        config.validate()?;
        
        let metadata = RegistryMetadata {
            loaded_at: std::time::SystemTime::now(),
            service_count: config.services.len(),
            version: 1,
            source: "<programmatic>".to_string(),
        };
        
        Ok(Self {
            services: config.services,
            config_path: None,
            metadata,
        })
    }
    
    /// Look up service configuration by name
    pub fn lookup(&self, service_name: &str) -> Option<&ServiceConfig> {
        self.services.get(service_name)
    }
    
    /// Get all registered service names
    pub fn service_names(&self) -> impl Iterator<Item = &String> {
        self.services.keys()
    }
    
    /// Get registry metadata
    pub fn metadata(&self) -> &RegistryMetadata {
        &self.metadata
    }
    
    /// Check if service exists
    pub fn contains_service(&self, service_name: &str) -> bool {
        self.services.contains_key(service_name)
    }
    
    /// Get service count
    pub fn service_count(&self) -> usize {
        self.services.len()
    }
    
    /// Reload registry from file (if loaded from file)
    pub fn reload(&mut self) -> Result<bool, String> {
        if let Some(path) = &self.config_path {
            let new_registry = Self::from_file(path)?;
            
            // Check if anything changed
            let changed = self.services != new_registry.services;
            
            if changed {
                self.services = new_registry.services;
                self.metadata.loaded_at = std::time::SystemTime::now();
                self.metadata.service_count = self.services.len();
                self.metadata.version += 1;
                
                tracing::info!("Registry reloaded with {} services", self.metadata.service_count);
            }
            
            Ok(changed)
        } else {
            Err("Cannot reload registry that wasn't loaded from file".to_string())
        }
    }
    
    /// Get services by type
    pub fn services_by_type(&self, sink_type: crate::config::SinkType) -> Vec<(&String, &ServiceConfig)> {
        self.services
            .iter()
            .filter(|(_, config)| config.sink_type == sink_type)
            .collect()
    }
    
    /// Get composite services that reference a given target
    pub fn services_targeting(&self, target_service: &str) -> Vec<(&String, &ServiceConfig)> {
        self.services
            .iter()
            .filter(|(_, config)| {
                config.targets.as_ref()
                    .map(|targets| targets.contains(&target_service.to_string()))
                    .unwrap_or(false)
            })
            .collect()
    }
    
    /// Validate all service references are resolvable
    pub fn validate_references(&self) -> Result<(), String> {
        for (service_name, config) in &self.services {
            if let Some(targets) = &config.targets {
                for target in targets {
                    if !self.contains_service(target) {
                        return Err(format!(
                            "Service '{}' references unknown target '{}'",
                            service_name, target
                        ));
                    }
                }
            }
        }
        Ok(())
    }
    
    /// Get dependency graph (which services depend on which)
    pub fn dependency_graph(&self) -> HashMap<String, Vec<String>> {
        let mut graph = HashMap::new();
        
        for (service_name, config) in &self.services {
            let dependencies = config.targets.clone().unwrap_or_default();
            graph.insert(service_name.clone(), dependencies);
        }
        
        graph
    }
    
    /// Check for circular dependencies
    pub fn detect_circular_dependencies(&self) -> Result<(), String> {
        let graph = self.dependency_graph();
        
        for (service, _) in &graph {
            if self.has_circular_dependency(service, &graph, &mut Vec::new())? {
                return Err(format!("Circular dependency detected involving service '{}'", service));
            }
        }
        
        Ok(())
    }
    
    /// Helper method to detect circular dependencies using DFS
    fn has_circular_dependency(
        &self,
        service: &str,
        graph: &HashMap<String, Vec<String>>,
        path: &mut Vec<String>,
    ) -> Result<bool, String> {
        if path.contains(&service.to_string()) {
            return Ok(true); // Circular dependency found
        }
        
        path.push(service.to_string());
        
        if let Some(dependencies) = graph.get(service) {
            for dep in dependencies {
                if self.has_circular_dependency(dep, graph, path)? {
                    return Ok(true);
                }
            }
        }
        
        path.pop();
        Ok(false)
    }
}

/// Thread-safe wrapper for ServiceRegistry
#[derive(Debug, Clone)]
pub struct ConcurrentServiceRegistry {
    inner: Arc<std::sync::RwLock<ServiceRegistry>>,
}

impl ConcurrentServiceRegistry {
    /// Create from ServiceRegistry
    pub fn new(registry: ServiceRegistry) -> Self {
        Self {
            inner: Arc::new(std::sync::RwLock::new(registry)),
        }
    }
    
    /// Create from file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let registry = ServiceRegistry::from_file(path)?;
        Ok(Self::new(registry))
    }
    
    /// Look up service configuration
    pub fn lookup(&self, service_name: &str) -> Option<ServiceConfig> {
        let registry = self.inner.read().map_err(|_| "Registry lock poisoned").ok()?;
        registry.lookup(service_name).cloned()
    }
    
    /// Get service count
    pub fn service_count(&self) -> Result<usize, String> {
        let registry = self.inner.read().map_err(|_| "Registry lock poisoned")?;
        Ok(registry.service_count())
    }
    
    /// Reload from file
    pub fn reload(&self) -> Result<bool, String> {
        let mut registry = self.inner.write().map_err(|_| "Registry lock poisoned")?;
        registry.reload()
    }
    
    /// Get metadata
    pub fn metadata(&self) -> Result<RegistryMetadata, String> {
        let registry = self.inner.read().map_err(|_| "Registry lock poisoned")?;
        Ok(registry.metadata().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{SinkType, CompositePattern};

    fn sample_config() -> &'static str {
        r#"
            [services.relay_service]
            type = "relay"
            endpoint = "unix:///tmp/relay.sock"
            
            [services.direct_service]
            type = "direct"
            endpoint = "tcp://localhost:8080"
            
            [services.target1]
            type = "direct"
            endpoint = "tcp://localhost:8001"
            
            [services.target2]
            type = "direct"
            endpoint = "tcp://localhost:8002"
            
            [services.fanout_service]
            type = "composite"
            pattern = "fanout"
            targets = ["target1", "target2"]
            
            [services.failover_service]
            type = "composite"
            pattern = "failover"
            targets = ["target1", "target2"]
        "#
    }
    
    #[test]
    fn test_registry_creation() {
        let registry = ServiceRegistry::from_toml_str(sample_config()).unwrap();
        
        assert_eq!(registry.service_count(), 6);
        assert!(registry.contains_service("relay_service"));
        assert!(registry.contains_service("fanout_service"));
        assert!(!registry.contains_service("nonexistent"));
    }
    
    #[test]
    fn test_service_lookup() {
        let registry = ServiceRegistry::from_toml_str(sample_config()).unwrap();
        
        let relay_config = registry.lookup("relay_service").unwrap();
        assert_eq!(relay_config.sink_type, SinkType::Relay);
        assert_eq!(relay_config.endpoint, Some("unix:///tmp/relay.sock".to_string()));
        
        let fanout_config = registry.lookup("fanout_service").unwrap();
        assert_eq!(fanout_config.sink_type, SinkType::Composite);
        assert_eq!(fanout_config.pattern, Some(CompositePattern::Fanout));
        assert_eq!(fanout_config.targets, Some(vec!["target1".to_string(), "target2".to_string()]));
    }
    
    #[test]
    fn test_services_by_type() {
        let registry = ServiceRegistry::from_toml_str(sample_config()).unwrap();
        
        let relay_services = registry.services_by_type(SinkType::Relay);
        assert_eq!(relay_services.len(), 1);
        
        let direct_services = registry.services_by_type(SinkType::Direct);
        assert_eq!(direct_services.len(), 3); // direct_service, target1, target2
        
        let composite_services = registry.services_by_type(SinkType::Composite);
        assert_eq!(composite_services.len(), 2); // fanout_service, failover_service
    }
    
    #[test]
    fn test_services_targeting() {
        let registry = ServiceRegistry::from_toml_str(sample_config()).unwrap();
        
        let targeting_target1 = registry.services_targeting("target1");
        assert_eq!(targeting_target1.len(), 2); // fanout_service, failover_service
        
        let targeting_nonexistent = registry.services_targeting("nonexistent");
        assert_eq!(targeting_nonexistent.len(), 0);
    }
    
    #[test]
    fn test_dependency_graph() {
        let registry = ServiceRegistry::from_toml_str(sample_config()).unwrap();
        let graph = registry.dependency_graph();
        
        assert_eq!(graph["fanout_service"], vec!["target1", "target2"]);
        assert_eq!(graph["failover_service"], vec!["target1", "target2"]);
        assert!(graph["direct_service"].is_empty());
    }
    
    #[test]
    fn test_circular_dependency_detection() {
        let circular_config = r#"
            [services.service_a]
            type = "composite"
            pattern = "fanout"
            targets = ["service_b"]
            
            [services.service_b]
            type = "composite"
            pattern = "fanout"
            targets = ["service_a"]
        "#;
        
        let registry = ServiceRegistry::from_toml_str(circular_config).unwrap();
        let result = registry.detect_circular_dependencies();
        
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Circular dependency"));
    }
    
    #[test]
    fn test_concurrent_registry() {
        let registry = ServiceRegistry::from_toml_str(sample_config()).unwrap();
        let concurrent_registry = ConcurrentServiceRegistry::new(registry);
        
        assert_eq!(concurrent_registry.service_count().unwrap(), 6);
        
        let config = concurrent_registry.lookup("relay_service").unwrap();
        assert_eq!(config.sink_type, SinkType::Relay);
    }
    
    #[test]
    fn test_registry_metadata() {
        let registry = ServiceRegistry::from_toml_str(sample_config()).unwrap();
        let metadata = registry.metadata();
        
        assert_eq!(metadata.service_count, 6);
        assert_eq!(metadata.version, 1);
        assert_eq!(metadata.source, "<inline>");
    }
    
    #[test]
    fn test_invalid_references() {
        let invalid_config = r#"
            [services.invalid_service]
            type = "composite"
            pattern = "fanout"
            targets = ["nonexistent_target"]
        "#;
        
        let result = ServiceRegistry::from_toml_str(invalid_config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown target"));
    }
}