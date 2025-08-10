/**
 * Market constants
 */

export const EXCHANGES = {
  COINBASE: {
    id: 'coinbase',
    name: 'Coinbase',
    websocket: 'wss://ws-feed.exchange.coinbase.com',
    rest: 'https://api.exchange.coinbase.com',
  },
  BINANCE: {
    id: 'binance',
    name: 'Binance',
    websocket: 'wss://stream.binance.com:9443/ws',
    rest: 'https://api.binance.com',
  },
  KRAKEN: {
    id: 'kraken',
    name: 'Kraken',
    websocket: 'wss://ws.kraken.com',
    rest: 'https://api.kraken.com',
  },
  ALPACA: {
    id: 'alpaca',
    name: 'Alpaca',
    websocket: 'wss://stream.data.alpaca.markets',
    rest: 'https://data.alpaca.markets',
  },
} as const;

export const TIMEFRAMES = {
  '1m': { label: '1 Minute', seconds: 60 },
  '5m': { label: '5 Minutes', seconds: 300 },
  '15m': { label: '15 Minutes', seconds: 900 },
  '30m': { label: '30 Minutes', seconds: 1800 },
  '1h': { label: '1 Hour', seconds: 3600 },
  '4h': { label: '4 Hours', seconds: 14400 },
  '1d': { label: '1 Day', seconds: 86400 },
  '1w': { label: '1 Week', seconds: 604800 },
} as const;

export const POPULAR_SYMBOLS = {
  crypto: [
    'BTC/USD',
    'ETH/USD',
    'BNB/USD',
    'SOL/USD',
    'XRP/USD',
    'ADA/USD',
    'DOGE/USD',
    'AVAX/USD',
    'DOT/USD',
    'MATIC/USD',
  ],
  stocks: [
    'AAPL',
    'MSFT',
    'GOOGL',
    'AMZN',
    'META',
    'TSLA',
    'NVDA',
    'JPM',
    'V',
    'JNJ',
  ],
  forex: [
    'EUR/USD',
    'GBP/USD',
    'USD/JPY',
    'AUD/USD',
    'USD/CAD',
    'USD/CHF',
    'NZD/USD',
    'EUR/GBP',
  ],
  commodities: [
    'GOLD',
    'SILVER',
    'OIL',
    'NATGAS',
    'COPPER',
    'WHEAT',
    'CORN',
    'SOYBEANS',
  ],
} as const;

export const MARKET_HOURS = {
  crypto: {
    open: '24/7',
    timezone: 'UTC',
  },
  stocks: {
    premarket: { start: '04:00', end: '09:30' },
    regular: { start: '09:30', end: '16:00' },
    afterhours: { start: '16:00', end: '20:00' },
    timezone: 'America/New_York',
  },
  forex: {
    sydney: { start: '22:00', end: '07:00' },
    tokyo: { start: '00:00', end: '09:00' },
    london: { start: '08:00', end: '17:00' },
    newyork: { start: '13:00', end: '22:00' },
    timezone: 'UTC',
  },
} as const;

export type ExchangeId = keyof typeof EXCHANGES;
export type Timeframe = keyof typeof TIMEFRAMES;
export type MarketType = keyof typeof POPULAR_SYMBOLS;