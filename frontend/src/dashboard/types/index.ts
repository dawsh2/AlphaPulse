// Type definitions for the dashboard - CANONICAL LOCATION

export interface Trade {
  timestamp: number;
  symbol_hash: string; // Changed to string for JavaScript number precision
  symbol?: string; // Human-readable if available
  price: number;
  volume: number;
  side: 'buy' | 'sell';
  // End-to-end latency tracking (all in microseconds)
  latency_collector_to_relay_us?: number;
  latency_relay_to_bridge_us?: number;
  latency_bridge_to_frontend_us?: number;
  latency_total_us?: number;
}

export interface OrderBookLevel {
  price: number;
  size: number;
}

export interface OrderBook {
  symbol_hash: string; // Changed to string for JavaScript number precision
  symbol?: string; // Human-readable if available
  timestamp: number;
  bids: OrderBookLevel[];
  asks: OrderBookLevel[];
}

export interface L2Update {
  side: 'bid' | 'ask';
  price: number;
  size: number;
  action: 'delete' | 'update' | 'insert';
}

export interface L2Delta {
  symbol_hash: string; // Changed to string for JavaScript number precision
  symbol?: string;
  timestamp: number;
  sequence: number;
  updates: L2Update[];
}

export interface L2Snapshot {
  symbol_hash: string; // Changed to string for JavaScript number precision
  symbol?: string;
  timestamp: number;
  sequence: number;
  bids: OrderBookLevel[];
  asks: OrderBookLevel[];
}

export interface SymbolMapping {
  symbol_hash: string; // Changed to string for JavaScript number precision
  symbol: string;
}

export interface Metrics {
  trades_per_second: number;
  orderbook_updates_per_second: number;
  total_trades: number;
  total_orderbook_updates: number;
  latency_ms: number;
  redis_stream_length: number;
  active_connections: number;
}

export interface SystemStatus {
  cpu_percent: number;
  memory_percent: number;
  disk_percent: number;
  network_rx_kb: number;
  network_tx_kb: number;
  uptime_seconds: number;
}

export interface WebSocketMessage {
  msg_type: 'trade' | 'orderbook' | 'l2_snapshot' | 'l2_delta' | 'symbol_mapping' | 'metrics' | 'system' | 'firehose' | 'subscribed';
  data?: any;
  streams?: Record<string, any>;
}

// Type guards for runtime validation
export function isTrade(data: any): data is Trade {
  return data 
    && typeof data.timestamp === 'number'
    && typeof data.symbol_hash === 'string' // Changed to string
    && typeof data.price === 'number'
    && typeof data.volume === 'number'
    && (data.side === 'buy' || data.side === 'sell');
}

export function isOrderBook(data: any): data is OrderBook {
  return data
    && typeof data.symbol_hash === 'string' // Changed to string
    && typeof data.timestamp === 'number'
    && Array.isArray(data.bids)
    && Array.isArray(data.asks);
}

export function isMetrics(data: any): data is Metrics {
  return data
    && typeof data.trades_per_second === 'number'
    && typeof data.orderbook_updates_per_second === 'number'
    && typeof data.latency_ms === 'number';
}

export function isSystemStatus(data: any): data is SystemStatus {
  return data
    && typeof data.cpu_percent === 'number'
    && typeof data.memory_percent === 'number'
    && typeof data.disk_percent === 'number';
}