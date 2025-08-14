import { useState, useEffect, useRef, useCallback } from 'react';
import type { 
  Trade, 
  OrderBook, 
  Metrics, 
  SystemStatus, 
  WebSocketMessage
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


  const connect = useCallback(() => {
    try {
      // Use new Rust WebSocket bridge service
      const wsBase = import.meta.env.VITE_WS_URL || 'ws://localhost:8765';
      // New Rust ws_bridge serves on /stream
      const wsUrl = endpoint === '/ws/firehose' ? `${wsBase}/stream` : `${wsBase}/stream`;
      
      console.log('Connecting to WebSocket:', wsUrl);
      ws.current = new WebSocket(wsUrl);

      ws.current.onopen = () => {
        console.log('WebSocket connected');
        setIsConnected(true);
        reconnectAttempt.current = 0; // Reset reconnect counter on successful connection
        
        // Subscribe to data streams with new Rust bridge format
        setTimeout(() => {
          if (ws.current?.readyState === WebSocket.OPEN) {
            ws.current.send(JSON.stringify({
              msg_type: 'subscribe',
              channels: ['trades', 'orderbook'],
              symbols: ['BTC-USD', 'ETH-USD', 'BTC-USDT', 'ETH-USDT']
            }));
          }
        }, 100);
      };

      ws.current.onmessage = (event) => {
        try {
          const message: any = JSON.parse(event.data);
          console.log('Received WebSocket message:', message.msg_type, message);
          
          switch (message.msg_type) {
            case 'trade':
              // Handle new Rust bridge trade format
              const trade: Trade = {
                timestamp: message.timestamp, // Already in milliseconds
                symbol: message.symbol,
                exchange: message.exchange,
                price: message.price,
                volume: message.volume,
                side: message.side as 'buy' | 'sell',
                trade_id: message.data?.trade_id || String(Date.now())
              };
              handleTrade(trade);
              break;
              
            case 'orderbook':
              // Handle new Rust bridge orderbook format
              const orderbook: OrderBook = {
                timestamp: message.timestamp, // Already in milliseconds
                symbol: message.symbol,
                exchange: message.exchange,
                bids: message.data?.bids || [],
                asks: message.data?.asks || [],
                spread: 0 // Calculate spread if needed
              };
              handleOrderbook(orderbook);
              break;
            default:
              console.log('Unknown message type:', message.msg_type, message);
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
  }, [endpoint, handleTrade, handleOrderbook]);

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