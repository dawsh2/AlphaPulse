/**
 * Data Service - Handles all data-related API calls to the backend
 * Clean abstraction layer for backend communication
 */

export interface QueryRequest {
  query: string;
}

export interface QueryResult {
  data: any[];
  rows: number;
  columns: string[];
}

export interface CorrelationRequest {
  symbols: string[];
  exchange?: string;
}

export interface CorrelationResult {
  correlation: number | null;
  symbol1_stats: Record<string, any>;
  symbol2_stats: Record<string, any>;
}

export interface DataSummary {
  total_bars: number;
  symbols: Array<{
    symbol: string;
    exchange: string;
    bar_count: number;
    first_bar: string;
    last_bar: string;
  }>;
}

export interface SaveDataRequest {
  symbol: string;
  exchange: string;
  candles: number[][];
  interval: string;
}

class DataService {
  private baseUrl = 'http://localhost:5001';

  /**
   * Get summary of all stored Parquet data
   */
  async getDataSummary(): Promise<DataSummary> {
    const response = await fetch(`${this.baseUrl}/api/data/summary`);
    
    if (!response.ok) {
      throw new Error(`Failed to fetch data summary: ${response.statusText}`);
    }
    
    return response.json();
  }

  /**
   * Execute SQL query on the backend DuckDB
   */
  async queryData(query: string): Promise<QueryResult> {
    const response = await fetch(`${this.baseUrl}/api/data/query`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ query }),
    });

    if (!response.ok) {
      const error = await response.json().catch(() => ({ error: response.statusText }));
      throw new Error(error.error || `Query failed: ${response.statusText}`);
    }

    return response.json();
  }

  /**
   * Get correlation between two symbols
   */
  async getCorrelation(symbol1: string, symbol2: string): Promise<CorrelationResult> {
    // Convert internal format (BTC/USD) to URL format (BTC-USD)
    const urlSymbol1 = symbol1.replace('/', '-');
    const urlSymbol2 = symbol2.replace('/', '-');
    
    const response = await fetch(`${this.baseUrl}/api/data/correlation/${urlSymbol1}/${urlSymbol2}`);
    
    if (!response.ok) {
      throw new Error(`Failed to get correlation: ${response.statusText}`);
    }
    
    return response.json();
  }

  /**
   * Get correlation matrix for multiple symbols
   */
  async getCorrelationMatrix(symbols: string[], exchange = 'coinbase'): Promise<any> {
    const response = await fetch(`${this.baseUrl}/api/analysis/correlation-matrix`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ symbols, exchange }),
    });

    if (!response.ok) {
      const error = await response.json().catch(() => ({ error: response.statusText }));
      throw new Error(error.error || `Correlation matrix failed: ${response.statusText}`);
    }

    return response.json();
  }

  /**
   * Save market data (for caching)
   */
  async saveMarketData(data: SaveDataRequest): Promise<{ status: string; message: string }> {
    const response = await fetch(`${this.baseUrl}/api/market-data/save`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(data),
    });

    if (!response.ok) {
      throw new Error(`Failed to save market data: ${response.statusText}`);
    }

    return response.json();
  }

  /**
   * Proxy request to Coinbase API (with automatic Parquet saving)
   */
  async proxyCoinbase(endpoint: string, params: Record<string, string> = {}): Promise<any> {
    const url = new URL(`${this.baseUrl}/api/proxy/coinbase/${endpoint}`);
    Object.entries(params).forEach(([key, value]) => {
      url.searchParams.append(key, value);
    });

    const response = await fetch(url.toString());

    if (!response.ok) {
      throw new Error(`Coinbase proxy failed: ${response.statusText}`);
    }

    return response.json();
  }

  /**
   * Get catalog of available data files
   */
  async getCatalogData(): Promise<any> {
    const response = await fetch(`${this.baseUrl}/api/catalog/list`);
    
    if (!response.ok) {
      throw new Error(`Failed to fetch catalog: ${response.statusText}`);
    }
    
    return response.json();
  }
}

// Export singleton instance
export const dataService = new DataService();