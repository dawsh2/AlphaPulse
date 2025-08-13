import { useState, useEffect, useRef, useCallback } from 'react';
import type { 
  Trade, 
  OrderBook, 
  Metrics, 
  SystemStatus, 
  WebSocketMessage
} from '../types';
import {
  isTrade,
  isOrderBook,
  isMetrics,
  isSystemStatus
} from '../types';

export function useWebSocketFirehose(endpoint: string) {
  const [trades, setTrades] = useState<Trade[]>([]);
  const [orderbooks, setOrderbooks] = useState<Record<string, OrderBook>>({});
  const [metrics, setMetrics] = useState<Metrics>({
    trades_per_second: 0,
    orderbook_updates_per_second: 0,
    total_trades: 0,
    total_orderbook_updates: 0,
    latency_ms: 0,
    redis_stream_length: 0,
    active_connections: 0
  });
  const [status, setStatus] = useState<SystemStatus>({
    cpu_percent: 0,
    memory_percent: 0,
    disk_percent: 0,
    network_rx_kb: 0,
    network_tx_kb: 0,
    uptime_seconds: 0
  });
  const [isConnected, setIsConnected] = useState(false);
  
  const ws = useRef<WebSocket | null>(null);
  const reconnectTimeout = useRef<NodeJS.Timeout>();
  const reconnectAttempt = useRef<number>(0);
  const MAX_TRADES = 1000; // Keep last 1000 trades
  const MAX_RECONNECT_ATTEMPTS = 10;

  // Define handlers first, before connect
  const handleTrade = useCallback((trade: Trade) => {
    setTrades(prev => [...prev, trade].slice(-MAX_TRADES));
  }, []);

  const handleOrderbook = useCallback((orderbook: OrderBook) => {
    const key = `${orderbook.exchange}:${orderbook.symbol}`;
    setOrderbooks(prev => ({
      ...prev,
      [key]: orderbook
    }));
  }, []);

  const handleFirehoseData = useCallback((streams: any) => {
    // Process Redis Stream data
    Object.entries(streams).forEach(([streamKey, messages]: [string, any]) => {
      if (streamKey.startsWith('trades:')) {
        // Process trade messages
        messages.forEach((msg: any) => {
          if (msg.fields) {
            const trade: Trade = {
              timestamp: parseInt(msg.fields.timestamp) * 1000, // Convert seconds to milliseconds
              symbol: msg.fields.symbol,
              exchange: msg.fields.exchange,
              price: parseFloat(msg.fields.price),
              volume: parseFloat(msg.fields.volume),
              side: msg.fields.side as 'buy' | 'sell',
              trade_id: msg.fields.trade_id
            };
            handleTrade(trade);
          }
        });
      } else if (streamKey.startsWith('orderbook:')) {
        // Process orderbook messages
        messages.forEach((msg: any) => {
          if (msg.fields) {
            const orderbook: OrderBook = {
              timestamp: parseInt(msg.fields.timestamp) * 1000, // Convert seconds to milliseconds
              symbol: msg.fields.symbol,
              exchange: msg.fields.exchange,
              bids: JSON.parse(msg.fields.bids || '[]'),
              asks: JSON.parse(msg.fields.asks || '[]'),
              spread: parseFloat(msg.fields.spread || '0')
            };
            handleOrderbook(orderbook);
          }
        });
      }
    });
  }, [handleTrade, handleOrderbook]);

  const connect = useCallback(() => {
    try {
      // Use Rust API server with shared memory integration
      const wsBase = import.meta.env.VITE_WS_URL || 'ws://localhost:3001';
      // API server uses /ws or /realtime, not /ws/firehose
      const wsUrl = endpoint === '/ws/firehose' ? `${wsBase}/ws` : `${wsBase}${endpoint}`;
      
      console.log('Connecting to WebSocket:', wsUrl);
      ws.current = new WebSocket(wsUrl);

      ws.current.onopen = () => {
        console.log('WebSocket connected');
        setIsConnected(true);
        reconnectAttempt.current = 0; // Reset reconnect counter on successful connection
        
        // Subscribe to all data streams after a small delay to ensure connection is ready
        setTimeout(() => {
          if (ws.current?.readyState === WebSocket.OPEN) {
            ws.current.send(JSON.stringify({
              exchanges: ['coinbase', 'kraken', 'binance'],
              symbols: ['BTC-USD', 'ETH-USD', 'BTC-USDT', 'ETH-USDT'],
              trades: true,
              orderbook: true,
              stats: true
            }));
          }
        }, 100);
      };

      ws.current.onmessage = (event) => {
        try {
          const message: any = JSON.parse(event.data);
          console.log('Received WebSocket message:', message.type, message);
          console.log('Message type comparison:', message.type === 'SystemStats', typeof message.type, message.type.length);
          
          switch (message.type) {
            case 'Trade':
              // Convert Rust Trade to frontend format
              if (message.data) {
                const trade: Trade = {
                  timestamp: message.data.timestamp * 1000, // Convert seconds to milliseconds
                  symbol: message.data.symbol,
                  exchange: message.data.exchange,
                  price: message.data.price,
                  volume: message.data.volume,
                  side: message.data.side === 'buy' ? 'buy' : 'sell',
                  trade_id: message.data.trade_id || String(Date.now())
                };
                handleTrade(trade);
              }
              break;
            case 'OrderBook':
              // Convert Rust OrderBookDelta to frontend format
              if (message.data) {
                const orderbook: OrderBook = {
                  timestamp: message.data.timestamp * 1000,
                  symbol: message.data.symbol,
                  exchange: message.data.exchange,
                  bids: message.data.changes?.filter((c: any) => c.side === 'bid').map((c: any) => ({
                    price: c.price,
                    size: c.volume
                  })) || [],
                  asks: message.data.changes?.filter((c: any) => c.side === 'ask').map((c: any) => ({
                    price: c.price,
                    size: c.volume
                  })) || []
                };
                handleOrderbook(orderbook);
              }
              break;
            case 'SystemStats':
              console.log('Processing SystemStats case', message.data);
              // Handle system stats from Rust WebSocket
              if (message.data) {
                setMetrics(prev => ({
                  ...prev,
                  latency_ms: message.data.latency_us / 1000,
                  active_connections: message.data.active_clients || 0,
                  total_trades: message.data.trades_processed || 0,
                  total_orderbook_updates: message.data.deltas_processed || 0,
                  trades_per_second: 0, // Not available yet
                  orderbook_updates_per_second: 0, // Not available yet
                  redis_stream_length: 0 // Not available yet
                }));
                console.log('Updated metrics with SystemStats');
              }
              break;
            case 'ServiceDiscovery':
              // Handle service discovery info
              if (message.data) {
                console.log('Service Discovery:', message.data);
                setMetrics(prev => ({
                  ...prev,
                  active_connections: message.data.trade_feeds + message.data.delta_feeds
                }));
              }
              break;
            // Legacy cases for backwards compatibility
            case 'trade':
              if (isTrade(message.data)) {
                handleTrade(message.data);
              }
              break;
            case 'orderbook':
              if (isOrderBook(message.data)) {
                handleOrderbook(message.data);
              }
              break;
            case 'metrics':
              if (isMetrics(message.data)) {
                setMetrics(message.data);
              }
              break;
            case 'system':
              if (isSystemStatus(message.data)) {
                setStatus(message.data);
              }
              break;
            case 'firehose':
              // Handle batch updates from Redis Streams
              if (message.streams) {
                handleFirehoseData(message.streams);
              }
              break;
            case 'subscribed':
              console.log('Successfully subscribed to data streams');
              break;
            default:
              console.log('Unknown message type:', message.type, message);
          }
        } catch (error) {
          console.error('Error parsing WebSocket message:', error);
        }
      };

      ws.current.onerror = (error) => {
        console.error('WebSocket error:', error);
      };

      ws.current.onclose = () => {
        console.log('WebSocket disconnected');
        setIsConnected(false);
        
        // Exponential backoff reconnection with maximum attempts
        if (reconnectAttempt.current < MAX_RECONNECT_ATTEMPTS) {
          const delay = Math.min(1000 * Math.pow(2, reconnectAttempt.current), 30000); // Cap at 30 seconds
          reconnectAttempt.current += 1;
          
          console.log(`Attempting to reconnect in ${delay}ms (attempt ${reconnectAttempt.current}/${MAX_RECONNECT_ATTEMPTS})`);
          
          reconnectTimeout.current = setTimeout(() => {
            connect();
          }, delay);
        } else {
          console.error('Maximum reconnection attempts reached. Please refresh the page.');
        }
      };
    } catch (error) {
      console.error('Failed to connect to WebSocket:', error);
    }
  }, [endpoint, handleTrade, handleOrderbook, handleFirehoseData]);

  useEffect(() => {
    connect();

    return () => {
      if (reconnectTimeout.current) {
        clearTimeout(reconnectTimeout.current);
      }
      if (ws.current) {
        ws.current.close();
      }
    };
  }, [connect]);

  return {
    trades,
    orderbooks,
    metrics,
    status,
    isConnected
  };
}