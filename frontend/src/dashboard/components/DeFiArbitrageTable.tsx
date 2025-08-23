import React, { useState, useEffect, useMemo } from 'react';
import './DeFiArbitrageTable.css';

interface ArbitrageOpportunity {
  id: string;
  timestamp: number;
  pair: string;
  buyExchange: string;
  sellExchange: string;
  buyPrice: number;
  sellPrice: number;
  spread: number;
  tradeSize: number;
  grossProfit: number;
  gasFee: number;
  dexFees: number;
  slippage: number;
  netProfit: number;
  netProfitPercent: number;
  buyPool: string;
  sellPool: string;
  confidence?: number;
  executable: boolean;
}

interface PoolSwap {
  pool_id: string;
  token_in_symbol: string;
  token_out_symbol: string;
  amount_in: { normalized: number; decimals: number };
  amount_out: { normalized: number; decimals: number };
  timestamp: number;
}

export const DeFiArbitrageTable: React.FC = () => {
  const [opportunities, setOpportunities] = useState<ArbitrageOpportunity[]>([]);
  const [poolSwaps, setPoolSwaps] = useState<PoolSwap[]>([]);
  const [socket, setSocket] = useState<WebSocket | null>(null);
  const [isConnected, setIsConnected] = useState(false);
  const [groupByPool, setGroupByPool] = useState(false);
  const [showPoolSwaps, setShowPoolSwaps] = useState(false);

  // Demo data generator - DISABLED to show real market data
  // useEffect(() => {
  //   const pairs = ['WMATIC/USDC', 'WETH/USDC', 'WBTC/USDC'];
  //   const exchanges = [
  //     ['QuickSwap', 'SushiSwap'],
  //     ['Uniswap V3', 'QuickSwap'], 
  //     ['SushiSwap', 'Uniswap V3']
  //   ];
    
  //   const generateDemoOpportunity = (): ArbitrageOpportunity => {
  //     const profit = 50 + Math.random() * 250;
  //     const spread = 1.5 + Math.random() * 3.0;
  //     const basePrice = 0.45 + Math.random() * 0.05;
      
  //     // Pick a random pair and exchange combination
  //     const pairIndex = Math.floor(Math.random() * pairs.length);
  //     const exchangePair = exchanges[pairIndex];
      
  //     return {
  //       id: `demo-arb-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
  //       timestamp: Date.now(),
  //       pair: pairs[pairIndex],
  //       buyExchange: exchangePair[0],
  //       sellExchange: exchangePair[1],
  //       buyPrice: basePrice,
  //       sellPrice: basePrice + (spread / 100),
  //       spread: spread,
  //       tradeSize: 10000,
  //       grossProfit: profit + 20,
  //       gasFee: 15,
  //       dexFees: 12,
  //       slippage: 5,
  //       netProfit: profit,
  //       netProfitPercent: (profit / 10000) * 100,
  //       buyPool: '0x6e7a5FAFcec6BB1e78bAE2A1F0B612012bf14827',
  //       sellPool: '0xc4e595acDD7d12feC6BB1e78bAE2A1F0B612012bf14827',
  //       confidence: 0.85 + Math.random() * 0.13,
  //       executable: true
  //     };
  //   };

  //   // Generate initial demo opportunities
  //   const initialOpportunities = Array.from({ length: 5 }, () => generateDemoOpportunity());
  //   setOpportunities(initialOpportunities);

  //   // Update opportunities every 5 seconds for demo (update existing instead of append)
  //   const interval = setInterval(() => {
  //     const updatedOpp = generateDemoOpportunity();
      
  //     setOpportunities(prev => {
  //       // Find existing opportunity for same pair and pools
  //       const existingIndex = prev.findIndex(opp => 
  //         opp.pair === updatedOpp.pair && 
  //         opp.buyExchange === updatedOpp.buyExchange && 
  //         opp.sellExchange === updatedOpp.sellExchange
  //       );
        
  //       if (existingIndex >= 0) {
  //         // Update existing opportunity
  //         const updated = [...prev];
  //         updated[existingIndex] = { ...updatedOpp, id: prev[existingIndex].id }; // Keep same ID
  //         return updated;
  //       } else {
  //         // Add new opportunity if pool pair doesn't exist
  //         return [updatedOpp, ...prev].slice(0, 20); // Limit to 20 opportunities max
  //       }
  //     });
      
  //     console.log('📊 Updated arbitrage opportunity:', updatedOpp.pair, `- $${updatedOpp.netProfit.toFixed(2)} profit`);
  //   }, 5000); // Update every 5 seconds instead of 10

  //   return () => clearInterval(interval);
  // }, []);

  useEffect(() => {
    const connectToScanner = () => {
      try {
        const ws = new WebSocket('ws://localhost:8080/ws');
        
        ws.onopen = () => {
          console.log('Connected to arbitrage scanner');
          setIsConnected(true);
        };

        ws.onmessage = (event) => {
          try {
            const message = JSON.parse(event.data);
            console.log('📨 Received WebSocket message:', message);
            
            if (message.msg_type === 'arbitrage_opportunity' || message.type === 'demo_defi_arbitrage') {
              const opp: ArbitrageOpportunity = {
                id: message.id || `${message.pair}-${message.timestamp}-${Math.random()}`,
                timestamp: message.timestamp || Date.now(),
                pair: message.pair,
                buyExchange: message.buyExchange || message.dex_buy,
                sellExchange: message.sellExchange || message.dex_sell,
                buyPrice: message.buyPrice || message.price_buy,
                sellPrice: message.sellPrice || message.price_sell,
                spread: ((message.sellPrice - message.buyPrice) / message.buyPrice * 100) || message.profitPercent,
                tradeSize: message.tradeSize || message.max_trade_size,
                grossProfit: message.grossProfit || message.estimated_profit,
                gasFee: message.gasFee || message.gas_fee_usd || 0,
                dexFees: message.dexFees || message.dex_fees_usd || 0,
                slippage: message.slippageCost || message.slippage_cost_usd || 0,
                netProfit: message.netProfit || message.net_profit_usd,
                netProfitPercent: message.netProfitPercent || message.net_profit_percent || 0,
                buyPool: message.buyPool,
                sellPool: message.sellPool,
                confidence: message.confidence || message.confidence_score,
                executable: message.executable
              };
              
              // Update existing opportunity or add new one
              setOpportunities(prev => {
                const existingIndex = prev.findIndex(existing => 
                  existing.pair === opp.pair && 
                  existing.buyExchange === opp.buyExchange && 
                  existing.sellExchange === opp.sellExchange
                );
                
                if (existingIndex >= 0) {
                  // Update existing opportunity
                  const updated = [...prev];
                  updated[existingIndex] = opp;
                  return updated;
                } else {
                  // Add new opportunity
                  return [opp, ...prev].slice(0, 20);
                }
              });
            } else if (message.type === 'pool_swap') {
              const swap: PoolSwap = {
                pool_id: message.pool_address || message.pool_id || 'Unknown',
                token_in_symbol: message.token_in || 'TokenIn',
                token_out_symbol: message.token_out || 'TokenOut',
                amount_in: message.amount_in || { normalized: 0, decimals: 18 },
                amount_out: message.amount_out || { normalized: 0, decimals: 18 },
                timestamp: message.timestamp || Date.now()
              };
              
              console.log('🔄 Received pool swap:', swap);
              setPoolSwaps(prev => [swap, ...prev].slice(0, 100));
            }
          } catch (error) {
            console.error('Failed to parse message:', error);
          }
        };

        ws.onclose = () => {
          console.log('Disconnected from scanner');
          setIsConnected(false);
          setTimeout(connectToScanner, 5000);
        };

        ws.onerror = (error) => {
          console.error('WebSocket error:', error);
        };

        setSocket(ws);
      } catch (error) {
        console.error('Failed to connect:', error);
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

  const groupedOpportunities = useMemo(() => {
    if (!groupByPool) return { all: opportunities };
    
    const groups: Record<string, ArbitrageOpportunity[]> = {};
    opportunities.forEach(opp => {
      const poolKey = `${opp.buyPool}-${opp.sellPool}`;
      if (!groups[poolKey]) {
        groups[poolKey] = [];
      }
      groups[poolKey].push(opp);
    });
    return groups;
  }, [opportunities, groupByPool]);

  const formatPrice = (price: number) => {
    if (price > 1000) return price.toFixed(2);
    if (price > 1) return price.toFixed(4);
    return price.toFixed(6);
  };

  const formatPercent = (percent: number) => {
    return `${percent >= 0 ? '+' : ''}${percent.toFixed(2)}%`;
  };

  return (
    <div className="defi-arbitrage-table">
      <div className="header">
        <h2>DeFi Arbitrage Monitor</h2>
        <div className="status">
          <span className={`indicator ${isConnected ? 'connected' : 'disconnected'}`} />
          {isConnected ? 'Connected' : 'Disconnected'}
        </div>
      </div>

      <div className="controls">
        <label>
          <input 
            type="checkbox" 
            checked={groupByPool}
            onChange={(e) => setGroupByPool(e.target.checked)}
          />
          Group by Pool Pair
        </label>
        <label>
          <input 
            type="checkbox" 
            checked={showPoolSwaps}
            onChange={(e) => setShowPoolSwaps(e.target.checked)}
          />
          Show Pool Swaps
        </label>
        <div className="stats">
          Total: {opportunities.length} | 
          Executable: {opportunities.filter(o => o.executable).length} |
          Avg Profit: ${(opportunities.reduce((sum, o) => sum + o.netProfit, 0) / (opportunities.length || 1)).toFixed(2)}
        </div>
      </div>

      {showPoolSwaps && (
        <div className="pool-swaps-section">
          <h3>Recent Pool Swaps</h3>
          <div className="pool-swaps-table">
            <table>
              <thead>
                <tr>
                  <th>Pool</th>
                  <th>Swap</th>
                  <th>Amount In</th>
                  <th>Amount Out</th>
                  <th>Time</th>
                </tr>
              </thead>
              <tbody>
                {poolSwaps.slice(0, 10).map((swap, idx) => (
                  <tr key={`${swap.pool_id}-${swap.timestamp}-${idx}`}>
                    <td className="mono">{swap.pool_id}</td>
                    <td>{swap.token_in_symbol} → {swap.token_out_symbol}</td>
                    <td>{swap.amount_in.normalized.toFixed(4)}</td>
                    <td>{swap.amount_out.normalized.toFixed(4)}</td>
                    <td>{new Date(swap.timestamp).toLocaleTimeString()}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      <div className="opportunities-table">
        <table>
          <thead>
            <tr>
              <th>Pair</th>
              <th>Buy</th>
              <th>Sell</th>
              <th>Spread</th>
              <th>Size</th>
              <th>Gross</th>
              <th>Gas</th>
              <th>DEX</th>
              <th>Slip</th>
              <th>Net</th>
              <th>%</th>
              <th>Conf</th>
              <th>Status</th>
            </tr>
          </thead>
          <tbody>
            {Object.entries(groupedOpportunities).map(([poolKey, opps]) => (
              <React.Fragment key={poolKey}>
                {groupByPool && poolKey !== 'all' && (
                  <tr className="pool-group-header">
                    <td colSpan={13}>
                      Pool Pair: {poolKey.substring(0, 10)}...{poolKey.substring(poolKey.length - 6)}
                    </td>
                  </tr>
                )}
                {opps.map((opp) => (
                  <tr 
                    key={opp.id}
                    className={`opportunity-row ${opp.executable ? 'executable' : ''} ${opp.netProfit > 25 ? 'high-profit' : ''}`}
                  >
                    <td className="pair">{opp.pair}</td>
                    <td className="exchange">{opp.buyExchange}</td>
                    <td className="exchange">{opp.sellExchange}</td>
                    <td className="spread">{formatPercent(opp.spread)}</td>
                    <td className="size">${opp.tradeSize.toFixed(0)}</td>
                    <td className="profit positive">${opp.grossProfit.toFixed(2)}</td>
                    <td className="fee">${opp.gasFee.toFixed(2)}</td>
                    <td className="fee">${opp.dexFees.toFixed(2)}</td>
                    <td className="fee">${opp.slippage.toFixed(2)}</td>
                    <td className={`net-profit ${opp.netProfit > 0 ? 'positive' : 'negative'}`}>
                      ${opp.netProfit.toFixed(2)}
                    </td>
                    <td className={`percent ${opp.netProfitPercent > 0 ? 'positive' : 'negative'}`}>
                      {formatPercent(opp.netProfitPercent)}
                    </td>
                    <td className="confidence">
                      {opp.confidence ? `${(opp.confidence * 100).toFixed(0)}%` : '-'}
                    </td>
                    <td className="status">
                      {opp.executable ? (
                        <span className="badge execute">EXEC</span>
                      ) : (
                        <span className="badge monitor">WAIT</span>
                      )}
                    </td>
                  </tr>
                ))}
              </React.Fragment>
            ))}
          </tbody>
        </table>
        {opportunities.length === 0 && (
          <div className="empty-state">
            {isConnected ? 'Waiting for arbitrage opportunities...' : 'Connecting to scanner...'}
          </div>
        )}
      </div>

      <div className="data-flow-info">
        <h4>Data Flow Architecture:</h4>
        <div className="flow-diagram">
          <span>Polygon DEX</span> → 
          <span>Collector (TLV)</span> → 
          <span>Relay</span> → 
          <span>Arbitrage Strategy</span> → 
          <span>WebSocket Bridge</span> → 
          <span>Dashboard</span>
        </div>
        <div className="validation-status">
          {showPoolSwaps ? (
            <p>Pool swaps show native precision: WMATIC/WETH (18 decimals), USDC/USDT (6 decimals)</p>
          ) : (
            <p>Enable "Show Pool Swaps" to see native precision validation</p>
          )}
        </div>
      </div>
    </div>
  );
};