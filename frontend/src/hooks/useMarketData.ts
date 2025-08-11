/**
 * useMarketData Hook - Custom hook for market data operations
 * Provides data fetching, caching, and state management
 */
import { useState, useEffect, useCallback } from 'react';
import { dataService, type DataSummary, type QueryResult } from '../services/api/dataService';

interface UseMarketDataResult {
  datasets: DataSummary['symbols'];
  loading: boolean;
  error: string | null;
  refreshDatasets: () => Promise<void>;
  queryData: (query: string) => Promise<QueryResult>;
  queryLoading: boolean;
}

export const useMarketData = (): UseMarketDataResult => {
  const [datasets, setDatasets] = useState<DataSummary['symbols']>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [queryLoading, setQueryLoading] = useState(false);

  const refreshDatasets = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const summary = await dataService.getDataSummary();
      setDatasets(summary.symbols);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch datasets');
      console.error('Error fetching datasets:', err);
    } finally {
      setLoading(false);
    }
  }, []);

  const queryData = useCallback(async (query: string): Promise<QueryResult> => {
    setQueryLoading(true);
    try {
      const result = await dataService.queryData(query);
      return result;
    } catch (err) {
      const error = err instanceof Error ? err.message : 'Query failed';
      throw new Error(error);
    } finally {
      setQueryLoading(false);
    }
  }, []);

  // Load datasets on mount
  useEffect(() => {
    refreshDatasets();
  }, [refreshDatasets]);

  return {
    datasets,
    loading,
    error,
    refreshDatasets,
    queryData,
    queryLoading,
  };
};

interface UseCorrelationResult {
  correlation: number | null;
  loading: boolean;
  error: string | null;
  getCorrelation: (symbol1: string, symbol2: string) => Promise<void>;
  symbol1Stats: Record<string, any> | null;
  symbol2Stats: Record<string, any> | null;
}

export const useCorrelation = (): UseCorrelationResult => {
  const [correlation, setCorrelation] = useState<number | null>(null);
  const [symbol1Stats, setSymbol1Stats] = useState<Record<string, any> | null>(null);
  const [symbol2Stats, setSymbol2Stats] = useState<Record<string, any> | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const getCorrelation = useCallback(async (symbol1: string, symbol2: string) => {
    try {
      setLoading(true);
      setError(null);
      const result = await dataService.getCorrelation(symbol1, symbol2);
      setCorrelation(result.correlation);
      setSymbol1Stats(result.symbol1_stats);
      setSymbol2Stats(result.symbol2_stats);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to get correlation');
      setCorrelation(null);
      setSymbol1Stats(null);
      setSymbol2Stats(null);
    } finally {
      setLoading(false);
    }
  }, []);

  return {
    correlation,
    loading,
    error,
    getCorrelation,
    symbol1Stats,
    symbol2Stats,
  };
};

// Hook for correlation matrix
interface UseCorrelationMatrixResult {
  correlations: Record<string, Record<string, number>> | null;
  statistics: Record<string, any> | null;
  loading: boolean;
  error: string | null;
  getCorrelationMatrix: (symbols: string[], exchange?: string) => Promise<void>;
}

export const useCorrelationMatrix = (): UseCorrelationMatrixResult => {
  const [correlations, setCorrelations] = useState<Record<string, Record<string, number>> | null>(null);
  const [statistics, setStatistics] = useState<Record<string, any> | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const getCorrelationMatrix = useCallback(async (symbols: string[], exchange = 'coinbase') => {
    try {
      setLoading(true);
      setError(null);
      const result = await dataService.getCorrelationMatrix(symbols, exchange);
      setCorrelations(result.correlations);
      setStatistics(result.statistics);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to get correlation matrix');
      setCorrelations(null);
      setStatistics(null);
    } finally {
      setLoading(false);
    }
  }, []);

  return {
    correlations,
    statistics,
    loading,
    error,
    getCorrelationMatrix,
  };
};