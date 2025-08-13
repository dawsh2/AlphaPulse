/**
 * Shared TypeScript type definitions
 * 
 * These types are used by both frontend and backend (via code generation)
 * Keep them in sync with Python models in python-common/models.py
 */

// Enums
export enum Exchange {
  COINBASE = 'coinbase',
  KRAKEN = 'kraken',
  BINANCE_US = 'binance_us',
  ALPACA = 'alpaca'
}

export enum OrderSide {
  BUY = 'buy',
  SELL = 'sell'
}

// Market Data Types
export interface Trade {
  timestamp: number;
  symbol: string;
  exchange: string;
  price: number;
  volume: number;
  side: OrderSide;
  trade_id: string;
}

export interface OrderBookLevel {
  price: number;
  size: number;
}

export interface OrderBook {
  timestamp: number;
  symbol: string;
  exchange: string;
  bids: OrderBookLevel[];
  asks: OrderBookLevel[];
  sequence?: number;
}

// API Types
export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
  timestamp: number;
}

export interface MarketDataRequest {
  symbol: string;
  exchange: Exchange;
  start_time: string;  // ISO 8601
  end_time: string;    // ISO 8601
  data_type?: 'trades' | 'orderbook' | 'both';
}

// WebSocket Message Types
export interface WebSocketMessage {
  type: 'trade' | 'orderbook' | 'metrics' | 'system' | 'error';
  data: any;
  timestamp?: number;
}

// System Monitoring Types
export interface SystemMetrics {
  cpu_percent: number;
  memory_percent: number;
  disk_percent: number;
  network_rx_kb: number;
  network_tx_kb: number;
  uptime_seconds: number;
}

export interface ServiceHealth {
  service: string;
  status: 'healthy' | 'degraded' | 'down';
  latency_ms?: number;
  error_rate?: number;
  last_check: string;
}