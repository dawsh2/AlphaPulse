/**
 * Data Analysis Service
 * Provides statistical analysis functions for market data
 */

import { StoredMarketData } from '../data/DataTypes';

export interface AnalysisResult {
  correlation?: number;
  autocorrelation?: number[];
  regression?: {
    slope: number;
    intercept: number;
    r2: number;
  };
  statistics?: {
    mean: number;
    std: number;
    skew: number;
    kurtosis: number;
  };
}

export class DataAnalysis {
  /**
   * Calculate returns from price series
   */
  static calculateReturns(data: StoredMarketData[], useLog: boolean = true): number[] {
    if (data.length < 2) return [];
    
    const returns: number[] = [];
    for (let i = 1; i < data.length; i++) {
      if (useLog) {
        returns.push(Math.log(data[i].close / data[i - 1].close));
      } else {
        returns.push((data[i].close - data[i - 1].close) / data[i - 1].close);
      }
    }
    return returns;
  }

  /**
   * Calculate correlation between two series
   */
  static correlation(x: number[], y: number[]): number {
    if (x.length !== y.length || x.length === 0) return NaN;
    
    const n = x.length;
    const meanX = x.reduce((a, b) => a + b, 0) / n;
    const meanY = y.reduce((a, b) => a + b, 0) / n;
    
    let numerator = 0;
    let denomX = 0;
    let denomY = 0;
    
    for (let i = 0; i < n; i++) {
      const dx = x[i] - meanX;
      const dy = y[i] - meanY;
      numerator += dx * dy;
      denomX += dx * dx;
      denomY += dy * dy;
    }
    
    return numerator / Math.sqrt(denomX * denomY);
  }

  /**
   * Calculate autocorrelation for different lags
   */
  static autocorrelation(data: number[], maxLag: number = 20): number[] {
    const autocorr: number[] = [];
    
    for (let lag = 1; lag <= maxLag; lag++) {
      if (lag >= data.length) break;
      
      const x = data.slice(0, -lag);
      const y = data.slice(lag);
      autocorr.push(this.correlation(x, y));
    }
    
    return autocorr;
  }

  /**
   * Simple linear regression
   */
  static linearRegression(x: number[], y: number[]): { slope: number; intercept: number; r2: number } {
    if (x.length !== y.length || x.length === 0) {
      return { slope: NaN, intercept: NaN, r2: NaN };
    }
    
    const n = x.length;
    const sumX = x.reduce((a, b) => a + b, 0);
    const sumY = y.reduce((a, b) => a + b, 0);
    const sumXY = x.reduce((acc, xi, i) => acc + xi * y[i], 0);
    const sumX2 = x.reduce((acc, xi) => acc + xi * xi, 0);
    
    const slope = (n * sumXY - sumX * sumY) / (n * sumX2 - sumX * sumX);
    const intercept = (sumY - slope * sumX) / n;
    
    // Calculate R-squared
    const meanY = sumY / n;
    const ssTotal = y.reduce((acc, yi) => acc + Math.pow(yi - meanY, 2), 0);
    const ssResidual = y.reduce((acc, yi, i) => {
      const predicted = slope * x[i] + intercept;
      return acc + Math.pow(yi - predicted, 2);
    }, 0);
    const r2 = 1 - (ssResidual / ssTotal);
    
    return { slope, intercept, r2 };
  }

  /**
   * Calculate basic statistics
   */
  static basicStats(data: number[]): { mean: number; std: number; skew: number; kurtosis: number } {
    const n = data.length;
    if (n === 0) return { mean: NaN, std: NaN, skew: NaN, kurtosis: NaN };
    
    const mean = data.reduce((a, b) => a + b, 0) / n;
    
    const variance = data.reduce((acc, x) => acc + Math.pow(x - mean, 2), 0) / n;
    const std = Math.sqrt(variance);
    
    const skew = data.reduce((acc, x) => acc + Math.pow((x - mean) / std, 3), 0) / n;
    const kurtosis = data.reduce((acc, x) => acc + Math.pow((x - mean) / std, 4), 0) / n - 3;
    
    return { mean, std, skew, kurtosis };
  }

  /**
   * Convert data to DataFrame-like format for analysis
   */
  static toDataFrame(data: StoredMarketData[]): {
    timestamps: Date[];
    open: number[];
    high: number[];
    low: number[];
    close: number[];
    volume: number[];
    returns: number[];
  } {
    const df = {
      timestamps: [] as Date[],
      open: [] as number[],
      high: [] as number[],
      low: [] as number[],
      close: [] as number[],
      volume: [] as number[],
      returns: [] as number[]
    };
    
    data.forEach((candle, i) => {
      df.timestamps.push(new Date(candle.timestamp * 1000));
      df.open.push(candle.open);
      df.high.push(candle.high);
      df.low.push(candle.low);
      df.close.push(candle.close);
      df.volume.push(candle.volume);
      
      if (i > 0) {
        df.returns.push(Math.log(candle.close / data[i - 1].close));
      } else {
        df.returns.push(0);
      }
    });
    
    return df;
  }

  /**
   * Export data to CSV format
   */
  static toCSV(data: StoredMarketData[]): string {
    const headers = ['timestamp', 'datetime', 'open', 'high', 'low', 'close', 'volume', 'returns'];
    const rows: string[] = [headers.join(',')];
    
    data.forEach((candle, i) => {
      const returns = i > 0 ? Math.log(candle.close / data[i - 1].close) : 0;
      const datetime = new Date(candle.timestamp * 1000).toISOString();
      
      rows.push([
        candle.timestamp,
        datetime,
        candle.open,
        candle.high,
        candle.low,
        candle.close,
        candle.volume,
        returns
      ].join(','));
    });
    
    return rows.join('\n');
  }

  /**
   * Export to NautilusTrader Bar format
   */
  static toNautilusFormat(data: StoredMarketData[], symbol: string): any[] {
    return data.map(candle => ({
      bar_type: `${symbol}-1-MINUTE-LAST-EXTERNAL`,
      timestamp_ns: candle.timestamp * 1_000_000_000, // Convert to nanoseconds
      open: candle.open.toString(),
      high: candle.high.toString(),
      low: candle.low.toString(),
      close: candle.close.toString(),
      volume: candle.volume.toString()
    }));
  }

  /**
   * Rolling correlation between two series
   */
  static rollingCorrelation(x: number[], y: number[], window: number = 20): number[] {
    const correlations: number[] = [];
    
    for (let i = window - 1; i < x.length; i++) {
      const xWindow = x.slice(i - window + 1, i + 1);
      const yWindow = y.slice(i - window + 1, i + 1);
      correlations.push(this.correlation(xWindow, yWindow));
    }
    
    return correlations;
  }

  /**
   * Volatility calculation (annualized)
   */
  static volatility(returns: number[], annualizationFactor: number = 252 * 24 * 60): number {
    const stats = this.basicStats(returns);
    return stats.std * Math.sqrt(annualizationFactor);
  }

  /**
   * Sharpe ratio calculation
   */
  static sharpeRatio(returns: number[], riskFreeRate: number = 0, annualizationFactor: number = 252 * 24 * 60): number {
    const meanReturn = returns.reduce((a, b) => a + b, 0) / returns.length;
    const excessReturn = meanReturn - riskFreeRate / annualizationFactor;
    const vol = this.volatility(returns, 1); // Don't annualize for this calculation
    return (excessReturn * Math.sqrt(annualizationFactor)) / vol;
  }
}

export default DataAnalysis;