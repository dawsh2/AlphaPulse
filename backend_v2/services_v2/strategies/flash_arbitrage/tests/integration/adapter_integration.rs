//! Integration tests with adapters service

use alphapulse_flash_arbitrage::pool_state::{PoolState, PoolStateManager};
use protocol_v2::{
    instrument_id::{PoolInstrumentId, VenueId},
    tlv::TLVMessageBuilder,
    SourceType, TLVType,
};
use rust_decimal_macros::dec;
use std::sync::Arc;
use tokio::sync::mpsc;

#[tokio::test]
async fn test_polygon_dex_collector_integration() {
    // Create channel for adapter output
    let (tx, mut rx) = mpsc::channel(100);

    // Create Polygon DEX collector

    // Start collector (currently a placeholder)
    collector.start().await.unwrap();

    // Verify venue
    assert_eq!(collector.venue(), VenueId::Polygon);

    // Check health
    let health = collector.health().await;
    assert!(
        health.level == alphapulse_adapters::input::HealthLevel::Healthy
            || health.level == alphapulse_adapters::input::HealthLevel::Unknown
    );
}

#[tokio::test]
async fn test_adapter_to_pool_state_flow() {
    let pool_manager = Arc::new(PoolStateManager::new());
    let (tx, mut rx) = mpsc::channel(100);

    // Simulate adapter sending pool update
    tokio::spawn(async move {
        let mut builder = TLVMessageBuilder::new(SourceType::PolygonCollector, 123456789);

        // Add simulated swap event data
        let pool_id = PoolInstrumentId {
            tokens: vec![1, 2],
            venue_id: VenueId::QuickSwap as u16,
            pool_type: 2,
        };

        // In real scenario, adapter would send TradeTLV
        // For now, just send the built message
        let message = builder.build().unwrap();
        tx.send(message).await.unwrap();
    });

    // Process messages and update pool state
    if let Some(_message) = rx.recv().await {
        // In real implementation, parse TLV and extract pool data
        let pool_id = PoolInstrumentId {
            tokens: vec![1, 2],
            venue_id: VenueId::QuickSwap as u16,
            pool_type: 2,
        };

        let state = PoolState::V2 {
            pool_id: pool_id.clone(),
            reserves: (dec!(1000), dec!(2000000)),
            fee_tier: 30,
            last_update_ns: 123456789000,
        };

        pool_manager.update_pool(state).unwrap();

        // Verify pool was added
        assert!(pool_manager.get_pool_by_id(&pool_id).is_some());
    }
}

#[tokio::test]
async fn test_multi_venue_adapter_coordination() {
    let pool_manager = Arc::new(PoolStateManager::new());

    // Simulate multiple adapters for different venues
    let venues = vec![
        (VenueId::Uniswap, vec![1, 2]),
        (VenueId::Sushiswap, vec![1, 2]),
        (VenueId::QuickSwap, vec![1, 2]),
    ];

    for (venue, tokens) in venues {
        let pool_id = PoolInstrumentId {
            tokens,
            venue_id: venue as u16,
            pool_type: 2,
        };

        let state = PoolState::V2 {
            pool_id,
            reserves: (dec!(1000), dec!(2000000)),
            fee_tier: 30,
            last_update_ns: 1000000,
        };

        pool_manager.update_pool(state).unwrap();
    }

    // Should have pools from all venues
    assert_eq!(pool_manager.stats().total_pools, 3);

    // Should find arbitrage pairs across venues
    let pairs = pool_manager.find_pools_for_pair(1, 2);
    assert_eq!(pairs.len(), 3);
}

#[tokio::test]
async fn test_adapter_circuit_breaker() {
    use alphapulse_adapters::common::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
    use std::time::Duration;

    let config = CircuitBreakerConfig {
        failure_threshold: 3,
        success_threshold: 2,
        timeout: Duration::from_secs(5),
        half_open_max_calls: 1,
    };

    let breaker = CircuitBreaker::new(config);

    // Simulate failures
    for _ in 0..3 {
        breaker.on_failure();
    }

    // Should be open after threshold
    assert!(!breaker.can_proceed());

    // Wait for timeout
    tokio::time::sleep(Duration::from_secs(6)).await;

    // Should be half-open
    assert!(breaker.can_proceed());

    // Success should start closing
    breaker.on_success();
    breaker.on_success();

    // Should be closed again
    assert!(breaker.can_proceed());
}

#[tokio::test]
async fn test_adapter_rate_limiting() {
    use alphapulse_adapters::common::rate_limit::RateLimiter;
    use std::time::Duration;

    let limiter = RateLimiter::new(5, Duration::from_secs(1)); // 5 per second

    // Should allow initial burst
    for _ in 0..5 {
        assert!(limiter.try_acquire().await);
    }

    // Should block 6th request
    assert!(!limiter.try_acquire().await);

    // Wait for refill
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Should allow again
    assert!(limiter.try_acquire().await);
}

#[tokio::test]
async fn test_adapter_metrics() {
    use alphapulse_adapters::common::AdapterMetrics;

    let metrics = AdapterMetrics::new();

    // Record some events
    metrics.record_message();
    metrics.record_message();
    metrics.record_error();

    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.messages_received, 2);
    assert_eq!(snapshot.errors, 1);
}
