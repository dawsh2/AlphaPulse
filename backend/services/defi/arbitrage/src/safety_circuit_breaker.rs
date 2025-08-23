// Production Safety Circuit Breaker System
// CRITICAL: Prevents financial losses from system failures

use anyhow::{Result, Context};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{info, warn, error, debug};
use serde::{Serialize, Deserialize};

use crate::price_oracle::LivePriceOracle;

/// Circuit breaker states
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CircuitState {
    Closed,    // Normal operation
    Open,      // Circuit broken - blocking all operations
    HalfOpen,  // Testing recovery
}

/// Safety metrics tracking
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SafetyMetrics {
    pub consecutive_failures: u64,
    pub total_failures_last_hour: u64,
    pub total_profit_lost_usd: f64,
    pub gas_cost_ratio_violations: u64,
    pub price_oracle_failures: u64,
    pub execution_timeouts: u64,
    pub last_failure_timestamp: u64,
    pub recovery_attempts: u64,
}

/// Production safety circuit breaker
pub struct SafetyCircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    metrics: Arc<RwLock<SafetyMetrics>>,
    config: SafetyConfig,
    price_oracle: Arc<RwLock<LivePriceOracle>>,
    failure_window: HashMap<u64, u64>, // timestamp -> failure_count
}

#[derive(Debug, Clone)]
pub struct SafetyConfig {
    pub max_consecutive_failures: u64,
    pub max_failures_per_hour: u64,
    pub max_profit_loss_usd: f64,
    pub max_gas_cost_ratio: f64,
    pub circuit_timeout_ms: u64,
    pub recovery_test_duration_ms: u64,
    pub price_staleness_threshold_sec: u64,
    pub execution_timeout_ms: u64,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            max_consecutive_failures: 5,
            max_failures_per_hour: 20,
            max_profit_loss_usd: 1000.0,
            max_gas_cost_ratio: 0.8, // 80% of profit
            circuit_timeout_ms: 300_000, // 5 minutes
            recovery_test_duration_ms: 60_000, // 1 minute
            price_staleness_threshold_sec: 300, // 5 minutes
            execution_timeout_ms: 30_000, // 30 seconds
        }
    }
}

#[derive(Debug, Clone)]
pub struct SafetyCheck {
    pub is_safe: bool,
    pub risk_level: RiskLevel,
    pub warnings: Vec<String>,
    pub blocked_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl SafetyCircuitBreaker {
    pub fn new(config: SafetyConfig, price_oracle: Arc<RwLock<LivePriceOracle>>) -> Self {
        info!("Initializing production safety circuit breaker with config: {:?}", config);
        
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            metrics: Arc::new(RwLock::new(SafetyMetrics::default())),
            config,
            price_oracle,
            failure_window: HashMap::new(),
        }
    }

    /// Primary safety check before any arbitrage execution
    pub async fn check_execution_safety(&mut self, 
        expected_profit_usd: f64, 
        gas_cost_usd: f64,
        complexity: usize) -> Result<SafetyCheck> {
        
        let current_state = self.state.read().clone();
        
        match current_state {
            CircuitState::Open => {
                warn!("Circuit breaker OPEN - blocking execution for safety");
                return Ok(SafetyCheck {
                    is_safe: false,
                    risk_level: RiskLevel::Critical,
                    warnings: vec!["Circuit breaker open due to safety concerns".to_string()],
                    blocked_reason: Some("Production safety circuit breaker engaged".to_string()),
                });
            }
            CircuitState::HalfOpen => {
                debug!("Circuit breaker HALF-OPEN - allowing limited execution for testing");
            }
            CircuitState::Closed => {
                debug!("Circuit breaker CLOSED - normal safety checks");
            }
        }

        let mut safety_check = SafetyCheck {
            is_safe: true,
            risk_level: RiskLevel::Low,
            warnings: Vec::new(),
            blocked_reason: None,
        };

        // 1. Price oracle health check
        if let Err(e) = self.check_price_oracle_health(&mut safety_check).await {
            error!("Price oracle health check failed: {}", e);
            self.record_failure("price_oracle_failure").await;
            safety_check.is_safe = false;
            safety_check.blocked_reason = Some("Price oracle unreliable".to_string());
            return Ok(safety_check);
        }

        // 2. Gas cost ratio check
        self.check_gas_cost_ratio(expected_profit_usd, gas_cost_usd, &mut safety_check);

        // 3. Complexity risk assessment
        self.assess_complexity_risk(complexity, &mut safety_check);

        // 4. Recent failure pattern analysis
        self.analyze_failure_patterns(&mut safety_check).await;

        // 5. Market volatility check
        if let Err(e) = self.check_market_volatility(&mut safety_check).await {
            warn!("Market volatility check failed: {}", e);
            safety_check.warnings.push("Unable to assess market volatility".to_string());
        }

        // Determine final risk level
        self.calculate_final_risk_level(&mut safety_check);

        // Auto-engage circuit breaker if critical risk
        if safety_check.risk_level == RiskLevel::Critical {
            self.engage_circuit_breaker("Critical risk detected".to_string()).await;
            safety_check.is_safe = false;
        }

        debug!("Safety check completed: safe={}, risk={:?}, warnings={}", 
               safety_check.is_safe, safety_check.risk_level, safety_check.warnings.len());

        Ok(safety_check)
    }

    /// Check price oracle health and reliability
    async fn check_price_oracle_health(&mut self, safety_check: &mut SafetyCheck) -> Result<()> {
        let mut oracle = self.price_oracle.write();
        
        // Test MATIC price freshness
        match oracle.get_live_matic_price().await {
            Ok(price) => {
                if price <= 0.0 || price > 10.0 {
                    safety_check.warnings.push(format!("MATIC price out of range: ${:.4}", price));
                    safety_check.risk_level = RiskLevel::High;
                }
                
                // Check if price oracle is reliable
                if !oracle.is_price_reliable("MATIC/USD").await {
                    safety_check.warnings.push("MATIC price oracle unreliable".to_string());
                    safety_check.risk_level = RiskLevel::Medium;
                }
            }
            Err(e) => {
                error!("Failed to get MATIC price for safety check: {}", e);
                let mut metrics = self.metrics.write();
                metrics.price_oracle_failures += 1;
                return Err(e);
            }
        }

        // Test gas price freshness
        match oracle.get_live_gas_prices().await {
            Ok(gas_prices) => {
                if gas_prices.fast > 500.0 {
                    safety_check.warnings.push(format!("Extremely high gas prices: {:.1} gwei", gas_prices.fast));
                    safety_check.risk_level = RiskLevel::High;
                }
            }
            Err(e) => {
                warn!("Failed to get gas prices for safety check: {}", e);
                safety_check.warnings.push("Gas price oracle unavailable".to_string());
                safety_check.risk_level = RiskLevel::Medium;
            }
        }

        Ok(())
    }

    /// Check gas cost vs profit ratio
    fn check_gas_cost_ratio(&self, expected_profit_usd: f64, gas_cost_usd: f64, safety_check: &mut SafetyCheck) {
        if gas_cost_usd <= 0.0 {
            safety_check.warnings.push("Gas cost calculation invalid".to_string());
            safety_check.risk_level = RiskLevel::Medium;
            return;
        }

        let gas_ratio = gas_cost_usd / expected_profit_usd;
        
        if gas_ratio > self.config.max_gas_cost_ratio {
            safety_check.warnings.push(format!("High gas cost ratio: {:.2} (max: {:.2})", 
                                               gas_ratio, self.config.max_gas_cost_ratio));
            safety_check.risk_level = RiskLevel::High;
            
            // Record metric
            let mut metrics = self.metrics.write();
            metrics.gas_cost_ratio_violations += 1;
        } else if gas_ratio > 0.5 {
            safety_check.warnings.push(format!("Moderate gas cost ratio: {:.2}", gas_ratio));
            if safety_check.risk_level == RiskLevel::Low {
                safety_check.risk_level = RiskLevel::Medium;
            }
        }

        debug!("Gas cost ratio check: ${:.2} gas / ${:.2} profit = {:.2}", 
               gas_cost_usd, expected_profit_usd, gas_ratio);
    }

    /// Assess risk based on strategy complexity
    fn assess_complexity_risk(&self, complexity: usize, safety_check: &mut SafetyCheck) {
        match complexity {
            0..=2 => {
                // Simple trades - low risk
            }
            3..=5 => {
                safety_check.warnings.push("Medium complexity strategy".to_string());
                if safety_check.risk_level == RiskLevel::Low {
                    safety_check.risk_level = RiskLevel::Medium;
                }
            }
            6..=10 => {
                safety_check.warnings.push("High complexity strategy".to_string());
                safety_check.risk_level = RiskLevel::High;
            }
            _ => {
                safety_check.warnings.push("Extremely high complexity strategy".to_string());
                safety_check.risk_level = RiskLevel::Critical;
            }
        }

        debug!("Complexity risk assessment: {} hops -> {:?}", complexity, safety_check.risk_level);
    }

    /// Analyze recent failure patterns
    async fn analyze_failure_patterns(&mut self, safety_check: &mut SafetyCheck) {
        let metrics = self.metrics.read().clone();
        let now = current_timestamp();
        
        // Check consecutive failures
        if metrics.consecutive_failures >= self.config.max_consecutive_failures {
            safety_check.warnings.push(format!("High consecutive failures: {}", metrics.consecutive_failures));
            safety_check.risk_level = RiskLevel::Critical;
        }
        
        // Check failures in last hour
        let hour_ago = now - 3600;
        let recent_failures: u64 = self.failure_window.iter()
            .filter(|(timestamp, _)| **timestamp > hour_ago)
            .map(|(_, count)| *count)
            .sum();
            
        if recent_failures >= self.config.max_failures_per_hour {
            safety_check.warnings.push(format!("Too many recent failures: {}/hour", recent_failures));
            safety_check.risk_level = RiskLevel::High;
        }

        // Check total profit lost
        if metrics.total_profit_lost_usd > self.config.max_profit_loss_usd {
            safety_check.warnings.push(format!("High cumulative losses: ${:.2}", metrics.total_profit_lost_usd));
            safety_check.risk_level = RiskLevel::Critical;
        }

        debug!("Failure pattern analysis: consecutive={}, recent={}, losses=${:.2}", 
               metrics.consecutive_failures, recent_failures, metrics.total_profit_lost_usd);
    }

    /// Check market volatility indicators
    async fn check_market_volatility(&mut self, safety_check: &mut SafetyCheck) -> Result<()> {
        let mut oracle = self.price_oracle.write();
        
        // Get price metrics to assess volatility
        let price_metrics = oracle.get_price_metrics().await;
        
        for (pair, (_price, staleness, _source)) in price_metrics {
            if staleness > self.config.price_staleness_threshold_sec {
                safety_check.warnings.push(format!("Stale price for {}: {}s old", pair, staleness));
                if safety_check.risk_level == RiskLevel::Low {
                    safety_check.risk_level = RiskLevel::Medium;
                }
            }
        }

        Ok(())
    }

    /// Calculate final risk level based on all factors
    fn calculate_final_risk_level(&self, safety_check: &mut SafetyCheck) {
        let warning_count = safety_check.warnings.len();
        
        // Escalate risk based on number of warnings
        if warning_count >= 5 {
            safety_check.risk_level = RiskLevel::Critical;
        } else if warning_count >= 3 && safety_check.risk_level != RiskLevel::Critical {
            safety_check.risk_level = RiskLevel::High;
        } else if warning_count >= 1 && safety_check.risk_level == RiskLevel::Low {
            safety_check.risk_level = RiskLevel::Medium;
        }
    }

    /// Record a system failure
    pub async fn record_failure(&mut self, failure_type: &str) {
        let now = current_timestamp();
        
        let consecutive_failures = {
            let mut metrics = self.metrics.write();
            metrics.consecutive_failures += 1;
            metrics.total_failures_last_hour += 1;
            metrics.last_failure_timestamp = now;
            metrics.consecutive_failures
        };
        
        // Add to failure window
        *self.failure_window.entry(now).or_insert(0) += 1;
        
        // Clean old entries (older than 1 hour)
        let hour_ago = now - 3600;
        self.failure_window.retain(|timestamp, _| *timestamp > hour_ago);
        
        warn!("System failure recorded: {} (consecutive: {})", failure_type, consecutive_failures);
        
        // Auto-engage circuit breaker if threshold reached
        if consecutive_failures >= self.config.max_consecutive_failures {
            let reason = format!("Max consecutive failures reached: {}", consecutive_failures);
            self.engage_circuit_breaker(reason).await;
        }
    }

    /// Record successful execution (resets consecutive failure count)
    pub async fn record_success(&mut self, profit_usd: f64) {
        let mut metrics = self.metrics.write();
        metrics.consecutive_failures = 0; // Reset on success
        
        debug!("Successful execution recorded: ${:.2} profit", profit_usd);
        
        // If circuit was half-open and we had success, close it
        let mut state = self.state.write();
        if *state == CircuitState::HalfOpen {
            *state = CircuitState::Closed;
            info!("Circuit breaker CLOSED after successful recovery test");
        }
    }

    /// Manually engage circuit breaker
    pub async fn engage_circuit_breaker(&mut self, reason: String) {
        let mut state = self.state.write();
        *state = CircuitState::Open;
        
        error!("🚨 CIRCUIT BREAKER ENGAGED: {}", reason);
        
        // Schedule automatic recovery attempt
        let timeout_ms = self.config.circuit_timeout_ms;
        let state_arc = Arc::clone(&self.state);
        
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(timeout_ms)).await;
            
            let mut state = state_arc.write();
            if *state == CircuitState::Open {
                *state = CircuitState::HalfOpen;
                info!("Circuit breaker moved to HALF-OPEN for recovery testing");
            }
        });
    }

    /// Get current safety status
    pub fn get_safety_status(&self) -> SafetyStatus {
        let state = self.state.read().clone();
        let metrics = self.metrics.read().clone();
        
        SafetyStatus {
            circuit_state: state,
            metrics,
            last_check_timestamp: current_timestamp(),
        }
    }

    /// Reset metrics (for testing or maintenance)
    pub fn reset_metrics(&mut self) {
        let mut metrics = self.metrics.write();
        *metrics = SafetyMetrics::default();
        self.failure_window.clear();
        
        let mut state = self.state.write();
        *state = CircuitState::Closed;
        
        info!("Safety circuit breaker metrics and state reset");
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyStatus {
    pub circuit_state: CircuitState,
    pub metrics: SafetyMetrics,
    pub last_check_timestamp: u64,
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers::providers::{Provider, Http};
    use std::str::FromStr;

    #[tokio::test]
    async fn test_safety_circuit_breaker() {
        let provider = Provider::<Http>::try_from("https://rpc-mumbai.maticvigil.com")
            .expect("Failed to create provider");
        let oracle = LivePriceOracle::new(Arc::new(provider), 80001);
        let oracle_arc = Arc::new(RwLock::new(oracle));
        
        let config = SafetyConfig::default();
        let mut breaker = SafetyCircuitBreaker::new(config, oracle_arc);
        
        // Test normal operation
        let safety_check = breaker.check_execution_safety(100.0, 10.0, 2).await
            .expect("Safety check should succeed");
        assert!(safety_check.is_safe);
        assert_eq!(safety_check.risk_level, RiskLevel::Low);
        
        // Test high gas cost ratio
        let safety_check = breaker.check_execution_safety(100.0, 90.0, 2).await
            .expect("Safety check should succeed");
        assert_eq!(safety_check.risk_level, RiskLevel::High);
        
        // Test circuit breaker engagement
        for _ in 0..5 {
            breaker.record_failure("test_failure").await;
        }
        
        let safety_check = breaker.check_execution_safety(100.0, 10.0, 2).await
            .expect("Safety check should succeed");
        assert!(!safety_check.is_safe);
        assert_eq!(safety_check.risk_level, RiskLevel::Critical);
    }
}