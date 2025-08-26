//! E2E Golden Path Tests - Layer 3 of Testing Pyramid
//!
//! End-to-end tests that validate the complete system pipeline.
//! These tests would catch the "$150 hardcoded profit" bug by using
//! real market data and verifying calculated results.
//!
//! ## Critical Test Cases
//! - Full arbitrage detection pipeline
//! - Real exchange data processing
//! - Profit calculations with varying market conditions
//! - Risk management integration
//! - Dashboard reporting accuracy

pub mod arbitrage_golden_path;
pub mod market_data_pipeline;
pub mod risk_integration;
pub mod dashboard_integration;