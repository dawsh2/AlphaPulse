//! Configuration validation tests

use alphapulse_topology::{TopologyConfig, TopologyError};

#[test]
fn test_load_single_node_config() {
    let config = TopologyConfig::from_file("examples/single_node.yaml").unwrap();

    assert_eq!(config.version, "1.0.0");
    assert_eq!(config.actors.len(), 2);
    assert_eq!(config.nodes.len(), 1);
    assert!(config.inter_node.is_none());

    // Validate specific actors
    assert!(config.actors.contains_key("polygon_collector"));
    assert!(config.actors.contains_key("flash_arbitrage"));

    // Validate node
    assert!(config.nodes.contains_key("dev_node"));
    let dev_node = &config.nodes["dev_node"];
    assert_eq!(dev_node.hostname, "localhost");
    assert_eq!(dev_node.numa_topology, vec![0]);
}

#[test]
fn test_load_multi_node_config() {
    let config = TopologyConfig::from_file("examples/multi_node.yaml").unwrap();

    assert_eq!(config.version, "1.0.0");
    assert_eq!(config.actors.len(), 5);
    assert_eq!(config.nodes.len(), 3);
    assert!(config.inter_node.is_some());

    // Validate inter-node configuration
    let inter_node = config.inter_node.as_ref().unwrap();
    assert_eq!(inter_node.routes.len(), 2);

    // Validate high-performance configuration
    let data_node = &config.nodes["data_node_01"];
    assert_eq!(data_node.numa_topology, vec![0, 1]);

    // Check NUMA-aware channel placement
    let market_data_channel = &data_node.local_channels["market_data"];
    assert_eq!(market_data_channel.numa_node, Some(0));
    assert!(market_data_channel.huge_pages);
}

#[test]
fn test_config_validation_errors() {
    // Test invalid configuration
    let invalid_yaml = r#"
version: "1.0.0"
actors:
  invalid_actor:
    actor_type: "Producer"
    inputs: ["should_be_empty_for_producer"]  # Invalid!
    outputs: []  # Invalid!
    source_id: 1
nodes:
  test_node:
    hostname: ""  # Invalid empty hostname
"#;

    let result = TopologyConfig::from_yaml(invalid_yaml);
    assert!(result.is_err());
}

#[test]
fn test_environment_variable_substitution() {
    std::env::set_var("TEST_HOSTNAME", "test.example.com");
    std::env::set_var("TEST_PORT", "9090");

    let yaml_with_env = r#"
version: "1.0.0"
actors:
  test_actor:
    actor_type: "Producer"
    outputs: ["test_channel"]
    source_id: 1
nodes:
  test_node:
    hostname: "${TEST_HOSTNAME}"
    local_channels:
      test_channel:
        channel_type: "SPMC"
        buffer_size: 1024
"#;

    let config = TopologyConfig::from_yaml(yaml_with_env).unwrap();
    assert_eq!(config.nodes["test_node"].hostname, "test.example.com");
}

#[test]
fn test_deployment_summary() {
    let config = TopologyConfig::from_file("examples/single_node.yaml").unwrap();
    let summary = config.deployment_summary();

    assert_eq!(summary.total_actors, 2);
    assert_eq!(summary.total_nodes, 1);
    assert_eq!(summary.producer_count, 1);
    assert_eq!(summary.transformer_count, 1);
    assert_eq!(summary.consumer_count, 0);
    assert_eq!(summary.total_channels, 2);
    assert_eq!(summary.inter_node_routes, 0);
}

#[test]
fn test_config_merge() {
    let mut base_config = TopologyConfig::from_yaml(
        r#"
version: "1.0.0"
actors:
  actor1:
    actor_type: "Producer"
    outputs: ["channel1"]
    source_id: 1
nodes:
  node1:
    hostname: "node1.local"
"#,
    )
    .unwrap();

    let additional_config = TopologyConfig::from_yaml(
        r#"
version: "1.0.0"
actors:
  actor2:
    actor_type: "Consumer"
    inputs: ["channel1"]
    source_id: 2
nodes:
  node2:
    hostname: "node2.local"
"#,
    )
    .unwrap();

    base_config.merge(additional_config).unwrap();

    assert_eq!(base_config.actors.len(), 2);
    assert_eq!(base_config.nodes.len(), 2);
    assert!(base_config.actors.contains_key("actor1"));
    assert!(base_config.actors.contains_key("actor2"));
}

#[test]
fn test_dependency_analysis() {
    let config = TopologyConfig::from_file("examples/single_node.yaml").unwrap();
    let dependencies = config.find_dependencies();

    // flash_arbitrage depends on polygon_collector (via market_data channel)
    assert!(dependencies.contains_key("flash_arbitrage"));
    let flash_deps = &dependencies["flash_arbitrage"];
    assert!(flash_deps.contains(&"polygon_collector".to_string()));

    // polygon_collector has no dependencies
    assert!(dependencies.contains_key("polygon_collector"));
    let polygon_deps = &dependencies["polygon_collector"];
    assert!(polygon_deps.is_empty());
}
