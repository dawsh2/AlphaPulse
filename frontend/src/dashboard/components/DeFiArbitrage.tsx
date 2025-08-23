import React, { useState, useEffect, useMemo } from 'react';
import './DeFiArbitrage.css';

interface ArbitrageOpportunity {
  id: string;
  timestamp: number;
  pair: string;
  token0Symbol: string;
  token1Symbol: string;
  buyPool: string;
  sellPool: string;
  buyExchange: string;
  sellExchange: string;
  buyPrice: number;
  sellPrice: number;
  tradeSize: number;
  grossProfit: number;
  profitPercent: number;
  gasFee: number;
  dexFees: number;
  slippageCost: number;
  totalFees: number;
  netProfit: number;
  netProfitPercent: number;
  executable: boolean;
  recommendation: string;
  buySlippage?: number;
  sellSlippage?: number;
  confidence?: number;
}

interface ScannerStatus {
  isConnected: boolean;
  lastMessageTime: number;
  totalOpportunities: number;
  executableOpportunities: number;
  scannerHealth: 'healthy' | 'degraded' | 'offline';
}

export const DeFiArbitrage: React.FC = () => {
  const [opportunities, setOpportunities] = useState<ArbitrageOpportunity[]>([]);
  const [scannerStatus, setScannerStatus] = useState<ScannerStatus>({
    isConnected: false,
    lastMessageTime: 0,
    totalOpportunities: 0,
    executableOpportunities: 0,
    scannerHealth: 'offline'
  });
  const [sortBy, setSortBy] = useState<'netProfit' | 'profitPercent' | 'totalFees' | 'timestamp'>('netProfit');
  const [minProfitFilter, setMinProfitFilter] = useState(0);
  const [socket, setSocket] = useState<WebSocket | null>(null);

  // Connect to scanner's arbitrage opportunity feed
  useEffect(() => {
    const connectToScanner = () => {
      try {
        // Connect to Protocol V2 dashboard websocket server for arbitrage opportunities
        // The new server runs on port 8081 and handles TLV protocol messages
        const ws = new WebSocket('ws://localhost:8080/ws');
        
        ws.onopen = () => {
          console.log('✅ Connected to arbitrage scanner feed');
          setScannerStatus(prev => ({
            ...prev,
            isConnected: true,
            scannerHealth: 'healthy'
          }));
        };

        ws.onmessage = (event) => {
          try {
            const message = JSON.parse(event.data);
            handleArbitrageMessage(message);
          } catch (error) {
            console.error('Failed to parse arbitrage message:', error);
          }
        };

        ws.onclose = () => {
          console.log('❌ Scanner feed disconnected');
          setScannerStatus(prev => ({
            ...prev,
            isConnected: false,
            scannerHealth: 'offline'
          }));
          
          // Reconnect after 5 seconds
          setTimeout(connectToScanner, 5000);
        };

        ws.onerror = (error) => {
          console.error('Scanner feed error:', error);
          setScannerStatus(prev => ({
            ...prev,
            scannerHealth: 'degraded'
          }));
        };

        setSocket(ws);
      } catch (error) {
        console.error('Failed to connect to scanner:', error);
        setTimeout(connectToScanner, 5000);
      }
    };

    connectToScanner();

    return () => {
      if (socket) {
        socket.close();
      }
    };
  }, []);

  const handleArbitrageMessage = (message: any) => {
    // Handle different message types from ws_bridge
    if (message.msg_type !== 'arbitrage_opportunity') {
      return; // Only process arbitrage opportunity messages
    }
    
    // Process enhanced ArbitrageOpportunity message from ws_bridge binary protocol
    // Message now contains comprehensive fee breakdown from enhanced protocol
    // NO ESTIMATIONS - only use exact data provided in the enhanced message
    const opportunity: ArbitrageOpportunity = {
      id: `${message.pair}-${message.detected_at}`,
      timestamp: message.detected_at,
      pair: message.pair,
      token0Symbol: message.token_a,
      token1Symbol: message.token_b,
      buyPool: message.dex_buy_router,
      sellPool: message.dex_sell_router,
      buyExchange: message.dex_buy,
      sellExchange: message.dex_sell,
      buyPrice: message.price_buy,
      sellPrice: message.price_sell,
      tradeSize: message.max_trade_size,
      grossProfit: message.estimated_profit,
      profitPercent: message.profit_percent,
      // Enhanced fee data from binary protocol
      gasFee: message.gas_fee_usd || 0, // Gas fee in USD from enhanced message
      dexFees: message.dex_fees_usd || 0, // DEX fees in USD from enhanced message
      slippageCost: message.slippage_cost_usd || 0, // Slippage cost in USD from enhanced message
      totalFees: (message.gas_fee_usd || 0) + (message.dex_fees_usd || 0) + (message.slippage_cost_usd || 0),
      netProfit: message.net_profit_usd || (message.estimated_profit - ((message.gas_fee_usd || 0) + (message.dex_fees_usd || 0) + (message.slippage_cost_usd || 0))),
      netProfitPercent: message.net_profit_percent || message.profit_percent,
      executable: Boolean(message.executable !== undefined ? message.executable : message.estimated_profit > 0),
      recommendation: getRecommendation(message.net_profit_percent || message.profit_percent),
      buySlippage: message.buy_slippage_percent,
      sellSlippage: message.sell_slippage_percent,
      confidence: message.confidence_score
    };

    setOpportunities(prev => {
      // Add new opportunity and keep only recent ones (last 100)
      const updated = [opportunity, ...prev].slice(0, 100);
      
      // Update scanner status
      setScannerStatus(prevStatus => ({
        ...prevStatus,
        lastMessageTime: Date.now(),
        totalOpportunities: updated.length,
        executableOpportunities: updated.filter(op => op.executable).length,
        scannerHealth: 'healthy'
      }));
      
      return updated;
    });
  };

  const getRecommendation = (netProfitPercent: number): string => {
    if (netProfitPercent > 2) return '🚀 HIGH PROFIT - Execute immediately';
    if (netProfitPercent > 0.5) return '✅ PROFITABLE - Good opportunity';
    if (netProfitPercent > 0.1) return '🔶 MARGINAL - Small profit';
    return '❌ UNPROFITABLE - Avoid';
  };

  // Sort and filter opportunities
  const displayedOpportunities = useMemo(() => {
    let filtered = opportunities;
    
    // Filter by minimum profit
    if (minProfitFilter > 0) {
      filtered = filtered.filter(op => op.netProfit >= minProfitFilter);
    }
    
    // Sort by selected criteria
    filtered.sort((a, b) => {
      switch (sortBy) {
        case 'netProfit': return b.netProfit - a.netProfit;
        case 'profitPercent': return b.netProfitPercent - a.netProfitPercent;
        case 'totalFees': return a.totalFees - b.totalFees;
        case 'timestamp': return b.timestamp - a.timestamp;
        default: return b.netProfit - a.netProfit;
      }
    });
    
    return filtered.slice(0, 30);
  }, [opportunities, sortBy, minProfitFilter]);

  const executableOpportunities = displayedOpportunities.filter(op => op.executable);

  return (
    <div className="defi-arbitrage">
      <div className="header">
        <h2>🎯 Live Arbitrage Opportunities</h2>
        <div className={`connection-status ${scannerStatus.isConnected ? 'connected' : 'error'}`}>
          {scannerStatus.isConnected ? (
            <>✅ Scanner Connected ({scannerStatus.scannerHealth})</>
          ) : (
            <>❌ Scanner Disconnected</>
          )}
        </div>
      </div>

      <div className="controls">
        <div className="control-group">
          <label>Sort by:</label>
          <select 
            value={sortBy} 
            onChange={(e) => setSortBy(e.target.value as any)}
          >
            <option value="netProfit">Net Profit ($)</option>
            <option value="profitPercent">Profit %</option>
            <option value="totalFees">Lowest Fees</option>
            <option value="timestamp">Latest</option>
          </select>
        </div>

        <div className="control-group">
          <label>Min Profit:</label>
          <select 
            value={minProfitFilter} 
            onChange={(e) => setMinProfitFilter(Number(e.target.value))}
          >
            <option value={0}>Show All</option>
            <option value={0.1}>$0.10+</option>
            <option value={1}>$1+</option>
            <option value={5}>$5+</option>
            <option value={10}>$10+</option>
            <option value={25}>$25+</option>
          </select>
        </div>

        <div className="stats">
          <span>{displayedOpportunities.length} opportunities</span>
          <span>•</span>
          <span className="executable">{executableOpportunities.length} executable</span>
          <span>•</span>
          <span>Scanner: {scannerStatus.scannerHealth}</span>
        </div>
      </div>

      <div className="trades-list">
        {displayedOpportunities.length === 0 ? (
          <div className="empty-state">
            {scannerStatus.isConnected ? (
              <div>
                <div>🔍 Waiting for arbitrage opportunities...</div>
                <div className="sub-text">
                  Scanner is running and analyzing markets
                  {scannerStatus.lastMessageTime > 0 && (
                    <span> • Last update: {Math.round((Date.now() - scannerStatus.lastMessageTime) / 1000)}s ago</span>
                  )}
                </div>
              </div>
            ) : (
              <div>
                <div>⚠️ Scanner not connected</div>
                <div className="sub-text">Attempting to reconnect...</div>
              </div>
            )}
          </div>
        ) : (
          displayedOpportunities.map((opportunity, index) => (
            <div 
              key={opportunity.id} 
              className={`trade-item ${opportunity.executable ? 'executable' : 'not-executable'}`}
            >
              <div className="trade-header">
                <div className="trade-rank">#{index + 1}</div>
                <div className="trade-pair">{opportunity.pair}</div>
                <div className={`trade-profit ${opportunity.netProfit > 0 ? 'positive' : 'negative'}`}>
                  ${opportunity.netProfit.toFixed(2)}
                  <span className="profit-percent">
                    ({opportunity.netProfitPercent > 0 ? '+' : ''}{opportunity.netProfitPercent.toFixed(2)}%)
                  </span>
                </div>
                <div className="trade-status">
                  {opportunity.executable ? '✅ EXECUTE' : '⏸️ MONITOR'}
                </div>
              </div>

              <div className="trade-details">
                <div className="price-info">
                  <div className="price-item buy">
                    <span className="label">Buy:</span>
                    <span className="price">${opportunity.buyPrice.toFixed(6)}</span>
                    <span className="exchange">{opportunity.buyExchange}</span>
                  </div>
                  <div className="price-arrow">→</div>
                  <div className="price-item sell">
                    <span className="label">Sell:</span>
                    <span className="price">${opportunity.sellPrice.toFixed(6)}</span>
                    <span className="exchange">{opportunity.sellExchange}</span>
                  </div>
                </div>

                <div className="fees-breakdown">
                  <div className="fee-item">
                    <span className="fee-label">Trade Size:</span>
                    <span className="fee-value">${opportunity.tradeSize.toFixed(0)}</span>
                  </div>
                  <div className="fee-item">
                    <span className="fee-label">Gross Profit:</span>
                    <span className="fee-value positive">${opportunity.grossProfit.toFixed(2)}</span>
                  </div>
                  <div className="fee-item">
                    <span className="fee-label">Gas Fee:</span>
                    <span className="fee-value negative">-${opportunity.gasFee.toFixed(2)}</span>
                  </div>
                  <div className="fee-item">
                    <span className="fee-label">DEX Fees:</span>
                    <span className="fee-value negative">-${opportunity.dexFees.toFixed(2)}</span>
                  </div>
                  <div className="fee-item">
                    <span className="fee-label">Slippage:</span>
                    <span className="fee-value negative">-${opportunity.slippageCost.toFixed(2)}</span>
                  </div>
                  <div className="fee-item total">
                    <span className="fee-label">Total Fees:</span>
                    <span className="fee-value">-${opportunity.totalFees.toFixed(2)}</span>
                  </div>
                </div>

                <div className="opportunity-meta">
                  <div className="meta-item">
                    <span className="meta-label">Recommendation:</span>
                    <span className="meta-value">{opportunity.recommendation}</span>
                  </div>
                  {opportunity.confidence && (
                    <div className="meta-item">
                      <span className="meta-label">Confidence:</span>
                      <span className="meta-value">{(opportunity.confidence * 100).toFixed(0)}%</span>
                    </div>
                  )}
                  <div className="meta-item">
                    <span className="meta-label">Age:</span>
                    <span className="meta-value">{Math.round((Date.now() - opportunity.timestamp) / 1000)}s</span>
                  </div>
                </div>

                <div className="pool-addresses">
                  <div className="pool-address">
                    <span className="pool-label">Buy Pool:</span>
                    <span 
                      className="pool-hash" 
                      title={opportunity.buyPool}
                      onClick={() => navigator.clipboard.writeText(opportunity.buyPool)}
                    >
                      {opportunity.buyPool.slice(0, 8)}...{opportunity.buyPool.slice(-6)}
                    </span>
                  </div>
                  <div className="pool-address">
                    <span className="pool-label">Sell Pool:</span>
                    <span 
                      className="pool-hash" 
                      title={opportunity.sellPool}
                      onClick={() => navigator.clipboard.writeText(opportunity.sellPool)}
                    >
                      {opportunity.sellPool.slice(0, 8)}...{opportunity.sellPool.slice(-6)}
                    </span>
                  </div>
                </div>
              </div>
            </div>
          ))
        )}
      </div>

      <div className="footer-stats">
        <div className="stat-item">
          <span className="stat-label">Scanner Status:</span>
          <span className={`stat-value ${scannerStatus.scannerHealth === 'healthy' ? 'status-good' : 'status-bad'}`}>
            {scannerStatus.scannerHealth.toUpperCase()}
          </span>
        </div>
        <div className="stat-item">
          <span className="stat-label">Total Opportunities:</span>
          <span className="stat-value">{scannerStatus.totalOpportunities}</span>
        </div>
        <div className="stat-item">
          <span className="stat-label">Executable:</span>
          <span className="stat-value">{scannerStatus.executableOpportunities}</span>
        </div>
        <div className="stat-item">
          <span className="stat-label">Best Opportunity:</span>
          <span className="stat-value">
            {executableOpportunities.length > 0 
              ? `${executableOpportunities[0].pair}: $${executableOpportunities[0].netProfit.toFixed(2)}`
              : 'None'
            }
          </span>
        </div>
      </div>
    </div>
  );
};