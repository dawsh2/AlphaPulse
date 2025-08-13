/**
 * CryptoDashboard - Crypto Market Making, Arbitrage & Flash Opportunities Dashboard
 * Monitors DEX/CEX arbitrage, AMM opportunities, flash loans, and MEV
 */
import React, { useState, useEffect, useRef } from 'react';
import { io, Socket } from 'socket.io-client';
import { Line, Bar } from 'react-chartjs-2';
import styles from './CryptoDashboard.module.css';

interface ArbitrageOpportunity {
  id: string;
  type: 'CEX-CEX' | 'DEX-CEX' | 'DEX-DEX' | 'Flash';
  pair: string;
  buyExchange: string;
  sellExchange: string;
  spread: number;
  spreadPercent: number;
  volume: number;
  profit: number;
  gasEstimate?: number;
  confidence: number;
  timestamp: Date;
}

interface MEVOpportunity {
  type: 'sandwich' | 'liquidation' | 'JIT';
  protocol: string;
  estimatedProfit: number;
  gasPrice: number;
  competition: number;
  successRate: number;
}

interface AMMPosition {
  protocol: string;
  pair: string;
  liquidity: number;
  feesEarned24h: number;
  impermanentLoss: number;
  apy: number;
}

interface FlashLoanMetrics {
  availableLiquidity: {
    aave: number;
    balancer: number;
    dydx: number;
  };
  recentLoans: number;
  successRate: number;
  avgProfit: number;
}

interface CryptoDashboardProps {
  className?: string;
}

const CryptoDashboard: React.FC<CryptoDashboardProps> = ({ className }) => {
  const socketRef = useRef<Socket | null>(null);
  const [isConnected, setIsConnected] = useState(false);
  
  // Arbitrage State
  const [arbitrageOps, setArbitrageOps] = useState<ArbitrageOpportunity[]>([]);
  const [profitHistory, setProfitHistory] = useState<{time: Date, value: number}[]>([]);
  const [totalProfit24h, setTotalProfit24h] = useState(0);
  
  // MEV State
  const [mevOps, setMevOps] = useState<MEVOpportunity[]>([]);
  const [gasPrice, setGasPrice] = useState(0);
  const [pendingTxCount, setPendingTxCount] = useState(0);
  
  // AMM State
  const [ammPositions, setAmmPositions] = useState<AMMPosition[]>([]);
  const [totalLiquidity, setTotalLiquidity] = useState(0);
  const [totalFees24h, setTotalFees24h] = useState(0);
  
  // Flash Loan State
  const [flashMetrics, setFlashMetrics] = useState<FlashLoanMetrics>({
    availableLiquidity: { aave: 0, balancer: 0, dydx: 0 },
    recentLoans: 0,
    successRate: 0,
    avgProfit: 0
  });
  
  // L2 Orderbook Depth
  const [orderbookDepth, setOrderbookDepth] = useState<{
    exchange: string;
    symbol: string;
    bidDepth: number;
    askDepth: number;
    spread: number;
  }[]>([]);

  useEffect(() => {
    const socket = io('http://localhost:5001', {
      transports: ['websocket'],
      path: '/socket.io/'
    });
    
    socketRef.current = socket;
    
    socket.on('connect', () => {
      console.log('✅ Connected to crypto metrics WebSocket');
      setIsConnected(true);
      socket.emit('subscribe_crypto_monitoring');
    });
    
    socket.on('disconnect', () => {
      console.log('❌ Disconnected from crypto metrics WebSocket');
      setIsConnected(false);
    });
    
    // Arbitrage updates
    socket.on('arbitrage_update', (data: any) => {
      setArbitrageOps(data.opportunities || []);
      setTotalProfit24h(data.totalProfit24h || 0);
    });
    
    // MEV updates
    socket.on('mev_update', (data: any) => {
      setMevOps(data.opportunities || []);
      setGasPrice(data.gasPrice || 0);
      setPendingTxCount(data.pendingCount || 0);
    });
    
    // AMM updates
    socket.on('amm_update', (data: any) => {
      setAmmPositions(data.positions || []);
      setTotalLiquidity(data.totalLiquidity || 0);
      setTotalFees24h(data.totalFees24h || 0);
    });
    
    // Flash loan updates
    socket.on('flash_update', (data: FlashLoanMetrics) => {
      setFlashMetrics(data);
    });
    
    // Orderbook updates
    socket.on('orderbook_depth_update', (data: any) => {
      setOrderbookDepth(data);
    });
    
    // Profit history
    socket.on('profit_history_update', (data: any) => {
      setProfitHistory(data);
    });
    
    return () => {
      if (socket.connected) {
        socket.emit('unsubscribe_crypto_monitoring');
        socket.disconnect();
      }
    };
  }, []);
  
  // Mock data for demonstration
  useEffect(() => {
    // Simulate some data for visualization
    const mockArbitrage: ArbitrageOpportunity[] = [
      {
        id: '1',
        type: 'CEX-CEX',
        pair: 'BTC/USD',
        buyExchange: 'Kraken',
        sellExchange: 'Coinbase',
        spread: 37.65,
        spreadPercent: 0.03,
        volume: 10000,
        profit: -343.06, // Negative due to fees
        confidence: 45,
        timestamp: new Date()
      },
      {
        id: '2',
        type: 'DEX-CEX',
        pair: 'ETH/USD',
        buyExchange: 'Uniswap',
        sellExchange: 'Binance',
        spread: 25.00,
        spreadPercent: 0.67,
        volume: 5000,
        profit: 12.50,
        gasEstimate: 45,
        confidence: 72,
        timestamp: new Date()
      },
      {
        id: '3',
        type: 'Flash',
        pair: 'USDC/USDT',
        buyExchange: 'Curve',
        sellExchange: 'Balancer',
        spread: 0.15,
        spreadPercent: 0.015,
        volume: 1000000,
        profit: 1450,
        gasEstimate: 120,
        confidence: 95,
        timestamp: new Date()
      }
    ];
    
    setArbitrageOps(mockArbitrage);
    
    // Mock MEV opportunities
    const mockMEV: MEVOpportunity[] = [
      {
        type: 'liquidation',
        protocol: 'Aave',
        estimatedProfit: 2500,
        gasPrice: 45,
        competition: 8,
        successRate: 15
      },
      {
        type: 'sandwich',
        protocol: 'Uniswap',
        estimatedProfit: 350,
        gasPrice: 45,
        competition: 25,
        successRate: 5
      }
    ];
    
    setMevOps(mockMEV);
    
    // Mock AMM positions
    const mockAMM: AMMPosition[] = [
      {
        protocol: 'Uniswap V3',
        pair: 'ETH/USDC',
        liquidity: 50000,
        feesEarned24h: 125.50,
        impermanentLoss: -45.20,
        apy: 42.5
      },
      {
        protocol: 'Curve',
        pair: 'USDC/USDT/DAI',
        liquidity: 100000,
        feesEarned24h: 85.30,
        impermanentLoss: -2.10,
        apy: 18.2
      }
    ];
    
    setAmmPositions(mockAMM);
    
    // Mock flash loan metrics
    setFlashMetrics({
      availableLiquidity: {
        aave: 500000000,
        balancer: 100000000,
        dydx: 999999999
      },
      recentLoans: 247,
      successRate: 12.5,
      avgProfit: 145
    });
  }, []);
  
  // Chart data
  const profitChartData = {
    labels: profitHistory.map(() => ''),
    datasets: [{
      data: profitHistory.map(d => d.value),
      borderColor: '#00ff88',
      backgroundColor: 'rgba(0, 255, 136, 0.1)',
      fill: true,
      tension: 0.4,
      borderWidth: 2
    }]
  };
  
  const chartOptions = {
    responsive: true,
    maintainAspectRatio: false,
    plugins: {
      legend: { display: false },
      tooltip: {
        backgroundColor: 'rgba(0, 0, 0, 0.9)',
        borderColor: '#00ff88',
        borderWidth: 1
      }
    },
    scales: {
      x: { display: false },
      y: {
        display: true,
        grid: { color: 'rgba(0, 255, 136, 0.1)' },
        ticks: { color: '#00ff88', font: { size: 10 } }
      }
    }
  };
  
  return (
    <div className={`${styles.cryptoDashboard} ${className}`}>
      {/* Header */}
      <div className={styles.dashboardHeader}>
        <h1>Crypto Trading Opportunities</h1>
        <div className={styles.connectionStatus}>
          <span className={`${styles.dot} ${isConnected ? styles.connected : ''}`} />
          <span>{isConnected ? 'Live' : 'Offline'}</span>
        </div>
      </div>
      
      {/* Key Metrics Bar */}
      <div className={styles.metricsBar}>
        <div className={styles.metricCard}>
          <span className={styles.metricLabel}>24h Profit</span>
          <span className={`${styles.metricValue} ${totalProfit24h >= 0 ? styles.positive : styles.negative}`}>
            ${Math.abs(totalProfit24h).toLocaleString()}
          </span>
        </div>
        <div className={styles.metricCard}>
          <span className={styles.metricLabel}>Active Arbs</span>
          <span className={styles.metricValue}>{arbitrageOps.length}</span>
        </div>
        <div className={styles.metricCard}>
          <span className={styles.metricLabel}>Gas Price</span>
          <span className={styles.metricValue}>{gasPrice || 45} gwei</span>
        </div>
        <div className={styles.metricCard}>
          <span className={styles.metricLabel}>MEV Opps</span>
          <span className={styles.metricValue}>{mevOps.length}</span>
        </div>
        <div className={styles.metricCard}>
          <span className={styles.metricLabel}>Total Liquidity</span>
          <span className={styles.metricValue}>${(totalLiquidity / 1000000).toFixed(1)}M</span>
        </div>
      </div>
      
      {/* Main Grid */}
      <div className={styles.mainGrid}>
        {/* Arbitrage Opportunities */}
        <div className={styles.panel}>
          <h2>Arbitrage Opportunities</h2>
          <div className={styles.arbTable}>
            <div className={styles.tableHeader}>
              <span>Type</span>
              <span>Pair</span>
              <span>Spread</span>
              <span>Profit</span>
              <span>Confidence</span>
            </div>
            {arbitrageOps.map(op => (
              <div key={op.id} className={`${styles.tableRow} ${op.profit > 0 ? styles.profitable : ''}`}>
                <span className={styles.arbType}>{op.type}</span>
                <span>{op.pair}</span>
                <span>{op.spreadPercent.toFixed(3)}%</span>
                <span className={op.profit >= 0 ? styles.positive : styles.negative}>
                  ${op.profit.toFixed(2)}
                </span>
                <span>
                  <div className={styles.confidenceBar}>
                    <div 
                      className={styles.confidenceFill} 
                      style={{ 
                        width: `${op.confidence}%`,
                        backgroundColor: op.confidence > 70 ? '#00ff88' : op.confidence > 40 ? '#ffaa00' : '#ff4444'
                      }}
                    />
                  </div>
                </span>
              </div>
            ))}
          </div>
        </div>
        
        {/* Flash Loan Liquidity */}
        <div className={styles.panel}>
          <h2>Flash Loan Liquidity</h2>
          <div className={styles.flashMetrics}>
            <div className={styles.liquidityPool}>
              <h3>Aave</h3>
              <div className={styles.poolAmount}>${(flashMetrics.availableLiquidity.aave / 1000000).toFixed(0)}M</div>
              <div className={styles.poolBar}>
                <div className={styles.poolFill} style={{ width: '75%' }} />
              </div>
            </div>
            <div className={styles.liquidityPool}>
              <h3>Balancer</h3>
              <div className={styles.poolAmount}>${(flashMetrics.availableLiquidity.balancer / 1000000).toFixed(0)}M</div>
              <div className={styles.poolBar}>
                <div className={styles.poolFill} style={{ width: '45%' }} />
              </div>
            </div>
            <div className={styles.liquidityPool}>
              <h3>dYdX</h3>
              <div className={styles.poolAmount}>Unlimited</div>
              <div className={styles.poolBar}>
                <div className={styles.poolFill} style={{ width: '100%' }} />
              </div>
            </div>
          </div>
          <div className={styles.flashStats}>
            <div>Recent Loans: {flashMetrics.recentLoans}</div>
            <div>Success Rate: {flashMetrics.successRate}%</div>
            <div>Avg Profit: ${flashMetrics.avgProfit}</div>
          </div>
        </div>
        
        {/* MEV Opportunities */}
        <div className={styles.panel}>
          <h2>MEV Opportunities</h2>
          <div className={styles.mevList}>
            {mevOps.map((mev, idx) => (
              <div key={idx} className={styles.mevCard}>
                <div className={styles.mevHeader}>
                  <span className={styles.mevType}>{mev.type}</span>
                  <span className={styles.mevProtocol}>{mev.protocol}</span>
                </div>
                <div className={styles.mevMetrics}>
                  <div>Est. Profit: ${mev.estimatedProfit}</div>
                  <div>Competition: {mev.competition} bots</div>
                  <div>Success: {mev.successRate}%</div>
                </div>
              </div>
            ))}
          </div>
        </div>
        
        {/* AMM Positions */}
        <div className={styles.panel}>
          <h2>AMM Positions</h2>
          <div className={styles.ammList}>
            {ammPositions.map((pos, idx) => (
              <div key={idx} className={styles.ammCard}>
                <div className={styles.ammHeader}>
                  <span>{pos.protocol}</span>
                  <span className={styles.ammPair}>{pos.pair}</span>
                </div>
                <div className={styles.ammStats}>
                  <div>Liquidity: ${(pos.liquidity / 1000).toFixed(1)}k</div>
                  <div>24h Fees: ${pos.feesEarned24h.toFixed(2)}</div>
                  <div className={pos.impermanentLoss < 0 ? styles.negative : ''}>
                    IL: ${Math.abs(pos.impermanentLoss).toFixed(2)}
                  </div>
                  <div className={styles.apy}>APY: {pos.apy}%</div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
      
      {/* Profit Chart */}
      <div className={styles.chartPanel}>
        <h2>24h Profit History</h2>
        <div className={styles.chartContainer}>
          {profitHistory.length > 0 && (
            <Line data={profitChartData} options={chartOptions} />
          )}
        </div>
      </div>
    </div>
  );
};

export default CryptoDashboard;