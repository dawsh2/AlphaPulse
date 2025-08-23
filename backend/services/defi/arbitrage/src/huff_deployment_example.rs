// Example integration of Huff deployment with MEV protection
// This demonstrates how the canary deployment system feeds into MEV protection decisions

use anyhow::Result;
use tracing::{info, warn};

use crate::mev_protection::{HuffMevIntegration, HuffDeploymentStatus, HuffMetrics};

/// Example function showing how Huff deployment updates affect MEV protection
pub async fn demonstrate_huff_mev_integration() -> Result<()> {
    info!("🚀 Demonstrating Huff MEV integration...");
    
    // Initialize the integration system
    let mut huff_mev = HuffMevIntegration::new(120); // 120ms execution speed
    
    // Initial state - no Huff deployment
    let initial_report = huff_mev.generate_integration_report()?;
    info!("Initial MEV state: deployment={:?}, health={:?}", 
          initial_report.current_deployment_status, initial_report.integration_health);
    
    // Simulate starting canary deployment at 1%
    info!("📈 Starting Huff canary deployment at 1%...");
    let initial_metrics = HuffMetrics {
        measured_huff_gas: 46_500,      // Slightly above target
        measured_solidity_gas: 185_000,
        gas_improvement_ratio: 185_000.0 / 46_500.0, // ~4.0x
        success_rate: 0.99,
        total_executions: 25,
        last_updated: 1640995200, // Unix timestamp
    };
    
    let canary_report = huff_mev.update_deployment_status(
        HuffDeploymentStatus::Canary(1), 
        Some(initial_metrics.clone())
    ).await?;
    
    info!("Canary 1% MEV impact:");
    info!("  - Break-even improvement: {:.2}x", canary_report.mev_protection_impact.break_even_improvement);
    info!("  - Profitable range expansion: {:.1}%", canary_report.mev_protection_impact.profitable_range_expansion);
    info!("  - Protection usage change: {:.1}%", canary_report.mev_protection_impact.protection_usage_change);
    
    // Test MEV decision with small profit
    let test_profit = 15.0; // $15 USD
    let mev_protection = huff_mev.get_mev_protection();
    let decision = mev_protection.should_use_protection(test_profit, 3, 120);
    info!("MEV decision for ${:.0} profit: use_protection={}, threat_prob={:.3}", 
          test_profit, decision.use_protection, decision.threat_probability);
    
    // Simulate canary expansion to 25%
    info!("📈 Expanding canary deployment to 25%...");
    let expanded_metrics = HuffMetrics {
        measured_huff_gas: 45_800,      // Improving with more usage
        measured_solidity_gas: 185_000,
        gas_improvement_ratio: 185_000.0 / 45_800.0, // ~4.0x
        success_rate: 0.995,
        total_executions: 150,
        last_updated: 1640995800,
    };
    
    let expanded_report = huff_mev.update_deployment_status(
        HuffDeploymentStatus::Canary(25), 
        Some(expanded_metrics.clone())
    ).await?;
    
    info!("Canary 25% MEV impact:");
    info!("  - Break-even improvement: {:.2}x", expanded_report.mev_protection_impact.break_even_improvement);
    info!("  - MEV advantage factor: {:.3}", expanded_report.current_deployment.mev_advantage.mev_advantage_factor);
    
    // Test same profit decision with expanded deployment
    let expanded_decision = huff_mev.get_mev_protection().should_use_protection(test_profit, 3, 120);
    info!("MEV decision for ${:.0} profit at 25% deployment: use_protection={}, threat_prob={:.3}", 
          test_profit, expanded_decision.use_protection, expanded_decision.threat_probability);
    
    // Simulate full deployment
    info!("🎯 Completing full Huff deployment...");
    let full_metrics = HuffMetrics {
        measured_huff_gas: 44_200,      // Achieved target efficiency
        measured_solidity_gas: 185_000,
        gas_improvement_ratio: 185_000.0 / 44_200.0, // ~4.2x
        success_rate: 0.998,
        total_executions: 500,
        last_updated: 1640996400,
    };
    
    let full_report = huff_mev.update_deployment_status(
        HuffDeploymentStatus::FullDeployment, 
        Some(full_metrics.clone())
    ).await?;
    
    info!("Full deployment MEV impact:");
    info!("  - Break-even improvement: {:.2}x", full_report.mev_protection_impact.break_even_improvement);
    info!("  - Threat reduction factor: {:.2}x", full_report.mev_protection_impact.threat_reduction_factor);
    info!("  - Protection usage change: {:.1}%", full_report.mev_protection_impact.protection_usage_change);
    
    // Test final MEV decision
    let final_decision = huff_mev.get_mev_protection().should_use_protection(test_profit, 3, 120);
    info!("MEV decision for ${:.0} profit with full Huff: use_protection={}, threat_prob={:.3}", 
          test_profit, final_decision.use_protection, final_decision.threat_probability);
    
    // Show advantage summary
    let advantage = huff_mev.get_mev_protection().get_huff_advantage_summary();
    info!("Final Huff advantage summary:");
    info!("  - Deployment: {}%", advantage.deployment_percentage);
    info!("  - Efficiency multiplier: {:.2}x", advantage.efficiency_multiplier);
    info!("  - Current gas usage: {} (vs {} Solidity)", advantage.current_gas_usage, 185_000);
    info!("  - MEV advantage factor: {:.3}", advantage.mev_advantage_factor);
    if let Some(improvement) = advantage.target_break_even_improvement {
        info!("  - Break-even improvement: {:.1}% better", (1.0 - improvement) * 100.0);
    }
    
    // Generate final recommendations
    info!("📋 Recommendations:");
    for (i, rec) in full_report.recommendations.iter().enumerate() {
        info!("  {}. {}", i + 1, rec);
    }
    
    // Simulate rollback scenario
    info!("⚠️  Simulating emergency rollback...");
    let rollback_report = huff_mev.update_deployment_status(
        HuffDeploymentStatus::Rollback, 
        None
    ).await?;
    
    let rollback_decision = huff_mev.get_mev_protection().should_use_protection(test_profit, 3, 120);
    info!("MEV decision after rollback: use_protection={}, threat_prob={:.3}", 
          rollback_decision.use_protection, rollback_decision.threat_probability);
    
    warn!("Rollback recommendations:");
    for rec in &rollback_report.recommendations {
        warn!("  - {}", rec);
    }
    
    info!("✅ Huff MEV integration demonstration complete!");
    
    Ok(())
}

/// Example of how the integration would be used in the arbitrage bot
pub async fn example_arbitrage_with_huff_mev(
    profit_usd: f64,
    path_complexity: usize,
    huff_mev: &HuffMevIntegration,
) -> Result<bool> {
    // Get MEV protection decision using current Huff deployment status
    let mev_protection = huff_mev.get_mev_protection();
    let decision = mev_protection.should_use_protection(profit_usd, path_complexity, 100);
    
    info!("Arbitrage opportunity: ${:.2} profit, complexity={}, MEV protection={}", 
          profit_usd, path_complexity, decision.use_protection);
    
    if decision.use_protection {
        info!("🔒 Using MEV protection - submitting via private mempool");
        info!("   Reasoning: {}", decision.reasoning);
        info!("   Threat probability: {:.1}%", decision.threat_probability * 100.0);
        info!("   Expected MEV loss: ${:.2}", decision.expected_mev_loss);
        info!("   Protection cost: ${:.2}", decision.protection_cost);
        
        // Here you would submit via Flashbots or similar
        // submit_via_private_mempool(tx).await?;
        
    } else {
        info!("🌐 Using public mempool - MEV risk acceptable");
        info!("   Reasoning: {}", decision.reasoning);
        
        // Here you would submit via public mempool
        // submit_via_public_mempool(tx).await?;
    }
    
    Ok(decision.use_protection)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_huff_mev_integration_demo() {
        // Initialize tracing for the test
        let _ = tracing_subscriber::fmt::try_init();
        
        let result = demonstrate_huff_mev_integration().await;
        assert!(result.is_ok(), "Integration demo should complete successfully");
    }

    #[tokio::test]
    async fn test_arbitrage_decision_progression() {
        let mut huff_mev = HuffMevIntegration::new(100);
        let test_profit = 20.0;
        
        // Test decision progression through deployment phases
        
        // Phase 1: No deployment
        let no_huff_decision = example_arbitrage_with_huff_mev(test_profit, 2, &huff_mev).await.unwrap();
        
        // Phase 2: Canary deployment
        let metrics = HuffMetrics {
            measured_huff_gas: 45_000,
            measured_solidity_gas: 180_000,
            gas_improvement_ratio: 4.0,
            success_rate: 0.99,
            total_executions: 100,
            last_updated: 1640995200,
        };
        
        huff_mev.update_deployment_status(HuffDeploymentStatus::Canary(50), Some(metrics.clone())).await.unwrap();
        let canary_decision = example_arbitrage_with_huff_mev(test_profit, 2, &huff_mev).await.unwrap();
        
        // Phase 3: Full deployment
        huff_mev.update_deployment_status(HuffDeploymentStatus::FullDeployment, Some(metrics)).await.unwrap();
        let full_decision = example_arbitrage_with_huff_mev(test_profit, 2, &huff_mev).await.unwrap();
        
        // The deployment should affect MEV protection decisions
        // (Specific assertion logic would depend on the exact profit amount and market conditions)
        println!("Decision progression: no_huff={}, canary={}, full={}", 
                 no_huff_decision, canary_decision, full_decision);
    }
}