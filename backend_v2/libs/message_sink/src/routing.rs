use crate::{Message, SinkError};
use alphapulse_types::common::identifiers::{AssetType, InstrumentId, VenueId};
use std::fmt::Debug;

/// Routing target that can use either InstrumentID or string-based routing
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RoutingTarget {
    /// Route based on bijective instrument identifier (preferred)
    Instrument(InstrumentId),
    /// Route to specific service by name
    Service(String),
    /// Route to specific node by ID
    Node(String),
    /// Broadcast to all targets
    Broadcast,
    /// Route based on message content (will be determined at send time)
    ContentBased,
}

impl RoutingTarget {
    /// Create routing target from instrument ID
    pub fn from_instrument(instrument_id: InstrumentId) -> Self {
        Self::Instrument(instrument_id)
    }

    /// Create routing target for service
    pub fn to_service(service: impl Into<String>) -> Self {
        Self::Service(service.into())
    }

    /// Create routing target for specific node
    pub fn to_node(node_id: impl Into<String>) -> Self {
        Self::Node(node_id.into())
    }

    /// Create broadcast routing target
    pub fn broadcast() -> Self {
        Self::Broadcast
    }

    /// Create content-based routing target
    pub fn content_based() -> Self {
        Self::ContentBased
    }

    /// Check if this is an instrument-based route
    pub fn is_instrument_route(&self) -> bool {
        matches!(self, RoutingTarget::Instrument(_))
    }

    /// Check if this is a service route
    pub fn is_service_route(&self) -> bool {
        matches!(self, RoutingTarget::Service(_))
    }

    /// Check if this is a broadcast route
    pub fn is_broadcast(&self) -> bool {
        matches!(self, RoutingTarget::Broadcast)
    }

    /// Get the target identifier as string for debugging
    pub fn target_string(&self) -> String {
        match self {
            RoutingTarget::Instrument(id) => format!("instrument:{:?}", id),
            RoutingTarget::Service(name) => format!("service:{}", name),
            RoutingTarget::Node(id) => format!("node:{}", id),
            RoutingTarget::Broadcast => "broadcast".to_string(),
            RoutingTarget::ContentBased => "content-based".to_string(),
        }
    }

    /// Extract venue from instrument routing target
    pub fn venue_id(&self) -> Option<VenueId> {
        match self {
            RoutingTarget::Instrument(id) => VenueId::try_from(id.venue).ok(),
            _ => None,
        }
    }

    /// Extract asset type from instrument routing target  
    pub fn asset_type(&self) -> Option<AssetType> {
        match self {
            RoutingTarget::Instrument(id) => AssetType::try_from(id.asset_type).ok(),
            _ => None,
        }
    }
}

impl Default for RoutingTarget {
    fn default() -> Self {
        Self::ContentBased
    }
}

impl From<InstrumentId> for RoutingTarget {
    fn from(instrument_id: InstrumentId) -> Self {
        Self::Instrument(instrument_id)
    }
}

impl From<String> for RoutingTarget {
    fn from(service_name: String) -> Self {
        Self::Service(service_name)
    }
}

impl From<&str> for RoutingTarget {
    fn from(service_name: &str) -> Self {
        Self::Service(service_name.to_string())
    }
}

/// Router that can resolve routing targets to actual destinations
pub trait MessageRouter: Send + Sync + Debug {
    /// Resolve a routing target to concrete destinations
    fn resolve_target(&self, target: &RoutingTarget) -> Result<Vec<String>, SinkError>;

    /// Get preferred routing target for a message
    fn route_message(&self, message: &Message) -> Result<RoutingTarget, SinkError>;

    /// Check if router supports a specific routing target type
    fn supports_target(&self, target: &RoutingTarget) -> bool;

    /// Get routing statistics
    fn routing_stats(&self) -> RoutingStats;
}

/// Statistics for message routing
#[derive(Debug, Clone, Default)]
pub struct RoutingStats {
    /// Total messages routed
    pub messages_routed: u64,
    /// Messages routed by instrument ID
    pub instrument_routes: u64,
    /// Messages routed by service name
    pub service_routes: u64,
    /// Messages routed by node ID
    pub node_routes: u64,
    /// Broadcast messages
    pub broadcast_routes: u64,
    /// Content-based routing decisions
    pub content_based_routes: u64,
    /// Failed routing attempts
    pub routing_failures: u64,
}

impl RoutingStats {
    /// Calculate total successful routes
    pub fn total_successful(&self) -> u64 {
        self.instrument_routes
            + self.service_routes
            + self.node_routes
            + self.broadcast_routes
            + self.content_based_routes
    }

    /// Calculate routing success rate
    pub fn success_rate(&self) -> f64 {
        if self.messages_routed == 0 {
            return 1.0;
        }
        self.total_successful() as f64 / self.messages_routed as f64
    }

    /// Get the most used routing type
    pub fn primary_routing_type(&self) -> &'static str {
        let mut max_count = 0;
        let mut max_type = "none";

        if self.instrument_routes > max_count {
            max_count = self.instrument_routes;
            max_type = "instrument";
        }
        if self.service_routes > max_count {
            max_count = self.service_routes;
            max_type = "service";
        }
        if self.node_routes > max_count {
            max_count = self.node_routes;
            max_type = "node";
        }
        if self.broadcast_routes > max_count {
            max_count = self.broadcast_routes;
            max_type = "broadcast";
        }
        if self.content_based_routes > max_count {
            max_type = "content_based";
        }

        max_type
    }
}

// Using InstrumentId from alphapulse_types::common::identifiers

// Using VenueId and AssetType from alphapulse_types::common::identifiers

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routing_target_creation() {
        let eth_usdc = InstrumentId::stock(VenueId::NYSE, "AAPL");

        let instrument_target = RoutingTarget::from_instrument(eth_usdc);
        assert!(instrument_target.is_instrument_route());
        assert_eq!(instrument_target.venue_id(), Some(VenueId::NYSE)); // venue 1 = NYSE
        assert_eq!(instrument_target.asset_type(), Some(AssetType::Stock)); // asset_type 1 = Stock

        let service_target = RoutingTarget::to_service("execution-relay");
        assert!(service_target.is_service_route());
        assert!(!service_target.is_broadcast());

        let broadcast_target = RoutingTarget::broadcast();
        assert!(broadcast_target.is_broadcast());
    }

    #[test]
    fn test_instrument_id_packing() {
        let original = InstrumentId::stock(VenueId::NASDAQ, "MSFT");
        let packed = original.to_u64();
        let unpacked = InstrumentId::from_u64(packed);

        assert_eq!(original, unpacked);
        // Note: exact field values depend on InstrumentId internal implementation
    }

    #[test]
    fn test_routing_stats() {
        let mut stats = RoutingStats::default();
        stats.messages_routed = 100;
        stats.instrument_routes = 60;
        stats.service_routes = 30;
        stats.routing_failures = 10;

        assert_eq!(stats.total_successful(), 90);
        assert_eq!(stats.success_rate(), 0.9);
        assert_eq!(stats.primary_routing_type(), "instrument");
    }

    #[test]
    fn test_venue_and_asset_type_conversion() {
        assert_eq!(VenueId::try_from(1), Ok(VenueId::NYSE));
        assert_eq!(VenueId::try_from(100), Ok(VenueId::Binance));
        assert!(VenueId::try_from(999).is_err()); // Invalid venue ID

        assert_eq!(AssetType::try_from(1), Ok(AssetType::Stock));
        assert_eq!(AssetType::try_from(50), Ok(AssetType::Token));
        assert!(AssetType::try_from(255).is_err()); // Invalid asset type
    }

    #[test]
    fn test_target_string_representation() {
        let eth_usdc = InstrumentId::stock(VenueId::NYSE, "GOOGL");
        let instrument_target = RoutingTarget::from_instrument(eth_usdc);

        assert!(instrument_target.target_string().starts_with("instrument:"));

        let service_target = RoutingTarget::to_service("market-data");
        assert_eq!(service_target.target_string(), "service:market-data");

        let broadcast_target = RoutingTarget::broadcast();
        assert_eq!(broadcast_target.target_string(), "broadcast");
    }
}
