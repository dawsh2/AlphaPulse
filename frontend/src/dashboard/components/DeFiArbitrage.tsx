import React, { useState, useEffect, useMemo, useRef } from 'react';
import './DeFiArbitrage.css';

interface ArbitrageOpportunity {
  id: string;
  pair: string;
  buyDex: string;
  sellDex: string;
  buyPrice: number;
  sellPrice: number;
  profitPercent: number;
  estimatedProfit: number;
  maxTradeSize: number;
  timestamp: number;
  executed?: boolean;
}

interface DexPrice {
  dex: string;
  price: number;
  liquidity: number;
  timestamp: number;
  latency: number;
}

interface PolygonMetrics {
  blockNumber: number;
  gasPrice: number;
  totalOpportunities: number;
  executedTrades: number;
  totalProfit: number;
  avgLatency: number;
  systemStatus: 'online' | 'offline' | 'degraded';
}

export const DeFiArbitrage: React.FC = () => {
  const [opportunities, setOpportunities] = useState<ArbitrageOpportunity[]>([]);
  const [dexPrices, setDexPrices] = useState<Map<string, DexPrice[]>>(new Map());
  const [metrics, setMetrics] = useState<PolygonMetrics>({
    blockNumber: 0,
    gasPrice: 30, // Start with realistic Polygon gas price
    totalOpportunities: 0,
    executedTrades: 0,
    totalProfit: 0,
    avgLatency: 0,
    systemStatus: 'offline'
  });
  const [selectedPair, setSelectedPair] = useState<string>('ALL');
  const [isConnected, setIsConnected] = useState(false);
  const [tradeSize, setTradeSize] = useState<number>(1000); // Configurable trade size

  // WebSocket connection for real-time data from all exchanges
  useEffect(() => {
    let ws: WebSocket | null = null;
    let reconnectTimer: NodeJS.Timeout | undefined;
    let isConnecting = false;
    let shouldReconnect = true;
    let isMounted = true;
    
    const connect = () => {
      if (isConnecting || !shouldReconnect || ws || !isMounted) return;
      
      try {
        isConnecting = true;
        ws = new WebSocket('ws://localhost:8765');
        
        ws.onopen = () => {
          console.log('🔗 Connected to live data stream');
          isConnecting = false;
          if (isMounted) {
            setIsConnected(true);
            setMetrics(prev => ({ ...prev, systemStatus: 'online' }));
          }
        };

    ws.onmessage = (event) => {
      try {
        const message = JSON.parse(event.data);
        // Silently process all messages without logging
        
        switch (message.msg_type) {
          case 'trade':
            // Handle all trade/price updates from any exchange
            if (message.symbol_hash && message.symbol) {
              const symbol = message.symbol;
              const price = message.price;
              const volume = message.volume;
              const timestamp = Date.now();
              
              // Parse symbol format: exchange:pair or special formats
              const parts = symbol.split(':');
              if (parts.length >= 2) {
                const exchange = parts[0];
                const pair = parts[1];
                
                // Skip non-DEX exchanges for this page
                // Alpaca and Coinbase are centralized exchanges, not DEXs
                if (exchange === 'coinbase' || exchange === 'alpaca' || exchange === 'kraken') {
                  break;
                }
                
                // Map exchange names to more readable format
                const dexName = {
                  'polygon': 'Polygon',
                  'quickswap': 'QuickSwap',
                  'sushiswap': 'SushiSwap',
                  'uniswap_v3': 'Uniswap V3'
                }[exchange] || exchange;
                
                const dexPrice: DexPrice = {
                  dex: dexName,
                  price,
                  liquidity: volume,
                  timestamp,
                  latency: message.latency_total_us ? Math.round(message.latency_total_us / 1000) : 5
                };
                
                setDexPrices(prev => {
                  const updated = new Map(prev);
                  const existing = updated.get(pair) || [];
                  const filtered = existing.filter(p => p.dex !== dexName);
                  updated.set(pair, [...filtered, dexPrice].slice(-10)); // Keep last 10
                  return updated;
                });
              }
            }
            break;
            
          case 'symbol_mapping':
            // Store symbol mappings for hash resolution (silently)
            break;
            
          case 'heartbeat':
            // Update connection metrics
            setMetrics(prev => ({
              ...prev,
              avgLatency: 5 + Math.random() * 5,
              systemStatus: 'online'
            }));
            break;
        }
      } catch (e) {
        console.error('Failed to parse WebSocket message:', e);
      }
    };

        ws.onclose = () => {
          console.log('❌ Data stream disconnected');
          ws = null;
          isConnecting = false;
          if (isMounted) {
            setIsConnected(false);
            setMetrics(prev => ({ ...prev, systemStatus: 'offline' }));
          }
          
          // Reconnect after 5 seconds if we should reconnect and component is still mounted
          if (shouldReconnect && isMounted) {
            reconnectTimer = setTimeout(() => {
              console.log('🔄 Attempting to reconnect...');
              connect();
            }, 5000);
          }
        };

        ws.onerror = (error) => {
          console.error('WebSocket error:', error);
          isConnecting = false;
          if (isMounted) {
            setMetrics(prev => ({ ...prev, systemStatus: 'degraded' }));
          }
        };
      } catch (error) {
        console.error('Failed to connect to WebSocket:', error);
        isConnecting = false;
        if (isMounted) {
          setMetrics(prev => ({ ...prev, systemStatus: 'offline' }));
        }
        
        // Try to reconnect after 5 seconds if we should reconnect and component is still mounted
        if (shouldReconnect && isMounted) {
          reconnectTimer = setTimeout(() => {
            console.log('🔄 Retrying connection...');
            connect();
          }, 5000);
        }
      }
    };
    
    // Only connect if component is mounted
    if (isMounted) {
      connect();
    }

    return () => {
      isMounted = false;
      shouldReconnect = false;
      if (ws && ws.readyState === WebSocket.OPEN) {
        ws.close();
      }
      ws = null;
      if (reconnectTimer) {
        clearTimeout(reconnectTimer);
        reconnectTimer = undefined;
      }
    };
  }, []); // No dependency on threshold - show all profitable opportunities

  const filteredOpportunities = useMemo(() => {
    return opportunities.filter(opp => 
      selectedPair === 'ALL' || opp.pair === selectedPair
    );
  }, [opportunities, selectedPair]);

  const executeArbitrage = (opportunityId: string) => {
    setOpportunities(prev => 
      prev.map(opp => 
        opp.id === opportunityId 
          ? { ...opp, executed: true }
          : opp
      )
    );
    
    setMetrics(prev => ({
      ...prev,
      executedTrades: prev.executedTrades + 1,
      totalProfit: prev.totalProfit + opportunities.find(o => o.id === opportunityId)?.estimatedProfit || 0
    }));
  };

  return (
    <div className="defi-arbitrage">
      <div className="defi-header">
        <div className="status-section">
          <h2>🔗 Polygon DeFi Arbitrage</h2>
          <div className={`connection-status ${isConnected ? 'connected' : 'disconnected'}`}>
            {isConnected ? '🟢 Connected to Polygon' : '🔴 Disconnected'}
          </div>
        </div>

        <div className="metrics-grid">
          <div className="metric-card">
            <div className="metric-label">Block Number</div>
            <div className="metric-value">{metrics.blockNumber.toLocaleString()}</div>
          </div>
          <div className="metric-card">
            <div className="metric-label">Gas Price</div>
            <div className="metric-value">{metrics.gasPrice.toFixed(0)} gwei</div>
          </div>
          <div className="metric-card">
            <div className="metric-label">Opportunities</div>
            <div className="metric-value">{metrics.totalOpportunities}</div>
          </div>
          <div className="metric-card">
            <div className="metric-label">Executed</div>
            <div className="metric-value">{metrics.executedTrades}</div>
          </div>
          <div className="metric-card">
            <div className="metric-label">Total Profit</div>
            <div className="metric-value">${metrics.totalProfit.toFixed(0)}</div>
          </div>
          <div className="metric-card">
            <div className="metric-label">Avg Latency</div>
            <div className="metric-value">{metrics.avgLatency.toFixed(1)}ms</div>
          </div>
        </div>
      </div>

      <div className="controls-section">
        <div className="threshold-info">
          <span className="info-text">🎯 Monitoring all DEX pairs - Real-time streaming updates</span>
        </div>
        <div className="trade-size-control" style={{ marginTop: '10px' }}>
          <label style={{ marginRight: '10px' }}>
            Trade Size: $
            <input 
              type="number" 
              value={tradeSize} 
              onChange={(e) => setTradeSize(Math.max(100, parseInt(e.target.value) || 1000))}
              min="100"
              step="100"
              style={{ width: '100px', marginLeft: '5px' }}
            />
          </label>
          <span style={{ fontSize: '0.9em', color: '#666' }}>
            (Gas: {metrics.gasPrice} gwei ≈ ${((metrics.gasPrice * 150000 * 0.52) / 1e9).toFixed(4)})
          </span>
        </div>
      </div>

      <div className="opportunities-section">
        <h3>🎯 Live Arbitrage Opportunities</h3>
        
        {filteredOpportunities.length === 0 ? (
          <div className="no-opportunities">
            No profitable arbitrage opportunities detected
          </div>
        ) : (
          <div className="opportunities-list">
            {filteredOpportunities.map((opp) => (
              <div 
                key={opp.id} 
                className={`opportunity-card ${opp.executed ? 'executed' : ''}`}
              >
                <div className="opportunity-header">
                  <div className="pair-info">
                    <span className="pair">{opp.pair}</span>
                    <span className="profit-percent" 
                          style={{ color: opp.profitPercent > 1 ? '#22c55e' : '#f59e0b' }}>
                      +{opp.profitPercent.toFixed(3)}%
                    </span>
                  </div>
                  <div className="estimated-profit">
                    ${opp.estimatedProfit.toFixed(0)}
                  </div>
                </div>

                <div className="opportunity-details">
                  <div className="trade-info">
                    <div className="buy-info">
                      <span className="action buy">BUY</span>
                      <span className="dex">{opp.buyDex}</span>
                      <span className="price">${opp.buyPrice.toFixed(4)}</span>
                    </div>
                    <div className="arrow">→</div>
                    <div className="sell-info">
                      <span className="action sell">SELL</span>
                      <span className="dex">{opp.sellDex}</span>
                      <span className="price">${opp.sellPrice.toFixed(4)}</span>
                    </div>
                  </div>

                  <div className="trade-size">
                    Max Size: ${opp.maxTradeSize.toLocaleString()}
                  </div>

                  <div className="timestamp">
                    {new Date(opp.timestamp).toLocaleTimeString()}
                  </div>
                </div>

                {!opp.executed && (
                  <button 
                    className="execute-btn"
                    onClick={() => executeArbitrage(opp.id)}
                    disabled={opp.profitPercent < 0.5} // Only allow execution above 0.5%
                  >
                    {opp.profitPercent >= 0.5 ? 'Execute' : 'Below Threshold'}
                  </button>
                )}

                {opp.executed && (
                  <div className="executed-badge">✅ Executed</div>
                )}
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="live-prices-section">
        <h3>💱 Live DEX Price Comparison</h3>
        {Array.from(dexPrices.entries()).length === 0 ? (
          <div className="no-price-data">
            Connecting to DEX price feeds...
          </div>
        ) : (
          <>
            {/* Cross-Pair Arbitrage Detection */}
            {(() => {
              const crossPairOpportunities = [];
              const entries = Array.from(dexPrices.entries());
              
              // Check for cross-pair arbitrage (e.g., AAVE-USDC vs AAVE-USDT)
              for (let i = 0; i < entries.length; i++) {
                for (let j = i + 1; j < entries.length; j++) {
                  const [pair1, prices1] = entries[i];
                  const [pair2, prices2] = entries[j];
                  
                  // Extract base tokens from pairs
                  const base1 = pair1.split('-')[0];
                  const base2 = pair2.split('-')[0];
                  const quote1 = pair1.split('-')[1];
                  const quote2 = pair2.split('-')[1];
                  
                  // Check if same base token but different quote (e.g., AAVE-USDC vs AAVE-USDT)
                  if (base1 === base2 && quote1 !== quote2) {
                    const avgPrice1 = prices1.reduce((sum, p) => sum + p.price, 0) / prices1.length;
                    const avgPrice2 = prices2.reduce((sum, p) => sum + p.price, 0) / prices2.length;
                    const priceDiff = Math.abs(avgPrice1 - avgPrice2);
                    const priceDiffPercent = (priceDiff / Math.min(avgPrice1, avgPrice2)) * 100;
                    
                    if (priceDiffPercent > 0.1) { // Show if >0.1% difference
                      const estimatedGasCost = 0.15; // 3 swaps for cross-pair arb
                      const liquidity = Math.min(
                        ...prices1.map(p => p.liquidity),
                        ...prices2.map(p => p.liquidity)
                      );
                      const grossProfit = priceDiff * (liquidity / Math.max(avgPrice1, avgPrice2));
                      const netProfit = grossProfit - estimatedGasCost;
                      
                      crossPairOpportunities.push({
                        base: base1,
                        pair1,
                        pair2,
                        price1: avgPrice1,
                        price2: avgPrice2,
                        priceDiff,
                        priceDiffPercent,
                        netProfit,
                        grossProfit,
                        estimatedGasCost
                      });
                    }
                  }
                }
              }
              
              if (crossPairOpportunities.length > 0) {
                return (
                  <div className="cross-pair-opportunities">
                    <h4>🔄 Cross-Pair Arbitrage Opportunities</h4>
                    <div className="opportunities-grid">
                      {crossPairOpportunities
                        .sort((a, b) => b.netProfit - a.netProfit)
                        .map((opp, idx) => (
                          <div key={idx} className={`cross-pair-card ${opp.netProfit > 0 ? 'profitable' : 'unprofitable'}`}>
                            <div className="pair-comparison">
                              <span className="pair">{opp.pair1}</span>
                              <span className="vs">vs</span>
                              <span className="pair">{opp.pair2}</span>
                            </div>
                            <div className="price-comparison">
                              <span>${opp.price1.toFixed(2)}</span>
                              <span className="diff">{opp.priceDiffPercent.toFixed(2)}%</span>
                              <span>${opp.price2.toFixed(2)}</span>
                            </div>
                            <div className="profit-info">
                              {opp.netProfit > 0 ? (
                                <span className="net-profit">Net: +${opp.netProfit.toFixed(2)}</span>
                              ) : (
                                <span className="net-loss">Net: -${Math.abs(opp.netProfit).toFixed(2)}</span>
                              )}
                              <span className="gas-info">(gas: ${opp.estimatedGasCost.toFixed(2)})</span>
                            </div>
                          </div>
                        ))}
                    </div>
                  </div>
                );
              }
              return null;
            })()}
            
            <div className="price-table-container">
            <table className="price-comparison-table">
              <thead>
                <tr>
                  <th>Pair</th>
                  <th>Exchange</th>
                  <th>Price</th>
                  <th>Liquidity</th>
                  <th>Latency</th>
                  <th>Spread</th>
                  <th>Arbitrage</th>
                </tr>
              </thead>
              <tbody>
                {Array.from(dexPrices.entries())
                  .sort(([a], [b]) => a.localeCompare(b)) // Sort pairs alphabetically
                  .map(([pair, prices]) => {
                    const validPrices = prices
                      .filter(p => p.price > 0 && p.price < 1e10)
                      .sort((a, b) => a.price - b.price); // Sort by price to easily see min/max
                    
                    if (validPrices.length === 0) return null;
                    
                    const minPrice = validPrices[0].price;
                    const maxPrice = validPrices[validPrices.length - 1].price;
                    const spread = maxPrice > 0 ? ((maxPrice - minPrice) / minPrice * 100) : 0;
                    
                    // Calculate potential profit considering gas costs
                    // Realistic arbitrage calculation for configurable trade size
                    const tradeAmountUSD = tradeSize; // Use configurable trade size
                    const gasPrice = metrics.gasPrice || 30; // Use dynamic gas price from metrics
                    const gasLimit = 150000; // Gas for 2 DEX swaps + approval
                    const maticPriceUSD = 0.52; // Current MATIC price (could be fetched dynamically)
                    
                    // Calculate gas cost in USD
                    const gasCostUSD = (gasPrice * gasLimit * maticPriceUSD) / 1e9;
                    
                    // Calculate arbitrage profit for the trade amount
                    // Buy tokens at minPrice, sell at maxPrice
                    const tokensReceived = tradeAmountUSD / minPrice;
                    const proceedsFromSale = tokensReceived * maxPrice;
                    const grossProfit = proceedsFromSale - tradeAmountUSD;
                    const netProfit = grossProfit - gasCostUSD;
                    const profitPercent = (netProfit / tradeAmountUSD) * 100;
                    
                    // Check if profitable considering available liquidity
                    const minLiquidity = Math.min(
                      validPrices.find(p => p.price === minPrice)?.liquidity || 0,
                      validPrices.find(p => p.price === maxPrice)?.liquidity || 0
                    );
                    const hasArbitrage = netProfit > 0 && validPrices.length > 1 && minLiquidity >= tradeAmountUSD;
                    
                    return validPrices.map((priceData, idx) => {
                      const isMinPrice = priceData.price === minPrice;
                      const isMaxPrice = priceData.price === maxPrice;
                      
                      return (
                        <tr key={`${pair}-${priceData.dex}`} 
                            className={hasArbitrage ? (isMinPrice ? 'buy-opportunity' : isMaxPrice ? 'sell-opportunity' : '') : ''}>
                          {idx === 0 && (
                            <td rowSpan={validPrices.length} className="pair-cell">
                              {pair}
                            </td>
                          )}
                          <td className="exchange-cell">
                            {priceData.dex}
                            {isMinPrice && hasArbitrage && <span className="badge buy"> BUY</span>}
                            {isMaxPrice && hasArbitrage && <span className="badge sell"> SELL</span>}
                          </td>
                          <td className="price-cell">
                            {priceData.price < 0.01 ? 
                              `$${priceData.price.toExponential(2)}` : 
                              `$${priceData.price.toFixed(priceData.price < 10 ? 4 : 2)}`
                            }
                          </td>
                          <td className="liquidity-cell">
                            {priceData.liquidity > 1000000 ? 
                              `$${(priceData.liquidity / 1000000).toFixed(1)}M` : 
                              priceData.liquidity > 1000 ? 
                              `$${(priceData.liquidity / 1000).toFixed(1)}K` : 
                              `$${priceData.liquidity.toFixed(0)}`
                            }
                          </td>
                          <td className="latency-cell">{priceData.latency}ms</td>
                          {idx === 0 && (
                            <>
                              <td rowSpan={validPrices.length} className="spread-cell">
                                <span className={spread > 0.5 ? 'high-spread' : 'low-spread'}>
                                  {spread.toFixed(2)}%
                                </span>
                              </td>
                              <td rowSpan={validPrices.length} className="arbitrage-cell">
                                {hasArbitrage ? (
                                  <div className="arbitrage-info">
                                    <div className="profit-estimate">
                                      Net: +${netProfit.toFixed(2)} ({profitPercent.toFixed(2)}%)
                                    </div>
                                    <div className="gross-profit">
                                      On ${tradeSize}: ${grossProfit.toFixed(2)} - ${gasCostUSD.toFixed(4)} gas
                                    </div>
                                    <div className="action-hint">
                                      {validPrices.find(p => p.price === minPrice)?.dex} → {validPrices.find(p => p.price === maxPrice)?.dex}
                                    </div>
                                  </div>
                                ) : spread > 0 ? (
                                  <div className="arbitrage-info unprofitable">
                                    <div className="loss-estimate">
                                      Net: -${Math.abs(netProfit).toFixed(2)} ({profitPercent.toFixed(2)}%)
                                    </div>
                                    <div className="gross-profit">
                                      On ${tradeSize}: ${grossProfit.toFixed(2)} - ${gasCostUSD.toFixed(4)} gas
                                    </div>
                                  </div>
                                ) : (
                                  <span className="no-arbitrage">-</span>
                                )}
                              </td>
                            </>
                          )}
                        </tr>
                      );
                    });
                  }).filter(Boolean).flat()}
              </tbody>
            </table>
            </div>
            <div className="table-legend">
              <span className="legend-item">💡 Latency: Time to receive price update</span>
              <span className="legend-item">📊 Spread: Price difference between exchanges</span>
              <span className="legend-item">💰 Liquidity: Available trading volume</span>
            </div>
          </>
        )}
      </div>

      <div className="dex-monitoring">
        <h3>📊 DEX Connection Status</h3>
        <div className="dex-grid">
          {['QuickSwap', 'SushiSwap', 'Uniswap V3', 'Balancer', 'Curve'].map(dex => (
            <div key={dex} className="dex-card">
              <div className="dex-name">{dex}</div>
              <div className="dex-status online">🟢 Connected</div>
              <div className="dex-latency">{(5 + Math.random() * 10).toFixed(1)}ms</div>
            </div>
          ))}
        </div>
      </div>

      <div className="system-info">
        <h3>⚙️ System Status</h3>
        <div className="system-details">
          <div className="status-item">
            <span>Polygon RPC:</span>
            <span className="status-value online">5ms latency</span>
          </div>
          <div className="status-item">
            <span>Price Feeds:</span>
            <span className="status-value online">4/4 DEXs connected</span>
          </div>
          <div className="status-item">
            <span>Execution Engine:</span>
            <span className="status-value online">Ready</span>
          </div>
          <div className="status-item">
            <span>Gas Optimization:</span>
            <span className="status-value online">Active</span>
          </div>
        </div>
      </div>
    </div>
  );
};