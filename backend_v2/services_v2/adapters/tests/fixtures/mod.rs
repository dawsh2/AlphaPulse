//! Real Exchange Data Fixtures
//!
//! Contains actual data from exchanges for validation testing.
//! NO MOCK DATA - only real provider responses.

use serde_json;
use web3::types::{Log, H160, H256, U256, U64};

pub mod kraken;
pub mod polygon;

/// Load fixture data from JSON files
pub fn load_fixture(path: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let fixture_path = format!("{}/{}", crate::config::FIXTURE_PATH, path);
    std::fs::read(&fixture_path)
        .map_err(|e| format!("Failed to load fixture {}: {}", fixture_path, e).into())
}

/// Parse JSON fixture into structured data
pub fn parse_json_fixture<T>(path: &str) -> Result<T, Box<dyn std::error::Error>>
where
    T: serde::de::DeserializeOwned,
{
    let data = load_fixture(path)?;
    serde_json::from_slice(&data)
        .map_err(|e| format!("Failed to parse JSON fixture {}: {}", path, e).into())
}
