import React, { useState, useEffect, useMemo } from 'react';
import { useWebSocketFirehose } from '../hooks/useWebSocketFirehose';
import type { Trade, OrderBook, L2Delta, SymbolMapping } from '../types';
import { OptionsChain } from './OptionsChain';
import { LatencyMonitor } from './LatencyMonitor';
import './AlpacaStreaming.css';

interface AlpacaStreamingProps {
  onSymbolSelect?: (symbol: string) => void;
}

interface OrderBookState {
  bids: Map<number, number>;
  asks: Map<number, number>;
  sequence: number;
}

export const AlpacaStreaming: React.FC<AlpacaStreamingProps> = ({ onSymbolSelect }) => {
  const { trades, orderbooks, isConnected } = useWebSocketFirehose('/ws/firehose');
  const [symbolMappings, setSymbolMappings] = useState<Map<number, string>>(new Map());
  const [selectedSymbol, setSelectedSymbol] = useState<string>('AAPL');
  const [orderbookStates, setOrderbookStates] = useState<Map<number, OrderBookState>>(new Map());
  const [showOptionsFor, setShowOptionsFor] = useState<string>('');

  // Process symbol mappings from WebSocket
  useEffect(() => {
    // This will be updated when we receive symbol_mapping messages
    const handleSymbolMapping = (mapping: SymbolMapping) => {
      setSymbolMappings(prev => new Map(prev.set(mapping.symbol_hash, mapping.symbol)));
    };

    // Listen for symbol mapping messages
    // In a real implementation, this would be part of the WebSocket hook
  }, []);

  // Note: Alpaca doesn't provide L2 data - this is for future use with other exchanges
  // For now, we'll aggregate trade data to show volume profiles instead
  const processL2Delta = (delta: L2Delta) => {
    // Skip L2 processing for Alpaca symbols as they don't provide L2 data
    const symbol = symbolMappings.get(delta.symbol_hash);
    if (symbol && alpacaSymbols.includes(symbol)) {
      return; // Alpaca doesn't provide L2 data
    }
    
    setOrderbookStates(prev => {
      const current = prev.get(delta.symbol_hash) || {
        bids: new Map(),
        asks: new Map(),
        sequence: 0
      };

      if (delta.sequence <= current.sequence) {
        return prev; // Ignore out-of-order updates
      }

      const newState = {
        bids: new Map(current.bids),
        asks: new Map(current.asks),
        sequence: delta.sequence
      };

      delta.updates.forEach(update => {
        const book = update.side === 'bid' ? newState.bids : newState.asks;
        
        switch (update.action) {
          case 'delete':
            book.delete(update.price);
            break;
          case 'update':
          case 'insert':
            if (update.size === 0) {
              book.delete(update.price);
            } else {
              book.set(update.price, update.size);
            }
            break;
        }
      });

      return new Map(prev.set(delta.symbol_hash, newState));
    });
  };

  // Alpaca stock symbols
  const alpacaSymbols = ['AAPL', 'MSFT', 'GOOGL', 'AMZN', 'TSLA', 'SPY', 'QQQ', 'NVDA', 'META', 'AMD'];
  
  // Filter trades for Alpaca (stocks)
  const alpacaTrades = useMemo(() => {
    return trades.filter(trade => {
      const symbol = symbolMappings.get(trade.symbol_hash);
      return symbol && alpacaSymbols.includes(symbol);
    }).slice(-50); // Keep last 50 trades
  }, [trades, symbolMappings]);

  // Get orderbook for selected symbol
  const selectedOrderbook = useMemo(() => {
    const symbolHash = Array.from(symbolMappings.entries())
      .find(([_, symbol]) => symbol === selectedSymbol)?.[0];
    
    if (!symbolHash) return null;

    const state = orderbookStates.get(symbolHash);
    if (!state) return null;

    // Convert to OrderBook format
    const bids = Array.from(state.bids.entries())
      .map(([price, size]) => ({ price, size }))
      .sort((a, b) => b.price - a.price)
      .slice(0, 10);

    const asks = Array.from(state.asks.entries())
      .map(([price, size]) => ({ price, size }))
      .sort((a, b) => a.price - b.price)
      .slice(0, 10);

    return { bids, asks, symbolHash };
  }, [selectedSymbol, orderbookStates, symbolMappings]);

  // Calculate real-time metrics
  const metrics = useMemo(() => {
    const symbolCounts = new Map<string, number>();
    const recentTrades = alpacaTrades.filter(t => Date.now() - t.timestamp < 60000);
    
    recentTrades.forEach(trade => {
      const symbol = symbolMappings.get(trade.symbol_hash) || 'UNKNOWN';
      symbolCounts.set(symbol, (symbolCounts.get(symbol) || 0) + 1);
    });

    return {
      totalTrades: alpacaTrades.length,
      tradesPerMinute: recentTrades.length,
      activeSymbols: symbolCounts.size,
      mostActiveSymbol: Array.from(symbolCounts.entries()).sort((a, b) => b[1] - a[1])[0]?.[0] || 'N/A'
    };
  }, [alpacaTrades, symbolMappings]);

  return (
    <div className="alpaca-streaming">
      <div className="alpaca-header">
        <div className="status-bar">
          <span className={`connection-status ${isConnected ? 'connected' : 'disconnected'}`}>
            {isConnected ? '🟢 Connected' : '🔴 Disconnected'}
          </span>
          <div className="metrics">
            <span>Trades: {metrics.totalTrades}</span>
            <span>Trades/min: {metrics.tradesPerMinute}</span>
            <span>Active: {metrics.activeSymbols} symbols</span>
            <span>Most Active: {metrics.mostActiveSymbol}</span>
          </div>
        </div>

        <div className="symbol-selector">
          <select 
            value={selectedSymbol} 
            onChange={(e) => {
              setSelectedSymbol(e.target.value);
              onSymbolSelect?.(e.target.value);
            }}
          >
            {alpacaSymbols.map(symbol => (
              <option key={symbol} value={symbol}>{symbol}</option>
            ))}
          </select>
          <button 
            className="options-btn"
            onClick={() => setShowOptionsFor(selectedSymbol)}
          >
            Show Options Chain
          </button>
        </div>
      </div>

      <div className="alpaca-content">
        <div className="left-panel">
          <div className="trades-panel">
            <h3>Recent Trades</h3>
            <div className="trades-list">
              {alpacaTrades.map((trade, idx) => {
                const symbol = symbolMappings.get(trade.symbol_hash) || `Hash:${trade.symbol_hash}`;
                return (
                  <div key={idx} className={`trade-row ${trade.side}`}>
                    <span className="symbol">{symbol}</span>
                    <span className="price">${trade.price.toFixed(2)}</span>
                    <span className="volume">{trade.volume.toFixed(0)}</span>
                    <span className="side">{trade.side}</span>
                    <span className="time">
                      {new Date(trade.timestamp).toLocaleTimeString()}
                    </span>
                  </div>
                );
              })}
            </div>
          </div>

          <div className="symbol-mappings">
            <h3>Symbol Hash Mappings</h3>
            <div className="mappings-list">
              {Array.from(symbolMappings.entries())
                .filter(([_, symbol]) => alpacaSymbols.includes(symbol))
                .map(([hash, symbol]) => (
                  <div key={hash} className="mapping-row">
                    <span className="hash">{hash}</span>
                    <span className="symbol">{symbol}</span>
                  </div>
                ))}
            </div>
          </div>
        </div>

        <div className="right-panel">
          <div className="orderbook-panel">
            <h3>Trade Volume Profile - {selectedSymbol}</h3>
            {selectedOrderbook ? (
              <div className="orderbook">
                <div className="asks">
                  <h4>Asks</h4>
                  {selectedOrderbook.asks.map((level, idx) => (
                    <div key={idx} className="level ask">
                      <span className="price">${level.price.toFixed(2)}</span>
                      <span className="size">{level.size.toFixed(0)}</span>
                    </div>
                  ))}
                </div>
                <div className="spread">
                  {selectedOrderbook.asks.length > 0 && selectedOrderbook.bids.length > 0 && (
                    <div className="spread-info">
                      Spread: ${(selectedOrderbook.asks[0].price - selectedOrderbook.bids[0].price).toFixed(2)}
                    </div>
                  )}
                </div>
                <div className="bids">
                  <h4>Bids</h4>
                  {selectedOrderbook.bids.map((level, idx) => (
                    <div key={idx} className="level bid">
                      <span className="price">${level.price.toFixed(2)}</span>
                      <span className="size">{level.size.toFixed(0)}</span>
                    </div>
                  ))}
                </div>
              </div>
            ) : (
              <div className="no-data">No volume profile data for {selectedSymbol} (Alpaca provides trade data only)</div>
            )}
          </div>
        </div>
      </div>

      {/* Latency Monitor */}
      <LatencyMonitor trades={alpacaTrades} />

      {showOptionsFor && (
        <OptionsChain 
          symbol={showOptionsFor}
          onClose={() => setShowOptionsFor('')}
        />
      )}
    </div>
  );
};