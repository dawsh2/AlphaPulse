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
  executable: boolean;
}

interface PoolSwap {
  pool_id: string;
  pool_address: string;
  venue_name: string;
  token_in: string;
  token_out: string;
  token_in_symbol?: string;
  token_out_symbol?: string;
  amount_in: { raw: string; normalized: number; decimals: number };
  amount_out: { raw: string; normalized: number; decimals: number };
  sqrt_price_x96_after?: string;
  tick_after?: number;
  liquidity_after?: string;
  timestamp: number;
  block_number?: number;
}

interface TokenPairActivity {
  token_pair: string;
  pool_address: string;
  venue_name: string;
  latest_swap: PoolSwap;
  activity_count: number;
  last_updated: number;
}

export const DeFiArbitrageTable: React.FC = () => {
  const [opportunities, setOpportunities] = useState<ArbitrageOpportunity[]>([]);
  const [poolSwaps, setPoolSwaps] = useState<PoolSwap[]>([]);
  const [tokenActivities, setTokenActivities] = useState<TokenPairActivity[]>([]);
  const [socket, setSocket] = useState<WebSocket | null>(null);
  const [isConnected, setIsConnected] = useState(false);
  const [groupByPool, setGroupByPool] = useState(false);
  const [showPoolSwaps, setShowPoolSwaps] = useState(true); // Default to true for token metadata
  const [viewMode, setViewMode] = useState<'token_activity' | 'arbitrage' | 'raw_swaps'>('token_activity');
  const [lastUpdateTime, setLastUpdateTime] = useState<number>(0);

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

            // Debug specific message types
            if (message.msg_type) {
              console.log(`🔍 Message type: ${message.msg_type} | TLV type: ${message.tlv_type || 'unknown'}`);
            }

            if (message.msg_type === 'arbitrage_opportunity' || message.type === 'demo_defi_arbitrage') {
              // Use enhanced metrics if available
              const metrics = message.arbitrage_metrics;

              const opp: ArbitrageOpportunity = {
                id: message.id || `${message.pair}-${message.timestamp}-${Math.random()}`,
                // Convert nanoseconds to milliseconds if needed
                timestamp: message.timestamp > 1e15 ? Math.floor(message.timestamp / 1e6) : message.timestamp || Date.now(),
                pair: message.pair,
                buyExchange: message.buyExchange || message.dex_buy,
                sellExchange: message.sellExchange || message.dex_sell,
                buyPrice: message.buyPrice || message.price_buy,
                sellPrice: message.sellPrice || message.price_sell,
                spread: metrics?.spread_percent || ((message.sellPrice - message.buyPrice) / message.buyPrice * 100) || message.profitPercent,
                tradeSize: metrics?.optimal_size_usd || message.tradeSize || message.max_trade_size,
                grossProfit: metrics?.net_calculation?.gross_profit || message.grossProfit || message.estimated_profit,
                gasFee: metrics?.gas_estimate?.cost_usd || message.gasFee || message.gas_fee_usd || 0,
                dexFees: metrics?.dex_fees?.total_fee_usd || message.dexFees || message.dex_fees_usd || 0,
                slippage: metrics?.slippage_estimate?.impact_usd || message.slippageCost || message.slippage_cost_usd || 0,
                netProfit: metrics?.net_calculation?.net_profit || message.netProfit || message.net_profit_usd,
                netProfitPercent: message.netProfitPercent || message.net_profit_percent || 0,
                buyPool: message.buyPool || message.pool_a,
                sellPool: message.sellPool || message.pool_b,
                executable: metrics?.executable || message.executable
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
            } else if (message.msg_type === 'pool_swap') {
              // Process pool swap for token activity view
              const swap: PoolSwap = {
                pool_id: message.pool_address || message.pool_id || 'Unknown',
                pool_address: message.pool_address || 'Unknown',
                venue_name: message.venue_name || 'Unknown',
                token_in: message.token_in || 'Unknown',
                token_out: message.token_out || 'Unknown',
                token_in_symbol: message.token_in_symbol || message.token_in || 'TokenIn',
                token_out_symbol: message.token_out_symbol || message.token_out || 'TokenOut',
                amount_in: message.amount_in || { raw: '0', normalized: 0, decimals: 18 },
                amount_out: message.amount_out || { raw: '0', normalized: 0, decimals: 18 },
                sqrt_price_x96_after: message.sqrt_price_x96_after,
                tick_after: message.tick_after,
                liquidity_after: message.liquidity_after,
                // Convert nanoseconds to milliseconds if needed
                timestamp: message.timestamp > 1e15 ? Math.floor(message.timestamp / 1e6) : message.timestamp || Date.now(),
                block_number: message.block_number
              };

              console.log('🔄 Received pool swap:', swap);
              console.log('📊 Raw message amounts:', {
                amount_in_from_message: message.amount_in,
                amount_out_from_message: message.amount_out,
                amount_in_raw: message.amount_in?.raw,
                amount_out_raw: message.amount_out?.raw,
                amount_in_normalized: message.amount_in?.normalized,
                amount_out_normalized: message.amount_out?.normalized
              });

              // Update pool swaps with sliding window
              setPoolSwaps(prev => [swap, ...prev].slice(0, 100));

              // Update token pair activities (throttled)
              const now = Date.now();
              if (now - lastUpdateTime > 200) { // Throttle to 200ms
                setLastUpdateTime(now);
                updateTokenActivities(swap);
              }
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
  }, [lastUpdateTime]);

  // Function to update token pair activities
  const updateTokenActivities = (swap: PoolSwap) => {
    setTokenActivities(prev => {
      const tokenPair = `${formatTokenAddress(swap.token_in)}/${formatTokenAddress(swap.token_out)}`;
      const existingIndex = prev.findIndex(activity =>
        activity.token_pair === tokenPair && activity.pool_address === swap.pool_address
      );

      if (existingIndex >= 0) {
        // Update existing activity
        const updated = [...prev];
        updated[existingIndex] = {
          ...updated[existingIndex],
          latest_swap: swap,
          activity_count: updated[existingIndex].activity_count + 1,
          last_updated: swap.timestamp
        };
        // Sort by most recent activity
        return updated.sort((a, b) => b.last_updated - a.last_updated);
      } else {
        // Add new activity
        const newActivity: TokenPairActivity = {
          token_pair: tokenPair,
          pool_address: swap.pool_address,
          venue_name: swap.venue_name,
          latest_swap: swap,
          activity_count: 1,
          last_updated: swap.timestamp
        };
        return [newActivity, ...prev].slice(0, 50); // Keep 50 most recent activities
      }
    });
  };

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

  // Token metadata formatting utilities
  const formatTokenAddress = (address: string) => {
    if (!address || address === 'Unknown') return address;
    return `${address.substring(0, 6)}...${address.substring(address.length - 4)}`;
  };

  const formatAmount = (amount: { raw: string; normalized: number; decimals: number }) => {
    if (amount.normalized === 0) return '0';
    if (amount.normalized > 1000000) return `${(amount.normalized / 1000000).toFixed(2)}M`;
    if (amount.normalized > 1000) return `${(amount.normalized / 1000).toFixed(2)}K`;
    if (amount.normalized > 1) return amount.normalized.toFixed(4);
    return amount.normalized.toFixed(6);
  };

  const formatRawAmount = (rawAmount: string) => {
    if (!rawAmount || rawAmount === '0') return '0';
    const length = rawAmount.length;
    if (length > 15) {
      return `${rawAmount.substring(0, 6)}...${rawAmount.substring(length - 4)}`;
    }
    return rawAmount;
  };

  const formatTimestamp = (timestamp: number) => {
    const now = Date.now();
    const diff = now - timestamp;

    if (diff < 60000) return `${Math.floor(diff / 1000)}s ago`;
    if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`;
    return new Date(timestamp).toLocaleTimeString();
  };

  return (
    <div className="defi-arbitrage-table">
      <div className="header">
        <h2>Live Token Activity Monitor</h2>
        <div className="status">
          <span className={`indicator ${isConnected ? 'connected' : 'disconnected'}`} />
          {isConnected ? 'Connected' : 'Disconnected'}
        </div>
      </div>

      <div className="controls">
        <div className="view-mode-selector">
          <label>
            <input
              type="radio"
              name="viewMode"
              checked={viewMode === 'token_activity'}
              onChange={() => setViewMode('token_activity')}
            />
            Token Activity
          </label>
          <label>
            <input
              type="radio"
              name="viewMode"
              checked={viewMode === 'arbitrage'}
              onChange={() => setViewMode('arbitrage')}
            />
            Arbitrage Opportunities
          </label>
          <label>
            <input
              type="radio"
              name="viewMode"
              checked={viewMode === 'raw_swaps'}
              onChange={() => setViewMode('raw_swaps')}
            />
            Raw Pool Swaps
          </label>
        </div>
        {viewMode === 'arbitrage' && (
          <label>
            <input
              type="checkbox"
              checked={groupByPool}
              onChange={(e) => setGroupByPool(e.target.checked)}
            />
            Group by Pool Pair
          </label>
        )}
        <div className="stats">
          {viewMode === 'token_activity' ? (
            <>Active Pairs: {tokenActivities.length} | Recent Swaps: {poolSwaps.length}</>
          ) : viewMode === 'arbitrage' ? (
            <>Total: {opportunities.length} | Executable: {opportunities.filter(o => o.executable).length}</>
          ) : (
            <>Recent Swaps: {poolSwaps.length} | Live Updates</>
          )}
        </div>
      </div>

      {viewMode === 'token_activity' && (
        <div className="token-activity-section">
          <h3>Live Token Pair Activity</h3>
          <div className="token-activity-table">
            <table>
              <thead>
                <tr>
                  <th>Token Pair</th>
                  <th>Venue</th>
                  <th>Pool</th>
                  <th>Latest Swap</th>
                  <th>Amount In</th>
                  <th>Amount Out</th>
                  <th>Protocol Data</th>
                  <th>Block</th>
                  <th>Activity</th>
                  <th>Time</th>
                </tr>
              </thead>
              <tbody>
                {tokenActivities.map((activity, idx) => {
                  const isRecent = Date.now() - activity.last_updated < 5000; // Highlight recent activity
                  return (
                    <tr key={`${activity.pool_address}-${activity.token_pair}`}
                        className={`token-activity-row ${isRecent ? 'recent-activity' : ''}`}>
                      <td className="token-pair">{activity.token_pair}</td>
                      <td className="venue">{activity.venue_name}</td>
                      <td className="mono pool-address">{formatTokenAddress(activity.pool_address)}</td>
                      <td className="swap-direction">
                        {formatTokenAddress(activity.latest_swap.token_in)} → {formatTokenAddress(activity.latest_swap.token_out)}
                      </td>
                      <td className="amount-in">
                        <div className="amount">{formatAmount(activity.latest_swap.amount_in)}</div>
                        <div className="raw-amount">Raw: {formatRawAmount(activity.latest_swap.amount_in.raw)}</div>
                        <div className="decimals">({activity.latest_swap.amount_in.decimals}d)</div>
                      </td>
                      <td className="amount-out">
                        <div className="amount">{formatAmount(activity.latest_swap.amount_out)}</div>
                        <div className="raw-amount">Raw: {formatRawAmount(activity.latest_swap.amount_out.raw)}</div>
                        <div className="decimals">({activity.latest_swap.amount_out.decimals}d)</div>
                      </td>
                      <td className="protocol-data">
                        {activity.latest_swap.sqrt_price_x96_after && activity.latest_swap.sqrt_price_x96_after !== "0" && (
                          <div className="price-data">
                            <div className="label">Price:</div>
                            <div className="mono">{
                              typeof activity.latest_swap.sqrt_price_x96_after === 'string' &&
                              activity.latest_swap.sqrt_price_x96_after.length > 8
                                ? activity.latest_swap.sqrt_price_x96_after.substring(0, 8) + '...'
                                : activity.latest_swap.sqrt_price_x96_after
                            }</div>
                          </div>
                        )}
                        {activity.latest_swap.tick_after !== undefined && activity.latest_swap.tick_after !== 0 && (
                          <div className="tick-data">
                            <div className="label">Tick:</div>
                            <div>{activity.latest_swap.tick_after}</div>
                          </div>
                        )}
                        {activity.latest_swap.liquidity_after && activity.latest_swap.liquidity_after !== "0" && (
                          <div className="liquidity-data">
                            <div className="label">Liquidity:</div>
                            <div className="mono">{
                              typeof activity.latest_swap.liquidity_after === 'string' &&
                              activity.latest_swap.liquidity_after.length > 8
                                ? activity.latest_swap.liquidity_after.substring(0, 8) + '...'
                                : activity.latest_swap.liquidity_after
                            }</div>
                          </div>
                        )}
                      </td>
                      <td className="block-number">
                        {activity.latest_swap.block_number || '-'}
                      </td>
                      <td className="activity-count">
                        <span className="count">{activity.activity_count}</span>
                        <span className="label">swaps</span>
                      </td>
                      <td className="timestamp">{formatTimestamp(activity.last_updated)}</td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
            {tokenActivities.length === 0 && (
              <div className="empty-state">
                {isConnected ? 'Waiting for token swap activity...' : 'Connecting to data feed...'}
              </div>
            )}
          </div>
        </div>
      )}

      {viewMode === 'raw_swaps' && (
        <div className="pool-swaps-section">
          <h3>Raw Pool Swaps Stream</h3>
          <div className="pool-swaps-table">
            <table>
              <thead>
                <tr>
                  <th>Venue</th>
                  <th>Pool</th>
                  <th>Token In → Out</th>
                  <th>Amount In</th>
                  <th>Amount Out</th>
                  <th>Block</th>
                  <th>Time</th>
                </tr>
              </thead>
              <tbody>
                {poolSwaps.slice(0, 20).map((swap, idx) => {
                  const isVeryRecent = Date.now() - swap.timestamp < 2000;
                  return (
                    <tr key={`${swap.pool_id}-${swap.timestamp}-${idx}`}
                        className={`swap-row ${isVeryRecent ? 'very-recent' : ''}`}>
                      <td className="venue">{swap.venue_name}</td>
                      <td className="mono pool-address">{formatTokenAddress(swap.pool_address)}</td>
                      <td className="token-swap">
                        <div>{formatTokenAddress(swap.token_in)}</div>
                        <div>↓</div>
                        <div>{formatTokenAddress(swap.token_out)}</div>
                      </td>
                      <td className="amount">
                        <div>{formatAmount(swap.amount_in)}</div>
                        <div className="raw-amount">({swap.amount_in.decimals}d)</div>
                      </td>
                      <td className="amount">
                        <div>{formatAmount(swap.amount_out)}</div>
                        <div className="raw-amount">({swap.amount_out.decimals}d)</div>
                      </td>
                      <td className="block">{swap.block_number || '-'}</td>
                      <td className="timestamp">{formatTimestamp(swap.timestamp)}</td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
            {poolSwaps.length === 0 && (
              <div className="empty-state">
                {isConnected ? 'Waiting for pool swap data...' : 'Connecting to scanner...'}
              </div>
            )}
          </div>
        </div>
      )}

      {viewMode === 'arbitrage' && (
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
              <th>Net ↓</th>
              <th>%</th>
              <th>Status</th>
            </tr>
          </thead>
          <tbody>
            {Object.entries(groupedOpportunities).map(([poolKey, opps]) => (
              <React.Fragment key={poolKey}>
                {groupByPool && poolKey !== 'all' && (
                  <tr className="pool-group-header">
                    <td colSpan={12}>
                      Pool Pair: {poolKey.substring(0, 10)}...{poolKey.substring(poolKey.length - 6)}
                    </td>
                  </tr>
                )}
                {opps
                  .sort((a, b) => b.netProfit - a.netProfit) // Sort by net profit descending
                  .map((opp) => (
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
      )}

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
