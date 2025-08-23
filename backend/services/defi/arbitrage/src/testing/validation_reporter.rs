// Validation Reporter - Tracks and compares predicted vs actual execution metrics
// Critical for understanding model accuracy and improving predictions

use anyhow::{Result, Context};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};
use ethers::prelude::*;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

use crate::{
    ArbitrageOpportunity,
    ArbitrageMetrics,
    config::ArbitrageConfig,
};

/// Tracks predictions vs actual results for validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationEntry {
    pub opportunity_id: String,
    pub timestamp: DateTime<Utc>,
    
    // Gas metrics
    pub gas_predicted: u64,
    pub gas_actual: Option<u64>,
    pub gas_price_predicted: f64,
    pub gas_price_actual: Option<f64>,
    
    // Slippage metrics
    pub slippage_predicted_pct: f64,
    pub slippage_actual_pct: Option<f64>,
    pub price_impact_predicted: f64,
    pub price_impact_actual: Option<f64>,
    
    // Execution metrics
    pub amount_in: U256,
    pub amount_out_predicted: U256,
    pub amount_out_actual: Option<U256>,
    
    // Profit metrics
    pub profit_predicted_usd: f64,
    pub profit_actual_usd: Option<f64>,
    
    // Status
    pub execution_status: ExecutionStatus,
    pub validation_errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Pending,
    Simulated,
    Executed,
    Failed(String),
    Reverted(String),
}

/// Manages validation tracking and reporting
pub struct ValidationReporter {
    entries: Arc<RwLock<Vec<ValidationEntry>>>,
    config: Arc<ArbitrageConfig>,
    metrics: Arc<RwLock<ValidationMetrics>>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ValidationMetrics {
    // Gas accuracy
    pub gas_predictions_count: usize,
    pub gas_prediction_error_sum: f64,
    pub gas_prediction_error_squared_sum: f64,
    pub gas_accuracy_pct: f64,
    
    // Slippage accuracy
    pub slippage_predictions_count: usize,
    pub slippage_prediction_error_sum: f64,
    pub slippage_prediction_error_squared_sum: f64,
    pub slippage_accuracy_pct: f64,
    
    // Profit accuracy
    pub profit_predictions_count: usize,
    pub profit_prediction_error_sum: f64,
    pub profit_accuracy_pct: f64,
    
    // Success rates
    pub total_opportunities: usize,
    pub successful_executions: usize,
    pub failed_executions: usize,
    pub reverted_executions: usize,
    
    // Model confidence
    pub model_confidence_score: f64,
    pub requires_recalibration: bool,
}

impl ValidationReporter {
    pub fn new(config: Arc<ArbitrageConfig>) -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            config,
            metrics: Arc::new(RwLock::new(ValidationMetrics::default())),
        }
    }

    /// Track a new prediction before execution
    pub async fn track_prediction(
        &self,
        opportunity: &ArbitrageOpportunity,
        gas_estimate: u64,
        slippage_estimate: f64,
        amount_out_predicted: U256,
    ) -> Result<String> {
        let entry = ValidationEntry {
            opportunity_id: opportunity.id.clone(),
            timestamp: Utc::now(),
            gas_predicted: gas_estimate,
            gas_actual: None,
            gas_price_predicted: self.estimate_gas_price().await?,
            gas_price_actual: None,
            slippage_predicted_pct: slippage_estimate,
            slippage_actual_pct: None,
            price_impact_predicted: slippage_estimate, // Simplified
            price_impact_actual: None,
            amount_in: opportunity.required_capital,
            amount_out_predicted,
            amount_out_actual: None,
            profit_predicted_usd: opportunity.profit_usd,
            profit_actual_usd: None,
            execution_status: ExecutionStatus::Pending,
            validation_errors: Vec::new(),
        };

        let mut entries = self.entries.write().await;
        entries.push(entry);
        
        let mut metrics = self.metrics.write().await;
        metrics.total_opportunities += 1;

        info!("📝 Tracking prediction for opportunity {}", opportunity.id);
        Ok(opportunity.id.clone())
    }

    /// Update with actual execution results
    pub async fn update_actual_results(
        &self,
        opportunity_id: &str,
        tx_receipt: &TransactionReceipt,
        amount_out_actual: U256,
        slippage_actual: f64,
    ) -> Result<()> {
        let mut entries = self.entries.write().await;
        
        let entry = entries.iter_mut()
            .find(|e| e.opportunity_id == opportunity_id)
            .ok_or_else(|| anyhow::anyhow!("Entry not found: {}", opportunity_id))?;

        // Update with actual values
        entry.gas_actual = Some(tx_receipt.gas_used.unwrap_or_default().as_u64());
        entry.gas_price_actual = Some(
            tx_receipt.effective_gas_price
                .unwrap_or_default()
                .as_u128() as f64 / 1e9 // Convert to Gwei
        );
        entry.slippage_actual_pct = Some(slippage_actual);
        entry.price_impact_actual = Some(slippage_actual);
        entry.amount_out_actual = Some(amount_out_actual);
        
        // Calculate actual profit
        let gas_cost = entry.gas_actual.unwrap_or(0) as f64 * 
                       entry.gas_price_actual.unwrap_or(0.0) / 1e9;
        let actual_profit = self.calculate_actual_profit(
            entry.amount_in,
            amount_out_actual,
            gas_cost
        ).await?;
        
        entry.profit_actual_usd = Some(actual_profit);
        entry.execution_status = ExecutionStatus::Executed;

        // Update metrics
        self.update_metrics(entry).await?;

        info!("✅ Updated actual results for {}", opportunity_id);
        Ok(())
    }

    /// Mark execution as failed
    pub async fn mark_failed(
        &self,
        opportunity_id: &str,
        reason: String,
    ) -> Result<()> {
        let mut entries = self.entries.write().await;
        
        if let Some(entry) = entries.iter_mut()
            .find(|e| e.opportunity_id == opportunity_id) {
            
            entry.execution_status = ExecutionStatus::Failed(reason.clone());
            entry.validation_errors.push(reason);
            
            let mut metrics = self.metrics.write().await;
            metrics.failed_executions += 1;
        }

        Ok(())
    }

    /// Update aggregate metrics
    async fn update_metrics(&self, entry: &ValidationEntry) -> Result<()> {
        let mut metrics = self.metrics.write().await;

        // Gas accuracy
        if let (Some(actual), predicted) = (entry.gas_actual, entry.gas_predicted) {
            let error = (actual as f64 - predicted as f64) / predicted as f64 * 100.0;
            metrics.gas_predictions_count += 1;
            metrics.gas_prediction_error_sum += error.abs();
            metrics.gas_prediction_error_squared_sum += error * error;
            metrics.gas_accuracy_pct = 100.0 - (metrics.gas_prediction_error_sum / 
                                                metrics.gas_predictions_count as f64);
        }

        // Slippage accuracy
        if let Some(actual) = entry.slippage_actual_pct {
            let error = (actual - entry.slippage_predicted_pct).abs();
            metrics.slippage_predictions_count += 1;
            metrics.slippage_prediction_error_sum += error;
            metrics.slippage_prediction_error_squared_sum += error * error;
            metrics.slippage_accuracy_pct = 100.0 - (metrics.slippage_prediction_error_sum / 
                                                     metrics.slippage_predictions_count as f64);
        }

        // Profit accuracy
        if let Some(actual) = entry.profit_actual_usd {
            let error = ((actual - entry.profit_predicted_usd) / 
                        entry.profit_predicted_usd * 100.0).abs();
            metrics.profit_predictions_count += 1;
            metrics.profit_prediction_error_sum += error;
            metrics.profit_accuracy_pct = 100.0 - (metrics.profit_prediction_error_sum / 
                                                   metrics.profit_predictions_count as f64);
        }

        // Update success rate
        metrics.successful_executions += 1;

        // Calculate model confidence
        metrics.model_confidence_score = self.calculate_confidence_score(&metrics);
        
        // Check if recalibration needed
        metrics.requires_recalibration = 
            metrics.gas_accuracy_pct < 80.0 ||
            metrics.slippage_accuracy_pct < 70.0 ||
            metrics.profit_accuracy_pct < 75.0;

        if metrics.requires_recalibration {
            warn!("⚠️ Model accuracy below threshold - recalibration recommended");
        }

        Ok(())
    }

    /// Calculate overall model confidence score
    fn calculate_confidence_score(&self, metrics: &ValidationMetrics) -> f64 {
        let gas_weight = 0.3;
        let slippage_weight = 0.4;
        let profit_weight = 0.3;

        let gas_score = metrics.gas_accuracy_pct / 100.0;
        let slippage_score = metrics.slippage_accuracy_pct / 100.0;
        let profit_score = metrics.profit_accuracy_pct / 100.0;

        (gas_score * gas_weight + 
         slippage_score * slippage_weight + 
         profit_score * profit_weight) * 100.0
    }

    /// Generate detailed validation report
    pub async fn generate_report(&self) -> Result<ValidationReport> {
        let entries = self.entries.read().await;
        let metrics = self.metrics.read().await;

        // Calculate statistical metrics
        let gas_rmse = if metrics.gas_predictions_count > 0 {
            (metrics.gas_prediction_error_squared_sum / 
             metrics.gas_predictions_count as f64).sqrt()
        } else { 0.0 };

        let slippage_rmse = if metrics.slippage_predictions_count > 0 {
            (metrics.slippage_prediction_error_squared_sum / 
             metrics.slippage_predictions_count as f64).sqrt()
        } else { 0.0 };

        // Identify patterns
        let patterns = self.identify_error_patterns(&entries).await?;

        let report = ValidationReport {
            timestamp: Utc::now(),
            total_entries: entries.len(),
            metrics: metrics.clone(),
            gas_rmse,
            slippage_rmse,
            error_patterns: patterns,
            recommendations: self.generate_recommendations(&metrics),
            detailed_entries: entries.clone(),
        };

        Ok(report)
    }

    /// Identify systematic error patterns
    async fn identify_error_patterns(&self, entries: &[ValidationEntry]) -> Result<Vec<ErrorPattern>> {
        let mut patterns = Vec::new();

        // Check for consistent underestimation of gas
        let gas_underestimations = entries.iter()
            .filter(|e| e.gas_actual.is_some())
            .filter(|e| e.gas_actual.unwrap() > e.gas_predicted)
            .count();

        if gas_underestimations > entries.len() / 2 {
            patterns.push(ErrorPattern {
                pattern_type: "Systematic Gas Underestimation".to_string(),
                frequency: gas_underestimations as f64 / entries.len() as f64,
                impact: "Higher than expected transaction costs".to_string(),
                suggestion: "Increase gas buffer multiplier".to_string(),
            });
        }

        // Check for high slippage on large trades
        let high_slippage_large_trades = entries.iter()
            .filter(|e| e.slippage_actual_pct.unwrap_or(0.0) > e.slippage_predicted_pct * 1.5)
            .filter(|e| e.amount_in > U256::from(10000) * U256::exp10(18))
            .count();

        if high_slippage_large_trades > 3 {
            patterns.push(ErrorPattern {
                pattern_type: "Large Trade Slippage".to_string(),
                frequency: high_slippage_large_trades as f64 / entries.len() as f64,
                impact: "Reduced profits on large positions".to_string(),
                suggestion: "Implement dynamic position sizing based on liquidity".to_string(),
            });
        }

        Ok(patterns)
    }

    /// Generate recommendations based on metrics
    fn generate_recommendations(&self, metrics: &ValidationMetrics) -> Vec<String> {
        let mut recommendations = Vec::new();

        if metrics.gas_accuracy_pct < 80.0 {
            recommendations.push(
                "Gas prediction accuracy is low. Consider updating gas estimation model.".to_string()
            );
        }

        if metrics.slippage_accuracy_pct < 70.0 {
            recommendations.push(
                "Slippage predictions are inaccurate. Review AMM math calculations.".to_string()
            );
        }

        if metrics.failed_executions > metrics.successful_executions / 10 {
            recommendations.push(
                "High failure rate detected. Review opportunity validation logic.".to_string()
            );
        }

        if metrics.model_confidence_score < 75.0 {
            recommendations.push(
                "Overall model confidence is low. Full recalibration recommended.".to_string()
            );
        }

        recommendations
    }

    /// Export report to JSON file
    pub async fn export_report(&self, filepath: &str) -> Result<()> {
        let report = self.generate_report().await?;
        
        let json = serde_json::to_string_pretty(&report)?;
        let mut file = File::create(filepath)?;
        file.write_all(json.as_bytes())?;

        info!("📊 Validation report exported to {}", filepath);
        Ok(())
    }

    /// Helper: Estimate current gas price
    async fn estimate_gas_price(&self) -> Result<f64> {
        // In production, this would query the network
        // For now, return a reasonable estimate
        Ok(30.0) // 30 Gwei
    }

    /// Helper: Calculate actual profit
    async fn calculate_actual_profit(
        &self,
        amount_in: U256,
        amount_out: U256,
        gas_cost: f64,
    ) -> Result<f64> {
        // Simplified calculation - in production would use price oracle
        let value_in = amount_in.as_u128() as f64 / 1e18;
        let value_out = amount_out.as_u128() as f64 / 1e18;
        
        Ok((value_out - value_in) - gas_cost)
    }

    /// Get current metrics summary
    pub async fn get_metrics_summary(&self) -> ValidationMetrics {
        self.metrics.read().await.clone()
    }

    /// Check if model needs recalibration
    pub async fn needs_recalibration(&self) -> bool {
        self.metrics.read().await.requires_recalibration
    }
}

/// Detailed validation report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub timestamp: DateTime<Utc>,
    pub total_entries: usize,
    pub metrics: ValidationMetrics,
    pub gas_rmse: f64,
    pub slippage_rmse: f64,
    pub error_patterns: Vec<ErrorPattern>,
    pub recommendations: Vec<String>,
    pub detailed_entries: Vec<ValidationEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPattern {
    pub pattern_type: String,
    pub frequency: f64,
    pub impact: String,
    pub suggestion: String,
}

impl ValidationReport {
    pub fn print_summary(&self) {
        println!("\n{}", "=".repeat(70));
        println!("📊 VALIDATION REPORT SUMMARY");
        println!("{}", "=".repeat(70));
        println!("Generated: {}", self.timestamp);
        println!("Total Entries: {}", self.total_entries);
        
        println!("\n⛽ Gas Prediction Accuracy:");
        println!("  Accuracy: {:.1}%", self.metrics.gas_accuracy_pct);
        println!("  RMSE: {:.2}", self.gas_rmse);
        println!("  Samples: {}", self.metrics.gas_predictions_count);
        
        println!("\n💧 Slippage Prediction Accuracy:");
        println!("  Accuracy: {:.1}%", self.metrics.slippage_accuracy_pct);
        println!("  RMSE: {:.4}%", self.slippage_rmse);
        println!("  Samples: {}", self.metrics.slippage_predictions_count);
        
        println!("\n💰 Profit Prediction Accuracy:");
        println!("  Accuracy: {:.1}%", self.metrics.profit_accuracy_pct);
        println!("  Samples: {}", self.metrics.profit_predictions_count);
        
        println!("\n📈 Execution Statistics:");
        println!("  Total Opportunities: {}", self.metrics.total_opportunities);
        println!("  Successful: {}", self.metrics.successful_executions);
        println!("  Failed: {}", self.metrics.failed_executions);
        println!("  Reverted: {}", self.metrics.reverted_executions);
        println!("  Success Rate: {:.1}%", 
                (self.metrics.successful_executions as f64 / 
                 self.metrics.total_opportunities.max(1) as f64) * 100.0);
        
        println!("\n🎯 Model Confidence: {:.1}%", self.metrics.model_confidence_score);
        if self.metrics.requires_recalibration {
            println!("⚠️  RECALIBRATION REQUIRED");
        }
        
        if !self.error_patterns.is_empty() {
            println!("\n🔍 Error Patterns Detected:");
            for pattern in &self.error_patterns {
                println!("  - {}: {:.1}% frequency", pattern.pattern_type, pattern.frequency * 100.0);
                println!("    Impact: {}", pattern.impact);
                println!("    Suggestion: {}", pattern.suggestion);
            }
        }
        
        if !self.recommendations.is_empty() {
            println!("\n💡 Recommendations:");
            for rec in &self.recommendations {
                println!("  - {}", rec);
            }
        }
        
        println!("\n{}", "=".repeat(70));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_validation_tracking() {
        let config = Arc::new(ArbitrageConfig::default());
        let reporter = ValidationReporter::new(config);

        let opportunity = ArbitrageOpportunity {
            id: "test_001".to_string(),
            token_path: vec![],
            dex_path: vec!["quickswap".to_string()],
            profit_usd: 10.0,
            profit_ratio: 0.02,
            gas_estimate: 150000,
            required_capital: U256::from(1000),
            complexity_score: 0.5,
            timestamp: 0,
        };

        let id = reporter.track_prediction(
            &opportunity,
            150000,
            1.5,
            U256::from(1020)
        ).await.unwrap();

        assert_eq!(id, "test_001");

        let metrics = reporter.get_metrics_summary().await;
        assert_eq!(metrics.total_opportunities, 1);
    }

    #[tokio::test]
    async fn test_accuracy_calculation() {
        let config = Arc::new(ArbitrageConfig::default());
        let reporter = ValidationReporter::new(config);

        // Track prediction
        let opportunity = ArbitrageOpportunity {
            id: "test_002".to_string(),
            token_path: vec![],
            dex_path: vec!["sushiswap".to_string()],
            profit_usd: 5.0,
            profit_ratio: 0.01,
            gas_estimate: 200000,
            required_capital: U256::from(500),
            complexity_score: 0.3,
            timestamp: 0,
        };

        reporter.track_prediction(
            &opportunity,
            200000,
            2.0,
            U256::from(510)
        ).await.unwrap();

        // Simulate actual results
        let receipt = TransactionReceipt {
            gas_used: Some(U256::from(190000)),
            effective_gas_price: Some(U256::from(30_000_000_000u64)), // 30 Gwei
            ..Default::default()
        };

        reporter.update_actual_results(
            "test_002",
            &receipt,
            U256::from(508),
            2.1
        ).await.unwrap();

        let metrics = reporter.get_metrics_summary().await;
        assert_eq!(metrics.gas_predictions_count, 1);
        assert_eq!(metrics.slippage_predictions_count, 1);
        assert!(metrics.gas_accuracy_pct > 90.0); // Should be ~95% accurate
    }
}