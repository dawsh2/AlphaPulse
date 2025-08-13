// Real-time market data display component using WebSocket
import React, { useState, useEffect, useCallback } from 'react';
import wsService, { Trade } from '../../../services/WebSocketService';
import './RealTimeMarketData.module.css';

interface OrderBook {
  bids: Array<[number, number]>; // [price, volume]
  asks: Array<[number, number]>; // [price, volume]
  timestamp: number;
}

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
        // Keep only last 20 trades per symbol
        const updatedTrades = [trade, ...symbolTrades].slice(0, 20);
        return { ...prev, [symbol]: updatedTrades };
      });
    };

    const handleOrderbook = (symbol: string, orderbook: OrderBook) => {
      setOrderbooks(prev => ({ ...prev, [symbol]: orderbook }));
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

  const formatPrice = (price: number) => {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      minimumFractionDigits: 2,
      maximumFractionDigits: 2
    }).format(price);
  };

  const formatVolume = (volume: number) => {
    return volume.toFixed(8);
  };

  const formatTime = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleTimeString();
  };

  return (
    <div className="realtime-market-data">
      <div className="connection-status">
        <span className={`status-indicator ${connected ? 'connected' : 'disconnected'}`} />
        {connected ? 'Connected' : 'Disconnected'}
        {error && <span className="error-message">{error}</span>}
      </div>

      <div className="market-data-grid">
        {symbols.map(symbol => (
          <div key={symbol} className="symbol-section">
            <h3>{symbol}</h3>
            
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
                  {(trades[symbol] || []).map((trade, idx) => (
                    <div key={`${trade.trade_id}-${idx}`} className={`trade-row ${trade.side?.toLowerCase()}`}>
                      <span>{formatTime(trade.timestamp)}</span>
                      <span>{formatPrice(trade.price)}</span>
                      <span>{formatVolume(trade.volume)}</span>
                      <span className={`side ${trade.side?.toLowerCase()}`}>
                        {trade.side || 'N/A'}
                      </span>
                    </div>
                  ))}
                  {!trades[symbol]?.length && (
                    <div className="no-data">Waiting for trades...</div>
                  )}
                </div>
              </div>

              {/* Orderbook Panel */}
              <div className="orderbook-panel">
                <h4>Order Book</h4>
                {orderbooks[symbol] ? (
                  <div className="orderbook">
                    <div className="orderbook-side asks">
                      <h5>Asks</h5>
                      <div className="orderbook-header">
                        <span>Price</span>
                        <span>Volume</span>
                      </div>
                      {(orderbooks[symbol].asks || []).slice(0, 10).reverse().map((ask, idx) => (
                        <div key={`ask-${idx}`} className="orderbook-row ask">
                          <span>{formatPrice(ask[0])}</span>
                          <span>{formatVolume(ask[1])}</span>
                        </div>
                      ))}
                    </div>
                    
                    <div className="spread-indicator">
                      {orderbooks[symbol].asks?.[0] && orderbooks[symbol].bids?.[0] && (
                        <span>
                          Spread: {formatPrice(orderbooks[symbol].asks[0][0] - orderbooks[symbol].bids[0][0])}
                        </span>
                      )}
                    </div>
                    
                    <div className="orderbook-side bids">
                      <h5>Bids</h5>
                      <div className="orderbook-header">
                        <span>Price</span>
                        <span>Volume</span>
                      </div>
                      {(orderbooks[symbol].bids || []).slice(0, 10).map((bid, idx) => (
                        <div key={`bid-${idx}`} className="orderbook-row bid">
                          <span>{formatPrice(bid[0])}</span>
                          <span>{formatVolume(bid[1])}</span>
                        </div>
                      ))}
                    </div>
                  </div>
                ) : (
                  <div className="no-data">Waiting for orderbook data...</div>
                )}
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};

export default RealTimeMarketData;