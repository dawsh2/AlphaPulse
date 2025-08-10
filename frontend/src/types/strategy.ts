/**
 * Strategy Types
 */

export type StrategyType = 
  | 'trend_following'
  | 'mean_reversion'
  | 'momentum'
  | 'breakout'
  | 'pairs_trading'
  | 'arbitrage'
  | 'ml_ensemble'
  | 'market_making'
  | 'custom';

export interface StrategyParameters {
  // Common parameters
  lookback?: number;
  threshold?: number;
  stopLoss?: number;
  takeProfit?: number;
  positionSize?: number;
  
  // Indicator parameters
  rsiPeriod?: number;
  rsiOversold?: number;
  rsiOverbought?: number;
  
  macdFast?: number;
  macdSlow?: number;
  macdSignal?: number;
  
  bbPeriod?: number;
  bbStdDev?: number;
  
  // ML parameters
  features?: string[];
  modelType?: string;
  trainSize?: number;
  
  // Custom parameters
  [key: string]: any;
}

export interface Strategy {
  id: string;
  name: string;
  type: StrategyType;
  description?: string;
  version: string;
  
  // Configuration
  parameters: StrategyParameters;
  symbols?: string[];
  timeframe?: string;
  
  // Code/Logic
  code?: string;
  indicators?: string[];
  entryRules?: Rule[];
  exitRules?: Rule[];
  
  // Metadata
  author?: string;
  created: string;
  updated: string;
  tags: string[];
  
  // Performance
  performance?: StrategyPerformance;
  
  // Sharing
  isPublic: boolean;
  isTemplate: boolean;
  likes?: number;
  uses?: number;
}

export interface Rule {
  id: string;
  type: 'indicator' | 'price' | 'volume' | 'time' | 'custom';
  condition: 'gt' | 'lt' | 'eq' | 'gte' | 'lte' | 'cross_above' | 'cross_below';
  left: string;  // e.g., "RSI"
  right: string | number; // e.g., 30 or "SMA"
  combinator?: 'AND' | 'OR';
}

export interface StrategyPerformance {
  // Returns
  totalReturn: number;
  annualizedReturn: number;
  
  // Risk metrics
  sharpeRatio: number;
  sortinoRatio: number;
  calmarRatio?: number;
  maxDrawdown: number;
  
  // Trade statistics
  winRate: number;
  profitFactor: number;
  totalTrades: number;
  avgWin: number;
  avgLoss: number;
  
  // Other metrics
  expectancy?: number;
  var95?: number;
  cvar95?: number;
}

export interface Backtest {
  id: string;
  strategyId: string;
  
  // Configuration
  symbol: string;
  timeframe: string;
  startDate: string;
  endDate: string;
  initialCapital: number;
  
  // Results
  performance: StrategyPerformance;
  trades: ExecutedTrade[];
  equityCurve: EquityPoint[];
  
  // Metadata
  executionTime: number;
  dataQuality: number;
  warnings?: string[];
}

export interface ExecutedTrade {
  id: string;
  entryTime: number;
  exitTime?: number;
  
  symbol: string;
  side: 'long' | 'short';
  
  entryPrice: number;
  exitPrice?: number;
  
  quantity: number;
  commission: number;
  slippage?: number;
  
  pnl?: number;
  pnlPercent?: number;
  
  entryReason?: string;
  exitReason?: string;
  
  status: 'open' | 'closed' | 'cancelled';
}

export interface EquityPoint {
  timestamp: number;
  equity: number;
  cash: number;
  positions: number;
  drawdown?: number;
}