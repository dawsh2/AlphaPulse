//! Bundle construction for MEV transactions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Transaction within a bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleTransaction {
    pub to: String,
    pub data: String,
    pub value: String,
    pub gas_limit: u64,
    pub max_fee_per_gas: u64,
    pub max_priority_fee_per_gas: u64,
}

/// Bundle of transactions for atomic execution
#[derive(Debug, Clone)]
pub struct Bundle {
    pub transactions: Vec<BundleTransaction>,
    pub target_block: u64,
    pub max_timestamp: Option<u64>,
    pub min_timestamp: Option<u64>,
    pub reverting_hashes: Vec<String>,
    pub metadata: HashMap<String, String>,
}

/// Builder for constructing MEV bundles
pub struct BundleBuilder {
    bundle: Bundle,
}

impl BundleBuilder {
    pub fn new(target_block: u64) -> Self {
        Self {
            bundle: Bundle {
                transactions: Vec::new(),
                target_block,
                max_timestamp: None,
                min_timestamp: None,
                reverting_hashes: Vec::new(),
                metadata: HashMap::new(),
            },
        }
    }

    pub fn add_transaction(mut self, tx: BundleTransaction) -> Self {
        self.bundle.transactions.push(tx);
        self
    }

    pub fn set_timestamp_range(mut self, min: Option<u64>, max: Option<u64>) -> Self {
        self.bundle.min_timestamp = min;
        self.bundle.max_timestamp = max;
        self
    }

    pub fn allow_reverting(mut self, tx_hash: String) -> Self {
        self.bundle.reverting_hashes.push(tx_hash);
        self
    }

    pub fn add_metadata(mut self, key: String, value: String) -> Self {
        self.bundle.metadata.insert(key, value);
        self
    }

    pub fn build(self) -> Bundle {
        self.bundle
    }
}

impl Bundle {
    pub fn transaction_count(&self) -> usize {
        self.transactions.len()
    }

    pub fn estimate_gas(&self) -> u64 {
        self.transactions.iter().map(|tx| tx.gas_limit).sum()
    }

    pub fn to_flashbots_bundle(&self) -> crate::flashbots::FlashbotsBundle {
        crate::flashbots::FlashbotsBundle {
            transactions: self
                .transactions
                .iter()
                .map(|tx| format!("0x{}", tx.data))
                .collect(),
            block_number: self.target_block,
            max_timestamp: self.max_timestamp,
            min_timestamp: self.min_timestamp,
            reverting_tx_hashes: self.reverting_hashes.clone(),
        }
    }
}
