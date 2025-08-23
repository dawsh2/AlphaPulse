/// Test execution path with enhanced gas estimation
use crate::{
    test_opportunities::OpportunityTester,
    execution_interface::{ExecutionInterface, ExecutionStatus, ChannelExecutionInterface},
    gas_estimation::{GasCalculator, ContractType}
};
use anyhow::Result;
use rust_decimal_macros::dec;
use tracing::{info, warn};
use tokio::sync::mpsc;

pub struct ExecutionTester {
    opportunity_tester: OpportunityTester,
    executor: ChannelExecutionInterface,
    gas_calculator: GasCalculator,
}

impl ExecutionTester {
    pub fn new() -> Self {
        let (executor, mut rx) = ChannelExecutionInterface::new();
        
        // Spawn a background task to simulate execution results
        tokio::spawn(async move {
            while let Some(opportunity) = rx.recv().await {
                info!("🔄 Simulating execution of opportunity: {}", opportunity.id);
                // In real implementation, this would connect to contracts
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });
        
        let opportunity_tester = OpportunityTester::new();
        let gas_calculator = GasCalculator::new(
            dec!(25), // 25 gwei gas price
            dec!(0.8), // $0.8 MATIC price
        );
        
        Self {
            opportunity_tester,
            executor,
            gas_calculator,
        }
    }
    
    /// Test the complete execution pipeline
    pub async fn test_execution_pipeline(&self) -> Result<()> {
        info!("🚀 Testing complete execution pipeline");
        
        // 1. Generate test opportunities
        let v2_opp = self.opportunity_tester.create_test_opportunity()?;
        let v3_opp = self.opportunity_tester.create_v3_test_opportunity()?;
        
        // 2. Test gas estimation for different contract types
        let contract_types = [
            ContractType::HuffMEV,
            ContractType::HuffExtreme, 
            ContractType::HuffUltra,
        ];
        
        for contract_type in &contract_types {
            let gas_cost_v2 = self.gas_calculator.calculate_execution_cost_usd(
                *contract_type,
                false // V2 is not complex
            );
            
            let gas_cost_v3 = self.gas_calculator.calculate_execution_cost_usd(
                *contract_type,
                true // V3 is complex
            );
            
            info!("⛽ {:?} Gas Costs:", contract_type);
            info!("   V2: ${:.6}", gas_cost_v2);
            info!("   V3: ${:.6}", gas_cost_v3);
        }
        
        // 3. Check profitability after gas costs
        self.check_profitability_with_gas(&v2_opp, "V2").await?;
        self.check_profitability_with_gas(&v3_opp, "V3").await?;
        
        // 4. Test execution interface
        info!("🔧 Testing execution interface:");
        
        let status_v2 = self.executor.submit_opportunity(&v2_opp).await?;
        let status_v3 = self.executor.submit_opportunity(&v3_opp).await?;
        
        info!("   V2 execution status: {:?}", status_v2);
        info!("   V3 execution status: {:?}", status_v3);
        
        // 5. Test queue management
        let queue_stats = self.executor.get_queue_stats().await?;
        info!("   Queue stats: {:?}", queue_stats);
        
        info!("✅ Execution pipeline test complete!");
        Ok(())
    }
    
    async fn check_profitability_with_gas(&self, opp: &crate::ArbitrageOpportunity, label: &str) -> Result<()> {
        let profit_after_gas = opp.profit_usd - opp.gas_cost_estimate;
        
        if profit_after_gas > dec!(0) {
            info!("✅ {} Opportunity PROFITABLE: ${:.4} (after ${:.6} gas)", 
                  label, profit_after_gas, opp.gas_cost_estimate);
            
            // In a real system, this would trigger execution
            info!("   -> Would execute: {} {} for {} {}", 
                  opp.amount_in, opp.token_in, opp.amount_out, opp.token_out);
        } else {
            warn!("❌ {} Opportunity NOT PROFITABLE: ${:.4} (gas: ${:.6})",
                  label, profit_after_gas, opp.gas_cost_estimate);
        }
        
        Ok(())
    }
    
    /// Test execution under different network conditions
    pub async fn test_execution_conditions(&self) -> Result<()> {
        info!("🌐 Testing execution under different network conditions");
        
        // Test different gas prices
        let gas_prices = vec![dec!(10), dec!(25), dec!(50), dec!(100)]; // gwei
        let matic_price = dec!(0.8);
        
        for gas_price in gas_prices {
            let gas_calc = GasCalculator::new(gas_price, matic_price);
            
            let mev_cost = gas_calc.calculate_execution_cost_usd(ContractType::HuffMEV, false);
            let ultra_cost = gas_calc.calculate_execution_cost_usd(ContractType::HuffUltra, true);
            
            info!("   {}gwei gas -> MEV: ${:.6}, Ultra: ${:.6}", gas_price, mev_cost, ultra_cost);
            
            // Check if our test opportunities would still be profitable
            let v2_opp = self.opportunity_tester.create_test_opportunity()?;
            let adjusted_profit = v2_opp.profit_usd - mev_cost;
            
            if adjusted_profit > dec!(0) {
                info!("     ✅ V2 still profitable at {}gwei: ${:.4}", gas_price, adjusted_profit);
            } else {
                info!("     ❌ V2 unprofitable at {}gwei: ${:.4}", gas_price, adjusted_profit);
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_execution_pipeline() {
        tracing_subscriber::fmt::try_init().ok(); // Ok to fail if already initialized
        
        let tester = ExecutionTester::new();
        tester.test_execution_pipeline().await.unwrap();
        tester.test_execution_conditions().await.unwrap();
    }
}