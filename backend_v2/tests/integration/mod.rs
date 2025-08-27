//! Integration Tests - Layer 2 of Testing Pyramid
//!
//! Tests component interaction and public API validation.
//! These tests validate that services work together correctly
//! but don't test the full end-to-end pipeline.
//!
//! ## Structure
//! - `adapters/` - Adapter plugin architecture and exchange integration tests
//! - `relay_communication/` - Tests for relay message passing
//! - `service_integration/` - Service boundary validation
//! - `protocol_compliance/` - Protocol conformance across services
//! - `real_data_processing/` - Real exchange data processing validation

pub mod adapters;
pub mod relay_communication;
pub mod service_integration;
pub mod protocol_compliance;
pub mod real_data_processing;