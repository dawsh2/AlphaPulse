use crate::ArbitrageOpportunity;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use rust_decimal::{Decimal, prelude::ToPrimitive};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use dashmap::DashMap;
use tracing::{debug, info};

/// Interface for submitting arbitrage opportunities to execution bots
#[async_trait::async_trait]
pub trait ExecutionInterface: Send + Sync {
    async fn submit_opportunity(&self, opportunity: &ArbitrageOpportunity) -> Result<String>;
    async fn get_execution_status(&self, opportunity_id: &str) -> Result<Option<ExecutionStatus>>;
    async fn get_queue_stats(&self) -> Result<QueueStats>;
}

/// Channel-based execution interface for high-performance communication
pub struct ChannelExecutionInterface {
    opportunity_sender: mpsc::UnboundedSender<ArbitrageOpportunity>,
    opportunity_storage: Arc<DashMap<String, ArbitrageOpportunity>>,
    execution_results: Arc<DashMap<String, ExecutionStatus>>,
}

impl ChannelExecutionInterface {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<ArbitrageOpportunity>) {
        let (sender, receiver) = mpsc::unbounded_channel();
        
        let interface = Self {
            opportunity_sender: sender,
            opportunity_storage: Arc::new(DashMap::new()),
            execution_results: Arc::new(DashMap::new()),
        };
        
        (interface, receiver)
    }
    
    /// Calculate priority score for opportunity queuing
    fn calculate_priority(&self, opportunity: &ArbitrageOpportunity) -> i64 {
        // Higher score = higher priority (Redis ZADD uses score for ordering)
        let mut priority = 0i64;
        
        // Profit-based priority (multiply by 1000 for precision)
        priority += (opportunity.net_profit_usd.to_f64().unwrap_or(0.0) * 1000.0) as i64;
        
        // Confidence-based priority
        priority += (opportunity.confidence_score * 100.0) as i64;
        
        // Time decay (newer opportunities get higher priority)
        let age_seconds = (chrono::Utc::now().timestamp() - opportunity.timestamp) as i64;
        priority -= age_seconds; // Reduce priority over time
        
        // Boost for cross-token arbitrage (often less competitive)
        if opportunity.token_in != opportunity.token_out {
            if self.is_cross_token_pair(&opportunity.token_in, &opportunity.token_out) {
                priority += 500; // Boost cross-token opportunities
            }
        }
        
        priority.max(0) // Ensure non-negative
    }
    
    /// Check if this is a cross-token arbitrage opportunity (USDC variants)
    fn is_cross_token_pair(&self, token_in: &str, token_out: &str) -> bool {
        let usdc_variants = [
            "0x2791bca1f2de4661ed88a30c99a7a9449aa84174", // USDC.e
            "0x3c499c542cef5e3811e1192ce70d8cc03d5c3359", // USDC
            "0xc2132d05d31c914a87c6611c10748aeb04b58e8f", // USDT
        ];
        
        usdc_variants.contains(&token_in) && usdc_variants.contains(&token_out)
    }
}

#[async_trait::async_trait]
impl ExecutionInterface for ChannelExecutionInterface {
    async fn submit_opportunity(&self, opportunity: &ArbitrageOpportunity) -> Result<String> {
        let priority = self.calculate_priority(opportunity);
        
        // Store opportunity in memory for status tracking
        self.opportunity_storage.insert(opportunity.id.clone(), opportunity.clone());
        
        // Send directly to execution handler via channel (zero-copy, microsecond latency)
        self.opportunity_sender.send(opportunity.clone())
            .map_err(|_| anyhow::anyhow!("Execution channel closed"))?;
        
        debug!("Submitted opportunity {} with priority {} (${:.4} profit)", 
               opportunity.id, priority, opportunity.net_profit_usd);
        
        Ok(opportunity.id.clone())
    }
    
    async fn get_execution_status(&self, opportunity_id: &str) -> Result<Option<ExecutionStatus>> {
        // Check in-memory results (much faster than Redis)
        Ok(self.execution_results.get(opportunity_id).map(|entry| entry.clone()))
    }
    
    async fn get_queue_stats(&self) -> Result<QueueStats> {
        // Get stats from in-memory storage (nanosecond access)
        let pending_opportunities = self.opportunity_storage.len();
        
        // Get top 5 opportunities by calculating priorities
        let mut top_priorities = Vec::new();
        for entry in self.opportunity_storage.iter().take(5) {
            let priority = self.calculate_priority(&entry);
            top_priorities.push((entry.id.clone(), priority));
        }
        
        // Sort by priority (highest first)
        top_priorities.sort_by(|a, b| b.1.cmp(&a.1));
        top_priorities.truncate(5);
        
        // Count completed results in last hour
        let one_hour_ago = chrono::Utc::now().timestamp() - 3600;
        let executions_last_hour = self.execution_results
            .iter()
            .filter(|entry| {
                entry.timestamp > one_hour_ago
            })
            .count();
        
        Ok(QueueStats {
            pending_opportunities,
            top_priorities,
            executions_last_hour,
        })
    }
}

/// Mock execution interface for testing
pub struct MockExecutionInterface {
    submitted_opportunities: std::sync::Arc<std::sync::Mutex<HashMap<String, ArbitrageOpportunity>>>,
    execution_results: std::sync::Arc<std::sync::Mutex<HashMap<String, ExecutionStatus>>>,
}

impl MockExecutionInterface {
    pub fn new() -> Self {
        Self {
            submitted_opportunities: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
            execution_results: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
        }
    }
    
    /// Simulate execution completion for testing
    pub fn simulate_execution(&self, opportunity_id: &str, success: bool, actual_profit: Decimal) {
        let status = ExecutionStatus {
            opportunity_id: opportunity_id.to_string(),
            status: if success { "completed".to_string() } else { "failed".to_string() },
            transaction_hash: if success { 
                Some(format!("0x{:064x}", rand::random::<u64>())) 
            } else { 
                None 
            },
            actual_profit_usd: actual_profit,
            gas_used: if success { 180_000 } else { 50_000 },
            execution_time_ms: rand::random::<u64>() % 5000 + 1000, // 1-6 seconds
            error_message: if success { None } else { Some("Simulation failure".to_string()) },
            timestamp: chrono::Utc::now().timestamp(),
        };
        
        self.execution_results.lock().unwrap().insert(opportunity_id.to_string(), status);
    }
}

#[async_trait::async_trait]
impl ExecutionInterface for MockExecutionInterface {
    async fn submit_opportunity(&self, opportunity: &ArbitrageOpportunity) -> Result<String> {
        self.submitted_opportunities.lock().unwrap()
            .insert(opportunity.id.clone(), opportunity.clone());
        
        info!("Mock: Submitted opportunity {} (${:.4} profit)", 
              opportunity.id, opportunity.net_profit_usd);
        
        // Simulate some opportunities completing automatically
        if rand::random::<f64>() < 0.3 { // 30% auto-complete
            let success = opportunity.net_profit_usd > Decimal::from_f64_retain(1.0).unwrap();
            let actual_profit = if success {
                opportunity.net_profit_usd * Decimal::from_f64_retain(0.9).unwrap() // 90% of expected
            } else {
                -opportunity.gas_cost_estimate // Lost gas costs
            };
            self.simulate_execution(&opportunity.id, success, actual_profit);
        }
        
        Ok(opportunity.id.clone())
    }
    
    async fn get_execution_status(&self, opportunity_id: &str) -> Result<Option<ExecutionStatus>> {
        Ok(self.execution_results.lock().unwrap().get(opportunity_id).cloned())
    }
    
    async fn get_queue_stats(&self) -> Result<QueueStats> {
        let submitted = self.submitted_opportunities.lock().unwrap();
        let results = self.execution_results.lock().unwrap();
        
        Ok(QueueStats {
            pending_opportunities: submitted.len() - results.len(),
            top_priorities: submitted.keys().take(5).map(|id| (id.clone(), 1000)).collect(),
            executions_last_hour: results.len(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStatus {
    pub opportunity_id: String,
    pub status: String, // "pending", "executing", "completed", "failed"
    pub transaction_hash: Option<String>,
    pub actual_profit_usd: Decimal,
    pub gas_used: u64,
    pub execution_time_ms: u64,
    pub error_message: Option<String>,
    pub timestamp: i64,
}

#[derive(Debug, Clone)]
pub struct QueueStats {
    pub pending_opportunities: usize,
    pub top_priorities: Vec<(String, i64)>,
    pub executions_last_hour: usize,
}

/// Execution configuration
#[derive(Debug, Clone)]
pub struct ExecutionConfig {
    pub redis_url: String,
    pub max_queue_size: usize,
    pub opportunity_ttl_seconds: u64,
    pub min_profit_threshold: Decimal,
    pub enable_cross_token_boost: bool,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            redis_url: "redis://localhost:6379".to_string(),
            max_queue_size: 1000,
            opportunity_ttl_seconds: 300, // 5 minutes
            min_profit_threshold: Decimal::from_f64_retain(0.01).unwrap(), // $0.01
            enable_cross_token_boost: true,
        }
    }
}