// Common types for all exchange integrations

export interface MarketData {
  time: number;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
  signal?: 'buy' | 'sell';
}

export interface ExchangeConfig {
  name: string;
  wsUrl: string;
  restUrl: string;
  symbols: {
    display: string;
    api: string;
  }[];
}

export interface ExchangeService {
  connect(symbol: string, onData: (data: MarketData) => void): WebSocket | null;
  disconnect(): void;
  fetchHistoricalData(symbol: string, limit?: number): Promise<MarketData[]>;
  validateCandle(candle: MarketData): boolean;
}

export type ExchangeType = 'kraken' | 'binance' | 'coinbase';