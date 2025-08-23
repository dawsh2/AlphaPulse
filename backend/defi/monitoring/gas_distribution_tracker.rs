// Gas Distribution Tracking System for Huff Migration
// Tracks gas usage distributions using percentiles instead of single expected values
// This prevents false positive alerts and provides more accurate anomaly detection

use std::collections::{HashMap, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use anyhow::{Result, ensure};

/// Gas distribution with percentile tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasDistribution {
    pub p50: u64,  // Median
    pub p90: u64,  // 90th percentile
    pub p95: u64,  // 95th percentile  
    pub p99: u64,  // 99th percentile
    pub min: u64,
    pub max: u64,
    pub mean: f64,
    pub std_dev: f64,
    pub samples: VecDeque<u64>,
    pub last_updated: u64,
    pub window_size: usize,
}

impl GasDistribution {
    pub fn new(window_size: usize) -> Self {
        Self {
            p50: 0,
            p90: 0,
            p95: 0,
            p99: 0,
            min: u64::MAX,
            max: 0,
            mean: 0.0,
            std_dev: 0.0,
            samples: VecDeque::with_capacity(window_size),
            last_updated: 0,
            window_size,
        }
    }
    
    /// Record new gas measurement and update distribution
    pub fn record_gas_usage(&mut self, gas_used: u64) {
        self.samples.push_back(gas_used);
        
        // Maintain rolling window
        while self.samples.len() > self.window_size {
            self.samples.pop_front();
        }
        
        self.update_statistics();
        self.last_updated = current_timestamp();
    }
    
    /// Update all statistical measures
    fn update_statistics(&mut self) {
        if self.samples.is_empty() {
            return;
        }
        
        // Calculate percentiles
        let mut sorted: Vec<u64> = self.samples.iter().copied().collect();
        sorted.sort_unstable();
        
        let len = sorted.len();
        self.p50 = sorted[len * 50 / 100];
        self.p90 = sorted[len * 90 / 100];
        self.p95 = sorted[len * 95 / 100];
        self.p99 = sorted[(len * 99 / 100).min(len - 1)];
        
        self.min = sorted[0];
        self.max = sorted[len - 1];
        
        // Calculate mean
        let sum: u64 = sorted.iter().sum();
        self.mean = sum as f64 / len as f64;
        
        // Calculate standard deviation
        let variance: f64 = sorted.iter()
            .map(|&x| {
                let diff = x as f64 - self.mean;
                diff * diff
            })
            .sum::<f64>() / len as f64;
        
        self.std_dev = variance.sqrt();
    }
    
    /// Check if gas usage is anomalous using statistical methods
    pub fn is_gas_anomalous(&self, actual_gas: u64) -> AnomalyDetection {
        if self.samples.len() < 10 {
            return AnomalyDetection::InsufficientData;
        }
        
        // Method 1: Percentile-based detection
        if actual_gas > self.p99 {
            let severity = if actual_gas > self.p99 * 2 {
                AnomalySeverity::Critical
            } else if actual_gas > (self.p99 as f64 * 1.5) as u64 {
                AnomalySeverity::High
            } else {
                AnomalySeverity::Medium
            };
            
            return AnomalyDetection::Anomalous {
                severity,
                method: DetectionMethod::Percentile,
                deviation: ((actual_gas as f64 - self.p99 as f64) / self.p99 as f64) * 100.0,
            };
        }
        
        // Method 2: Standard deviation detection (Z-score)
        let z_score = (actual_gas as f64 - self.mean) / self.std_dev;
        
        if z_score.abs() > 3.0 {
            let severity = match z_score.abs() {
                3.0..=4.0 => AnomalySeverity::Low,
                4.0..=5.0 => AnomalySeverity::Medium,
                5.0..=6.0 => AnomalySeverity::High,
                _ => AnomalySeverity::Critical,
            };
            
            return AnomalyDetection::Anomalous {
                severity,
                method: DetectionMethod::ZScore,
                deviation: z_score * 100.0 / 3.0, // Normalize to percentage
            };
        }
        
        // Method 3: IQR-based detection
        let iqr = self.p95 - self.p50;
        let lower_bound = self.p50.saturating_sub(iqr * 3 / 2);
        let upper_bound = self.p95 + iqr * 3 / 2;
        
        if actual_gas < lower_bound || actual_gas > upper_bound {
            let deviation = if actual_gas < lower_bound {
                ((lower_bound - actual_gas) as f64 / self.p50 as f64) * 100.0
            } else {
                ((actual_gas - upper_bound) as f64 / self.p95 as f64) * 100.0
            };
            
            return AnomalyDetection::Anomalous {
                severity: AnomalySeverity::Low,
                method: DetectionMethod::IQR,
                deviation,
            };
        }
        
        AnomalyDetection::Normal
    }
    
    /// Get confidence interval for gas usage
    pub fn get_confidence_interval(&self, confidence_level: f64) -> (u64, u64) {
        if self.samples.is_empty() {
            return (0, 0);
        }
        
        // Use percentiles for confidence interval
        let (lower_percentile, upper_percentile) = match confidence_level {
            0.90 => (self.p50.saturating_sub((self.p90 - self.p50) / 2), self.p90),
            0.95 => (self.p50.saturating_sub((self.p95 - self.p50) / 2), self.p95),
            0.99 => (self.p50.saturating_sub((self.p99 - self.p50) / 2), self.p99),
            _ => (self.min, self.max),
        };
        
        (lower_percentile, upper_percentile)
    }
    
    /// Predict gas usage for new scenario
    pub fn predict_gas_usage(&self, scenario_features: &ScenarioFeatures) -> GasPrediction {
        if self.samples.is_empty() {
            return GasPrediction::default();
        }
        
        // Base prediction on median
        let mut predicted = self.p50 as f64;
        
        // Adjust for complexity
        predicted *= 1.0 + (scenario_features.path_length as f64 - 2.0) * 0.15;
        
        // Adjust for trade size
        if scenario_features.trade_size_usd > 10000.0 {
            predicted *= 1.1; // Large trade overhead
        }
        
        // Adjust for network congestion
        predicted *= scenario_features.congestion_multiplier;
        
        GasPrediction {
            expected: predicted as u64,
            lower_bound: (predicted * 0.8) as u64,
            upper_bound: (predicted * 1.2) as u64,
            confidence: self.calculate_prediction_confidence(),
        }
    }
    
    fn calculate_prediction_confidence(&self) -> f64 {
        if self.samples.len() < 10 {
            return 0.0;
        }
        
        // Confidence based on sample size and consistency
        let sample_confidence = (self.samples.len() as f64 / self.window_size as f64).min(1.0);
        let consistency_confidence = 1.0 - (self.std_dev / self.mean).min(1.0);
        
        (sample_confidence * 0.3 + consistency_confidence * 0.7).min(0.95)
    }
}

/// Anomaly detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomalyDetection {
    Normal,
    InsufficientData,
    Anomalous {
        severity: AnomalySeverity,
        method: DetectionMethod,
        deviation: f64, // Percentage deviation
    },
}

/// Anomaly severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomalySeverity {
    Low,      // Minor deviation, likely normal variation
    Medium,   // Notable deviation, worth investigating
    High,     // Significant deviation, possible issue
    Critical, // Extreme deviation, immediate attention needed
}

/// Detection method used
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DetectionMethod {
    Percentile, // Based on percentile thresholds
    ZScore,     // Based on standard deviations
    IQR,        // Based on interquartile range
}

/// Features for scenario-based prediction
#[derive(Debug, Clone)]
pub struct ScenarioFeatures {
    pub path_length: usize,
    pub trade_size_usd: f64,
    pub congestion_multiplier: f64,
    pub is_flash_loan: bool,
    pub uses_v3: bool,
}

/// Gas usage prediction
#[derive(Debug, Clone, Default)]
pub struct GasPrediction {
    pub expected: u64,
    pub lower_bound: u64,
    pub upper_bound: u64,
    pub confidence: f64,
}

/// Multi-scenario gas tracker
pub struct GasDistributionTracker {
    distributions: HashMap<String, GasDistribution>,
    global_distribution: GasDistribution,
    anomaly_log: VecDeque<AnomalyEvent>,
    config: TrackerConfig,
}

/// Configuration for gas tracker
#[derive(Debug, Clone)]
pub struct TrackerConfig {
    pub window_size: usize,
    pub anomaly_log_size: usize,
    pub auto_adjust_thresholds: bool,
    pub min_samples_for_detection: usize,
}

impl Default for TrackerConfig {
    fn default() -> Self {
        Self {
            window_size: 1000,
            anomaly_log_size: 100,
            auto_adjust_thresholds: true,
            min_samples_for_detection: 10,
        }
    }
}

/// Anomaly event record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyEvent {
    pub timestamp: u64,
    pub scenario: String,
    pub actual_gas: u64,
    pub expected_range: (u64, u64),
    pub severity: AnomalySeverity,
    pub method: DetectionMethod,
    pub deviation_percentage: f64,
    pub context: HashMap<String, String>,
}

impl GasDistributionTracker {
    pub fn new(config: TrackerConfig) -> Self {
        Self {
            distributions: HashMap::new(),
            global_distribution: GasDistribution::new(config.window_size),
            anomaly_log: VecDeque::with_capacity(config.anomaly_log_size),
            config,
        }
    }
    
    /// Record gas usage for a specific scenario
    pub fn record_scenario_gas(
        &mut self,
        scenario: &str,
        gas_used: u64,
        context: HashMap<String, String>,
    ) -> AnomalyDetection {
        // Update scenario-specific distribution
        let distribution = self.distributions
            .entry(scenario.to_string())
            .or_insert_with(|| GasDistribution::new(self.config.window_size));
        
        distribution.record_gas_usage(gas_used);
        
        // Update global distribution
        self.global_distribution.record_gas_usage(gas_used);
        
        // Check for anomalies
        let detection = distribution.is_gas_anomalous(gas_used);
        
        // Log anomalies
        if let AnomalyDetection::Anomalous { severity, method, deviation } = &detection {
            self.log_anomaly(AnomalyEvent {
                timestamp: current_timestamp(),
                scenario: scenario.to_string(),
                actual_gas: gas_used,
                expected_range: distribution.get_confidence_interval(0.95),
                severity: severity.clone(),
                method: method.clone(),
                deviation_percentage: *deviation,
                context,
            });
        }
        
        detection
    }
    
    /// Log anomaly event
    fn log_anomaly(&mut self, event: AnomalyEvent) {
        self.anomaly_log.push_back(event);
        
        // Maintain log size
        while self.anomaly_log.len() > self.config.anomaly_log_size {
            self.anomaly_log.pop_front();
        }
    }
    
    /// Get distribution for a scenario
    pub fn get_scenario_distribution(&self, scenario: &str) -> Option<&GasDistribution> {
        self.distributions.get(scenario)
    }
    
    /// Get global gas distribution
    pub fn get_global_distribution(&self) -> &GasDistribution {
        &self.global_distribution
    }
    
    /// Get recent anomalies
    pub fn get_recent_anomalies(&self, limit: usize) -> Vec<AnomalyEvent> {
        self.anomaly_log.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }
    
    /// Analyze gas trends across scenarios
    pub fn analyze_trends(&self) -> TrendAnalysis {
        let mut scenario_trends = HashMap::new();
        
        for (scenario, distribution) in &self.distributions {
            if distribution.samples.len() < 20 {
                continue;
            }
            
            // Calculate trend using linear regression on recent samples
            let recent_samples: Vec<(f64, f64)> = distribution.samples
                .iter()
                .rev()
                .take(20)
                .enumerate()
                .map(|(i, &gas)| (i as f64, gas as f64))
                .collect();
            
            let trend = calculate_linear_trend(&recent_samples);
            
            scenario_trends.insert(scenario.clone(), trend);
        }
        
        // Calculate global trend
        let global_trend = if self.global_distribution.samples.len() >= 20 {
            let recent_samples: Vec<(f64, f64)> = self.global_distribution.samples
                .iter()
                .rev()
                .take(50)
                .enumerate()
                .map(|(i, &gas)| (i as f64, gas as f64))
                .collect();
            
            calculate_linear_trend(&recent_samples)
        } else {
            GasTrend::Stable
        };
        
        TrendAnalysis {
            global_trend,
            scenario_trends,
            anomaly_rate: self.calculate_anomaly_rate(),
        }
    }
    
    /// Calculate recent anomaly rate
    fn calculate_anomaly_rate(&self) -> f64 {
        let recent_window = 3600; // Last hour
        let current_time = current_timestamp();
        
        let recent_anomalies = self.anomaly_log.iter()
            .filter(|e| current_time - e.timestamp < recent_window)
            .count();
        
        let total_recent_samples = self.global_distribution.samples.len().min(100);
        
        if total_recent_samples > 0 {
            recent_anomalies as f64 / total_recent_samples as f64
        } else {
            0.0
        }
    }
    
    /// Generate comprehensive gas report
    pub fn generate_report(&self) -> GasReport {
        let mut scenario_summaries = HashMap::new();
        
        for (scenario, distribution) in &self.distributions {
            scenario_summaries.insert(scenario.clone(), DistributionSummary {
                sample_count: distribution.samples.len(),
                p50: distribution.p50,
                p90: distribution.p90,
                p95: distribution.p95,
                p99: distribution.p99,
                mean: distribution.mean,
                std_dev: distribution.std_dev,
                min: distribution.min,
                max: distribution.max,
            });
        }
        
        GasReport {
            timestamp: current_timestamp(),
            global_summary: DistributionSummary {
                sample_count: self.global_distribution.samples.len(),
                p50: self.global_distribution.p50,
                p90: self.global_distribution.p90,
                p95: self.global_distribution.p95,
                p99: self.global_distribution.p99,
                mean: self.global_distribution.mean,
                std_dev: self.global_distribution.std_dev,
                min: self.global_distribution.min,
                max: self.global_distribution.max,
            },
            scenario_summaries,
            recent_anomalies: self.get_recent_anomalies(10),
            trend_analysis: self.analyze_trends(),
        }
    }
}

/// Gas usage trend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GasTrend {
    Increasing(f64),  // Slope of increase
    Decreasing(f64),  // Slope of decrease
    Stable,           // No significant trend
    Volatile,         // High variance
}

/// Trend analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    pub global_trend: GasTrend,
    pub scenario_trends: HashMap<String, GasTrend>,
    pub anomaly_rate: f64,
}

/// Distribution summary for reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionSummary {
    pub sample_count: usize,
    pub p50: u64,
    pub p90: u64,
    pub p95: u64,
    pub p99: u64,
    pub mean: f64,
    pub std_dev: f64,
    pub min: u64,
    pub max: u64,
}

/// Comprehensive gas report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasReport {
    pub timestamp: u64,
    pub global_summary: DistributionSummary,
    pub scenario_summaries: HashMap<String, DistributionSummary>,
    pub recent_anomalies: Vec<AnomalyEvent>,
    pub trend_analysis: TrendAnalysis,
}

/// Calculate linear trend from data points
fn calculate_linear_trend(points: &[(f64, f64)]) -> GasTrend {
    if points.len() < 2 {
        return GasTrend::Stable;
    }
    
    let n = points.len() as f64;
    let sum_x: f64 = points.iter().map(|(x, _)| x).sum();
    let sum_y: f64 = points.iter().map(|(_, y)| y).sum();
    let sum_xy: f64 = points.iter().map(|(x, y)| x * y).sum();
    let sum_x2: f64 = points.iter().map(|(x, _)| x * x).sum();
    
    let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x * sum_x);
    
    // Calculate R-squared for trend strength
    let mean_y = sum_y / n;
    let ss_tot: f64 = points.iter().map(|(_, y)| (y - mean_y).powi(2)).sum();
    let ss_res: f64 = points.iter()
        .map(|(x, y)| {
            let predicted = slope * x + (sum_y - slope * sum_x) / n;
            (y - predicted).powi(2)
        })
        .sum();
    
    let r_squared = 1.0 - (ss_res / ss_tot);
    
    // Determine trend based on slope and R-squared
    if r_squared < 0.3 {
        GasTrend::Volatile
    } else if slope.abs() < 0.01 {
        GasTrend::Stable
    } else if slope > 0.0 {
        GasTrend::Increasing(slope)
    } else {
        GasTrend::Decreasing(slope.abs())
    }
}

/// Get current timestamp in seconds
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_gas_distribution() {
        let mut distribution = GasDistribution::new(100);
        
        // Add sample data
        for i in 100000..100100 {
            distribution.record_gas_usage(i);
        }
        
        assert!(distribution.p50 > 0);
        assert!(distribution.p90 > distribution.p50);
        assert!(distribution.p95 > distribution.p90);
        assert!(distribution.p99 > distribution.p95);
    }
    
    #[test]
    fn test_anomaly_detection() {
        let mut distribution = GasDistribution::new(100);
        
        // Add normal samples
        for _ in 0..50 {
            distribution.record_gas_usage(100000);
        }
        
        // Test normal case
        let detection = distribution.is_gas_anomalous(100000);
        matches!(detection, AnomalyDetection::Normal);
        
        // Test anomalous case
        let detection = distribution.is_gas_anomalous(500000);
        matches!(detection, AnomalyDetection::Anomalous { .. });
    }
    
    #[test]
    fn test_trend_calculation() {
        let increasing_points = vec![
            (0.0, 100.0),
            (1.0, 110.0),
            (2.0, 120.0),
            (3.0, 130.0),
            (4.0, 140.0),
        ];
        
        let trend = calculate_linear_trend(&increasing_points);
        matches!(trend, GasTrend::Increasing(_));
        
        let stable_points = vec![
            (0.0, 100.0),
            (1.0, 101.0),
            (2.0, 99.0),
            (3.0, 100.0),
            (4.0, 100.0),
        ];
        
        let trend = calculate_linear_trend(&stable_points);
        matches!(trend, GasTrend::Stable);
    }
}