/**
 * Analysis Service - Handles statistical analysis and computations
 * Interface to backend analysis endpoints
 */

export interface StatisticsResult {
  symbol: string;
  exchange: string;
  mean_return?: number;
  volatility?: number;
  skewness?: number;
  kurtosis?: number;
  min_return?: number;
  max_return?: number;
  total_bars: number;
  annualized_volatility?: number;
  sharpe_ratio?: number;
}

export interface RiskMetrics {
  symbol: string;
  data_points: number;
  mean_return_annualized: number;
  volatility_annualized: number;
  sharpe_ratio: number;
  sortino_ratio: number;
  var_95: number;
  var_99: number;
  expected_shortfall_95: number;
  expected_shortfall_99: number;
  max_drawdown: number;
  skewness: number;
  kurtosis: number;
  risk_free_rate: number;
}

export interface RollingStats {
  symbol: string;
  window: number;
  data_points: number;
  rolling_stats: {
    rolling_mean: (number | null)[];
    rolling_std: (number | null)[];
    rolling_sharpe: (number | null)[];
    rolling_min: (number | null)[];
    rolling_max: (number | null)[];
    timestamps: number[];
  };
}

export interface BacktestConfig {
  symbol: string;
  exchange?: string;
  strategy_name: string;
  parameters?: Record<string, any>;
  start_date?: string;
  end_date?: string;
  initial_capital?: number;
  commission?: number;
}

export interface BacktestResult {
  symbol: string;
  strategy: Record<string, any>;
  total_return: number;
  annualized_return: number;
  volatility: number;
  sharpe_ratio: number;
  max_drawdown: number;
  total_trades: number;
  data_points: number;
  backtest_period: {
    start?: string;
    end?: string;
  };
}

export interface MarketRegimeAnalysis {
  symbols: string[];
  regime_analysis: Record<string, {
    regime_counts: Record<string, number>;
    current_regime: string;
    data_points: number;
  }>;
  regimes: string[];
}

class AnalysisService {
  private baseUrl = 'http://localhost:5001';

  /**
   * Get basic statistics for a symbol
   */
  async getStatistics(symbol: string, exchange = 'coinbase'): Promise<StatisticsResult> {
    const urlSymbol = symbol.replace('/', '-');
    const response = await fetch(`${this.baseUrl}/api/analysis/statistics/${urlSymbol}?exchange=${exchange}`);
    
    if (!response.ok) {
      throw new Error(`Failed to get statistics: ${response.statusText}`);
    }
    
    const result = await response.json();
    return result.statistics;
  }

  /**
   * Get rolling statistics for a symbol
   */
  async getRollingStatistics(symbol: string, window = 20, exchange = 'coinbase'): Promise<RollingStats> {
    const urlSymbol = symbol.replace('/', '-');
    const response = await fetch(`${this.baseUrl}/api/analysis/rolling-stats/${urlSymbol}?window=${window}&exchange=${exchange}`);
    
    if (!response.ok) {
      throw new Error(`Failed to get rolling statistics: ${response.statusText}`);
    }
    
    return response.json();
  }

  /**
   * Get comprehensive risk metrics for a symbol
   */
  async getRiskMetrics(symbol: string, exchange = 'coinbase', riskFreeRate = 0.02): Promise<RiskMetrics> {
    const urlSymbol = symbol.replace('/', '-');
    const response = await fetch(`${this.baseUrl}/api/analysis/risk-metrics/${urlSymbol}?exchange=${exchange}&risk_free_rate=${riskFreeRate}`);
    
    if (!response.ok) {
      throw new Error(`Failed to get risk metrics: ${response.statusText}`);
    }
    
    return response.json();
  }

  /**
   * Run backtesting analysis on a strategy
   */
  async runBacktest(config: BacktestConfig): Promise<BacktestResult> {
    const response = await fetch(`${this.baseUrl}/api/analysis/backtest`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(config),
    });

    if (!response.ok) {
      const error = await response.json().catch(() => ({ error: response.statusText }));
      throw new Error(error.error || `Backtest failed: ${response.statusText}`);
    }

    return response.json();
  }

  /**
   * Get market regime analysis for multiple symbols
   */
  async getMarketRegime(symbols: string[], exchange = 'coinbase'): Promise<MarketRegimeAnalysis> {
    const response = await fetch(`${this.baseUrl}/api/analysis/market-regime`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ symbols, exchange }),
    });

    if (!response.ok) {
      const error = await response.json().catch(() => ({ error: response.statusText }));
      throw new Error(error.error || `Market regime analysis failed: ${response.statusText}`);
    }

    return response.json();
  }

  /**
   * Calculate local statistics (client-side)
   */
  calculateBasicStats(data: number[]): {
    mean: number;
    std: number;
    min: number;
    max: number;
    count: number;
  } {
    if (data.length === 0) {
      return { mean: 0, std: 0, min: 0, max: 0, count: 0 };
    }

    const mean = data.reduce((sum, val) => sum + val, 0) / data.length;
    const variance = data.reduce((sum, val) => sum + Math.pow(val - mean, 2), 0) / data.length;
    const std = Math.sqrt(variance);
    const min = Math.min(...data);
    const max = Math.max(...data);

    return { mean, std, min, max, count: data.length };
  }

  /**
   * Calculate correlation coefficient between two arrays
   */
  calculateCorrelation(x: number[], y: number[]): number {
    if (x.length !== y.length || x.length === 0) {
      return 0;
    }

    const n = x.length;
    const meanX = x.reduce((sum, val) => sum + val, 0) / n;
    const meanY = y.reduce((sum, val) => sum + val, 0) / n;

    let numerator = 0;
    let sumXSquared = 0;
    let sumYSquared = 0;

    for (let i = 0; i < n; i++) {
      const deltaX = x[i] - meanX;
      const deltaY = y[i] - meanY;
      numerator += deltaX * deltaY;
      sumXSquared += deltaX * deltaX;
      sumYSquared += deltaY * deltaY;
    }

    const denominator = Math.sqrt(sumXSquared * sumYSquared);
    return denominator === 0 ? 0 : numerator / denominator;
  }

  /**
   * Calculate percentage returns from price series
   */
  calculateReturns(prices: number[]): number[] {
    if (prices.length < 2) return [];
    
    const returns: number[] = [];
    for (let i = 1; i < prices.length; i++) {
      returns.push((prices[i] - prices[i - 1]) / prices[i - 1]);
    }
    return returns;
  }

  /**
   * Calculate log returns from price series
   */
  calculateLogReturns(prices: number[]): number[] {
    if (prices.length < 2) return [];
    
    const logReturns: number[] = [];
    for (let i = 1; i < prices.length; i++) {
      logReturns.push(Math.log(prices[i] / prices[i - 1]));
    }
    return logReturns;
  }
}

// Export singleton instance
export const analysisService = new AnalysisService();