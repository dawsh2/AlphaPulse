// Type definitions for the dashboard

export interface Trade {
  timestamp: number;
  symbol: string;
  exchange: string;
  price: number;
  volume: number;
  side: 'buy' | 'sell';
  trade_id: string;
}

export interface OrderBookLevel {
  price: number;
  size: number;
}

export interface OrderBook {
  symbol: string;
  exchange: string;
  timestamp: number;
  bids: OrderBookLevel[];
  asks: OrderBookLevel[];
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
  type: 'trade' | 'orderbook' | 'metrics' | 'system' | 'firehose' | 'subscribed';
  data?: any;
  streams?: Record<string, any>;
}

// Type guards for runtime validation
export function isTrade(data: any): data is Trade {
  return data 
    && typeof data.timestamp === 'number'
    && typeof data.symbol === 'string'
    && typeof data.exchange === 'string'
    && typeof data.price === 'number'
    && typeof data.volume === 'number'
    && (data.side === 'buy' || data.side === 'sell')
    && typeof data.trade_id === 'string';
}

export function isOrderBook(data: any): data is OrderBook {
  return data
    && typeof data.symbol === 'string'
    && typeof data.exchange === 'string'
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