// Real-time market data display component using WebSocket
import React, { useState, useEffect, useCallback } from 'react';
import wsService from '../../../services/WebSocketService';
import type { Trade } from '../../../services/WebSocketService';
// import type { OrderBook } from '../../../dashboard/types';

// Local interface for testing
interface OrderBook {
  symbol_hash: number;
  symbol?: string;
  timestamp: number;
  bids: Array<{price: number; size: number}>;
  asks: Array<{price: number; size: number}>;
}
import './RealTimeMarketData.module.css';

interface RealTimeMarketDataProps {
  symbols?: string[];
  channels?: string[];
}

const RealTimeMarketData: React.FC<RealTimeMarketDataProps> = ({
  symbols = ['BTC-USD', 'ETH-USD'],
  channels = ['trades', 'orderbook']
}) => {
  const [connected, setConnected] = useState(false);
  const [trades, setTrades] = useState<Record<string, Trade[]>>({});
  const [orderbooks, setOrderbooks] = useState<Record<string, OrderBook>>({});
  const [error, setError] = useState<string | null>(null);
  const [lastUpdate, setLastUpdate] = useState<Record<string, number>>({});

  useEffect(() => {
    // Setup event listeners
    const handleConnected = () => {
      setConnected(true);
      setError(null);
      console.log('WebSocket connected in component');
    };

    const handleDisconnected = () => {
      setConnected(false);
      console.log('WebSocket disconnected in component');
    };

    const handleError = (err: any) => {
      setError('WebSocket connection error');
      console.error('WebSocket error in component:', err);
    };

    const handleTrade = (symbol: string, trade: Trade) => {
      setTrades(prev => {
        const symbolTrades = prev[symbol] || [];
        // Keep only last 50 trades per symbol for better performance
        const updatedTrades = [trade, ...symbolTrades].slice(0, 50);
        return { ...prev, [symbol]: updatedTrades };
      });
      setLastUpdate(prev => ({ ...prev, [symbol]: Date.now() }));
    };

    const handleOrderbook = (symbol: string, orderbook: OrderBook) => {
      setOrderbooks(prev => ({ ...prev, [symbol]: orderbook }));
      setLastUpdate(prev => ({ ...prev, [symbol]: Date.now() }));
    };

    // Add event listeners
    wsService.on('connected', handleConnected);
    wsService.on('disconnected', handleDisconnected);
    wsService.on('error', handleError);
    wsService.on('trade', handleTrade);
    wsService.on('orderbook', handleOrderbook);

    // Connect and subscribe
    wsService.connect()
      .then(() => {
        wsService.subscribe(channels, symbols);
      })
      .catch(err => {
        setError('Failed to connect to WebSocket');
        console.error('Connection error:', err);
      });

    // Cleanup
    return () => {
      wsService.off('connected', handleConnected);
      wsService.off('disconnected', handleDisconnected);
      wsService.off('error', handleError);
      wsService.off('trade', handleTrade);
      wsService.off('orderbook', handleOrderbook);
      wsService.unsubscribe();
    };
  }, [symbols, channels]);

  const formatPrice = (price: number | undefined | null) => {
    if (price == null || isNaN(price)) return '$0.00';
    // Fix the fraction digits to avoid RangeError
    const fractionDigits = price > 10000 ? 0 : 2;
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      minimumFractionDigits: fractionDigits,
      maximumFractionDigits: fractionDigits
    }).format(price);
  };

  const formatVolume = (volume: number | undefined | null) => {
    if (volume == null || isNaN(volume)) return '0.00000000';
    return volume < 1 ? volume.toFixed(8) : volume.toFixed(4);
  };

  const formatTime = (timestamp: number | undefined | null) => {
    if (timestamp == null || isNaN(timestamp)) return '--:--:--';
    return new Date(timestamp * 1000).toLocaleTimeString();
  };

  const getLastUpdateText = (symbol: string) => {
    const lastTime = lastUpdate[symbol];
    if (!lastTime) return 'No data';
    const secondsAgo = Math.floor((Date.now() - lastTime) / 1000);
    if (secondsAgo < 1) return 'Live';
    if (secondsAgo < 60) return `${secondsAgo}s ago`;
    return `${Math.floor(secondsAgo / 60)}m ago`;
  };

  // Calculate mid price
  const getMidPrice = (orderbook: OrderBook | undefined) => {
    if (!orderbook || !orderbook.bids?.[0] || !orderbook.asks?.[0]) return null;
    return (orderbook.bids[0].price + orderbook.asks[0].price) / 2;
  };

  // Calculate spread
  const getSpread = (orderbook: OrderBook | undefined) => {
    if (!orderbook || !orderbook.bids?.[0] || !orderbook.asks?.[0]) return null;
    return orderbook.asks[0].price - orderbook.bids[0].price;
  };

  // Calculate total volume
  const getTotalVolume = (trades: Trade[]) => {
    return trades.reduce((sum, trade) => sum + (trade.volume || 0), 0);
  };

  return (
    <div className="realtime-market-data">
      <div className="header-section">
        <div className="connection-status">
          <span className={`status-indicator ${connected ? 'connected' : 'disconnected'}`} />
          <div className="status-text">
            <span className="status-label">WebSocket Status:</span>
            <span className={`status-value ${connected ? 'connected' : 'disconnected'}`}>
              {connected ? 'Connected' : 'Disconnected'}
            </span>
          </div>
          {error && <span className="error-message">{error}</span>}
        </div>
        <div className="header-info">
          <span>Real-Time Market Data Stream</span>
        </div>
      </div>

      <div className="market-data-grid">
        {symbols.map(symbol => {
          const symbolTrades = trades[symbol] || [];
          const symbolOrderbook = orderbooks[symbol];
          const midPrice = getMidPrice(symbolOrderbook);
          const spread = getSpread(symbolOrderbook);
          const totalVolume = getTotalVolume(symbolTrades);
          
          return (
            <div key={symbol} className="symbol-section">
              <div className="symbol-header">
                <h2>{symbol}</h2>
                <span className={`update-status ${getLastUpdateText(symbol) === 'Live' ? 'live' : ''}`}>
                  {getLastUpdateText(symbol)}
                </span>
              </div>
              
              {/* Market Summary */}
              <div className="market-summary">
                <div className="summary-item">
                  <span className="summary-label">Mid Price</span>
                  <span className="summary-value">{formatPrice(midPrice)}</span>
                </div>
                <div className="summary-item">
                  <span className="summary-label">Spread</span>
                  <span className="summary-value">{formatPrice(spread)}</span>
                </div>
                <div className="summary-item">
                  <span className="summary-label">24h Volume</span>
                  <span className="summary-value">{formatVolume(totalVolume)} {symbol.split('-')[0]}</span>
                </div>
                <div className="summary-item">
                  <span className="summary-label">Trades (50)</span>
                  <span className="summary-value">{symbolTrades.length}</span>
                </div>
              </div>
              
              <div className="data-panels">
                {/* Trades Panel */}
                <div className="trades-panel">
                  <h4>Recent Trades</h4>
                  <div className="trades-list">
                    <div className="trades-header">
                      <span>Time</span>
                      <span>Price</span>
                      <span>Volume</span>
                      <span>Side</span>
                    </div>
                    <div className="trades-content">
                      {symbolTrades.map((trade, idx) => (
                        <div 
                          key={`${trade.trade_id}-${idx}`} 
                          className={`trade-row ${trade.side?.toLowerCase()} ${idx === 0 ? 'latest' : ''}`}
                        >
                          <span className="time">{formatTime(trade.timestamp)}</span>
                          <span className="price">{formatPrice(trade.price)}</span>
                          <span className="volume">{formatVolume(trade.volume)}</span>
                          <span className={`side ${trade.side?.toLowerCase()}`}>
                            {trade.side || 'N/A'}
                          </span>
                        </div>
                      ))}
                      {!symbolTrades.length && (
                        <div className="no-data">Waiting for trades...</div>
                      )}
                    </div>
                  </div>
                </div>

                {/* Orderbook Panel */}
                <div className="orderbook-panel">
                  <h4>Order Book (Top 10 Levels)</h4>
                  {symbolOrderbook ? (
                    <div className="orderbook">
                      <div className="orderbook-side asks">
                        <div className="orderbook-header">
                          <span>Price (Ask)</span>
                          <span>Volume</span>
                        </div>
                        <div className="orderbook-content">
                          {(symbolOrderbook.asks || []).slice(0, 10).reverse().map((ask, idx) => (
                            <div key={`ask-${idx}`} className="orderbook-row ask">
                              <span className="price">{formatPrice(ask.price)}</span>
                              <span className="volume">{formatVolume(ask.size)}</span>
                              <div className="volume-bar ask" style={{ 
                                width: `${Math.min(100, (ask.size / Math.max(...(symbolOrderbook.asks || []).slice(0, 10).map(a => a.size))) * 100)}%` 
                              }} />
                            </div>
                          ))}
                        </div>
                      </div>
                      
                      <div className="spread-indicator">
                        <div className="spread-value">
                          <span className="label">Spread:</span>
                          <span className="value">{formatPrice(spread)}</span>
                          {spread && midPrice && (
                            <span className="percentage">({((spread / midPrice) * 100).toFixed(3)}%)</span>
                          )}
                        </div>
                      </div>
                      
                      <div className="orderbook-side bids">
                        <div className="orderbook-header">
                          <span>Price (Bid)</span>
                          <span>Volume</span>
                        </div>
                        <div className="orderbook-content">
                          {(symbolOrderbook.bids || []).slice(0, 10).map((bid, idx) => (
                            <div key={`bid-${idx}`} className="orderbook-row bid">
                              <span className="price">{formatPrice(bid.price)}</span>
                              <span className="volume">{formatVolume(bid.size)}</span>
                              <div className="volume-bar bid" style={{ 
                                width: `${Math.min(100, (bid.size / Math.max(...(symbolOrderbook.bids || []).slice(0, 10).map(b => b.size))) * 100)}%` 
                              }} />
                            </div>
                          ))}
                        </div>
                      </div>
                    </div>
                  ) : (
                    <div className="no-data">Waiting for orderbook data...</div>
                  )}
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};

export default RealTimeMarketData;