//! State management and invalidation for input adapters
//!
//! Ensures state consistency by immediately invalidating tracked instruments
//! on disconnection to prevent phantom arbitrage opportunities.

use alphapulse_types::{
    tlv::{self, StateInvalidationTLV},
    InstrumentId, InvalidationReason, RelayDomain, SourceType, TLVType, VenueId,
};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;

use crate::input::ConnectionState;
use crate::AdapterMetrics;
use crate::{AdapterError, Result};

/// State manager for tracking instruments and handling invalidations
pub struct StateManager {
    venue: VenueId,

    /// Tracked instruments per venue
    instruments: Arc<RwLock<HashMap<VenueId, HashSet<InstrumentId>>>>,

    /// Last state sequence number for each venue
    sequence_numbers: Arc<RwLock<HashMap<VenueId, u64>>>,

    /// Current connection state
    connection_state: Arc<RwLock<ConnectionState>>,

    /// Metrics for monitoring
    metrics: Arc<AdapterMetrics>,

    /// Maximum time before automatic invalidation
    max_stale_duration: Duration,
}

impl Default for StateManager {
    fn default() -> Self {
        Self::new()
    }
}

impl StateManager {
    /// Create a new state manager
    pub fn new() -> Self {
        let metrics = Arc::new(AdapterMetrics::new());
        Self::with_venue_and_metrics(VenueId::Coinbase, metrics)
    }

    /// Create a new state manager with specific venue and metrics
    pub fn with_venue_and_metrics(venue: VenueId, metrics: Arc<AdapterMetrics>) -> Self {
        Self {
            venue,
            instruments: Arc::new(RwLock::new(HashMap::new())),
            sequence_numbers: Arc::new(RwLock::new(HashMap::new())),
            connection_state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            metrics,
            max_stale_duration: Duration::from_millis(100), // <100ms invalidation requirement
        }
    }

    /// Set connection state
    pub async fn set_connection_state(&self, state: ConnectionState) {
        *self.connection_state.write().await = state;

        // Update metrics based on connection state
        match state {
            ConnectionState::Connected => {
                self.metrics
                    .active_connections
                    .fetch_add(1, Ordering::Relaxed);
            }
            ConnectionState::Disconnected | ConnectionState::Failed => {
                // When disconnected, ensure we have at least 1 to subtract
                let current = self.metrics.active_connections.load(Ordering::Relaxed);
                if current > 0 {
                    self.metrics
                        .active_connections
                        .fetch_sub(1, Ordering::Relaxed);
                }
            }
            _ => {} // Connecting state doesn't change metrics
        }
    }

    /// Get current connection state
    pub async fn connection_state(&self) -> ConnectionState {
        *self.connection_state.read().await
    }

    /// Set connection state for tracking (legacy method)
    pub fn set_connected(&self, connected: bool) {
        if connected {
            self.metrics
                .active_connections
                .fetch_add(1, Ordering::Relaxed);
        } else {
            // When disconnected, ensure we have at least 1 to subtract
            let current = self.metrics.active_connections.load(Ordering::Relaxed);
            if current > 0 {
                self.metrics
                    .active_connections
                    .fetch_sub(1, Ordering::Relaxed);
            }
        }
    }

    /// Check if connection is active (simplified for compatibility)
    pub fn is_connected(&self) -> bool {
        self.metrics.active_connections.load(Ordering::Relaxed) > 0
    }

    /// Track a new instrument
    pub async fn track_instrument(&self, instrument: InstrumentId) {
        let mut instruments = self.instruments.write().await;
        instruments
            .entry(self.venue)
            .or_insert_with(HashSet::new)
            .insert(instrument);

        let count = instruments.get(&self.venue).map(|s| s.len()).unwrap_or(0);
        self.metrics.update_instrument_count(self.venue, count);

        tracing::debug!(
            "Tracking instrument {:?} for venue {:?} (total: {})",
            instrument,
            self.venue,
            count
        );
    }

    /// Stop tracking an instrument
    pub async fn untrack_instrument(&self, instrument: InstrumentId) {
        let mut instruments = self.instruments.write().await;
        if let Some(venue_instruments) = instruments.get_mut(&self.venue) {
            venue_instruments.remove(&instrument);
            let count = venue_instruments.len();
            self.metrics.update_instrument_count(self.venue, count);

            tracing::debug!(
                "Untracked instrument {:?} for venue {:?} (remaining: {})",
                instrument,
                self.venue,
                count
            );
        }
    }

    /// Get all tracked instruments for the venue
    pub async fn get_tracked_instruments(&self) -> Vec<InstrumentId> {
        self.instruments
            .read()
            .await
            .get(&self.venue)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Generate state invalidation message for disconnection
    pub async fn generate_invalidation(&self) -> Result<Vec<u8>> {
        let instruments = self.get_tracked_instruments().await;
        let instrument_count = instruments.len();

        if instrument_count == 0 {
            return Err(AdapterError::Internal(
                "No instruments to invalidate".to_string(),
            ));
        }

        // Increment sequence number
        let sequence = {
            let mut sequences = self.sequence_numbers.write().await;
            let seq = sequences.entry(self.venue).or_insert(0);
            *seq += 1;
            *seq
        };

        // Create invalidation TLV using constructor
        let invalidation = StateInvalidationTLV::new(
            self.venue,
            sequence,
            &instruments,
            InvalidationReason::Disconnection,
            current_nanos(),
        )
        .expect("Failed to create StateInvalidationTLV");

        // Convert to TLV message
        let tlv_message = tlv::build_message_direct(
            RelayDomain::MarketData,
            SourceType::StateManager,
            TLVType::StateInvalidation,
            &invalidation,
        )
        .map_err(|e| AdapterError::Internal(format!("TLV message build failed: {}", e)))?;

        tracing::warn!(
            "Generated state invalidation for {} instruments on venue {:?} (seq: {})",
            instrument_count,
            self.venue,
            sequence
        );

        self.metrics
            .record_state_invalidation(self.venue, instrument_count);

        Ok(tlv_message)
    }

    /// Clear all tracked state (called after invalidation)
    pub async fn clear_state(&self) {
        let mut instruments = self.instruments.write().await;
        let count = instruments.get(&self.venue).map(|s| s.len()).unwrap_or(0);

        instruments.remove(&self.venue);
        self.metrics.update_instrument_count(self.venue, 0);

        tracing::info!("Cleared {} instruments for venue {:?}", count, self.venue);
    }

    /// Handle venue reconnection - generate recovery message
    pub async fn handle_reconnection(&self) -> Result<Vec<u8>> {
        // Clear sequence to indicate fresh start
        self.sequence_numbers.write().await.insert(self.venue, 0);

        // Create recovery TLV using constructor
        let recovery = StateInvalidationTLV::new(
            self.venue,
            0,   // 0 indicates recovery/fresh start
            &[], // Empty instruments slice for recovery
            InvalidationReason::Recovery,
            current_nanos(),
        )
        .expect("Failed to create recovery StateInvalidationTLV");

        let tlv_message = tlv::build_message_direct(
            RelayDomain::MarketData,
            SourceType::StateManager,
            TLVType::StateInvalidation,
            &recovery,
        )
        .map_err(|e| AdapterError::Internal(format!("TLV message build failed: {}", e)))?;

        tracing::info!("Generated recovery message for venue {:?}", self.venue);

        Ok(tlv_message)
    }

    /// Check if any instruments need invalidation due to staleness
    pub async fn check_staleness(&self, last_message_time: u64) -> bool {
        let age = Duration::from_nanos(current_nanos() - last_message_time);

        if age > self.max_stale_duration {
            let count = self
                .instruments
                .read()
                .await
                .get(&self.venue)
                .map(|s| s.len())
                .unwrap_or(0);

            if count > 0 {
                tracing::warn!(
                    "Stale data detected for {} instruments on venue {:?} (age: {}ms)",
                    count,
                    self.venue,
                    age.as_millis()
                );
                return true;
            }
        }

        false
    }

    /// Start periodic staleness checker
    pub fn start_staleness_monitor(
        self: Arc<Self>,
        last_message_time: Arc<RwLock<u64>>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_millis(50)); // Check every 50ms

            loop {
                ticker.tick().await;

                let last_time = *last_message_time.read().await;
                if self.check_staleness(last_time).await {
                    // Trigger invalidation through the connection manager
                    tracing::error!(
                        "Staleness detected - connection manager should handle invalidation"
                    );
                }
            }
        })
    }
}

// InvalidationReason is now imported from the protocol

// Use the protocol's built-in TLV conversion methods

/// Get current time in nanoseconds since epoch
fn current_nanos() -> u64 {
    alphapulse_network::time::safe_system_timestamp_ns()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_instrument_tracking() {
        let metrics = Arc::new(AdapterMetrics::new());
        let manager = StateManager::with_venue_and_metrics(VenueId::Binance, metrics);

        // Track some instruments
        manager.track_instrument(InstrumentId::from_u64(1001)).await;
        manager.track_instrument(InstrumentId::from_u64(1002)).await;
        manager.track_instrument(InstrumentId::from_u64(1003)).await;

        let tracked = manager.get_tracked_instruments().await;
        assert_eq!(tracked.len(), 3);

        // Untrack one
        manager
            .untrack_instrument(InstrumentId::from_u64(1002))
            .await;

        let tracked = manager.get_tracked_instruments().await;
        assert_eq!(tracked.len(), 2);
    }

    #[tokio::test]
    async fn test_invalidation_generation() {
        let metrics = Arc::new(AdapterMetrics::new());
        let manager = StateManager::with_venue_and_metrics(VenueId::Polygon, metrics);

        // Track instruments
        manager.track_instrument(InstrumentId::from_u64(2001)).await;
        manager.track_instrument(InstrumentId::from_u64(2002)).await;

        // Generate invalidation
        let tlv_bytes = manager.generate_invalidation().await.unwrap();

        // The returned bytes are a complete Protocol V2 message with 32-byte header + TLV payload
        assert!(
            tlv_bytes.len() >= 32,
            "Message should have at least 32-byte header"
        );

        // Check magic number (first 4 bytes should be 0xDEADBEEF)
        assert_eq!(&tlv_bytes[0..4], &[0xDE, 0xAD, 0xBE, 0xEF]);

        // Verify sequence increments
        let tlv_bytes2 = manager.generate_invalidation().await.unwrap();
        // Sequence is at bytes 16-24 in the header
        assert_ne!(
            &tlv_bytes[16..24],
            &tlv_bytes2[16..24],
            "Sequence should increment"
        );
    }

    #[tokio::test]
    async fn test_state_clearing() {
        let metrics = Arc::new(AdapterMetrics::new());
        let manager = StateManager::with_venue_and_metrics(VenueId::Coinbase, metrics);

        // Track instruments
        manager.track_instrument(InstrumentId::from_u64(3001)).await;
        manager.track_instrument(InstrumentId::from_u64(3002)).await;

        assert_eq!(manager.get_tracked_instruments().await.len(), 2);

        // Clear state
        manager.clear_state().await;

        assert_eq!(manager.get_tracked_instruments().await.len(), 0);
    }

    #[tokio::test]
    async fn test_staleness_detection() {
        let metrics = Arc::new(AdapterMetrics::new());
        let manager = StateManager::with_venue_and_metrics(VenueId::Binance, metrics);

        manager.track_instrument(InstrumentId::from_u64(4001)).await;

        // Fresh timestamp - should not be stale
        let fresh_time = current_nanos();
        assert!(!manager.check_staleness(fresh_time).await);

        // Old timestamp - should be stale
        let stale_time = current_nanos() - 200_000_000; // 200ms ago
        assert!(manager.check_staleness(stale_time).await);
    }
}
