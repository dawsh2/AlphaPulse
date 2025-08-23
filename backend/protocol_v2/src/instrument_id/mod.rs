//! Bijective Instrument ID System
//! 
//! This module provides self-describing instrument identifiers that are:
//! - Bijective: Can be reversed to extract venue, asset type, and details
//! - Deterministic: Same input always produces the same ID
//! - Collision-free: Construction methods prevent conflicts
//! - Cache-friendly: Converts to u64/u128 for O(1) lookups

pub mod core;
pub mod venues;
pub mod pairing;

pub use core::*;
pub use venues::*;
pub use pairing::*;