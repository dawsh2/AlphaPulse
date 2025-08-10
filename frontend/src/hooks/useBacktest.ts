/**
 * Hook for running backtests
 */

import { useState, useCallback } from 'react';
import { AlphaPulseAPI } from '../services/api';
import { generateManifestHash } from '../utils/hash';
import type { AnalysisManifest, BacktestResult } from '../services/api/types';

interface UseBacktestOptions {
  cacheResults?: boolean;
  onProgress?: (progress: number) => void;
}

export function useBacktest(options: UseBacktestOptions = {}) {
  const { cacheResults = true, onProgress } = options;

  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  const [result, setResult] = useState<BacktestResult | null>(null);
  const [progress, setProgress] = useState(0);
  const [cached, setCached] = useState(false);

  const runBacktest = useCallback(async (
    params: Omit<AnalysisManifest, 'hash'>
  ): Promise<BacktestResult | null> => {
    try {
      setLoading(true);
      setError(null);
      setProgress(0);
      setCached(false);

      // Generate manifest with hash
      const hash = await generateManifestHash(params);
      const manifest: AnalysisManifest = {
        ...params,
        hash,
      };

      // Check cache first if enabled
      if (cacheResults) {
        const cacheStatus = await AlphaPulseAPI.analysis.checkCache(hash);
        if (cacheStatus.exists) {
          setCached(true);
          onProgress?.(100);
          setProgress(100);
        }
      }

      // Simulate progress updates (in real app, this would come from WebSocket)
      const progressInterval = setInterval(() => {
        setProgress(prev => {
          const next = Math.min(prev + 10, 90);
          onProgress?.(next);
          return next;
        });
      }, 500);

      // Run analysis
      const backtestResult = await AlphaPulseAPI.analysis.runAnalysis(manifest);
      
      clearInterval(progressInterval);
      setProgress(100);
      onProgress?.(100);
      
      setResult(backtestResult);
      return backtestResult;
    } catch (err) {
      setError(err as Error);
      return null;
    } finally {
      setLoading(false);
    }
  }, [cacheResults, onProgress]);

  const runStrategyBacktest = useCallback(async (
    strategyId: string,
    params: {
      symbol: string;
      timeframe: string;
      start: string;
      end: string;
      parameters?: Record<string, any>;
    }
  ): Promise<BacktestResult | null> => {
    try {
      setLoading(true);
      setError(null);
      setProgress(0);

      const backtestResult = await AlphaPulseAPI.strategies.backtest(
        strategyId,
        params
      );
      
      setResult(backtestResult);
      setProgress(100);
      return backtestResult;
    } catch (err) {
      setError(err as Error);
      return null;
    } finally {
      setLoading(false);
    }
  }, []);

  const clear = useCallback(() => {
    setResult(null);
    setError(null);
    setProgress(0);
    setCached(false);
  }, []);

  return {
    loading,
    error,
    result,
    progress,
    cached,
    runBacktest,
    runStrategyBacktest,
    clear,
  };
}