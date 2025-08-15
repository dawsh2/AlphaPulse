import React, { useState, useEffect, useMemo, useRef } from 'react';
import { useWebSocketFirehose } from '../hooks/useWebSocketFirehose';
import './DeFiArbitrage.css';

interface PoolPrice {
  poolAddress: string;
  price: number;
  liquidity: number;
  tradeSize: number; // Size of this specific trade (for sizing reference)
  timestamp: number;
  latency: number;
  gasCost: number;
}

interface AssetPairData {
  pair: string;
  pools: PoolPrice[];
  minPrice: number;
  maxPrice: number;
  priceDifference: number;
  priceDifferencePercent: number;
  bestBuyPool: string;
  bestSellPool: string;
  arbitrageProfit: number;
  totalLiquidity: number;
  avgGasCost: number;
  profitPotential: number;
  lastUpdate: number;
}

interface SystemMetrics {
  connectedPools: number;
  totalPairs: number;
  avgLatency: number;
  systemStatus: 'online' | 'offline' | 'degraded';
}

export const DeFiArbitrage: React.FC = () => {
  const [assetPairs, setAssetPairs] = useState<AssetPairData[]>([]);
  const [poolPrices, setPoolPrices] = useState<Map<string, PoolPrice[]>>(new Map());
  const [minDifferenceFilter, setMinDifferenceFilter] = useState(0.01); // Lower threshold to show more pairs
  const [pairVolumeHistory, setPairVolumeHistory] = useState<Map<string, {timestamp: number, volume: number}[]>>(new Map());

  // Use the shared WebSocket connection
  const { trades, isConnected } = useWebSocketFirehose('ws://localhost:8765');

  // Throttle updates to prevent flickering
  const updateThrottleRef = useRef<Map<string, number>>(new Map());
  const UPDATE_THROTTLE_MS = 500; // Only update each pool every 500ms

  // Process incoming trades into pool price data
  useEffect(() => {
    if (trades.length === 0) return;

    const latestTrade = trades[trades.length - 1];
    if (!latestTrade.symbol || !latestTrade.price) return;

    // Parse symbol format (e.g., "polygon:0xABC123:DAI/LGNS" or legacy "quickswap:DAI-USDT")
    const parts = latestTrade.symbol.split(':');
    if (parts.length >= 2) {
      const exchange = parts[0].toLowerCase();
      // Handle new pool-specific format: "polygon:0xABC123:DAI/LGNS"
      const pair = parts.length >= 3 ? parts[2] : parts[1];

      // Filter for DeFi/DEX exchanges only (exclude traditional exchanges)
      const SUPPORTED_EXCHANGES = new Set([
        'polygon', 'quickswap', 'sushiswap', 'dfyn', 'polyswap', 'comethswap', 'uniswap'
      ]);

      if (!SUPPORTED_EXCHANGES.has(exchange)) {
        console.log('Skipping non-DEX exchange:', exchange);
        return;
      }
      

      // Generate pool identifier - use actual pool address from new format or fallback
      const poolAddress = parts.length >= 3 ? parts[1] : `${exchange}-${pair}`;
      
      // Throttle updates to prevent flickering
      const now = Date.now();
      const lastUpdate = updateThrottleRef.current.get(poolAddress) || 0;
      if (now - lastUpdate < UPDATE_THROTTLE_MS) {
        return; // Skip this update to prevent flickering
      }
      updateThrottleRef.current.set(poolAddress, now);

      // Estimate gas cost based on exchange type (simplified)
      const estimatedGasCost = exchange === 'polygon' ? 0.01 : 0.03; // Polygon is cheaper than mainnet
      
      // Track volume history for cumulative calculations (store {timestamp, volume} objects)
      setPairVolumeHistory(prev => {
        const updated = new Map(prev);
        const history = updated.get(pair) || [];
        const now = Date.now();
        
        // Add new trade volume with timestamp
        const newEntry = { timestamp: now, volume: latestTrade.volume || 0 };
        const updatedHistory = [...history, newEntry];
        
        // Keep last 100 trades (sliding window for performance)
        const recentHistory = updatedHistory.slice(-100);
        updated.set(pair, recentHistory);
        return updated;
      });

      // Calculate cumulative volume from recent trades (not 24h, but recent activity)
      const volume24h = pairVolumeHistory.get(pair)?.reduce((sum, entry) => sum + entry.volume, 0) || 0;

      const poolPrice: PoolPrice = {
        poolAddress,
        price: latestTrade.price,
        liquidity: latestTrade.volume || 0,
        volume24h,
        timestamp: latestTrade.timestamp,
        latency: latestTrade.latency_total_us ? Math.round(latestTrade.latency_total_us / 1000) : 5,
        gasCost: estimatedGasCost
      };

      setPoolPrices(prev => {
        const updated = new Map(prev);
        const existing = updated.get(pair) || [];
        const filtered = existing.filter(p => p.poolAddress !== poolAddress);
        updated.set(pair, [...filtered, poolPrice]);
        return updated;
      });
    }
  }, [trades]);

  // Process pool prices into asset pair data - sorted by LARGEST arbitrage first
  useEffect(() => {
    const processAssetPairs = () => {
      const pairData: AssetPairData[] = [];
      
      poolPrices.forEach((pools, pair) => {
        // Show single pools as well, not just arbitrage opportunities
        const validPools = pools.filter(p => p.price > 0);
        if (validPools.length < 1) return;
        
        const sortedPools = [...validPools].sort((a, b) => a.price - b.price);
        const minPrice = sortedPools[0].price;
        const maxPrice = sortedPools[sortedPools.length - 1].price;
        const priceDifference = maxPrice - minPrice;
        const priceDifferencePercent = validPools.length > 1 ? (priceDifference / minPrice) * 100 : 0;
        
        // Include all pairs regardless of price difference
        if (true) {
          const bestBuyPool = sortedPools[0].poolAddress;
          const bestSellPool = validPools.length > 1 ? sortedPools[sortedPools.length - 1].poolAddress : sortedPools[0].poolAddress;
          
          // Calculate metrics
          const totalLiquidity = validPools.reduce((sum, pool) => sum + pool.liquidity, 0);
          const avgGasCost = validPools.reduce((sum, pool) => sum + pool.gasCost, 0) / validPools.length;
          
          // Calculate profit potential (arbitrage % minus gas costs)
          const profitPotential = Math.max(0, priceDifferencePercent - (avgGasCost * 100 / minPrice));
          
          pairData.push({
            pair,
            pools: validPools,
            minPrice,
            maxPrice,
            priceDifference,
            priceDifferencePercent,
            bestBuyPool,
            bestSellPool,
            arbitrageProfit: priceDifferencePercent,
            totalLiquidity,
            avgGasCost,
            profitPotential,
            lastUpdate: Math.max(...validPools.map(p => p.timestamp))
          });
        }
      });
      
      // Sort by LARGEST price difference first (descending)
      pairData.sort((a, b) => b.priceDifferencePercent - a.priceDifferencePercent);
      
      setAssetPairs(pairData);
    };
    
    processAssetPairs();
  }, [poolPrices, minDifferenceFilter]);

  const displayedPairs = useMemo(() => {
    return assetPairs.slice(0, 50); // Limit to top 50 pairs for performance
  }, [assetPairs]);

  // Calculate metrics
  const connectedPools = Array.from(poolPrices.values()).reduce((sum, pools) => sum + pools.length, 0);
  

  return (
    <div className="defi-arbitrage">
      {!isConnected && (
        <div style={{ padding: '2rem', textAlign: 'center', color: '#ef4444' }}>
          Not connected to WebSocket
        </div>
      )}
      
      {isConnected && displayedPairs.length === 0 && (
        <div style={{ padding: '2rem', textAlign: 'center', color: '#666' }}>
          Waiting for pool data... ({poolPrices.size} pairs, {connectedPools} pools, {trades.length} trades received)
        </div>
      )}
      
      <div className="pairs-list">
        {displayedPairs.map((pairData, idx) => (
          <div key={pairData.pair} className="pair-item">
            <div className="pair-header">
              <span className="pair-name">{pairData.pair}</span>
              <span className="price-difference">{pairData.priceDifferencePercent.toFixed(3)}%</span>
            </div>
            <div className="pair-metrics">
              <div className="metric">
                <span className="metric-label">Liquidity:</span>
                <span className="metric-value">${pairData.totalLiquidity.toLocaleString()}</span>
              </div>
              <div className="metric">
                <span className="metric-label">Gas Cost:</span>
                <span className="metric-value">${pairData.avgGasCost.toFixed(3)}</span>
              </div>
              <div className="metric">
                <span className="metric-label">Net Profit:</span>
                <span className={`metric-value ${pairData.profitPotential > 0 ? 'profit-positive' : 'profit-negative'}`}>
                  {pairData.profitPotential.toFixed(3)}%
                </span>
              </div>
            </div>
            <div className="pools">
              {pairData.pools
                .sort((a, b) => a.price - b.price)
                .map((pool, poolIdx) => (
                  <div key={poolIdx} className="pool">
                    <span className="pool-name">{pool.poolAddress.slice(0, 8)}...{pool.poolAddress.slice(-6)}</span>
                    <span className="pool-price">${pool.price.toFixed(6)}</span>
                    <span className="pool-volume">Vol: ${pool.volume24h.toFixed(0)}</span>
                  </div>
                ))}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};