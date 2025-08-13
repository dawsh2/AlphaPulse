/**
 * GreeksDashboard - Market Greeks & Options Analytics Dashboard
 * Displays Delta, Gamma, Theta, Vega, implied volatility, and options flow
 */
import React, { useState, useEffect, useRef } from 'react';
import { io, Socket } from 'socket.io-client';
import { Line, Bar, Scatter } from 'react-chartjs-2';
import styles from './GreeksDashboard.module.css';

interface OptionGreeks {
  symbol: string;
  strike: number;
  expiry: string;
  delta: number;
  gamma: number;
  theta: number;
  vega: number;
  rho: number;
  iv: number; // Implied Volatility
}

interface VolatilitySurface {
  strikes: number[];
  expiries: string[];
  surface: number[][];
}

interface OptionsFlow {
  time: Date;
  symbol: string;
  strike: number;
  expiry: string;
  type: 'CALL' | 'PUT';
  side: 'BUY' | 'SELL';
  volume: number;
  premium: number;
  sentiment: 'BULLISH' | 'BEARISH' | 'NEUTRAL';
}

interface MarketStructure {
  putCallRatio: number;
  maxPain: number;
  gammaExposure: number;
  vixLevel: number;
  skew: number;
  termStructure: { expiry: string; iv: number }[];
}

interface GreeksDashboardProps {
  className?: string;
}

const GreeksDashboard: React.FC<GreeksDashboardProps> = ({ className }) => {
  const socketRef = useRef<Socket | null>(null);
  const [isConnected, setIsConnected] = useState(false);
  
  // Greeks State
  const [portfolioGreeks, setPortfolioGreeks] = useState({
    totalDelta: 0,
    totalGamma: 0,
    totalTheta: 0,
    totalVega: 0,
    totalRho: 0
  });
  
  const [topOptions, setTopOptions] = useState<OptionGreeks[]>([]);
  const [volSurface, setVolSurface] = useState<VolatilitySurface | null>(null);
  const [optionsFlow, setOptionsFlow] = useState<OptionsFlow[]>([]);
  const [marketStructure, setMarketStructure] = useState<MarketStructure>({
    putCallRatio: 0,
    maxPain: 0,
    gammaExposure: 0,
    vixLevel: 0,
    skew: 0,
    termStructure: []
  });
  
  // Historical data for charts
  const [ivHistory, setIvHistory] = useState<{time: Date, value: number}[]>([]);
  const [gammaHistory, setGammaHistory] = useState<{time: Date, value: number}[]>([]);
  const [flowHistory, setFlowHistory] = useState<{time: Date, calls: number, puts: number}[]>([]);

  useEffect(() => {
    // Mock data for demonstration
    const mockGreeks: OptionGreeks[] = [
      {
        symbol: 'SPY',
        strike: 580,
        expiry: '2025-02-21',
        delta: 0.52,
        gamma: 0.018,
        theta: -0.85,
        vega: 12.5,
        rho: 8.2,
        iv: 18.5
      },
      {
        symbol: 'QQQ',
        strike: 500,
        expiry: '2025-02-21',
        delta: 0.48,
        gamma: 0.021,
        theta: -0.92,
        vega: 14.2,
        rho: 7.8,
        iv: 22.3
      },
      {
        symbol: 'NVDA',
        strike: 140,
        expiry: '2025-01-17',
        delta: 0.65,
        gamma: 0.025,
        theta: -1.2,
        vega: 8.5,
        rho: 4.2,
        iv: 45.2
      },
      {
        symbol: 'TSLA',
        strike: 410,
        expiry: '2025-01-17',
        delta: 0.33,
        gamma: 0.012,
        theta: -2.1,
        vega: 15.8,
        rho: 3.5,
        iv: 52.8
      }
    ];
    
    setTopOptions(mockGreeks);
    
    // Calculate portfolio Greeks
    const totalDelta = mockGreeks.reduce((sum, opt) => sum + opt.delta * 100, 0);
    const totalGamma = mockGreeks.reduce((sum, opt) => sum + opt.gamma * 100, 0);
    const totalTheta = mockGreeks.reduce((sum, opt) => sum + opt.theta * 100, 0);
    const totalVega = mockGreeks.reduce((sum, opt) => sum + opt.vega * 100, 0);
    
    setPortfolioGreeks({
      totalDelta,
      totalGamma,
      totalTheta,
      totalVega,
      totalRho: 0
    });
    
    // Mock options flow
    const mockFlow: OptionsFlow[] = [
      {
        time: new Date(),
        symbol: 'SPY',
        strike: 580,
        expiry: '2025-02-21',
        type: 'CALL',
        side: 'BUY',
        volume: 5000,
        premium: 250000,
        sentiment: 'BULLISH'
      },
      {
        time: new Date(),
        symbol: 'QQQ',
        strike: 490,
        expiry: '2025-01-17',
        type: 'PUT',
        side: 'BUY',
        volume: 3000,
        premium: 180000,
        sentiment: 'BEARISH'
      },
      {
        time: new Date(),
        symbol: 'NVDA',
        strike: 145,
        expiry: '2025-01-17',
        type: 'CALL',
        side: 'SELL',
        volume: 1000,
        premium: 85000,
        sentiment: 'NEUTRAL'
      }
    ];
    
    setOptionsFlow(mockFlow);
    
    // Mock market structure
    setMarketStructure({
      putCallRatio: 0.85,
      maxPain: 575,
      gammaExposure: -2.5e9,
      vixLevel: 16.2,
      skew: -1.2,
      termStructure: [
        { expiry: '1W', iv: 15.2 },
        { expiry: '2W', iv: 16.1 },
        { expiry: '1M', iv: 17.5 },
        { expiry: '2M', iv: 18.2 },
        { expiry: '3M', iv: 19.1 }
      ]
    });
    
    // Generate mock historical data
    const now = new Date();
    const mockIvHistory = Array.from({ length: 30 }, (_, i) => ({
      time: new Date(now.getTime() - (29 - i) * 24 * 60 * 60 * 1000),
      value: 15 + Math.random() * 10
    }));
    setIvHistory(mockIvHistory);
    
    const mockGammaHistory = Array.from({ length: 30 }, (_, i) => ({
      time: new Date(now.getTime() - (29 - i) * 24 * 60 * 60 * 1000),
      value: -3e9 + Math.random() * 2e9
    }));
    setGammaHistory(mockGammaHistory);
  }, []);
  
  // IV Chart
  const ivChartData = {
    labels: ivHistory.map(() => ''),
    datasets: [{
      label: 'Implied Volatility',
      data: ivHistory.map(d => d.value),
      borderColor: '#00d4ff',
      backgroundColor: 'rgba(0, 212, 255, 0.1)',
      fill: true,
      tension: 0.4,
      borderWidth: 2
    }]
  };
  
  // Gamma Exposure Chart
  const gammaChartData = {
    labels: gammaHistory.map(() => ''),
    datasets: [{
      label: 'Gamma Exposure',
      data: gammaHistory.map(d => d.value / 1e9),
      borderColor: marketStructure.gammaExposure < 0 ? '#ff4444' : '#00ff88',
      backgroundColor: marketStructure.gammaExposure < 0 ? 'rgba(255, 68, 68, 0.1)' : 'rgba(0, 255, 136, 0.1)',
      fill: true,
      tension: 0.4,
      borderWidth: 2
    }]
  };
  
  // Term Structure Chart
  const termStructureData = {
    labels: marketStructure.termStructure.map(t => t.expiry),
    datasets: [{
      label: 'IV Term Structure',
      data: marketStructure.termStructure.map(t => t.iv),
      borderColor: '#ffd43b',
      backgroundColor: 'rgba(255, 212, 59, 0.1)',
      fill: false,
      tension: 0.2,
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
        borderColor: '#00d4ff',
        borderWidth: 1
      }
    },
    scales: {
      x: { 
        display: true,
        grid: { color: 'rgba(0, 212, 255, 0.1)' },
        ticks: { color: '#00d4ff', font: { size: 10 } }
      },
      y: {
        display: true,
        grid: { color: 'rgba(0, 212, 255, 0.1)' },
        ticks: { color: '#00d4ff', font: { size: 10 } }
      }
    }
  };
  
  return (
    <div className={`${styles.greeksDashboard} ${className}`}>
      {/* Header */}
      <div className={styles.dashboardHeader}>
        <h1>Market Greeks & Options Analytics</h1>
        <div className={styles.vixIndicator}>
          <span className={styles.vixLabel}>VIX</span>
          <span className={`${styles.vixValue} ${marketStructure.vixLevel > 20 ? styles.high : styles.normal}`}>
            {marketStructure.vixLevel.toFixed(1)}
          </span>
        </div>
      </div>
      
      {/* Portfolio Greeks Summary */}
      <div className={styles.greeksBar}>
        <div className={styles.greekCard}>
          <span className={styles.greekLabel}>Δ Delta</span>
          <span className={`${styles.greekValue} ${portfolioGreeks.totalDelta >= 0 ? styles.positive : styles.negative}`}>
            {portfolioGreeks.totalDelta >= 0 ? '+' : ''}{portfolioGreeks.totalDelta.toFixed(0)}
          </span>
        </div>
        <div className={styles.greekCard}>
          <span className={styles.greekLabel}>Γ Gamma</span>
          <span className={styles.greekValue}>
            {portfolioGreeks.totalGamma.toFixed(1)}
          </span>
        </div>
        <div className={styles.greekCard}>
          <span className={styles.greekLabel}>Θ Theta</span>
          <span className={`${styles.greekValue} ${styles.negative}`}>
            {portfolioGreeks.totalTheta.toFixed(0)}
          </span>
        </div>
        <div className={styles.greekCard}>
          <span className={styles.greekLabel}>ν Vega</span>
          <span className={styles.greekValue}>
            {portfolioGreeks.totalVega.toFixed(0)}
          </span>
        </div>
        <div className={styles.greekCard}>
          <span className={styles.greekLabel}>P/C Ratio</span>
          <span className={`${styles.greekValue} ${marketStructure.putCallRatio > 1 ? styles.bearish : styles.bullish}`}>
            {marketStructure.putCallRatio.toFixed(2)}
          </span>
        </div>
        <div className={styles.greekCard}>
          <span className={styles.greekLabel}>Max Pain</span>
          <span className={styles.greekValue}>
            ${marketStructure.maxPain}
          </span>
        </div>
      </div>
      
      {/* Main Grid */}
      <div className={styles.mainGrid}>
        {/* Options Chain */}
        <div className={styles.panel}>
          <h2>Top Options Positions</h2>
          <div className={styles.optionsTable}>
            <div className={styles.tableHeader}>
              <span>Symbol</span>
              <span>Strike</span>
              <span>Exp</span>
              <span>Δ</span>
              <span>Γ</span>
              <span>Θ</span>
              <span>IV</span>
            </div>
            {topOptions.map((opt, idx) => (
              <div key={idx} className={styles.tableRow}>
                <span className={styles.symbol}>{opt.symbol}</span>
                <span>${opt.strike}</span>
                <span className={styles.expiry}>{opt.expiry.slice(5)}</span>
                <span className={opt.delta >= 0.5 ? styles.itm : styles.otm}>
                  {opt.delta.toFixed(2)}
                </span>
                <span>{opt.gamma.toFixed(3)}</span>
                <span className={styles.negative}>{opt.theta.toFixed(2)}</span>
                <span className={styles.iv}>{opt.iv.toFixed(1)}%</span>
              </div>
            ))}
          </div>
        </div>
        
        {/* Options Flow */}
        <div className={styles.panel}>
          <h2>Unusual Options Activity</h2>
          <div className={styles.flowList}>
            {optionsFlow.map((flow, idx) => (
              <div key={idx} className={`${styles.flowCard} ${styles[flow.sentiment.toLowerCase()]}`}>
                <div className={styles.flowHeader}>
                  <span className={styles.flowSymbol}>{flow.symbol}</span>
                  <span className={`${styles.flowType} ${styles[flow.type.toLowerCase()]}`}>
                    {flow.type}
                  </span>
                  <span className={`${styles.flowSide} ${styles[flow.side.toLowerCase()]}`}>
                    {flow.side}
                  </span>
                </div>
                <div className={styles.flowDetails}>
                  <span>${flow.strike} {flow.expiry.slice(5)}</span>
                  <span>{flow.volume.toLocaleString()} × ${(flow.premium / flow.volume).toFixed(2)}</span>
                  <span className={styles.flowPremium}>${(flow.premium / 1000).toFixed(0)}K</span>
                </div>
              </div>
            ))}
          </div>
        </div>
        
        {/* Gamma Exposure */}
        <div className={styles.panel}>
          <h2>Gamma Exposure (Billions)</h2>
          <div className={styles.chartContainer}>
            <Line data={gammaChartData} options={chartOptions} />
          </div>
          <div className={styles.gammaStats}>
            <span>Current: ${(marketStructure.gammaExposure / 1e9).toFixed(2)}B</span>
            <span className={marketStructure.gammaExposure < 0 ? styles.negative : styles.positive}>
              {marketStructure.gammaExposure < 0 ? 'Negative Gamma' : 'Positive Gamma'}
            </span>
          </div>
        </div>
        
        {/* IV Surface */}
        <div className={styles.panel}>
          <h2>Volatility Surface</h2>
          <div className={styles.chartContainer}>
            <Line data={ivChartData} options={chartOptions} />
          </div>
          <div className={styles.skewIndicator}>
            <span>Skew: {marketStructure.skew.toFixed(2)}</span>
            <span className={marketStructure.skew < -1 ? styles.bearish : styles.neutral}>
              {marketStructure.skew < -1 ? 'Put Skew' : 'Balanced'}
            </span>
          </div>
        </div>
      </div>
      
      {/* Term Structure */}
      <div className={styles.termPanel}>
        <h2>Volatility Term Structure</h2>
        <div className={styles.termChartContainer}>
          <Line data={termStructureData} options={chartOptions} />
        </div>
      </div>
    </div>
  );
};

export default GreeksDashboard;