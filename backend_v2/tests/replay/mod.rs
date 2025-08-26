//! Live Market Replay Infrastructure
//!
//! Infrastructure for capturing and replaying real market data for testing.
//! This enables deterministic testing with real market conditions and
//! historical scenario replay.
//!
//! ## Components
//! - `capture/` - Live data capture utilities
//! - `replay/` - Market data replay engine
//! - `scenarios/` - Predefined test scenarios
//! - `validation/` - Replay accuracy validation

pub mod capture;
pub mod replay;
pub mod scenarios;
pub mod validation;