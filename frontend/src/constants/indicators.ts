/**
 * Technical indicator constants
 */

export const INDICATORS = {
  // Trend Indicators
  SMA: {
    name: 'Simple Moving Average',
    category: 'trend',
    params: ['period'],
    defaultParams: { period: 20 },
  },
  EMA: {
    name: 'Exponential Moving Average',
    category: 'trend',
    params: ['period'],
    defaultParams: { period: 20 },
  },
  MACD: {
    name: 'MACD',
    category: 'trend',
    params: ['fast', 'slow', 'signal'],
    defaultParams: { fast: 12, slow: 26, signal: 9 },
  },
  
  // Momentum Indicators
  RSI: {
    name: 'Relative Strength Index',
    category: 'momentum',
    params: ['period'],
    defaultParams: { period: 14 },
  },
  STOCH: {
    name: 'Stochastic Oscillator',
    category: 'momentum',
    params: ['k_period', 'd_period'],
    defaultParams: { k_period: 14, d_period: 3 },
  },
  MOM: {
    name: 'Momentum',
    category: 'momentum',
    params: ['period'],
    defaultParams: { period: 10 },
  },
  
  // Volatility Indicators
  BB: {
    name: 'Bollinger Bands',
    category: 'volatility',
    params: ['period', 'std_dev'],
    defaultParams: { period: 20, std_dev: 2 },
  },
  ATR: {
    name: 'Average True Range',
    category: 'volatility',
    params: ['period'],
    defaultParams: { period: 14 },
  },
  
  // Volume Indicators
  OBV: {
    name: 'On Balance Volume',
    category: 'volume',
    params: [],
    defaultParams: {},
  },
  VWAP: {
    name: 'Volume Weighted Average Price',
    category: 'volume',
    params: [],
    defaultParams: {},
  },
} as const;

export const INDICATOR_CATEGORIES = [
  'trend',
  'momentum',
  'volatility',
  'volume',
] as const;

export type IndicatorType = keyof typeof INDICATORS;
export type IndicatorCategory = typeof INDICATOR_CATEGORIES[number];