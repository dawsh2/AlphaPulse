//! Integration tests for network crate consolidation
//! 
//! Validates that the consolidation of topology and transport into the network
//! crate works correctly and preserves all original functionality.

use network::{
    // Transport types
    TransportConfig, TransportMode, ProtocolType, CompressionType,
    // Topology types  
    TopologyConfig, TopologyResolver, Actor, ActorType, Node,
    // Error types  
    TransportError, TopologyError,
};
// Import precision from correct location
use torq_types::precision::{TokenAmount, ExchangePrice, validate_precision};
use std::collections::HashMap;

#[tokio::test]
async fn test_basic_imports_work() {
    // Test that all major types can be imported and created
    let _config = TransportConfig {
        mode: TransportMode::Direct,
        protocol: Some(ProtocolType::Tcp),
        compression: CompressionType::None,
        encryption: network::EncryptionType::None,
        priority: network::Priority::Normal,
        criticality: network::Criticality::Standard,
        reliability: network::Reliability::BestEffort,
        max_message_size: 1024 * 1024,
        connection_timeout_secs: 10,
    };

    let topology_config = TopologyConfig {
        version: "1.0.0".to_string(),
        actors: HashMap::new(),
        nodes: HashMap::new(),
        inter_node: None,
    };

    assert_eq!(topology_config.version, "1.0.0");
}

// DISABLED: Protocol validation logic belongs in codec/types crates, not network layer
// Network layer should only handle transport, not validate business logic
/* 
#[test]
fn test_protocol_v2_validation() {
    // This test moved to torq-codec crate where validation belongs
}
*/

#[test]
fn test_precision_handling() {
    // Test DEX token precision
    let weth = TokenAmount::new_weth(1_500_000_000_000_000_000); // 1.5 WETH
    let usdc = TokenAmount::new_usdc(2_000_000); // 2.0 USDC
    
    assert_eq!(weth.decimals, 18);
    assert_eq!(usdc.decimals, 6);
    assert!(weth.validate_precision().is_ok());
    assert!(usdc.validate_precision().is_ok());
    
    // Test traditional exchange precision
    let btc_price = ExchangePrice::from_usd(4_500_000_000_000); // $45,000.00
    assert!(btc_price.validate_precision().is_ok());
    
    // Test cross-validation
    assert!(validate_precision(&weth, &btc_price).is_ok());
    
    // Test display formatting preserves precision info
    let weth_display = weth.to_display_string();
    assert!(weth_display.contains("1.500000000000000000"));
    assert!(weth_display.contains("WETH"));
}

#[test]
fn test_error_integration() {
    // Test that topology errors convert properly to transport errors
    let topology_error = TopologyError::ActorNotFound {
        actor: "test_actor".to_string(),
    };
    
    let transport_error: TransportError = topology_error.into();
    assert!(transport_error.to_string().contains("Actor 'test_actor' not found"));
    assert_eq!(transport_error.category(), "topology");
    
    // Test not implemented errors
    let not_impl = TransportError::not_implemented("test feature", "future version");
    assert!(not_impl.to_string().contains("test feature"));
    assert!(!not_impl.is_retryable());
    
    // Test precision errors
    let precision_err = TransportError::precision("Invalid token decimals");
    assert!(precision_err.to_string().contains("Invalid token decimals"));
    assert_eq!(precision_err.category(), "precision");
}

#[tokio::test]
async fn test_topology_resolver_integration() {
    let resolver = TopologyResolver::new(HashMap::new());
    
    // Test that resolver integrates properly with transport layer
    // This tests that the consolidation didn't break the API
    
    let result = resolver.resolve_actor_node("test_actor");
    assert!(result.is_err()); // Should fail for unknown actor
    
    // Test error propagation
    match result {
        Err(e) => {
            assert!(e.to_string().contains("not found"));
        }
        Ok(_) => panic!("Should have failed"),
    }
}

#[test]
fn test_channel_config_disambiguation() {
    // Test that the ChannelConfig name collision has been resolved
    use network::{TransportChannelConfig, TopologyChannelConfig, ChannelConfig};
    
    // Default ChannelConfig should be the transport version for backward compatibility
    let _transport_channel: ChannelConfig = Default::default();
    let _explicit_transport: TransportChannelConfig = Default::default();
    
    // Topology channel config should be available with explicit naming
    let _topology_channel = TopologyChannelConfig {
        channel_name: "test_channel".to_string(),
        actor_inputs: vec!["test_input".to_string()],
        actor_outputs: vec!["test_output".to_string()],
        is_bidirectional: false,
        max_queue_depth: Some(1000),
        criticality_level: Some(network::topology::nodes::CriticalityLevel::Standard),
        security_requirements: None,
    };
    
    assert_eq!(_topology_channel.channel_name, "test_channel");
}

#[test]
fn test_feature_flag_integration() {
    // Test that Protocol V2 integration works when feature is enabled
    #[cfg(feature = "protocol-integration")]
    {
        let validator = ProtocolV2Validator::new();
        assert_eq!(validator.domain_name(1), Some("MarketData"));
    }
    
    // Test that all standard features are available
    let _compression = CompressionType::Lz4;
    let _encryption = network::EncryptionType::Tls;
    let _transport_mode = TransportMode::Direct;
}

#[test]
fn test_timestamp_precision_validation() {
    use torq_types::precision::validate_timestamp_precision;
    
    let current_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    
    // Valid nanosecond timestamp
    assert!(validate_timestamp_precision(current_ns).is_ok());
    
    // Invalid: looks like microseconds
    let microsecond_ts = current_ns / 1000;
    assert!(validate_timestamp_precision(microsecond_ts).is_err());
    
    // Invalid: looks like milliseconds
    let millisecond_ts = current_ns / 1_000_000;
    assert!(validate_timestamp_precision(millisecond_ts).is_err());
}

#[test]
fn test_consolidated_exports() {
    // Test that all major types are available from the root module
    use network::*;
    
    // Transport types should be available
    let _transport_config = TransportConfig {
        mode: TransportMode::Auto,
        protocol: None,
        compression: CompressionType::None,
        encryption: EncryptionType::None,
        priority: Priority::Normal,
        criticality: Criticality::Standard,
        reliability: Reliability::BestEffort,
        max_message_size: 1024,
        connection_timeout_secs: 10,
    };
    
    // Topology types should be available
    let _actor = Actor {
        id: "test".to_string(),
        actor_type: ActorType::Producer,
        inputs: vec![],
        outputs: vec![],
        source_id: 1,
        state: ActorState::Inactive,
        persistence: ActorPersistence::None,
        resources: network::topology::actors::ResourceRequirements {
            min_memory_mb: 100,
            max_memory_mb: 500,
            min_cpu_cores: 1,
            max_cpu_cores: 2,
            cpu_affinity: None,
            numa_node: None,
            priority: network::topology::actors::ProcessPriority::Normal,
        },
        health_check: network::topology::actors::HealthCheckConfig {
            enabled: true,
            interval_seconds: 30,
            timeout_seconds: 5,
            max_failures: 3,
            endpoint: None,
        },
        created_at: std::time::SystemTime::now(),
        updated_at: std::time::SystemTime::now(),
    };
    
    // Protocol V2 types should be available
    let _validator = ProtocolV2Validator::new();
    
    // Precision types should be available
    let _token = TokenAmount::new_weth(1000);
    let _price = ExchangePrice::from_usd(1000);
}

#[tokio::test]  
async fn test_backward_compatibility() {
    // Test that existing code patterns still work after consolidation
    
    // Pattern 1: Transport configuration
    let transport_config = TransportConfig::default();
    assert_eq!(transport_config.mode, TransportMode::Auto);
    
    // Pattern 2: Topology configuration  
    let topology_config = TopologyConfig {
        version: "1.0.0".to_string(),
        actors: HashMap::new(),
        nodes: HashMap::new(),
        inter_node: None,
    };
    assert!(!topology_config.actors.is_empty() == false); // Should be empty
    
    // Pattern 3: Error handling
    let error = TransportError::network("Test error");
    assert!(error.is_retryable());
    assert_eq!(error.category(), "network");
}

#[test]
fn test_performance_constants() {
    // Test that performance constants are maintained
    assert_eq!(network::MAX_MESSAGE_SIZE, 16 * 1024 * 1024);
    assert_eq!(network::DEFAULT_CONNECTION_POOL_SIZE, 4);
    assert_eq!(network::DEFAULT_TCP_BUFFER_SIZE, 64 * 1024);
    assert_eq!(network::DEFAULT_UDP_BUFFER_SIZE, 8 * 1024);
    
    // Test topology constants
    assert_eq!(network::TOPOLOGY_VERSION, "1.0.0");
    assert_eq!(network::MAX_ACTORS_PER_NODE, 64);
    assert_eq!(network::MAX_CPU_CORES_PER_ACTOR, 16);
    
    // Test transport version
    assert_eq!(network::TRANSPORT_VERSION, "0.1.0");
}

#[test]
fn test_no_floating_point_validation() {
    use network::validate_no_float_in_price;
    
    // Valid code (no floating point)
    assert!(validate_no_float_in_price("let price = 100i64;"));
    assert!(validate_no_float_in_price("let amount = TokenAmount::new_weth(1000);"));
    assert!(validate_no_float_in_price("let result = price * quantity;"));
    
    // Invalid code (uses floating point)
    assert!(!validate_no_float_in_price("let price = 100.0f64;"));
    assert!(!validate_no_float_in_price("let ratio: f32 = 0.5;"));
    assert!(!validate_no_float_in_price("let result = price * 1.5;"));
    assert!(!validate_no_float_in_price("100.0 as float"));
}

#[test]
fn test_consolidation_file_structure() {
    // This test validates that the consolidation worked by checking
    // that types from different original crates are now accessible
    // from the same unified crate
    
    // Previously from torq-transport
    let _transport = TransportMode::Direct;
    let _protocol = ProtocolType::Tcp;
    let _compression = CompressionType::Lz4;
    
    // Previously from torq-topology  
    let _actor_type = ActorType::Producer;
    let _topology_config = TopologyConfig {
        version: "1.0.0".to_string(),
        actors: HashMap::new(),
        nodes: HashMap::new(),
        inter_node: None,
    };
    
    // Previously from torq-network (now the unified crate)
    let _network_transport = network::NetworkConfig {
        node_id: "test".to_string(),
        protocol: network::NetworkProtocol {
            protocol_type: ProtocolType::Tcp,
            listen_addr: "127.0.0.1:8080".parse().unwrap(),
            options: network::ProtocolOptions {
                tcp: Some(network::TcpOptions {
                    nodelay: true,
                    keepalive: true,
                    recv_buffer_size: Some(64 * 1024),
                    send_buffer_size: Some(64 * 1024),
                    backlog: Some(128),
                }),
                udp: None,
                #[cfg(feature = "quic")]
                quic: None,
            },
        },
        compression: CompressionType::None,
        encryption: network::EncryptionType::None,
        connection: network::ConnectionConfig {
            max_connections_per_node: 4,
            connect_timeout: std::time::Duration::from_secs(10),
            idle_timeout: std::time::Duration::from_secs(300),
            heartbeat_interval: std::time::Duration::from_secs(30),
            max_reconnect_attempts: 5,
            backoff_strategy: network::BackoffStrategy::Exponential {
                initial_delay: std::time::Duration::from_millis(100),
                multiplier: 2.0,
                max_delay: std::time::Duration::from_secs(60),
            },
        },
        performance: network::PerformanceConfig {
            batching: None,
            send_queue_size: 1000,
            recv_queue_size: 1000,
            worker_threads: None,
            zero_copy: true,
        },
    };
    
    // All types should be accessible without import issues
    assert_eq!(_network_transport.node_id, "test");
}