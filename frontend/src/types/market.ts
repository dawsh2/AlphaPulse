/**
 * Market Data Types
 */

export type Timeframe = '1m' | '5m' | '15m' | '30m' | '1h' | '4h' | '1d' | '1w';

export interface MarketBar {
  time: number;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
}

export interface MarketData extends MarketBar {
  // Additional fields for enhanced data
  vwap?: number;
  trades?: number;
  turnover?: number;
}

export interface Ticker {
  symbol: string;
  bid: number;
  ask: number;
  last: number;
  volume: number;
  timestamp: number;
}


export interface Trade {
  id: string;
  symbol: string;
  price: number;
  size: number;
  side: 'buy' | 'sell';
  timestamp: number;
}

export interface Symbol {
  symbol: string;
  base: string;
  quote: string;
  exchange: string;
  active: boolean;
  minSize?: number;
  tickSize?: number;
  maker?: number;
  taker?: number;
}

export interface Exchange {
  id: string;
  name: string;
  countries: string[];
  urls: {
    api: string;
    www: string;
    doc: string;
  };
  has: {
    spot: boolean;
    futures: boolean;
    options: boolean;
  };
  timeframes: Timeframe[];
}