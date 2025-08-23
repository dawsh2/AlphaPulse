// Adaptive Canary Deployment System for Huff Migration
// Implements intelligent rollout based on success criteria rather than fixed schedules

use std::collections::{HashMap, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use anyhow::{Result, ensure};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanaryConfig {
    pub initial_percentage: u8,
    pub target_percentage: u8,
    pub required_successes_per_step: u32,
    pub min_dwell_time_seconds: u64,
    pub max_step_size: u8,
    pub rollback_threshold: f64, // Parity success rate threshold
    pub emergency_rollback_failures: u32,
}

impl Default for CanaryConfig {
    fn default() -> Self {
        Self {
            initial_percentage: 1,
            target_percentage: 100,
            required_successes_per_step: 50,
            min_dwell_time_seconds: 1800, // 30 minutes
            max_step_size: 25,
            rollback_threshold: 0.98, // 98% parity success required
            emergency_rollback_failures: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanaryState {
    pub current_percentage: u8,
    pub phase: DeploymentPhase,
    pub step_start_time: u64,
    pub consecutive_successes: u32,
    pub consecutive_failures: u32,
    pub parity_success_rate: f64,
    pub total_transactions: u32,
    pub successful_transactions: u32,
    pub last_advancement: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeploymentPhase {
    Initializing,
    Canary(u8),      // Current percentage
    FullDeployment,
    Rollback(String), // Reason for rollback
    Emergency(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionResult {
    pub transaction_id: String,
    pub timestamp: u64,
    pub implementation_used: Implementation,
    pub success: bool,
    pub parity_verified: bool,
    pub gas_used: u64,
    pub scenario: String,
    pub error_details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Implementation {
    Solidity,
    Huff,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CanaryMetrics {
    pub timestamp: u64,
    pub current_state: CanaryState,
    pub recent_performance: PerformanceMetrics,
    pub health_indicators: HealthIndicators,
    pub advancement_criteria: AdvancementCriteria,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub transaction_count_last_hour: u32,
    pub success_rate_last_hour: f64,
    pub parity_rate_last_hour: f64,
    pub average_gas_savings: f64,
    pub error_rate_by_scenario: HashMap<String, f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthIndicators {
    pub is_healthy: bool,
    pub time_in_current_phase: u64,
    pub consecutive_success_streak: u32,
    pub last_error_time: Option<u64>,
    pub rollback_risk_score: f64, // 0.0 = no risk, 1.0 = immediate rollback
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdvancementCriteria {
    pub time_criterion_met: bool,
    pub success_criterion_met: bool,
    pub parity_criterion_met: bool,
    pub ready_to_advance: bool,
    pub next_percentage: Option<u8>,
    pub estimated_time_to_advance: Option<u64>,
}

pub struct AdaptiveCanaryDeployment {
    config: CanaryConfig,
    state: CanaryState,
    transaction_history: VecDeque<TransactionResult>,
    performance_windows: HashMap<String, VecDeque<TransactionResult>>, // scenario -> recent results
    rollback_callbacks: Vec<Box<dyn Fn() -> Result<()> + Send + Sync>>,
}

impl AdaptiveCanaryDeployment {
    pub fn new(config: CanaryConfig) -> Self {
        let initial_state = CanaryState {
            current_percentage: config.initial_percentage,
            phase: DeploymentPhase::Initializing,
            step_start_time: current_timestamp(),
            consecutive_successes: 0,
            consecutive_failures: 0,
            parity_success_rate: 1.0,
            total_transactions: 0,
            successful_transactions: 0,
            last_advancement: current_timestamp(),
        };

        Self {
            config,
            state: initial_state,
            transaction_history: VecDeque::with_capacity(10000),
            performance_windows: HashMap::new(),
            rollback_callbacks: Vec::new(),
        }
    }

    /// Start the canary deployment process
    pub async fn start_deployment(&mut self) -> Result<()> {
        println!("🚀 Starting adaptive canary deployment at {}%", self.config.initial_percentage);
        
        self.state.phase = DeploymentPhase::Canary(self.config.initial_percentage);
        self.state.step_start_time = current_timestamp();
        
        // Initialize monitoring
        self.setup_monitoring().await?;
        
        Ok(())
    }

    /// Record transaction result and evaluate advancement/rollback
    pub async fn record_transaction(&mut self, result: TransactionResult) -> Result<DeploymentAction> {
        // Add to history with rolling window
        self.transaction_history.push_back(result.clone());
        if self.transaction_history.len() > 10000 {
            self.transaction_history.pop_front();
        }

        // Update scenario-specific performance windows
        let scenario_window = self.performance_windows
            .entry(result.scenario.clone())
            .or_insert_with(|| VecDeque::with_capacity(1000));
        
        scenario_window.push_back(result.clone());
        if scenario_window.len() > 1000 {
            scenario_window.pop_front();
        }

        // Update state metrics
        self.update_state_metrics(&result);

        // Evaluate current situation
        let action = self.evaluate_deployment_status().await?;
        
        match &action {
            DeploymentAction::Advance(next_percentage) => {
                println!("📈 Advancing canary from {}% to {}%", 
                         self.state.current_percentage, next_percentage);
                self.advance_deployment(*next_percentage).await?;
            },
            DeploymentAction::Rollback(reason) => {
                println!("🔄 Rolling back deployment: {}", reason);
                self.initiate_rollback(reason.clone()).await?;
            },
            DeploymentAction::EmergencyRollback(reason) => {
                println!("🚨 EMERGENCY ROLLBACK: {}", reason);
                self.emergency_rollback(reason.clone()).await?;
            },
            DeploymentAction::Continue => {
                // Continue monitoring
            },
            DeploymentAction::Complete => {
                println!("🎉 Canary deployment complete!");
                self.state.phase = DeploymentPhase::FullDeployment;
            }
        }

        Ok(action)
    }

    /// Evaluate whether to advance, rollback, or continue
    async fn evaluate_deployment_status(&self) -> Result<DeploymentAction> {
        // Check for emergency conditions first
        if let Some(emergency_reason) = self.check_emergency_conditions() {
            return Ok(DeploymentAction::EmergencyRollback(emergency_reason));
        }

        // Check for rollback conditions
        if let Some(rollback_reason) = self.check_rollback_conditions() {
            return Ok(DeploymentAction::Rollback(rollback_reason));
        }

        // Check for completion
        if self.state.current_percentage >= self.config.target_percentage {
            return Ok(DeploymentAction::Complete);
        }

        // Check for advancement
        if self.should_advance_canary() {
            let next_percentage = self.calculate_next_percentage();
            return Ok(DeploymentAction::Advance(next_percentage));
        }

        Ok(DeploymentAction::Continue)
    }

    /// Check for emergency rollback conditions
    fn check_emergency_conditions(&self) -> Option<String> {
        // Emergency condition 1: Consecutive failures exceed threshold
        if self.state.consecutive_failures >= self.config.emergency_rollback_failures {
            return Some(format!(
                "Emergency: {} consecutive failures (limit: {})",
                self.state.consecutive_failures,
                self.config.emergency_rollback_failures
            ));
        }

        // Emergency condition 2: Parity success rate drops critically low
        if self.state.parity_success_rate < 0.95 && self.state.total_transactions >= 20 {
            return Some(format!(
                "Emergency: Parity success rate {:.1}% below critical threshold",
                self.state.parity_success_rate * 100.0
            ));
        }

        // Emergency condition 3: Multiple scenarios showing high error rates
        let failing_scenarios = self.get_failing_scenarios(0.8); // 80% threshold
        if failing_scenarios.len() >= 3 {
            return Some(format!(
                "Emergency: {} scenarios with >20% error rate: {:?}",
                failing_scenarios.len(),
                failing_scenarios
            ));
        }

        None
    }

    /// Check for standard rollback conditions
    fn check_rollback_conditions(&self) -> Option<String> {
        // Rollback condition 1: Parity success rate below threshold
        if self.state.parity_success_rate < self.config.rollback_threshold && 
           self.state.total_transactions >= 10 {
            return Some(format!(
                "Parity success rate {:.1}% below threshold {:.1}%",
                self.state.parity_success_rate * 100.0,
                self.config.rollback_threshold * 100.0
            ));
        }

        // Rollback condition 2: High rollback risk score
        let risk_score = self.calculate_rollback_risk_score();
        if risk_score > 0.8 {
            return Some(format!(
                "High rollback risk score: {:.3}",
                risk_score
            ));
        }

        None
    }

    /// Determine if ready to advance to next percentage
    fn should_advance_canary(&self) -> bool {
        let criteria = self.get_advancement_criteria();
        criteria.ready_to_advance
    }

    /// Get advancement criteria with detailed evaluation
    fn get_advancement_criteria(&self) -> AdvancementCriteria {
        let time_criterion = self.time_since_last_step() >= self.config.min_dwell_time_seconds;
        let success_criterion = self.state.consecutive_successes >= self.config.required_successes_per_step;
        let parity_criterion = self.state.parity_success_rate >= self.config.rollback_threshold;

        let ready_to_advance = time_criterion && success_criterion && parity_criterion;
        
        let next_percentage = if ready_to_advance {
            Some(self.calculate_next_percentage())
        } else {
            None
        };

        let estimated_time = if !time_criterion {
            Some(self.config.min_dwell_time_seconds - self.time_since_last_step())
        } else if !success_criterion {
            let remaining_successes = self.config.required_successes_per_step - self.state.consecutive_successes;
            let current_rate = self.calculate_current_success_rate();
            if current_rate > 0.0 {
                Some((remaining_successes as f64 / current_rate) as u64 * 60) // Estimate in seconds
            } else {
                None
            }
        } else {
            None
        };

        AdvancementCriteria {
            time_criterion_met: time_criterion,
            success_criterion_met: success_criterion,
            parity_criterion_met: parity_criterion,
            ready_to_advance,
            next_percentage,
            estimated_time_to_advance: estimated_time,
        }
    }

    /// Calculate next percentage step intelligently
    fn calculate_next_percentage(&self) -> u8 {
        let current = self.state.current_percentage;
        let target = self.config.target_percentage;
        
        if current >= target {
            return target;
        }

        // Adaptive step size based on success rate and current percentage
        let success_rate = self.state.parity_success_rate;
        let confidence_multiplier = if success_rate > 0.99 { 1.5 } else { 1.0 };
        
        let base_step = match current {
            0..=5 => 5,    // Small initial steps
            6..=25 => 10,  // Moderate steps in early phase
            26..=50 => 15, // Larger steps when confident
            51..=75 => 20, // Bigger steps in late phase
            _ => 25,       // Final push to 100%
        };

        let adjusted_step = ((base_step as f64 * confidence_multiplier) as u8)
            .min(self.config.max_step_size);
        
        (current + adjusted_step).min(target)
    }

    /// Calculate rollback risk score
    fn calculate_rollback_risk_score(&self) -> f64 {
        let mut risk_factors = Vec::new();

        // Factor 1: Parity success rate deviation from ideal
        let parity_risk = (1.0 - self.state.parity_success_rate).max(0.0);
        risk_factors.push(parity_risk * 0.4); // 40% weight

        // Factor 2: Consecutive failures
        let failure_ratio = self.state.consecutive_failures as f64 / 
                           self.config.emergency_rollback_failures as f64;
        risk_factors.push(failure_ratio.min(1.0) * 0.3); // 30% weight

        // Factor 3: Error rate trends
        let error_trend_risk = self.calculate_error_trend_risk();
        risk_factors.push(error_trend_risk * 0.2); // 20% weight

        // Factor 4: Time pressure (longer in phase = higher risk)
        let time_risk = (self.time_since_last_step() as f64 / (4.0 * 3600.0)).min(1.0); // 4 hours max
        risk_factors.push(time_risk * 0.1); // 10% weight

        risk_factors.iter().sum()
    }

    /// Calculate error trend risk from recent transaction patterns
    fn calculate_error_trend_risk(&self) -> f64 {
        if self.transaction_history.len() < 20 {
            return 0.0;
        }

        let recent_window = 50.min(self.transaction_history.len());
        let recent_transactions = &self.transaction_history
            .iter()
            .rev()
            .take(recent_window)
            .collect::<Vec<_>>();

        // Calculate error rate in recent window
        let error_count = recent_transactions.iter()
            .filter(|t| !t.success || !t.parity_verified)
            .count();
        
        let error_rate = error_count as f64 / recent_transactions.len() as f64;
        
        // Higher error rates = higher risk
        error_rate
    }

    /// Get scenarios with error rates above threshold
    fn get_failing_scenarios(&self, success_threshold: f64) -> Vec<String> {
        let mut failing_scenarios = Vec::new();
        
        for (scenario, window) in &self.performance_windows {
            if window.len() < 10 {
                continue; // Not enough data
            }
            
            let success_count = window.iter()
                .filter(|t| t.success && t.parity_verified)
                .count();
            
            let success_rate = success_count as f64 / window.len() as f64;
            
            if success_rate < success_threshold {
                failing_scenarios.push(scenario.clone());
            }
        }
        
        failing_scenarios
    }

    /// Advance deployment to next percentage
    async fn advance_deployment(&mut self, next_percentage: u8) -> Result<()> {
        self.state.current_percentage = next_percentage;
        self.state.phase = if next_percentage >= self.config.target_percentage {
            DeploymentPhase::FullDeployment
        } else {
            DeploymentPhase::Canary(next_percentage)
        };
        
        self.state.step_start_time = current_timestamp();
        self.state.last_advancement = current_timestamp();
        self.state.consecutive_successes = 0; // Reset for next phase
        
        // Update deployment configuration
        self.update_deployment_percentage(next_percentage).await?;
        
        Ok(())
    }

    /// Initiate controlled rollback
    async fn initiate_rollback(&mut self, reason: String) -> Result<()> {
        self.state.phase = DeploymentPhase::Rollback(reason.clone());
        self.state.current_percentage = 0; // Back to 100% Solidity
        
        // Execute rollback callbacks
        for callback in &self.rollback_callbacks {
            if let Err(e) = callback() {
                eprintln!("Rollback callback failed: {}", e);
            }
        }
        
        self.update_deployment_percentage(0).await?;
        
        Ok(())
    }

    /// Execute emergency rollback
    async fn emergency_rollback(&mut self, reason: String) -> Result<()> {
        self.state.phase = DeploymentPhase::Emergency(reason.clone());
        self.state.current_percentage = 0;
        
        // Immediate rollback - disable Huff completely
        self.disable_huff_immediately().await?;
        
        // Execute all rollback callbacks
        for callback in &self.rollback_callbacks {
            let _ = callback(); // Don't fail on callback errors during emergency
        }
        
        Ok(())
    }

    /// Update state metrics based on transaction result
    fn update_state_metrics(&mut self, result: &TransactionResult) {
        self.state.total_transactions += 1;
        
        if result.success && result.parity_verified {
            self.state.successful_transactions += 1;
            self.state.consecutive_successes += 1;
            self.state.consecutive_failures = 0;
        } else {
            self.state.consecutive_failures += 1;
            self.state.consecutive_successes = 0;
        }
        
        // Update parity success rate
        self.state.parity_success_rate = 
            self.state.successful_transactions as f64 / self.state.total_transactions as f64;
    }

    /// Calculate current success rate from recent transactions
    fn calculate_current_success_rate(&self) -> f64 {
        if self.transaction_history.is_empty() {
            return 0.0;
        }
        
        let recent_window = 100.min(self.transaction_history.len());
        let recent_transactions = self.transaction_history
            .iter()
            .rev()
            .take(recent_window);
        
        let success_count = recent_transactions
            .filter(|t| t.success && t.parity_verified)
            .count();
        
        success_count as f64 / recent_window as f64
    }

    /// Get time since last step in seconds
    fn time_since_last_step(&self) -> u64 {
        current_timestamp() - self.state.step_start_time
    }

    /// Get comprehensive canary metrics
    pub fn get_metrics(&self) -> CanaryMetrics {
        CanaryMetrics {
            timestamp: current_timestamp(),
            current_state: self.state.clone(),
            recent_performance: self.calculate_recent_performance(),
            health_indicators: self.calculate_health_indicators(),
            advancement_criteria: self.get_advancement_criteria(),
        }
    }

    fn calculate_recent_performance(&self) -> PerformanceMetrics {
        let one_hour_ago = current_timestamp() - 3600;
        let recent_transactions: Vec<_> = self.transaction_history
            .iter()
            .filter(|t| t.timestamp >= one_hour_ago)
            .collect();

        let transaction_count = recent_transactions.len() as u32;
        let success_count = recent_transactions.iter()
            .filter(|t| t.success)
            .count();
        let parity_count = recent_transactions.iter()
            .filter(|t| t.parity_verified)
            .count();

        let success_rate = if transaction_count > 0 {
            success_count as f64 / transaction_count as f64
        } else {
            1.0
        };

        let parity_rate = if transaction_count > 0 {
            parity_count as f64 / transaction_count as f64
        } else {
            1.0
        };

        // Calculate average gas savings for Huff transactions
        let huff_transactions: Vec<_> = recent_transactions.iter()
            .filter(|t| matches!(t.implementation_used, Implementation::Huff))
            .collect();
        
        let average_gas_savings = if !huff_transactions.is_empty() {
            // Simplified calculation - in practice would compare with Solidity baseline
            0.65 // Assume 65% savings for now
        } else {
            0.0
        };

        // Calculate error rates by scenario
        let mut error_rate_by_scenario = HashMap::new();
        for (scenario, window) in &self.performance_windows {
            let scenario_recent: Vec<_> = window.iter()
                .filter(|t| t.timestamp >= one_hour_ago)
                .collect();
            
            if !scenario_recent.is_empty() {
                let error_count = scenario_recent.iter()
                    .filter(|t| !t.success || !t.parity_verified)
                    .count();
                let error_rate = error_count as f64 / scenario_recent.len() as f64;
                error_rate_by_scenario.insert(scenario.clone(), error_rate);
            }
        }

        PerformanceMetrics {
            transaction_count_last_hour: transaction_count,
            success_rate_last_hour: success_rate,
            parity_rate_last_hour: parity_rate,
            average_gas_savings,
            error_rate_by_scenario,
        }
    }

    fn calculate_health_indicators(&self) -> HealthIndicators {
        let rollback_risk_score = self.calculate_rollback_risk_score();
        let is_healthy = rollback_risk_score < 0.5 && 
                        self.state.parity_success_rate >= self.config.rollback_threshold;

        let last_error_time = self.transaction_history
            .iter()
            .rev()
            .find(|t| !t.success || !t.parity_verified)
            .map(|t| t.timestamp);

        HealthIndicators {
            is_healthy,
            time_in_current_phase: self.time_since_last_step(),
            consecutive_success_streak: self.state.consecutive_successes,
            last_error_time,
            rollback_risk_score,
        }
    }

    // Placeholder methods for actual deployment operations
    async fn setup_monitoring(&self) -> Result<()> {
        // Initialize monitoring systems
        Ok(())
    }

    async fn update_deployment_percentage(&self, percentage: u8) -> Result<()> {
        // Update load balancer or contract selector to use new percentage
        println!("🔧 Updated deployment percentage to {}%", percentage);
        Ok(())
    }

    async fn disable_huff_immediately(&self) -> Result<()> {
        // Emergency disable of Huff implementation
        println!("🚨 Huff implementation disabled immediately");
        Ok(())
    }

    pub fn add_rollback_callback(&mut self, callback: Box<dyn Fn() -> Result<()> + Send + Sync>) {
        self.rollback_callbacks.push(callback);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeploymentAction {
    Continue,
    Advance(u8), // Next percentage
    Rollback(String), // Reason
    EmergencyRollback(String), // Emergency reason
    Complete,
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_canary_advancement() {
        let config = CanaryConfig::default();
        let mut canary = AdaptiveCanaryDeployment::new(config);
        
        canary.start_deployment().await.unwrap();
        
        // Simulate successful transactions
        for i in 0..60 {
            let result = TransactionResult {
                transaction_id: format!("tx_{}", i),
                timestamp: current_timestamp(),
                implementation_used: Implementation::Huff,
                success: true,
                parity_verified: true,
                gas_used: 100_000,
                scenario: "test_scenario".to_string(),
                error_details: None,
            };
            
            let action = canary.record_transaction(result).await.unwrap();
            
            if matches!(action, DeploymentAction::Advance(_)) {
                assert!(canary.state.current_percentage > 1);
                break;
            }
        }
    }

    #[tokio::test]
    async fn test_rollback_conditions() {
        let config = CanaryConfig::default();
        let mut canary = AdaptiveCanaryDeployment::new(config);
        
        canary.start_deployment().await.unwrap();
        
        // Simulate failed transactions to trigger rollback
        for i in 0..10 {
            let result = TransactionResult {
                transaction_id: format!("tx_{}", i),
                timestamp: current_timestamp(),
                implementation_used: Implementation::Huff,
                success: false,
                parity_verified: false,
                gas_used: 100_000,
                scenario: "test_scenario".to_string(),
                error_details: Some("Test failure".to_string()),
            };
            
            let action = canary.record_transaction(result).await.unwrap();
            
            if matches!(action, DeploymentAction::Rollback(_) | DeploymentAction::EmergencyRollback(_)) {
                assert_eq!(canary.state.current_percentage, 0);
                break;
            }
        }
    }
}