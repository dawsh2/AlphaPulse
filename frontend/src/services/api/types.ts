/**
 * API Type Definitions
 * 
 * All types used by the API service layer.
 * These types define the contract between frontend and backend.
 */

// Configuration
export interface ApiConfig {
  baseUrl: string;
  wsUrl: string;
  timeout: number;
  retryAttempts: number;
  retryDelay: number;
}

// Base response types
export interface ApiResponse<T> {
  success: boolean;
  data: T;
  error?: string;
  metadata?: Record<string, any>;
}

export class ApiError extends Error {
  public status: number;
  
  constructor(status: number, message: string) {
    super(message);
    this.status = status;
    this.name = 'ApiError';
  }
}

// Core data structures
export interface AnalysisManifest {
  symbol: string | string[];
  timeframe: '1m' | '5m' | '15m' | '1h' | '1d';
  dateRange: {
    start: string; // ISO 8601
    end: string;
  };
  strategy: {
    type: 'trend_following' | 'mean_reversion' | 'momentum' | 'ml_ensemble' | 'custom';
    version: string;
    parameters: Record<string, any>;
  };
  indicators: string[];
  features: string[];
  hash: string;
}

export interface BacktestResult {
  manifest_hash: string;
  metrics: {
    total_return: number;
    annualized_return: number;
    sharpe_ratio: number;
    sortino_ratio: number;
    max_drawdown: number;
    win_rate: number;
    profit_factor: number;
    total_trades: number;
    avg_trade_return: number;
    calmar_ratio?: number;
    var_95?: number;
    cvar_95?: number;
  };
  equity_curve: Array<{
    timestamp: number;
    value: number;
  }>;
  trades: Trade[];
  signals: Signal[];
  cached: boolean;
  computation_time_ms: number;
  metadata?: {
    data_quality_score?: number;
    market_regime?: string;
    warnings?: string[];
  };
}

export interface Signal {
  timestamp: number;
  symbol: string;
  indicator: string;
  value: number;
  action?: 'buy' | 'sell' | 'hold';
  confidence?: number;
  metadata?: Record<string, any>;
}

export interface Trade {
  id: string;
  entry_time: number;
  exit_time?: number;
  symbol: string;
  side: 'long' | 'short';
  entry_price: number;
  exit_price?: number;
  quantity: number;
  pnl?: number;
  pnl_percentage?: number;
  status: 'open' | 'closed' | 'cancelled';
  fees?: number;
  slippage?: number;
}

// Strategy types
export interface Strategy {
  id: string;
  name: string;
  type: string;
  description?: string;
  code?: string;
  parameters: Record<string, any>;
  created_at: string;
  updated_at: string;
  version: string;
  performance?: {
    sharpe_ratio: number;
    total_return: number;
    win_rate: number;
  };
  is_public: boolean;
  is_template: boolean;
  tags: string[];
}

// Market data types
export interface MarketBar {
  time: number;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
}

// Trading types
export interface Position {
  symbol: string;
  quantity: number;
  side: 'long' | 'short';
  entry_price: number;
  current_price: number;
  market_value: number;
  pnl: number;
  pnl_percentage: number;
  opened_at: string;
}

export interface Order {
  id: string;
  symbol: string;
  side: 'buy' | 'sell';
  quantity: number;
  type: 'market' | 'limit' | 'stop' | 'stop_limit';
  price?: number;
  stop_price?: number;
  status: 'pending' | 'submitted' | 'filled' | 'cancelled' | 'rejected';
  filled_quantity?: number;
  filled_price?: number;
  created_at: string;
  updated_at: string;
}

export interface OrderRequest {
  symbol: string;
  side: 'buy' | 'sell';
  quantity: number;
  type: 'market' | 'limit' | 'stop' | 'stop_limit';
  price?: number;
  stop_price?: number;
  time_in_force?: 'day' | 'gtc' | 'ioc' | 'fok';
}

export interface AccountInfo {
  account_id: string;
  buying_power: number;
  cash: number;
  portfolio_value: number;
  equity: number;
  margin_used: number;
  account_type: 'paper' | 'live';
  broker_name: string;
  market_open: boolean;
  account_status: string;
}

// Template types
export interface ButtonTemplate {
  id: string;
  name: string;
  description: string;
  category: string;
  buttons: Array<{
    label: string;
    action: string;
    parameters?: Record<string, any>;
    code?: string;
    icon?: string;
    color?: string;
  }>;
  created_at: string;
  is_public: boolean;
}

export interface NotebookTemplate {
  id: string;
  name: string;
  description: string;
  cells: Array<{
    type: 'code' | 'markdown';
    content: string;
    metadata?: Record<string, any>;
  }>;
  created_at: string;
  is_public: boolean;
  tags: string[];
}

// Data management types
export interface Dataset {
  id: string;
  name: string;
  description?: string;
  created_at: string;
  updated_at: string;
  rows: number;
  columns: string[];
  size_mb: number;
  format: 'csv' | 'parquet' | 'json';
  metadata?: Record<string, any>;
}

export interface DatasetMetadata {
  name: string;
  description?: string;
  tags?: string[];
  source?: string;
}

// Event types
export interface Event {
  id: string;
  timestamp: number;
  type: 'trade' | 'signal' | 'alert' | 'error' | 'info';
  source: string;
  data: Record<string, any>;
  severity?: 'low' | 'medium' | 'high' | 'critical';
}

// Request parameter types
export interface SignalRequest {
  symbol: string;
  indicators: string[];
  timeframe: string;
  start: string;
  end: string;
}

export interface BacktestParams {
  symbol: string;
  timeframe: string;
  start: string;
  end: string;
  parameters?: Record<string, any>;
}

export interface OrderParams {
  status?: 'open' | 'closed' | 'all';
  limit?: number;
  offset?: number;
}

export interface EventParams {
  types?: string[];
  limit?: number;
  since?: number;
}

export interface CompileResult {
  valid: boolean;
  errors: string[];
  warnings: string[];
  metadata?: {
    required_libraries?: string[];
    estimated_runtime?: number;
  };
}