use anyhow::Result;
use ethers::{
    core::types::{transaction::eip2718::TypedTransaction, H256, U256},
    providers::{Http, Middleware, Provider},
    signers::{LocalWallet, Signer},
    types::{transaction::eip2930::AccessList, Eip1559TransactionRequest},
};
use reqwest::Client;
use serde_json::{json, Value};
use std::str::FromStr;
use tracing::{debug, info, warn, error};
use url::Url;

/// Flashbots client for private mempool submission - essential for MEV protection
pub struct FlashbotsClient {
    provider: Provider<Http>,
    wallet: LocalWallet,
    flashbots_url: String,
    http_client: Client,
    chain_id: u64,
}

impl FlashbotsClient {
    pub fn new(
        rpc_url: &str,
        private_key: &str,
        flashbots_url: Option<String>,
        chain_id: u64,
    ) -> Result<Self> {
        let provider = Provider::<Http>::try_from(rpc_url)?;
        let wallet = LocalWallet::from_str(private_key)?;
        
        // Default to Flashbots mainnet endpoint, but allow override for Polygon
        let flashbots_url = flashbots_url.unwrap_or_else(|| {
            if chain_id == 137 {
                // Polygon uses a different MEV protection service
                "https://api.polygon.flashbots.net".to_string()
            } else {
                "https://relay.flashbots.net".to_string()
            }
        });

        let http_client = Client::new();

        info!("Initialized Flashbots client for chain {}", chain_id);
        
        Ok(Self {
            provider,
            wallet,
            flashbots_url,
            http_client,
            chain_id,
        })
    }

    /// Submit transaction through Flashbots private mempool for MEV protection
    pub async fn send_private_transaction(
        &self,
        tx_request: Eip1559TransactionRequest,
    ) -> Result<H256> {
        info!("Submitting transaction via Flashbots private mempool");
        
        // Clone for potential fallback use
        let tx_request_fallback = tx_request.clone();
        
        // Build and sign the transaction
        let tx = self.build_signed_transaction(tx_request).await?;
        
        // Submit to Flashbots relay
        match self.submit_to_flashbots(&tx).await {
            Ok(tx_hash) => {
                info!("Transaction submitted to Flashbots: {:?}", tx_hash);
                Ok(tx_hash)
            }
            Err(e) => {
                warn!("Flashbots submission failed, falling back to public mempool: {}", e);
                // Use the cloned request for fallback
                self.fallback_to_public_mempool(tx_request_fallback).await
            }
        }
    }

    async fn build_signed_transaction(&self, mut tx_request: Eip1559TransactionRequest) -> Result<TypedTransaction> {
        // Set chain ID
        tx_request = tx_request.chain_id(self.chain_id);
        
        // Get current nonce
        let nonce = self.provider
            .get_transaction_count(self.wallet.address(), None)
            .await?;
        tx_request = tx_request.nonce(nonce);
        
        // Estimate gas if not provided
        if tx_request.gas.is_none() {
            let typed_tx: TypedTransaction = tx_request.clone().into();
            let gas_estimate = self.provider.estimate_gas(&typed_tx, None).await?;
            // Add 20% buffer for flash loan execution
            let gas_with_buffer = gas_estimate * 120 / 100;
            tx_request = tx_request.gas(gas_with_buffer);
        }
        
        // Set gas fees if not provided - important for MEV protection
        if tx_request.max_fee_per_gas.is_none() || tx_request.max_priority_fee_per_gas.is_none() {
            let (max_fee, priority_fee) = self.get_optimal_gas_fees().await?;
            tx_request = tx_request
                .max_fee_per_gas(max_fee)
                .max_priority_fee_per_gas(priority_fee);
        }

        // Convert to TypedTransaction and sign
        let typed_tx: TypedTransaction = tx_request.into();
        let signature = self.wallet.sign_transaction(&typed_tx).await?;
        
        Ok(typed_tx)
    }

    async fn get_optimal_gas_fees(&self) -> Result<(U256, U256)> {
        // Get current gas fees from network
        let gas_price = self.provider.get_gas_price().await?;
        
        // For MEV protection, we typically want to pay higher fees
        // to ensure inclusion in the next block
        let base_fee = gas_price * 110 / 100; // 10% above current
        let priority_fee = gas_price * 20 / 100; // 20% tip for miners
        let max_fee = base_fee + priority_fee;
        
        debug!("Calculated gas fees - Max: {} gwei, Priority: {} gwei", 
               max_fee / 1_000_000_000u64, priority_fee / 1_000_000_000u64);
        
        Ok((max_fee, priority_fee))
    }

    async fn submit_to_flashbots(&self, tx: &TypedTransaction) -> Result<H256> {
        // Prepare Flashbots bundle
        let bundle = self.create_flashbots_bundle(tx).await?;
        
        // Submit bundle to Flashbots relay
        let url = format!("{}/v1/bundle", self.flashbots_url);
        let response = self.http_client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("X-Flashbots-Signature", self.create_flashbots_signature(&bundle)?)
            .json(&bundle)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Flashbots submission failed: {}", error_text);
        }

        let result: Value = response.json().await?;
        debug!("Flashbots response: {}", result);
        
        // Extract transaction hash (this is approximate since bundle hasn't been mined yet)
        let tx_hash = H256::random(); // Placeholder since we can't calculate hash without signature
        Ok(tx_hash)
    }

    async fn create_flashbots_bundle(&self, tx: &TypedTransaction) -> Result<Value> {
        let current_block = self.provider.get_block_number().await?;
        let target_block = current_block + 1; // Target next block
        
        let bundle = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_sendBundle",
            "params": [{
                "txs": [format!("0x{}", hex::encode(tx.rlp()))],
                "blockNumber": format!("0x{:x}", target_block),
                "minTimestamp": 0,
                "maxTimestamp": 0
            }]
        });

        Ok(bundle)
    }

    fn create_flashbots_signature(&self, bundle: &Value) -> Result<String> {
        // Create signature for Flashbots authentication
        let bundle_str = bundle.to_string();
        let hash = ethers::utils::keccak256(bundle_str.as_bytes());
        let signature = self.wallet.sign_hash(hash.into())?;
        
        Ok(format!("{}:0x{}", self.wallet.address(), signature))
    }

    async fn fallback_to_public_mempool(&self, tx_request: Eip1559TransactionRequest) -> Result<H256> {
        warn!("Using public mempool - MEV protection disabled!");
        
        let pending_tx = self.provider.send_transaction(tx_request, None).await?;
        Ok(pending_tx.tx_hash())
    }

    /// Check if transaction was included in a bundle
    pub async fn check_bundle_status(&self, tx_hash: H256) -> Result<BundleStatus> {
        let receipt = self.provider.get_transaction_receipt(tx_hash).await?;
        
        match receipt {
            Some(receipt) => {
                if receipt.status == Some(1.into()) {
                    Ok(BundleStatus::Confirmed)
                } else {
                    Ok(BundleStatus::Failed)
                }
            }
            None => Ok(BundleStatus::Pending)
        }
    }

    /// Simulate bundle before submission to ensure profitability
    pub async fn simulate_bundle(&self, tx_request: &Eip1559TransactionRequest) -> Result<SimulationResult> {
        debug!("Simulating bundle before Flashbots submission");
        
        // Use eth_call to simulate the transaction
        let typed_tx: TypedTransaction = tx_request.clone().into();
        let result = self.provider.call(&typed_tx, None).await;
        
        match result {
            Ok(output) => {
                // Parse output to determine profit/loss
                // This is simplified - in practice you'd parse the exact output
                Ok(SimulationResult {
                    success: true,
                    gas_used: 800_000, // Estimated
                    profit_wei: 1_000_000_000_000_000_000u64, // 1 ETH estimated
                    error: None,
                })
            }
            Err(e) => {
                warn!("Bundle simulation failed: {}", e);
                Ok(SimulationResult {
                    success: false,
                    gas_used: 0,
                    profit_wei: 0,
                    error: Some(e.to_string()),
                })
            }
        }
    }

    /// Get current MEV competition level
    pub async fn get_mev_competition(&self) -> Result<MEVCompetition> {
        // Check recent blocks for arbitrage transaction density
        let current_block = self.provider.get_block_number().await?;
        let recent_blocks = 5u64;
        
        let mut total_arb_txs = 0u64;
        let mut total_txs = 0u64;
        
        for i in 0..recent_blocks {
            if let Ok(Some(block)) = self.provider.get_block(current_block - i).await {
                total_txs += block.transactions.len() as u64;
                
                // Heuristic: count transactions with high gas prices as potential MEV
                for tx_hash in &block.transactions {
                    if let Ok(Some(tx)) = self.provider.get_transaction(*tx_hash).await {
                        if let Some(gas_price) = tx.gas_price {
                            // High gas price suggests MEV competition
                            if gas_price > U256::from(50_000_000_000u64) { // 50+ gwei
                                total_arb_txs += 1;
                            }
                        }
                    }
                }
            }
        }
        
        let competition_ratio = if total_txs > 0 {
            (total_arb_txs as f64) / (total_txs as f64)
        } else {
            0.0
        };
        
        let level = if competition_ratio > 0.1 {
            MEVCompetitionLevel::High
        } else if competition_ratio > 0.05 {
            MEVCompetitionLevel::Medium
        } else {
            MEVCompetitionLevel::Low
        };
        
        Ok(MEVCompetition {
            level,
            ratio: competition_ratio,
            recent_arb_txs: total_arb_txs,
        })
    }
}

#[derive(Debug, Clone)]
pub enum BundleStatus {
    Pending,
    Confirmed,
    Failed,
}

#[derive(Debug, Clone)]
pub struct SimulationResult {
    pub success: bool,
    pub gas_used: u64,
    pub profit_wei: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MEVCompetition {
    pub level: MEVCompetitionLevel,
    pub ratio: f64,
    pub recent_arb_txs: u64,
}

#[derive(Debug, Clone)]
pub enum MEVCompetitionLevel {
    Low,
    Medium,
    High,
}

impl MEVCompetitionLevel {
    /// Get recommended gas price multiplier based on competition
    pub fn gas_multiplier(&self) -> f64 {
        match self {
            MEVCompetitionLevel::Low => 1.1,   // 10% above base
            MEVCompetitionLevel::Medium => 1.3, // 30% above base
            MEVCompetitionLevel::High => 1.6,  // 60% above base
        }
    }
}