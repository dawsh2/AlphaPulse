//! Configuration Loading and Management
//!
//! Handles YAML configuration loading with validation and templating support

use super::nodes::InterNodeConfig;
use super::validation::TopologyValidator;
use super::{Actor, Node, Result, TopologyError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Complete topology configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyConfig {
    #[serde(default = "default_version")]
    pub version: String,

    pub actors: HashMap<String, Actor>,
    pub nodes: HashMap<String, Node>,
    pub inter_node: Option<InterNodeConfig>,

    #[serde(default)]
    pub metadata: ConfigMetadata,
}

/// Configuration metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConfigMetadata {
    pub name: Option<String>,
    pub description: Option<String>,
    pub environment: Option<String>,
    pub tags: Vec<String>,
    pub created_at: Option<String>,
    pub created_by: Option<String>,
}

fn default_version() -> String {
    crate::TOPOLOGY_VERSION.to_string()
}

impl TopologyConfig {
    /// Load configuration from YAML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref()).map_err(TopologyError::Io)?;

        Self::from_yaml(&content)
    }

    /// Load configuration from YAML string
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        // Apply environment variable substitution
        let expanded_yaml = Self::expand_env_vars(yaml)?;

        let config: TopologyConfig =
            serde_yaml::from_str(&expanded_yaml).map_err(TopologyError::YamlParse)?;

        // Validate configuration
        config.validate()?;

        Ok(config)
    }

    /// Save configuration to YAML file
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let yaml = self.to_yaml()?;
        std::fs::write(path.as_ref(), yaml).map_err(TopologyError::Io)?;
        Ok(())
    }

    /// Convert configuration to YAML string
    pub fn to_yaml(&self) -> Result<String> {
        serde_yaml::to_string(self).map_err(TopologyError::YamlParse)
    }

    /// Validate entire configuration
    pub fn validate(&self) -> Result<()> {
        // Check version compatibility
        if self.version != crate::TOPOLOGY_VERSION {
            return Err(TopologyError::Config {
                message: format!(
                    "Configuration version {} not compatible with expected {}",
                    self.version,
                    crate::TOPOLOGY_VERSION
                ),
            });
        }

        // Validate individual actors
        for (actor_id, actor) in &self.actors {
            actor.validate().map_err(|e| TopologyError::Config {
                message: format!("Actor '{}': {}", actor_id, e),
            })?;
        }

        // Validate individual nodes
        for (node_id, node) in &self.nodes {
            node.validate().map_err(|e| TopologyError::Config {
                message: format!("Node '{}': {}", node_id, e),
            })?;
        }

        // Run comprehensive validation
        let validator = TopologyValidator::new(self);
        let _report = validator.validate_all()?;

        Ok(())
    }

    /// Environment variable substitution
    fn expand_env_vars(yaml: &str) -> Result<String> {
        let mut result = yaml.to_string();

        // Simple regex-based substitution for ${VAR} and ${VAR:default}
        let env_var_regex = regex::Regex::new(r"\$\{([^}:]+)(?::([^}]*))?\}").map_err(|e| {
            TopologyError::Config {
                message: format!("Invalid environment variable pattern: {}", e),
            }
        })?;

        for captures in env_var_regex.captures_iter(yaml) {
            let full_match = captures.get(0).unwrap().as_str();
            let var_name = captures.get(1).unwrap().as_str();
            let default_value = captures.get(2).map(|m| m.as_str()).unwrap_or("");

            let replacement = std::env::var(var_name).unwrap_or_else(|_| default_value.to_string());

            result = result.replace(full_match, &replacement);
        }

        Ok(result)
    }

    /// Merge configurations (for composition)
    pub fn merge(&mut self, other: TopologyConfig) -> Result<()> {
        // Merge actors
        for (actor_id, actor) in other.actors {
            if self.actors.contains_key(&actor_id) {
                return Err(TopologyError::Config {
                    message: format!("Duplicate actor ID during merge: {}", actor_id),
                });
            }
            self.actors.insert(actor_id, actor);
        }

        // Merge nodes
        for (node_id, node) in other.nodes {
            if self.nodes.contains_key(&node_id) {
                return Err(TopologyError::Config {
                    message: format!("Duplicate node ID during merge: {}", node_id),
                });
            }
            self.nodes.insert(node_id, node);
        }

        // Merge inter-node config
        if let Some(other_inter_node) = other.inter_node {
            match &mut self.inter_node {
                Some(existing) => {
                    existing.routes.extend(other_inter_node.routes);
                }
                None => {
                    self.inter_node = Some(other_inter_node);
                }
            }
        }

        // Merge metadata tags
        self.metadata.tags.extend(other.metadata.tags);

        Ok(())
    }

    /// Get all actors of a specific type
    pub fn actors_by_type(&self, actor_type: super::actors::ActorType) -> Vec<(&String, &Actor)> {
        self.actors
            .iter()
            .filter(|(_, actor)| actor.actor_type == actor_type)
            .collect()
    }

    /// Get all nodes with available capacity
    pub fn nodes_with_capacity(
        &self,
        min_cpu: usize,
        min_memory_mb: usize,
    ) -> Vec<(&String, &Node)> {
        self.nodes
            .iter()
            .filter(|(_, node)| node.has_capacity_for(min_cpu, min_memory_mb))
            .collect()
    }

    /// Find channel dependencies between actors
    pub fn find_dependencies(&self) -> HashMap<String, Vec<String>> {
        let mut dependencies = HashMap::new();

        for (actor_id, actor) in &self.actors {
            let mut deps = Vec::new();

            // Find producers of input channels
            for input_channel in &actor.inputs {
                for (producer_id, producer) in &self.actors {
                    if producer.outputs.contains(input_channel) {
                        deps.push(producer_id.clone());
                    }
                }
            }

            dependencies.insert(actor_id.clone(), deps);
        }

        dependencies
    }

    /// Generate deployment summary
    pub fn deployment_summary(&self) -> DeploymentSummary {
        let mut summary = DeploymentSummary {
            total_actors: self.actors.len(),
            total_nodes: self.nodes.len(),
            ..Default::default()
        };

        for actor in self.actors.values() {
            match actor.actor_type {
                super::actors::ActorType::Producer => summary.producer_count += 1,
                super::actors::ActorType::Transformer => summary.transformer_count += 1,
                super::actors::ActorType::Consumer => summary.consumer_count += 1,
            }

            summary.total_memory_mb += actor.resources.min_memory_mb;
            summary.total_cpu_cores += actor.resources.min_cpu_cores;
        }

        // Count channels
        for node in self.nodes.values() {
            summary.total_channels += node.local_channels.len();
        }

        if let Some(inter_node) = &self.inter_node {
            summary.inter_node_routes = inter_node.routes.len();
        }

        summary
    }
}

/// Deployment summary statistics
#[derive(Debug, Default)]
pub struct DeploymentSummary {
    pub total_actors: usize,
    pub total_nodes: usize,
    pub producer_count: usize,
    pub transformer_count: usize,
    pub consumer_count: usize,
    pub total_memory_mb: usize,
    pub total_cpu_cores: usize,
    pub total_channels: usize,
    pub inter_node_routes: usize,
}

/// Configuration template system
pub struct ConfigTemplate {
    pub name: String,
    pub description: String,
    pub template: String,
    pub required_vars: Vec<String>,
    pub optional_vars: Vec<(String, String)>, // (name, default)
}

impl ConfigTemplate {
    /// Render template with variables
    pub fn render(&self, vars: HashMap<String, String>) -> Result<String> {
        let mut result = self.template.clone();

        // Check required variables
        for required_var in &self.required_vars {
            if !vars.contains_key(required_var) {
                return Err(TopologyError::Config {
                    message: format!("Required template variable missing: {}", required_var),
                });
            }
        }

        // Apply variable substitution
        for (var_name, var_value) in vars {
            let pattern = format!("{{{{ {} }}}}", var_name);
            result = result.replace(&pattern, &var_value);
        }

        // Apply optional variable defaults
        for (var_name, default_value) in &self.optional_vars {
            let pattern = format!("{{{{ {} }}}}", var_name);
            if result.contains(&pattern) {
                result = result.replace(&pattern, default_value);
            }
        }

        Ok(result)
    }
}

/// Built-in configuration templates
pub struct ConfigTemplates;

impl ConfigTemplates {
    /// Single node development template
    pub fn single_node_dev() -> ConfigTemplate {
        ConfigTemplate {
            name: "single-node-dev".to_string(),
            description: "Single node development environment".to_string(),
            template: r#"
version: "1.0.0"
metadata:
  name: "{{ deployment_name }}"
  environment: "development"

actors:
  {{ collector_name }}:
    type: producer
    outputs: [market_data]
    source_id: 1
    resources:
      min_memory_mb: 256
      min_cpu_cores: 1

  {{ strategy_name }}:
    type: transformer
    inputs: [market_data]
    outputs: [signals]
    source_id: 20
    resources:
      min_memory_mb: 512
      min_cpu_cores: 2

nodes:
  dev_node:
    hostname: "{{ hostname }}"
    numa_topology: [0]
    local_channels:
      market_data:
        channel_type: SPMC
        buffer_size: 67108864  # 64MB
        numa_node: 0
        huge_pages: false
      signals:
        channel_type: MPSC
        buffer_size: 16777216  # 16MB
        numa_node: 0
        huge_pages: false
    actor_placements:
      {{ collector_name }}:
        numa: 0
        cpu: [0]
      {{ strategy_name }}:
        numa: 0
        cpu: [1]
"#
            .to_string(),
            required_vars: vec![
                "deployment_name".to_string(),
                "collector_name".to_string(),
                "strategy_name".to_string(),
            ],
            optional_vars: vec![("hostname".to_string(), "localhost".to_string())],
        }
    }

    /// Multi-node production template
    pub fn multi_node_production() -> ConfigTemplate {
        ConfigTemplate {
            name: "multi-node-production".to_string(),
            description: "Multi-node production deployment".to_string(),
            template: r#"
version: "1.0.0"
metadata:
  name: "{{ deployment_name }}"
  environment: "production"

actors:
  polygon_collector:
    type: producer
    outputs: [market_data]
    source_id: 1
    resources:
      min_memory_mb: 1024
      min_cpu_cores: 2
    state:
      state_type:
        Persistent:
          storage_backend:
            LocalFile:
              base_path: "/var/lib/alphapulse/polygon_collector"
              sync_writes: true
          consistency_level: Strong

  flash_arbitrage:
    type: transformer
    inputs: [market_data]
    outputs: [arbitrage_signals]
    source_id: 20
    resources:
      min_memory_mb: 2048
      min_cpu_cores: 4

  execution_coordinator:
    type: consumer
    inputs: [arbitrage_signals]
    source_id: 40
    resources:
      min_memory_mb: 512
      min_cpu_cores: 1

nodes:
  data_node:
    hostname: "{{ data_node_hostname }}"
    numa_topology: [0, 1]
    local_channels:
      market_data:
        channel_type: SPMC
        buffer_size: 536870912  # 512MB
        numa_node: 0
        huge_pages: true
    actor_placements:
      polygon_collector:
        numa: 0
        cpu: [0, 1]
        memory_limit_mb: 1024

  strategy_node:
    hostname: "{{ strategy_node_hostname }}"
    numa_topology: [0, 1]
    local_channels:
      arbitrage_signals:
        channel_type: MPSC
        buffer_size: 67108864  # 64MB
        numa_node: 1
        huge_pages: true
    actor_placements:
      flash_arbitrage:
        numa: 0
        cpu: [0, 1, 2, 3]
        memory_limit_mb: 2048
      execution_coordinator:
        numa: 1
        cpu: [4]
        memory_limit_mb: 512

inter_node:
  routes:
    - source_node: "data_node"
      target_node: "strategy_node"
      channels: ["market_data"]
      bandwidth_limit_mbps: 1000
      latency_target_ms: 1.0
"#
            .to_string(),
            required_vars: vec![
                "deployment_name".to_string(),
                "data_node_hostname".to_string(),
                "strategy_node_hostname".to_string(),
            ],
            optional_vars: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_var_expansion() {
        std::env::set_var("TEST_VAR", "test_value");
        std::env::set_var("PORT", "8080");

        let yaml = r#"
hostname: "${TEST_VAR}"
port: ${PORT}
default_value: "${MISSING_VAR:default}"
"#;

        let expanded = TopologyConfig::expand_env_vars(yaml).unwrap();

        assert!(expanded.contains("hostname: \"test_value\""));
        assert!(expanded.contains("port: 8080"));
        assert!(expanded.contains("default_value: \"default\""));
    }

    #[test]
    fn test_template_rendering() {
        let template = ConfigTemplates::single_node_dev();

        let mut vars = HashMap::new();
        vars.insert("deployment_name".to_string(), "test-deployment".to_string());
        vars.insert(
            "collector_name".to_string(),
            "polygon_collector".to_string(),
        );
        vars.insert("strategy_name".to_string(), "flash_arbitrage".to_string());

        let rendered = template.render(vars).unwrap();

        assert!(rendered.contains("name: \"test-deployment\""));
        assert!(rendered.contains("polygon_collector:"));
        assert!(rendered.contains("flash_arbitrage:"));
        assert!(rendered.contains("hostname: \"localhost\"")); // Default value
    }
}
