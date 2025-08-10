/**
 * Hook for managing market data subscriptions
 */

import { useState, useEffect, useRef, useCallback } from 'react';
import { AlphaPulseAPI } from '../services/api';
import type { MarketBar } from '../types';

interface UseMarketDataOptions {
  autoConnect?: boolean;
  maxDataPoints?: number;
  throttleMs?: number;
}

export function useMarketData(
  symbol: string,
  timeframe: string = '1m',
  options: UseMarketDataOptions = {}
) {
  const {
    autoConnect = true,
    maxDataPoints = 10000,
    throttleMs = 1000,
  } = options;

  const [data, setData] = useState<MarketBar[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  const [connected, setConnected] = useState(false);
  const [livePrice, setLivePrice] = useState<number | null>(null);

  const wsRef = useRef<WebSocket | null>(null);
  const lastUpdateRef = useRef<number>(0);

  // Load historical data
  const loadHistoricalData = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      
      const historicalData = await AlphaPulseAPI.marketData.getBars(
        symbol,
        timeframe,
        500 // Load last 500 bars
      );
      
      setData(historicalData);
      
      if (historicalData.length > 0) {
        setLivePrice(historicalData[historicalData.length - 1].close);
      }
    } catch (err) {
      setError(err as Error);
    } finally {
      setLoading(false);
    }
  }, [symbol, timeframe]);

  // Connect to WebSocket
  const connect = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      return;
    }

    try {
      const ws = AlphaPulseAPI.marketData.connectLive(
        [symbol],
        (newData) => {
          const now = Date.now();
          
          // Throttle updates
          if (throttleMs && now - lastUpdateRef.current < throttleMs) {
            return;
          }
          lastUpdateRef.current = now;

          // Update live price
          if ('price' in newData) {
            setLivePrice(newData.price);
          }

          // Update or add candle
          if ('time' in newData && 'close' in newData) {
            setData(prev => {
              const updated = [...prev];
              const existingIndex = updated.findIndex(d => d.time === newData.time);
              
              if (existingIndex >= 0) {
                updated[existingIndex] = newData as MarketBar;
              } else {
                updated.push(newData as MarketBar);
                
                // Limit data points
                if (updated.length > maxDataPoints) {
                  updated.shift();
                }
              }
              
              return updated;
            });
          }
        }
      );

      ws.onopen = () => setConnected(true);
      ws.onclose = () => setConnected(false);
      ws.onerror = (err) => setError(new Error('WebSocket error'));

      wsRef.current = ws;
    } catch (err) {
      setError(err as Error);
    }
  }, [symbol, throttleMs, maxDataPoints]);

  // Disconnect from WebSocket
  const disconnect = useCallback(() => {
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
      setConnected(false);
    }
  }, []);

  // Auto-connect on mount if enabled
  useEffect(() => {
    if (autoConnect) {
      loadHistoricalData().then(() => {
        connect();
      });
    }

    return () => {
      disconnect();
    };
  }, [symbol, timeframe, autoConnect]);

  return {
    data,
    loading,
    error,
    connected,
    livePrice,
    connect,
    disconnect,
    refresh: loadHistoricalData,
  };
}