import React, { useMemo, useRef, useEffect } from 'react';
import './OrderbookVisualizer.css';
import type { OrderBook } from '../types';

interface Props {
  orderbook?: OrderBook;
  symbol: string;
  exchange: string;
  maxLevels?: number;
}

export function OrderbookVisualizer({ 
  orderbook, 
  symbol, 
  exchange,
  maxLevels = 20 
}: Props) {
  const contentRef = useRef<HTMLDivElement>(null);
  const spreadRef = useRef<HTMLDivElement>(null);
  const { bids, asks, spread, midPrice, totalBidVolume, totalAskVolume } = useMemo(() => {
    if (!orderbook) {
      return {
        bids: [],
        asks: [],
        spread: 0,
        midPrice: 0,
        totalBidVolume: 0,
        totalAskVolume: 0
      };
    }

    const sortedBids = [...orderbook.bids].sort((a, b) => b.price - a.price).slice(0, maxLevels);
    const sortedAsks = [...orderbook.asks].sort((a, b) => a.price - b.price).slice(0, maxLevels);
    
    const bestBid = sortedBids[0]?.price || 0;
    const bestAsk = sortedAsks[0]?.price || 0;
    
    return {
      bids: sortedBids,
      asks: sortedAsks,
      spread: bestAsk - bestBid,
      midPrice: (bestBid + bestAsk) / 2,
      totalBidVolume: sortedBids.reduce((sum, level) => sum + level.size, 0),
      totalAskVolume: sortedAsks.reduce((sum, level) => sum + level.size, 0)
    };
  }, [orderbook, maxLevels]);

  // Center spread on initial load and symbol changes
  useEffect(() => {
    if (contentRef.current && spreadRef.current && orderbook && bids.length > 0 && asks.length > 0) {
      // Wait for DOM to update with new orderbook data, then wait a bit more for layout
      requestAnimationFrame(() => {
        setTimeout(() => {
          const container = contentRef.current;
          const spread = spreadRef.current;
          
          if (container && spread) {
            const containerHeight = container.clientHeight;
            const spreadOffsetTop = spread.offsetTop;
            const spreadHeight = spread.clientHeight;
            
            // Calculate position to center the spread
            const targetScroll = spreadOffsetTop - (containerHeight / 2) + (spreadHeight / 2);
            
            // Set scroll position smoothly
            container.scrollTo({
              top: targetScroll,
              behavior: 'smooth'
            });
          }
        }, 100);  // Small delay to ensure layout is complete
      });
    }
  }, [symbol, orderbook, bids.length, asks.length]); // Re-center when symbol, orderbook, or data changes

  const maxVolume = useMemo(() => {
    const allLevels = [...bids, ...asks];
    return Math.max(...allLevels.map(l => l.size), 0.1);
  }, [bids, asks]);

  if (!orderbook) {
    return (
      <div className="orderbook-visualizer">
        <div className="panel-header">
          <h3 className="panel-title">Order Book - {exchange.toUpperCase()} {symbol}</h3>
          <span className="status-indicator">
            <span className="status-dot disconnected" />
            <span>No Data</span>
          </span>
        </div>
        <div className="no-data">Waiting for orderbook data...</div>
      </div>
    );
  }


  return (
    <div className="orderbook-visualizer">
      <div className="panel-header">
        <h3 className="panel-title">Order Book - {exchange.toUpperCase()} {symbol}</h3>
        <div className="orderbook-stats">
          <span className="stat">
            Mid: <span className="value">${midPrice.toFixed(2)}</span>
          </span>
          <span className="stat">
            Spread: <span className="value">${spread.toFixed(2)}</span>
          </span>
          <span className="stat">
            Levels: <span className="value">{bids.length}/{asks.length}</span>
          </span>
        </div>
      </div>

      <div className="orderbook-container">
        <div className="orderbook-header">
          <div className="header-row">
            <span>Price</span>
            <span>Size</span>
            <span>Total</span>
          </div>
        </div>

        <div className="orderbook-content" ref={contentRef}>
          {/* Asks (sells) - reversed order for display */}
          <div className="asks-section">
            {asks.slice().reverse().map((level, index) => {
              const cumulative = asks.slice(0, asks.length - index).reduce((sum, l) => sum + l.size, 0);
              const volumePercent = (level.size / maxVolume) * 100;
              
              return (
                <div key={`ask-${index}`} className="orderbook-row ask">
                  <div 
                    className="volume-bar ask-bar" 
                    style={{ width: `${volumePercent}%` }}
                  />
                  <span className="price">${level.price.toFixed(2)}</span>
                  <span className="size">{level.size.toFixed(4)}</span>
                  <span className="total">{cumulative.toFixed(4)}</span>
                </div>
              );
            })}
          </div>

          {/* Spread indicator */}
          <div className="spread-indicator" ref={spreadRef}>
            <span className="spread-label">SPREAD</span>
            <span className="spread-value">${spread.toFixed(2)}</span>
            <span className="spread-percent">
              ({((spread / midPrice) * 100).toFixed(3)}%)
            </span>
          </div>

          {/* Bids (buys) */}
          <div className="bids-section">
            {bids.map((level, index) => {
              const cumulative = bids.slice(0, index + 1).reduce((sum, l) => sum + l.size, 0);
              const volumePercent = (level.size / maxVolume) * 100;
              
              return (
                <div key={`bid-${index}`} className="orderbook-row bid">
                  <div 
                    className="volume-bar bid-bar" 
                    style={{ width: `${volumePercent}%` }}
                  />
                  <span className="price">${level.price.toFixed(2)}</span>
                  <span className="size">{level.size.toFixed(4)}</span>
                  <span className="total">{cumulative.toFixed(4)}</span>
                </div>
              );
            })}
          </div>
        </div>

        <div className="orderbook-footer">
          <div className="volume-summary">
            <span className="bid-volume">
              Bid Vol: {totalBidVolume.toFixed(2)}
            </span>
            <span className="ask-volume">
              Ask Vol: {totalAskVolume.toFixed(2)}
            </span>
            <span className="imbalance">
              Imbalance: {((totalBidVolume / (totalBidVolume + totalAskVolume)) * 100).toFixed(1)}%
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}