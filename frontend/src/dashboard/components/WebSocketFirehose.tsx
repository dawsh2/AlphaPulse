import React, { useState, useRef, useEffect } from 'react';
import './WebSocketFirehose.css';
import type { Trade, OrderBook } from '../types';

interface Props {
  trades: Trade[];
  orderbooks: Record<string, OrderBook>;
}

export function WebSocketFirehose({ trades, orderbooks }: Props) {
  const [filter, setFilter] = useState('all');
  const [isPaused, setIsPaused] = useState(false);
  const [messages, setMessages] = useState<any[]>([]);
  const containerRef = useRef<HTMLDivElement>(null);
  const MAX_MESSAGES = 500;

  // Track last processed trade to avoid duplicates
  const lastProcessedTradeRef = useRef<string | null>(null);
  
  useEffect(() => {
    if (isPaused) return;
    
    // Get the last trade
    const lastTrade = trades[trades.length - 1];
    if (!lastTrade) return;
    
    // Check if we've already processed this trade
    if (lastProcessedTradeRef.current === lastTrade.trade_id) return;
    
    // Process only new trades since last update
    const startIndex = lastProcessedTradeRef.current 
      ? trades.findIndex(t => t.trade_id === lastProcessedTradeRef.current) + 1
      : Math.max(0, trades.length - 5);
    
    const newTrades = trades.slice(startIndex);
    
    if (newTrades.length > 0) {
      const newMessages = newTrades.map(trade => ({
        type: 'trade',
        timestamp: trade.timestamp,
        data: trade
      }));
      
      setMessages(prev => [...prev, ...newMessages].slice(-MAX_MESSAGES));
      lastProcessedTradeRef.current = lastTrade.trade_id;
    }
  }, [trades, isPaused]);

  useEffect(() => {
    if (containerRef.current && !isPaused) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [messages, isPaused]);

  const filteredMessages = messages.filter(msg => {
    if (filter === 'all') return true;
    return msg.type === filter;
  });

  return (
    <div className="websocket-firehose">
      <div className="panel-header">
        <h3 className="panel-title">WebSocket Firehose (Raw)</h3>
        <div className="firehose-controls">
          <select 
            value={filter} 
            onChange={(e) => setFilter(e.target.value)}
            className="filter-select"
          >
            <option value="all">All</option>
            <option value="trade">Trades</option>
            <option value="orderbook">Orderbook</option>
            <option value="metrics">Metrics</option>
          </select>
          <button 
            className={`pause-btn ${isPaused ? 'paused' : ''}`}
            onClick={() => setIsPaused(!isPaused)}
          >
            {isPaused ? '▶ Resume' : '⏸ Pause'}
          </button>
          <button 
            className="clear-btn"
            onClick={() => setMessages([])}
          >
            Clear
          </button>
        </div>
      </div>

      <div className="firehose-container" ref={containerRef}>
        {filteredMessages.map((msg, index) => (
          <div key={index} className={`message-row ${msg.type}`}>
            <span className="message-time">
              {new Date(msg.timestamp).toLocaleTimeString('en-US', {
                hour12: false,
                hour: '2-digit',
                minute: '2-digit',
                second: '2-digit',
                fractionalSecondDigits: 3
              })}
            </span>
            <span className="message-type">[{msg.type.toUpperCase()}]</span>
            <span className="message-data">
              {JSON.stringify(msg.data, null, 2)}
            </span>
          </div>
        ))}
      </div>

      <div className="firehose-footer">
        <span className="message-count">
          {filteredMessages.length} messages
        </span>
        <span className="buffer-info">
          Buffer: {messages.length}/{MAX_MESSAGES}
        </span>
      </div>
    </div>
  );
}