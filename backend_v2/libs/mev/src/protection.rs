//! MEV protection mechanisms

use rust_decimal::Decimal;
use std::collections::HashSet;
use std::time::{Duration, Instant};

/// MEV protection configuration
#[derive(Debug, Clone)]
pub struct MevProtectionConfig {
    pub max_slippage_bps: u32,
    pub private_mempool: bool,
    pub commit_reveal_scheme: bool,
    pub time_delay_seconds: u64,
}

impl Default for MevProtectionConfig {
    fn default() -> Self {
        Self {
            max_slippage_bps: 50, // 0.5%
            private_mempool: true,
            commit_reveal_scheme: false,
            time_delay_seconds: 0,
        }
    }
}

/// MEV protection mechanisms
pub struct MevProtection {
    config: MevProtectionConfig,
    pending_commits: HashSet<String>,
    protected_addresses: HashSet<String>,
    last_transaction_time: Option<Instant>,
}

impl MevProtection {
    pub fn new(config: MevProtectionConfig) -> Self {
        Self {
            config,
            pending_commits: HashSet::new(),
            protected_addresses: HashSet::new(),
            last_transaction_time: None,
        }
    }

    /// Check if a transaction should be protected from MEV
    pub fn should_protect(&self, transaction_value: Decimal, gas_price: u64) -> bool {
        // Protect high-value transactions or when gas prices are high (MEV activity)
        let high_value = transaction_value > Decimal::from(1000); // $1000+
        let high_gas = gas_price > 100_000_000_000; // 100+ gwei

        high_value || high_gas || self.config.private_mempool
    }

    /// Calculate minimum output with MEV protection
    pub fn calculate_protected_min_output(&self, expected_output: Decimal) -> Decimal {
        let protection_buffer = Decimal::from(self.config.max_slippage_bps) / Decimal::from(10000);
        expected_output * (Decimal::ONE - protection_buffer)
    }

    /// Submit a commit for commit-reveal scheme
    pub fn submit_commit(&mut self, commit_hash: String) -> bool {
        if self.config.commit_reveal_scheme {
            self.pending_commits.insert(commit_hash)
        } else {
            false
        }
    }

    /// Reveal a previously submitted commit
    pub fn reveal_commit(&mut self, commit_hash: String, reveal_data: &str) -> bool {
        if self.pending_commits.remove(&commit_hash) {
            // Validate reveal matches commit
            // This is a simplified version - real implementation would hash reveal_data
            true
        } else {
            false
        }
    }

    /// Add an address to protection list
    pub fn protect_address(&mut self, address: String) {
        self.protected_addresses.insert(address);
    }

    /// Check if an address is protected
    pub fn is_address_protected(&self, address: &str) -> bool {
        self.protected_addresses.contains(address)
    }

    /// Check if enough time has passed since last transaction (time delay protection)
    pub fn can_execute_transaction(&mut self) -> bool {
        if self.config.time_delay_seconds == 0 {
            return true;
        }

        if let Some(last_time) = self.last_transaction_time {
            let delay = Duration::from_secs(self.config.time_delay_seconds);
            if last_time.elapsed() < delay {
                return false;
            }
        }

        self.last_transaction_time = Some(Instant::now());
        true
    }

    /// Get protection statistics
    pub fn get_protection_stats(&self) -> MevProtectionStats {
        MevProtectionStats {
            protected_addresses_count: self.protected_addresses.len(),
            pending_commits_count: self.pending_commits.len(),
            private_mempool_enabled: self.config.private_mempool,
            max_slippage_bps: self.config.max_slippage_bps,
        }
    }
}

/// Statistics for MEV protection monitoring
#[derive(Debug, Clone)]
pub struct MevProtectionStats {
    pub protected_addresses_count: usize,
    pub pending_commits_count: usize,
    pub private_mempool_enabled: bool,
    pub max_slippage_bps: u32,
}
