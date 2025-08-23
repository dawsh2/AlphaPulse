import React, { useState, useRef, useEffect } from 'react';
import './WebSocketFirehose.css';
import type { Trade, OrderBook } from '../types';

interface Props {
  trades: Trade[];
  orderbooks: Record<number, OrderBook>;
}

export function WebSocketFirehose({ trades, orderbooks }: Props) {
  const [filter, setFilter] = useState('all');
  const [isPaused, setIsPaused] = useState(false);
  const [messages, setMessages] = useState<any[]>([]);
  const [uniquePairsWS, setUniquePairsWS] = useState<Set<string>>(new Set());
  const containerRef = useRef<HTMLDivElement>(null);
  const MAX_MESSAGES = 2000; // Increased buffer for longer observation

  // Track last processed trade index to avoid duplicates
  const lastProcessedIndexRef = useRef<number>(-1);
  
  useEffect(() => {
    if (isPaused) return;
    
    // Process only new trades since last update
    const startIndex = lastProcessedIndexRef.current + 1;
    const newTrades = trades.slice(startIndex);
    
    if (newTrades.length > 0) {
      const newMessages = newTrades.map(trade => ({
        type: 'trade',
        timestamp: trade.timestamp,
        data: trade
      }));
      
      // Track unique pairs from WebSocket
      newTrades.forEach(trade => {
        if (trade.symbol) {
          // Extract pair name similar to DeFi dashboard logic
          const parts = trade.symbol.split(':');
          let pairName = '';
          
          if (parts.length >= 3) {
            // Format: "polygon:0xABC123:DAI/LGNS"
            pairName = parts[2];
          } else if (parts.length >= 2) {
            // Format: "exchange:PAIR"
            pairName = parts[1];
          } else {
            pairName = trade.symbol;
          }
          
          setUniquePairsWS(prevSet => {
            const newSet = new Set([...prevSet, pairName]);
            // Log every 5th unique pair to see what's accumulating, and always log first 30
            if (newSet.size % 5 === 0 || newSet.size <= 30) {
              console.log(`📈 WebSocket unique pairs: ${newSet.size} (latest: ${pairName})`);
              if (newSet.size <= 10) {
                console.log(`Current pairs:`, Array.from(newSet));
              }
            }
            return newSet;
          });
        }
      });
      
      setMessages(prev => [...prev, ...newMessages].slice(-MAX_MESSAGES));
      lastProcessedIndexRef.current = trades.length - 1;
    }
  }, [trades, isPaused]);

  // Periodic comparison report
  useEffect(() => {
    const interval = setInterval(async () => {
      console.log(`🔥 WebSocket Firehose Status: ${uniquePairsWS.size} unique pairs, ${messages.length} messages`);
      
      if (uniquePairsWS.size > 0) {
        console.log('📊 PAIR COMPARISON REPORT:');
        console.log(`WebSocket Firehose unique pairs: ${uniquePairsWS.size}`);
        
        // Try to get arbitrage dashboard data from window
        const arbComponent = (window as any).__defiArbPairs;
        if (arbComponent) {
          console.log(`Arbitrage Dashboard unique pairs: ${arbComponent.size}`);
          
          const wsOnly = Array.from(uniquePairsWS).filter(p => !arbComponent.has(p));
          const arbOnly = Array.from(arbComponent).filter(p => !uniquePairsWS.has(p));
          
          if (wsOnly.length > 0) {
            console.log(`Pairs ONLY in WebSocket (${wsOnly.length}):`, wsOnly.slice(0, 10));
          }
          if (arbOnly.length > 0) {
            console.log(`Pairs ONLY in Arbitrage (${arbOnly.length}):`, arbOnly.slice(0, 10));
          }
          
          const common = Array.from(uniquePairsWS).filter(p => arbComponent.has(p));
          console.log(`Common pairs: ${common.length}`);
          
          // Save report to disk
          try {
            const reportData = {
              websocket_pairs: Array.from(uniquePairsWS),
              dashboard_pairs: Array.from(arbComponent),
              timestamp: new Date().toISOString()
            };
            
            const response = await fetch('/api/pair-comparison-report', {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify(reportData)
            });
            
            if (response.ok) {
              const result = await response.json();
              console.log(`💾 Report saved to disk: ${result.filename}`);
              console.log(`📈 Filtering ratio: ${result.summary.filtering_ratio}% of pairs filtered out`);
            } else {
              console.error('Failed to save report to disk:', response.status);
            }
          } catch (error) {
            console.error('Error saving report to disk:', error);
          }
        }
      }
    }, 30000); // Every 30 seconds (less frequent disk writes)
    
    return () => clearInterval(interval);
  }, [uniquePairsWS]);

  useEffect(() => {
    if (containerRef.current && !isPaused) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [messages, isPaused]);

  const triggerManualReport = async () => {
    console.log('🔍 MANUAL PAIR COMPARISON:');
    console.log(`WebSocket Firehose unique pairs: ${uniquePairsWS.size}`);
    console.log('Sample WebSocket pairs:', Array.from(uniquePairsWS).slice(0, 20));
    
    const arbComponent = (window as any).__defiArbPairs;
    if (arbComponent) {
      console.log(`Arbitrage Dashboard unique pairs: ${arbComponent.size}`);
      console.log('Sample Dashboard pairs:', Array.from(arbComponent).slice(0, 20));
      
      const wsOnly = Array.from(uniquePairsWS).filter(p => !arbComponent.has(p));
      console.log(`Pairs ONLY in WebSocket (${wsOnly.length}):`, wsOnly.slice(0, 20));
    }
  };

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
          <button 
            onClick={triggerManualReport}
            style={{
              marginLeft: '8px',
              padding: '4px 8px',
              backgroundColor: '#3b82f6',
              color: 'white',
              border: 'none',
              borderRadius: '4px',
              fontSize: '12px',
              cursor: 'pointer'
            }}
          >
            📊 Compare
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