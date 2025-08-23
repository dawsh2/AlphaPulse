# Implementation Guide - Dashboard & System Integration

## Overview
Integrate mempool monitoring with the existing AlphaPulse arbitrage dashboard and execution engine to provide predictive insights and MEV protection.

## Current System Architecture

### Existing Components
```
Frontend (React Dashboard)
    ↓
FastAPI Backend
    ↓
Pool Analysis Service
    ↓
DEX Quoter
    ↓
Blockchain (Ankr RPC)
```

### Enhanced Architecture with Mempool
```
Frontend (React Dashboard + Mempool Panel)
    ↓
FastAPI Backend
    ├── Pool Analysis Service
    ├── DEX Quoter
    └── Mempool Monitor Service (NEW)
         ├── WebSocket Connection (Ankr)
         ├── Transaction Decoder
         ├── Predictive Engine
         └── MEV Detector
```

## Phase 1: Backend Mempool Service

### Service Structure
```python
# backend/services/mempool_monitor/main.py
import asyncio
import websockets
from fastapi import FastAPI, WebSocket
from typing import Dict, List
import json

class MempoolMonitor:
    def __init__(self):
        self.ws_url = "wss://rpc.ankr.com/polygon/ws/{API_KEY}"
        self.pending_txs = []
        self.predictions = {}
        self.connected_clients = []
        
    async def start(self):
        """Initialize WebSocket connection to Ankr"""
        async with websockets.connect(self.ws_url) as ws:
            # Subscribe to pending transactions
            await ws.send(json.dumps({
                "method": "eth_subscribe",
                "params": ["newPendingTransactions"]
            }))
            
            async for message in ws:
                await self.process_transaction(json.loads(message))
    
    async def process_transaction(self, data):
        """Process and analyze pending transaction"""
        if "params" in data and "result" in data["params"]:
            tx_hash = data["params"]["result"]
            
            # Fetch full transaction details
            tx_details = await self.get_transaction_details(tx_hash)
            
            # Analyze transaction
            analysis = await self.analyze_transaction(tx_details)
            
            # Broadcast to connected dashboard clients
            await self.broadcast_to_clients(analysis)
    
    async def analyze_transaction(self, tx):
        """Run predictive analysis on transaction"""
        return {
            "hash": tx["hash"],
            "type": self.classify_transaction(tx),
            "impact": self.predict_impact(tx),
            "mev_opportunity": self.detect_mev(tx),
            "timestamp": time.time()
        }
```

### FastAPI Integration
```python
# backend/app_fastapi.py (additions)
from services.mempool_monitor import MempoolMonitor

mempool_monitor = MempoolMonitor()

@app.on_event("startup")
async def startup_event():
    """Start mempool monitoring on server startup"""
    asyncio.create_task(mempool_monitor.start())

@app.websocket("/ws/mempool")
async def mempool_websocket(websocket: WebSocket):
    """WebSocket endpoint for dashboard mempool feed"""
    await websocket.accept()
    mempool_monitor.connected_clients.append(websocket)
    
    try:
        while True:
            # Keep connection alive
            await websocket.receive_text()
    except:
        mempool_monitor.connected_clients.remove(websocket)

@app.get("/api/mempool/stats")
async def get_mempool_stats():
    """Get current mempool statistics"""
    return {
        "tx_rate": mempool_monitor.get_tx_rate(),
        "pending_count": len(mempool_monitor.pending_txs),
        "predictions": mempool_monitor.predictions,
        "mev_opportunities": mempool_monitor.get_mev_opportunities()
    }
```

## Phase 2: Dashboard Integration

### Enhanced Dashboard Component
```typescript
// frontend/src/dashboard/components/MempoolMonitor.tsx
import React, { useState, useEffect } from 'react';
import { Card } from 'antd';

interface MempoolData {
  txRate: number;
  pendingSwaps: PendingSwap[];
  mevOpportunities: MEVOpportunity[];
  predictions: PricePrediction[];
}

export const MempoolMonitor: React.FC = () => {
  const [mempoolData, setMempoolData] = useState<MempoolData>();
  const [ws, setWs] = useState<WebSocket>();

  useEffect(() => {
    // Connect to mempool WebSocket
    const websocket = new WebSocket('ws://localhost:8000/ws/mempool');
    
    websocket.onmessage = (event) => {
      const data = JSON.parse(event.data);
      setMempoolData(prev => ({
        ...prev,
        ...data
      }));
    };
    
    setWs(websocket);
    
    return () => websocket.close();
  }, []);

  return (
    <div className="mempool-monitor">
      <Card title="Mempool Intelligence" className="mempool-stats">
        <div className="stat-grid">
          <div className="stat">
            <span className="label">TX Rate</span>
            <span className="value">{mempoolData?.txRate.toFixed(1)}/sec</span>
          </div>
          <div className="stat">
            <span className="label">Pending Swaps</span>
            <span className="value">{mempoolData?.pendingSwaps.length}</span>
          </div>
          <div className="stat">
            <span className="label">MEV Opportunities</span>
            <span className="value">{mempoolData?.mevOpportunities.length}</span>
          </div>
        </div>
      </Card>

      <Card title="Price Predictions" className="predictions">
        {mempoolData?.predictions.map(pred => (
          <div key={pred.pair} className="prediction-row">
            <span>{pred.pair}</span>
            <span className={pred.direction === 'UP' ? 'text-green' : 'text-red'}>
              {pred.direction} {pred.confidence.toFixed(1)}%
            </span>
            <span>Impact: {pred.impact.toFixed(2)}%</span>
          </div>
        ))}
      </Card>

      <Card title="MEV Alerts" className="mev-alerts">
        {mempoolData?.mevOpportunities.map(opp => (
          <div key={opp.id} className="mev-opportunity">
            <div className="mev-type">{opp.type}</div>
            <div className="mev-profit">
              Profit: ${opp.estimatedProfit.toFixed(2)}
            </div>
            <button onClick={() => executeM MEV(opp)}>
              Execute
            </button>
          </div>
        ))}
      </Card>
    </div>
  );
};
```

### Enhanced Arbitrage Component
```typescript
// frontend/src/dashboard/components/DeFiArbitrage.tsx (additions)
interface EnhancedOpportunity extends ArbitrageOpportunity {
  mempool: {
    pendingImpact: number;
    sandwichRisk: 'LOW' | 'MEDIUM' | 'HIGH';
    predictedSlippage: number;
    competingBots: number;
    recommendation: string;
  };
}

const analyzeWithMempool = async (opportunity: ArbitrageOpportunity) => {
  // Fetch mempool analysis for this opportunity
  const mempoolAnalysis = await fetch('/api/mempool/analyze', {
    method: 'POST',
    body: JSON.stringify({
      tokenIn: opportunity.tokenIn,
      tokenOut: opportunity.tokenOut,
      pools: opportunity.pools
    })
  }).then(r => r.json());
  
  return {
    ...opportunity,
    mempool: mempoolAnalysis
  };
};

// In the component render
<Card className="opportunity-card">
  <div className="mempool-indicators">
    <Badge 
      status={opportunity.mempool.sandwichRisk === 'LOW' ? 'success' : 'warning'}
      text={`Sandwich Risk: ${opportunity.mempool.sandwichRisk}`}
    />
    <Tooltip title="Predicted price impact from pending transactions">
      <span>Pending Impact: {opportunity.mempool.pendingImpact.toFixed(2)}%</span>
    </Tooltip>
    {opportunity.mempool.competingBots > 0 && (
      <Alert 
        message={`${opportunity.mempool.competingBots} competing bots detected`}
        type="warning"
      />
    )}
  </div>
  
  <div className="recommendation">
    <Icon type="bulb" />
    {opportunity.mempool.recommendation}
  </div>
</Card>
```

## Phase 3: Execution Engine Integration

### MEV-Protected Execution
```rust
// backend/services/capital_arb_bot/src/mempool_protection.rs
use ethers::prelude::*;

pub struct MempoolProtectedExecutor {
    mempool_monitor: MempoolMonitor,
    executor: ArbExecutor,
}

impl MempoolProtectedExecutor {
    pub async fn execute_with_protection(
        &self,
        opportunity: &ArbitrageOpportunity
    ) -> Result<TxHash, Error> {
        // Check mempool for threats
        let mempool_analysis = self.mempool_monitor
            .analyze_current_mempool()
            .await?;
        
        // Detect sandwich risk
        if mempool_analysis.sandwich_risk > 0.7 {
            // Use private mempool or split trade
            return self.execute_private_mempool(opportunity).await;
        }
        
        // Optimize gas based on mempool competition
        let optimal_gas = self.calculate_optimal_gas(
            &mempool_analysis.competing_transactions
        );
        
        // Check for front-running bots
        if mempool_analysis.known_bots.len() > 0 {
            // Add randomization to avoid detection
            tokio::time::sleep(Duration::from_millis(rand::random::<u64>() % 1000)).await;
        }
        
        // Execute with optimized parameters
        self.executor.execute_with_gas(opportunity, optimal_gas).await
    }
    
    fn calculate_optimal_gas(&self, competing_txs: &[Transaction]) -> U256 {
        // Slightly outbid the highest competing transaction
        let max_gas = competing_txs
            .iter()
            .map(|tx| tx.gas_price.unwrap_or_default())
            .max()
            .unwrap_or_default();
        
        max_gas + U256::from(1_000_000_000) // Add 1 gwei
    }
}
```

### Predictive Positioning
```rust
// backend/services/capital_arb_bot/src/predictive_executor.rs
pub struct PredictiveExecutor {
    predictor: PricePredictor,
    executor: ArbExecutor,
}

impl PredictiveExecutor {
    pub async fn execute_predictive(
        &self,
        mempool_data: &MempoolSnapshot
    ) -> Result<Vec<TxHash>, Error> {
        let mut executed_txs = Vec::new();
        
        // Analyze pending swaps
        for pending_swap in &mempool_data.pending_swaps {
            // Predict price impact
            let impact = self.predictor.predict_impact(pending_swap)?;
            
            // If significant impact predicted, position accordingly
            if impact.price_change > 0.005 { // 0.5% threshold
                let position = self.calculate_position(impact);
                
                // Submit transaction to execute after the pending swap
                let tx = self.executor
                    .execute_after(pending_swap.hash, position)
                    .await?;
                    
                executed_txs.push(tx);
            }
        }
        
        Ok(executed_txs)
    }
}
```

## Phase 4: Real-Time Data Pipeline

### Message Protocol Extension
```rust
// backend/protocol/src/mempool_messages.rs
#[derive(Serialize, Deserialize)]
pub enum MempoolMessage {
    PendingSwap {
        hash: H256,
        dex: String,
        token_in: Address,
        token_out: Address,
        amount_in: U256,
        predicted_impact: f64,
    },
    MEVOpportunity {
        opportunity_type: MEVType,
        target_tx: H256,
        estimated_profit: U256,
        confidence: f64,
        execution_params: ExecutionParams,
    },
    SandwichAlert {
        risk_level: RiskLevel,
        attacking_bot: Option<Address>,
        recommended_action: String,
    },
    LiquidationPrediction {
        protocol: String,
        position: Address,
        health_factor: f64,
        liquidation_price: U256,
        collateral_value: U256,
    },
}
```

### Stream Processing
```python
# backend/services/stream_processor.py
import asyncio
from aiokafka import AIOKafkaProducer, AIOKafkaConsumer

class MempoolStreamProcessor:
    def __init__(self):
        self.producer = AIOKafkaProducer(
            bootstrap_servers='localhost:9092'
        )
        self.consumer = AIOKafkaConsumer(
            'mempool-transactions',
            bootstrap_servers='localhost:9092'
        )
    
    async def process_stream(self):
        """Process mempool transaction stream"""
        await self.consumer.start()
        
        async for msg in self.consumer:
            tx = json.loads(msg.value)
            
            # Run analysis pipeline
            analysis = await self.analyze_pipeline(tx)
            
            # Publish results to different topics
            if analysis['is_swap']:
                await self.producer.send('mempool-swaps', analysis)
            
            if analysis['mev_opportunity']:
                await self.producer.send('mev-opportunities', analysis)
            
            if analysis['liquidation_risk']:
                await self.producer.send('liquidation-alerts', analysis)
```

## Phase 5: Monitoring & Alerting

### Grafana Dashboard Config
```yaml
# monitoring/dashboards/mempool.yaml
dashboard:
  title: "Mempool Monitoring"
  panels:
    - title: "Transaction Rate"
      type: graph
      targets:
        - expr: rate(mempool_transactions_total[1m])
    
    - title: "MEV Opportunities"
      type: stat
      targets:
        - expr: sum(mempool_mev_opportunities_total)
    
    - title: "Prediction Accuracy"
      type: gauge
      targets:
        - expr: mempool_prediction_accuracy
    
    - title: "Sandwich Attack Attempts"
      type: counter
      targets:
        - expr: increase(mempool_sandwich_attempts[1h])
```

### Alert Rules
```yaml
# monitoring/alerts/mempool_alerts.yaml
groups:
  - name: mempool
    rules:
      - alert: HighSandwichRisk
        expr: mempool_sandwich_risk > 0.8
        for: 1m
        annotations:
          summary: "High sandwich attack risk detected"
          
      - alert: MempoolDisconnected
        expr: up{job="mempool_monitor"} == 0
        for: 30s
        annotations:
          summary: "Mempool monitor disconnected"
          
      - alert: LowPredictionAccuracy
        expr: mempool_prediction_accuracy < 0.7
        for: 10m
        annotations:
          summary: "Prediction accuracy below threshold"
```

## Testing Strategy

### Integration Tests
```python
# tests/test_mempool_integration.py
import pytest
from unittest.mock import Mock, patch

@pytest.fixture
async def mempool_monitor():
    monitor = MempoolMonitor()
    await monitor.initialize()
    return monitor

async def test_mempool_prediction_accuracy(mempool_monitor):
    """Test prediction accuracy with historical data"""
    historical_data = load_historical_mempool_data()
    
    predictions = []
    actuals = []
    
    for block in historical_data:
        # Make prediction
        pred = await mempool_monitor.predict(block['mempool'])
        predictions.append(pred)
        
        # Compare with actual
        actuals.append(block['actual'])
    
    accuracy = calculate_accuracy(predictions, actuals)
    assert accuracy > 0.8  # 80% accuracy threshold

async def test_sandwich_detection(mempool_monitor):
    """Test sandwich attack detection"""
    # Create mock sandwich scenario
    victim_tx = create_mock_swap_tx(amount=10000)
    
    sandwich = await mempool_monitor.detect_sandwich(victim_tx)
    
    assert sandwich is not None
    assert sandwich['profit'] > 0
    assert sandwich['confidence'] > 0.7
```

## Deployment Checklist

### Pre-Production
- [ ] Load test WebSocket connection (target: 100+ tx/sec)
- [ ] Validate prediction accuracy on testnet
- [ ] Test MEV protection mechanisms
- [ ] Verify dashboard real-time updates
- [ ] Test failover and reconnection logic

### Production Rollout
- [ ] Deploy mempool monitor service
- [ ] Enable monitoring and alerting
- [ ] Deploy dashboard updates
- [ ] Configure rate limiting
- [ ] Enable circuit breakers
- [ ] Document runbooks

### Post-Deployment
- [ ] Monitor prediction accuracy
- [ ] Track MEV capture rate
- [ ] Analyze missed opportunities
- [ ] Tune prediction models
- [ ] Optimize gas strategies

## Configuration

### Environment Variables
```bash
# .env
ANKR_API_KEY=e6fac469b91ea8fd98406aca0820653ae6fe5c2400f44819450f6022dd2792e2
MEMPOOL_WS_URL=wss://rpc.ankr.com/polygon/ws/${ANKR_API_KEY}
MEMPOOL_BUFFER_SIZE=10000
MEMPOOL_WORKER_COUNT=10
PREDICTION_CONFIDENCE_THRESHOLD=0.7
MEV_MIN_PROFIT=100
SANDWICH_RISK_THRESHOLD=0.8
```

### Service Configuration
```yaml
# config/mempool.yaml
mempool_monitor:
  connection:
    url: ${MEMPOOL_WS_URL}
    reconnect_delay: 1
    max_reconnect_delay: 60
    ping_interval: 30
  
  processing:
    buffer_size: ${MEMPOOL_BUFFER_SIZE}
    worker_count: ${MEMPOOL_WORKER_COUNT}
    batch_size: 100
  
  prediction:
    confidence_threshold: ${PREDICTION_CONFIDENCE_THRESHOLD}
    lookback_window: 1000
    update_frequency: 100
  
  mev:
    min_profit: ${MEV_MIN_PROFIT}
    gas_buffer: 1.2
    private_mempool: false
```