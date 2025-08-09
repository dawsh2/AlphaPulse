// Data types for market data storage

export interface StoredMarketData {
  id?: number; // Auto-incremented by IndexedDB
  symbol: string;
  exchange: string;
  interval: string; // '1m', '5m', '1h', etc.
  timestamp: number; // Unix timestamp in seconds
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
  metadata?: {
    fetchedAt: number;
    source: string;
  };
}

export interface DatasetInfo {
  symbol: string;
  exchange: string;
  interval: string;
  startTime: number;
  endTime: number;
  candleCount: number;
  lastUpdated: number;
}

export interface DataQuery {
  symbol: string;
  exchange?: string;
  interval?: string;
  startTime?: number;
  endTime?: number;
  limit?: number;
}

export interface DataStorageConfig {
  dbName: string;
  version: number;
  maxCandles: number; // Max candles to keep per symbol
}

export const DEFAULT_CONFIG: DataStorageConfig = {
  dbName: 'AlphaPulseMarketData',
  version: 1,
  maxCandles: 50000 // ~35 days of 1-minute data
};