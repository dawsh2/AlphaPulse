//! TLV Type Definitions
//! 
//! Defines all TLV types organized by relay domain for clean separation

use num_enum::TryFromPrimitive;

/// TLV types organized by relay domain
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
pub enum TLVType {
    // Market Data Domain (1-19) - Routes through MarketDataRelay
    Trade = 1,
    Quote = 2,
    OrderBook = 3,
    InstrumentMeta = 4,
    L2Snapshot = 5,
    L2Delta = 6,
    L2Reset = 7,
    PriceUpdate = 8,
    VolumeUpdate = 9,
    // Reserved 10-19 for future market data types
    
    // Strategy Signal Domain (20-39) - Routes through SignalRelay  
    SignalIdentity = 20,
    AssetCorrelation = 21,
    Economics = 22,
    ExecutionAddresses = 23,
    VenueMetadata = 24,
    StateReference = 25,
    ExecutionControl = 26,
    PoolAddresses = 27,
    MEVBundle = 28,
    TertiaryVenue = 29,
    RiskParameters = 30,
    PerformanceMetrics = 31,
    // Reserved 32-39 for future strategy signal types
    
    // Execution Domain (40-59) - Routes through ExecutionRelay
    OrderRequest = 40,
    OrderStatus = 41,
    Fill = 42,
    OrderCancel = 43,
    OrderModify = 44,
    ExecutionReport = 45,
    Portfolio = 46,
    Position = 47,
    Balance = 48,
    TradeConfirmation = 49,
    // Reserved 50-59 for future execution types
    
    // System Domain (100-109) - Direct connections or SystemRelay
    Heartbeat = 100,
    Snapshot = 101,
    Error = 102,
    ConfigUpdate = 103,
    ServiceDiscovery = 104,
    MetricsReport = 105,
    // Reserved 106-109 for future system types
    
    // Recovery Domain (110-119)
    RecoveryRequest = 110,
    RecoveryResponse = 111,
    SequenceSync = 112,
    // Reserved 113-119 for future recovery types
    
    // Extended TLV marker (255)
    ExtendedTLV = 255,
}

impl TLVType {
    /// Get the relay domain for this TLV type
    pub fn relay_domain(&self) -> crate::RelayDomain {
        match *self as u8 {
            1..=19 => crate::RelayDomain::MarketData,
            20..=39 => crate::RelayDomain::Signal,
            40..=59 => crate::RelayDomain::Execution,
            _ => crate::RelayDomain::MarketData, // System/recovery go to market data relay for now
        }
    }
    
    /// Check if this is a standard TLV type (not extended)
    pub fn is_standard(&self) -> bool {
        *self != TLVType::ExtendedTLV
    }
    
    /// Check if this TLV type is reserved/undefined
    pub fn is_reserved(&self) -> bool {
        match *self as u8 {
            10..=19 | 32..=39 | 50..=59 | 60..=99 | 106..=109 | 113..=119 | 120..=199 | 200..=254 => true,
            _ => false,
        }
    }
    
    /// Get expected payload size for fixed-size TLVs (returns None for variable-size)
    pub fn expected_payload_size(&self) -> Option<usize> {
        match self {
            TLVType::Trade => Some(24),
            TLVType::Quote => Some(32),
            TLVType::SignalIdentity => Some(16),
            TLVType::AssetCorrelation => Some(24),
            TLVType::Economics => Some(32),
            TLVType::ExecutionAddresses => Some(84),
            TLVType::VenueMetadata => Some(12),
            TLVType::StateReference => Some(24),
            TLVType::ExecutionControl => Some(16),
            TLVType::PoolAddresses => Some(44),
            TLVType::MEVBundle => Some(40),
            TLVType::TertiaryVenue => Some(24),
            TLVType::OrderRequest => Some(32),
            TLVType::OrderStatus => Some(24),
            TLVType::Fill => Some(32),
            TLVType::OrderCancel => Some(16),
            TLVType::OrderModify => Some(24),
            TLVType::ExecutionReport => Some(48),
            TLVType::Heartbeat => Some(16),
            TLVType::RecoveryRequest => Some(18),
            // Variable-size TLVs
            _ => None,
        }
    }
}

/// Vendor/Private TLV type range (200-254)
/// These are available for custom extensions and experimental features
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
pub enum VendorTLVType {
    // Example vendor extensions
    CustomMetrics = 200,
    ExperimentalSignal = 201,
    ProprietaryData = 202,
    // Reserved 203-254 for other vendors
}

impl VendorTLVType {
    /// Convert to standard TLV type value
    pub fn as_tlv_type(&self) -> u8 {
        *self as u8
    }
    
    /// Check if a TLV type is in the vendor range
    pub fn is_vendor_type(tlv_type: u8) -> bool {
        (200..=254).contains(&tlv_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tlv_domain_mapping() {
        assert_eq!(TLVType::Trade.relay_domain(), crate::RelayDomain::MarketData);
        assert_eq!(TLVType::SignalIdentity.relay_domain(), crate::RelayDomain::Signal);
        assert_eq!(TLVType::OrderRequest.relay_domain(), crate::RelayDomain::Execution);
    }
    
    #[test]
    fn test_reserved_types() {
        assert!(TLVType::Trade.is_reserved() == false);
        // Note: We can't easily test reserved types since they're not defined as enum variants
        // This would need to be tested with raw u8 values
    }
    
    #[test]
    fn test_expected_sizes() {
        assert_eq!(TLVType::Trade.expected_payload_size(), Some(24));
        assert_eq!(TLVType::Economics.expected_payload_size(), Some(32));
        assert_eq!(TLVType::OrderBook.expected_payload_size(), None); // Variable size
    }
    
    #[test]
    fn test_vendor_types() {
        assert!(VendorTLVType::is_vendor_type(200));
        assert!(VendorTLVType::is_vendor_type(254));
        assert!(!VendorTLVType::is_vendor_type(199));
        assert!(!VendorTLVType::is_vendor_type(255));
        
        assert_eq!(VendorTLVType::CustomMetrics.as_tlv_type(), 200);
    }
}