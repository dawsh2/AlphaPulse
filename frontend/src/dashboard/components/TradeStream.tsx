import React, { useEffect, useRef } from 'react';
import './TradeStream.css';
import type { Trade } from '../types';

interface Props {
  trades: Trade[];
  symbol: string;
  exchange: string;
  maxTrades?: number;
}

export function TradeStream({ trades, symbol, exchange, maxTrades = 100 }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const autoScroll = useRef(true);

  // Auto-scroll to bottom when new trades arrive
  useEffect(() => {
    if (autoScroll.current && containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [trades]);

  const handleScroll = () => {
    if (!containerRef.current) return;
    
    const { scrollTop, scrollHeight, clientHeight } = containerRef.current;
    // Disable auto-scroll if user scrolls up
    autoScroll.current = Math.abs(scrollHeight - clientHeight - scrollTop) < 10;
  };

  const recentTrades = trades.slice(-maxTrades);
  
  return (
    <div className="trade-stream">
      <div className="panel-header">
        <h3 className="panel-title">Trade Stream</h3>
        <div className="trade-stats">
          <span className="stat">
            Count: <span className="value">{trades.length}</span>
          </span>
          <span className="stat">
            Rate: <span className="value">
              {trades.filter(t => Date.now() - t.timestamp < 1000).length}/s
            </span>
          </span>
        </div>
      </div>

      <div 
        className="trades-container" 
        ref={containerRef}
        onScroll={handleScroll}
      >
        <div className="trades-header">
          <span>Time</span>
          <span>Price</span>
          <span>Size</span>
        </div>
        
        <div className="trades-list">
          {recentTrades.map((trade, index) => (
            <div 
              key={`${trade.trade_id}-${index}`} 
              className={`trade-row ${trade.side}`}
            >
              <span className="trade-time">
                {new Date(trade.timestamp).toLocaleTimeString('en-US', {
                  hour12: false,
                  hour: '2-digit',
                  minute: '2-digit',
                  second: '2-digit',
                  fractionalSecondDigits: 3
                })}
              </span>
              <span className="trade-price">
                ${trade.price.toFixed(2)}
              </span>
              <span className="trade-volume">
                {trade.volume.toFixed(4)}
              </span>
              <span className={`trade-indicator ${trade.side}`}>
                {trade.side === 'buy' ? '▲' : '▼'}
              </span>
            </div>
          ))}
        </div>
      </div>

      <div className="trade-stream-footer">
        <button 
          className={`auto-scroll-btn ${autoScroll.current ? 'active' : ''}`}
          onClick={() => {
            autoScroll.current = !autoScroll.current;
            if (autoScroll.current && containerRef.current) {
              containerRef.current.scrollTop = containerRef.current.scrollHeight;
            }
          }}
        >
          Auto-Scroll: {autoScroll.current ? 'ON' : 'OFF'}
        </button>
      </div>
    </div>
  );
}