import { useState, useEffect, useRef, useCallback } from 'react';
import { resolveSymbolHash, addSymbolMapping } from '../utils/symbolHash';
import type { 
  Trade, 
  OrderBook, 
  L2Delta,
  L2Snapshot,
  SymbolMapping,
  Metrics, 
  SystemStatus,
  StatusUpdate,
  WebSocketMessage
} from '../types';

export function useWebSocketFirehose(endpoint: string) {
  const [trades, setTrades] = useState<Trade[]>([]);
  const [orderbooks, setOrderbooks] = useState<Record<string, OrderBook>>({});
  const [symbolMappings, setSymbolMappings] = useState<Map<string, string>>(new Map());
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
  const [statusUpdate, setStatusUpdate] = useState<StatusUpdate | null>(null);
  const [isConnected, setIsConnected] = useState(false);
  
  const ws = useRef<WebSocket | null>(null);
  const reconnectTimeout = useRef<NodeJS.Timeout>();
  const pingInterval = useRef<NodeJS.Timeout>();
  const lastMessageTime = useRef<number>(Date.now());
  const reconnectAttempt = useRef<number>(0);
  const MAX_TRADES = 2000; // Keep last 2000 trades (increased)
  const MAX_RECONNECT_ATTEMPTS = 50; // Increased reconnection attempts
  const PING_INTERVAL = 30000; // Ping every 30 seconds
  const MESSAGE_TIMEOUT = 60000; // Consider connection dead if no message for 60 seconds

  // Define handlers first, before connect
  const handleTrade = useCallback((trade: Trade) => {
    // Automatically update symbol mapping from trade data
    if (trade.symbol && trade.symbol_hash) {
      setSymbolMappings(prev => new Map(prev.set(trade.symbol_hash, trade.symbol)));
    }
    setTrades(prev => [...prev, trade].slice(-MAX_TRADES));
  }, []);

  const handleOrderbook = useCallback((orderbook: OrderBook) => {
    // Automatically update symbol mapping from orderbook data
    if (orderbook.symbol && orderbook.symbol_hash) {
      setSymbolMappings(prev => new Map(prev.set(orderbook.symbol_hash, orderbook.symbol)));
    }
    setOrderbooks(prev => ({
      ...prev,
      [orderbook.symbol_hash]: orderbook
    }));
  }, []);
  
  const handleSymbolMapping = useCallback((mapping: SymbolMapping) => {
    setSymbolMappings(prev => new Map(prev.set(mapping.symbol_hash, mapping.symbol)));
  }, []);
  
  const handleL2Delta = useCallback((delta: L2Delta) => {
    // Apply L2 delta updates to existing orderbook
    if (delta.symbol_hash && delta.updates && delta.updates.length > 0) {
      setOrderbooks(prev => {
        const existing = prev[delta.symbol_hash];
        
        // If no existing orderbook, create an empty one and let it build from deltas
        const baseOrderbook = existing || {
          symbol_hash: delta.symbol_hash,
          symbol: delta.symbol || resolveSymbolHash(delta.symbol_hash) || `UNKNOWN_${delta.symbol_hash}`,
          timestamp: delta.timestamp || Date.now(),
          bids: [],
          asks: [],
          sequence: delta.sequence
        };

        // Clone existing orderbook
        const updated = { ...baseOrderbook };
        const newBids = [...baseOrderbook.bids];
        const newAsks = [...baseOrderbook.asks];

        // Apply each update
        for (const update of delta.updates) {
          const price = update.price;
          const size = update.size;
          const isBid = update.side === 'bid';
          const levels = isBid ? newBids : newAsks;

          // Find existing level at this price
          const levelIndex = levels.findIndex(level => level.price === price);

          if (update.action === 'delete' || size === 0) {
            // Remove level
            if (levelIndex >= 0) {
              levels.splice(levelIndex, 1);
            }
          } else if (update.action === 'update' || update.action === 'insert') {
            // Update or insert level
            const newLevel = { price, size };
            if (levelIndex >= 0) {
              levels[levelIndex] = newLevel;
            } else {
              // Insert and maintain sorted order
              const insertIndex = levels.findIndex(level => 
                isBid ? level.price < price : level.price > price
              );
              if (insertIndex >= 0) {
                levels.splice(insertIndex, 0, newLevel);
              } else {
                levels.push(newLevel);
              }
            }
          }
        }

        updated.bids = newBids;
        updated.asks = newAsks;
        updated.timestamp = delta.timestamp;

        return {
          ...prev,
          [delta.symbol_hash]: updated
        };
      });
    }
  }, []);
  
  const handleL2Snapshot = useCallback((snapshot: L2Snapshot) => {
    // Convert L2Snapshot to OrderBook format for compatibility
    const orderbook: OrderBook = {
      symbol_hash: snapshot.symbol_hash,
      symbol: snapshot.symbol || resolveSymbolHash(snapshot.symbol_hash) || `UNKNOWN_${snapshot.symbol_hash}`,
      timestamp: snapshot.timestamp,
      bids: snapshot.bids,
      asks: snapshot.asks
    };
    handleOrderbook(orderbook);
  }, [handleOrderbook]);


  const connect = useCallback(() => {
    try {
      // Use new Rust WebSocket bridge service
      const wsBase = import.meta.env.VITE_WS_URL || 'ws://localhost:8080/ws';
      // Connect to dashboard WebSocket endpoint
      const wsUrl = wsBase;
      
      console.log('Connecting to WebSocket:', wsUrl);
      ws.current = new WebSocket(wsUrl);

      ws.current.onopen = () => {
        console.log('WebSocket connected');
        setIsConnected(true);
        reconnectAttempt.current = 0; // Reset reconnect counter on successful connection
        
        // Subscribe to ALL data streams with new Rust bridge format
        setTimeout(() => {
          if (ws.current?.readyState === WebSocket.OPEN) {
            // Subscribe to ALL available channels and symbols
            ws.current.send(JSON.stringify({
              msg_type: 'subscribe',
              channels: ['trades', 'orderbook', 'l2_updates', 'status_updates'],
              symbols: [] // Empty array = subscribe to ALL symbols
            }));
            
            console.log('🔔 Subscribed to ALL WebSocket channels and symbols');
          }
        }, 100);
        
        // Start keepalive monitoring
        const startKeepAlive = () => {
          if (pingInterval.current) {
            clearInterval(pingInterval.current);
          }
          
          pingInterval.current = setInterval(() => {
            const now = Date.now();
            const timeSinceLastMessage = now - lastMessageTime.current;
            
            if (timeSinceLastMessage > MESSAGE_TIMEOUT) {
              console.warn(`⚠️ No WebSocket messages for ${timeSinceLastMessage}ms, reconnecting...`);
              if (ws.current) {
                ws.current.close();
              }
              return;
            }
            
            // Send ping if connection is open
            if (ws.current?.readyState === WebSocket.OPEN) {
              try {
                ws.current.send(JSON.stringify({ msg_type: 'ping' }));
              } catch (error) {
                console.warn('Ping failed, connection may be dead:', error);
              }
            }
          }, PING_INTERVAL);
        };
        
        startKeepAlive();
      };

      ws.current.onmessage = (event) => {
        try {
          // Update last message time for keepalive monitoring
          lastMessageTime.current = Date.now();
          
          const message: any = JSON.parse(event.data);
          // Debug logging removed - messages processed silently
          
          
          // Handle ws-bridge's tagged enum format with msg_type
          if (message.msg_type === 'trade') {
            // Resolve symbol using client-side hash lookup
            const resolvedSymbol = resolveSymbolHash(message.symbol_hash) || message.symbol;
            
            const trade: Trade = {
              timestamp: message.timestamp,
              symbol_hash: message.symbol_hash,
              symbol: resolvedSymbol,
              price: message.price,
              volume: message.volume,
              side: message.side as 'buy' | 'sell',
              latency_collector_to_relay_us: message.latency_collector_to_relay_us,
              latency_relay_to_bridge_us: message.latency_relay_to_bridge_us,
              latency_bridge_to_frontend_us: message.latency_bridge_to_frontend_us,
              latency_total_us: message.latency_total_us
            };
            handleTrade(trade);
          } else if (message.msg_type === 'orderbook') {
            const resolvedSymbol = resolveSymbolHash(message.symbol_hash) || message.symbol;
            
            const orderbook: OrderBook = {
              timestamp: message.timestamp,
              symbol_hash: message.symbol_hash,
              symbol: resolvedSymbol,
              bids: message.data?.bids || [],
              asks: message.data?.asks || []
            };
            handleOrderbook(orderbook);
          } else if (message.msg_type === 'l2_snapshot') {
            console.log('Received L2 snapshot for symbol_hash:', message.symbol_hash);
            const resolvedSymbol = resolveSymbolHash(message.symbol_hash) || message.symbol;
            
            const snapshot: L2Snapshot = {
              symbol_hash: message.symbol_hash,
              symbol: resolvedSymbol,
              timestamp: message.timestamp,
              sequence: message.sequence,
              bids: message.bids || [],
              asks: message.asks || []
            };
            handleL2Snapshot(snapshot);
          } else if (message.msg_type === 'l2_delta') {
            const resolvedSymbol = resolveSymbolHash(message.symbol_hash) || message.symbol;
            
            // Filter out L2 delta messages for exchanges that don't provide L2 data
            // Alpaca provides trade data only, not L2 order book data
            if (resolvedSymbol && (resolvedSymbol.includes('alpaca:') || 
                ['AAPL', 'MSFT', 'GOOGL', 'AMZN', 'TSLA', 'SPY', 'QQQ', 'NVDA', 'META', 'AMD'].includes(resolvedSymbol))) {
              // Silently skip L2 delta for stock symbols from Alpaca
              return;
            }
            
            const delta: L2Delta = {
              symbol_hash: message.symbol_hash,
              symbol: resolvedSymbol,
              timestamp: message.timestamp,
              sequence: message.sequence,
              updates: message.updates || []
            };
            handleL2Delta(delta);
          } else if (message.msg_type === 'symbol_mapping') {
            // Add to client-side symbol cache
            addSymbolMapping(message.symbol_hash, message.symbol);
            
            const mapping: SymbolMapping = {
              symbol_hash: message.symbol_hash,
              symbol: message.symbol
            };
            handleSymbolMapping(mapping);
          } else if (message.msg_type === 'status_update') {
            // Handle status update with gas prices and POL price
            if (message.data) {
              const update: StatusUpdate = {
                gas_price_gwei: message.data.gas_price_gwei || 0,
                gas_price_fast: message.data.gas_price_fast || 0,
                gas_price_instant: message.data.gas_price_instant || 0,
                native_price_usd: message.data.native_price_usd || 0,
                block_number: message.data.block_number || 0,
                timestamp: message.data.timestamp || Date.now()
              };
              setStatusUpdate(update);
              console.log('Received StatusUpdate:', update);
            }
          } else if (message.msg_type === 'pong') {
            // Handle pong response (keepalive)
            console.log('🏓 Received pong, connection alive');
          } else {
            console.log('Unknown message format:', message);
          }
        } catch (error) {
          console.error('Error parsing WebSocket message:', error, event.data);
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
    let isActive = true;
    
    // Delay connection to avoid React StrictMode double execution
    const connectTimer = setTimeout(() => {
      if (isActive) {
        connect();
      }
    }, 100);

    return () => {
      isActive = false;
      clearTimeout(connectTimer);
      if (reconnectTimeout.current) {
        clearTimeout(reconnectTimeout.current);
      }
      if (pingInterval.current) {
        clearInterval(pingInterval.current);
      }
      if (ws.current) {
        ws.current.close();
      }
    };
  }, [connect]);

  return {
    trades,
    orderbooks,
    symbolMappings,
    metrics,
    status,
    statusUpdate,
    isConnected
  };
}